//! HTTP 客户端（§19–§22/§96）：reqwest blocking + rustls。
//!
//! 安全边界：
//! - 描述性 User-Agent（§20）；
//! - connect/total 超时分离：metadata 请求有总超时，音频流只有
//!   connect 超时 + 消费侧 read 空闲检测（流不能被总超时杀死）；
//! - 重定向 ≤5，每一跳重新走 `validate_url`（scheme/凭据/封锁表/私网，§16）；
//! - 自定义 DNS 解析器：解析结果逐 IP 复检私网策略（§18，防 rebinding）；
//! - 文本响应体大小上限（§21）。

use std::io::Read;
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use crate::error::{short, Result, SourceError};
use crate::platform_block::PlatformBlocklist;
use crate::url_policy::{validate_url, NetworkPolicy};

/// 干净字节流抽象（§122）：M5 cache 层与 HTTP 客户端之间的最小边界。
/// aura-cache 只见本 trait，不直接依赖 reqwest Response 类型。
pub mod byte_stream {
    use std::io::Read;

    /// 可读、可发送、线程安全、可取消（Drop 即断连）的网络字节源。
    pub trait ByteStream: Read + Send + Sync {}

    impl ByteStream for reqwest::blocking::Response {}
}

/// 描述性 UA（§20）：不伪装浏览器、不为空。
pub const USER_AGENT: &str = concat!("ifuyo Aura/", env!("CARGO_PKG_VERSION"));

/// 默认重定向上限（§16）。
pub const MAX_REDIRECTS: usize = 5;

/// 文本响应默认上限：8 MiB（Radio Browser 大搜索结果）。
pub const DEFAULT_TEXT_LIMIT: u64 = 8 << 20;

/// HTTP 客户端配置。
#[derive(Clone)]
pub struct HttpConfig {
    pub policy: NetworkPolicy,
    pub connect_timeout: Duration,
    /// metadata 类请求的总超时（流式请求不适用，见模块注释）。
    pub metadata_timeout: Duration,
    /// 流式读取空闲上限：消费侧检测到无进展超过该时长即取消（§19 read timeout）。
    pub stream_idle_timeout: Duration,
    pub max_redirects: usize,
    /// 测试/高级装配：替换系统 DNS（§125 的 request-spy 注入点）。
    pub dns_override: Option<DnsOverride>,
}

/// DNS 解析函数类型（`host -> IPs`）。
pub type DnsOverride = Arc<dyn Fn(&str) -> std::io::Result<Vec<std::net::IpAddr>> + Send + Sync>;

impl std::fmt::Debug for HttpConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpConfig")
            .field("policy", &self.policy)
            .field("connect_timeout", &self.connect_timeout)
            .field("metadata_timeout", &self.metadata_timeout)
            .field("stream_idle_timeout", &self.stream_idle_timeout)
            .field("max_redirects", &self.max_redirects)
            .field("dns_override", &self.dns_override.is_some())
            .finish()
    }
}

impl Default for HttpConfig {
    fn default() -> Self {
        Self {
            policy: NetworkPolicy::deny_private(),
            connect_timeout: Duration::from_secs(5),
            metadata_timeout: Duration::from_secs(10),
            stream_idle_timeout: Duration::from_secs(20),
            max_redirects: MAX_REDIRECTS,
            dns_override: None,
        }
    }
}

/// 自定义 DNS 解析器（§18）：解析后逐 IP 复检私网策略，
/// 拒绝时直接解析失败 —— hyper 只会拿到已验证的地址。
struct PolicyResolver {
    policy: NetworkPolicy,
    /// `None` = 系统 DNS；注入 = 固定解析结果（测试 spy）。
    fixed: Option<DnsOverride>,
}

impl reqwest::dns::Resolve for PolicyResolver {
    fn resolve(&self, name: reqwest::dns::Name) -> reqwest::dns::Resolving {
        let host = name.as_str().to_string();
        let resolved: std::io::Result<Vec<std::net::IpAddr>> = match &self.fixed {
            Some(f) => f(&host),
            None => {
                use std::net::ToSocketAddrs;
                (host.as_str(), 0)
                    .to_socket_addrs()
                    .map(|it| it.map(|a| a.ip()).collect())
            }
        };
        let out: std::result::Result<Vec<SocketAddr>, SourceError> = match resolved {
            Err(e) => Err(SourceError::SourceUnavailable {
                reason: short(&e.to_string()),
            }),
            Ok(ips) => {
                if ips.is_empty() {
                    Err(SourceError::SourceUnavailable {
                        reason: format!("no addresses for {host}"),
                    })
                } else if !self.policy.allow_private
                    && ips.iter().any(crate::url_policy::is_ip_private)
                {
                    // 拒绝整个解析：不给 hyper 任何可用地址（§18）。
                    Err(SourceError::PrivateNetworkBlocked {
                        host: crate::url_policy::redact_url(&host),
                    })
                } else {
                    Ok(ips.into_iter().map(|ip| SocketAddr::new(ip, 0)).collect())
                }
            }
        };
        Box::pin(async move {
            out.map(|addrs| Box::new(addrs.into_iter()) as reqwest::dns::Addrs)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)
        })
    }
}

/// 共享 HTTP 客户端。构造一次，多来源复用。
#[derive(Clone)]
pub struct HttpClient {
    metadata: reqwest::blocking::Client,
    streaming: reqwest::blocking::Client,
    config: HttpConfig,
    blocklist: Arc<PlatformBlocklist>,
}

impl std::fmt::Debug for HttpClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HttpClient")
            .field("config", &self.config)
            .finish_non_exhaustive()
    }
}

impl HttpClient {
    /// 构建（metadata + streaming 双客户端）。
    ///
    /// # Errors
    /// TLS 后端初始化失败时返回 [`SourceError::NetworkError`]。
    pub fn new(config: HttpConfig, blocklist: Arc<PlatformBlocklist>) -> Result<Self> {
        let resolver = Arc::new(PolicyResolver {
            policy: config.policy,
            fixed: config.dns_override.clone(),
        });
        let build = |total_timeout: Option<Duration>| -> Result<reqwest::blocking::Client> {
            let max_redirects = config.max_redirects;
            let policy_ref = config.policy;
            let blocklist_ref = Arc::clone(&blocklist);
            let mut b = reqwest::blocking::Client::builder()
                .user_agent(USER_AGENT)
                .connect_timeout(config.connect_timeout)
                .dns_resolver(resolver.clone())
                // §18 的私网复检发生在**我们自己的解析器**里；一旦有代理，
                // 目标主机名交给代理去解析，这层检查根本不会被调用——实测
                // 设了 ALL_PROXY/HTTP_PROXY 时 `rebind.test` 直接出了公网
                // （拿到上游 502）。所以环境代理必须显式禁用，而不是默认继承。
                // 代价：确实需要经代理出网的场景不支持，要做得做成显式开关
                // （涉及威胁模型，改动需写进 DECISIONS）。
                .no_proxy()
                .redirect(reqwest::redirect::Policy::custom(move |attempt| {
                    // §16：每一跳全部重新校验（首个 URL 合法 ≠ 目的地合法）。
                    if attempt.previous().len() >= max_redirects {
                        return attempt.error(SourceError::RedirectLimit);
                    }
                    match validate_url(attempt.url().as_str(), &policy_ref, &blocklist_ref) {
                        Ok(_) => attempt.follow(),
                        Err(e) => attempt.error(e),
                    }
                }))
                .tcp_keepalive(Duration::from_secs(15));
            b = match total_timeout {
                Some(t) => b.timeout(t),
                None => b,
            };
            b.build().map_err(|e| SourceError::NetworkError {
                reason: short(&e.to_string()),
            })
        };
        Ok(Self {
            metadata: build(Some(config.metadata_timeout))?,
            streaming: build(None)?,
            config,
            blocklist,
        })
    }

    #[must_use]
    pub fn config(&self) -> &HttpConfig {
        &self.config
    }

    #[must_use]
    pub fn blocklist(&self) -> &PlatformBlocklist {
        &self.blocklist
    }

    /// 校验 URL（所有请求的统一入口校验，§14）。
    ///
    /// # Errors
    /// 见 [`crate::url_policy::validate_url`]。
    pub fn validate(&self, raw: &str) -> Result<url::Url> {
        validate_url(raw, &self.config.policy, &self.blocklist)
    }

    /// GET 文本（JSON/RSS/playlist）：总超时 + 大小上限（§21/§50）。
    ///
    /// # Errors
    /// 网络/状态码/超限/重定向策略拒绝。
    pub fn get_text(&self, url: &url::Url, max_bytes: u64) -> Result<String> {
        let resp = self
            .metadata
            .get(url.clone())
            .send()
            .map_err(SourceError::from)?;
        check_status(resp.status())?;
        let len_hint = resp.content_length().unwrap_or(0);
        if len_hint > max_bytes {
            return Err(SourceError::BodyTooLarge {
                limit_bytes: max_bytes,
            });
        }
        let mut buf: Vec<u8> = Vec::with_capacity(len_hint.min(1 << 20) as usize);
        resp.take(max_bytes.saturating_add(1))
            .read_to_end(&mut buf)
            .map_err(SourceError::from)?;
        if buf.len() as u64 > max_bytes {
            return Err(SourceError::BodyTooLarge {
                limit_bytes: max_bytes,
            });
        }
        // §34/§80：UTF-8 安全转换；无效字节是解析层的问题，不 panic。
        String::from_utf8(buf).map_err(|_| SourceError::ParseError {
            reason: "response is not valid UTF-8".into(),
        })
    }

    /// GET 音频流（不设总超时；空闲检测由消费侧执行，§19/§22）。
    ///
    /// # Errors
    /// 网络/状态码/重定向策略拒绝。
    pub fn get_stream(&self, url: &url::Url) -> Result<reqwest::blocking::Response> {
        self.get_stream_with_headers(url, &[])
    }

    /// GET 流（装箱字节源；M5 cache 层经此消费响应，§122：
    /// aura-cache 不直接依赖 reqwest Response 类型）。
    ///
    /// # Errors
    /// 同 [`HttpClient::get_stream`]。
    pub fn get_stream_boxed(
        &self,
        url: &url::Url,
        headers: &[(&str, String)],
    ) -> Result<Box<dyn byte_stream::ByteStream>> {
        let resp = self.get_stream_with_headers(url, headers)?;
        Ok(Box::new(resp) as Box<dyn byte_stream::ByteStream>)
    }

    /// 带头的 metadata GET（条件 revalidation 用，§40–§42）。
    ///
    /// # Errors
    /// 同 [`HttpClient::get_stream`]。
    pub fn metadata_get_with_headers(
        &self,
        url: &url::Url,
        headers: &[(&str, String)],
    ) -> Result<reqwest::blocking::RequestBuilder> {
        let mut req = self.metadata.get(url.clone());
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        Ok(req)
    }

    /// 同 [`HttpClient::get_stream`]，附加请求头（如 Range、Icy-MetaData）。
    ///
    /// # Errors
    /// 同 [`HttpClient::get_stream`]。
    pub fn get_stream_with_headers(
        &self,
        url: &url::Url,
        headers: &[(&str, String)],
    ) -> Result<reqwest::blocking::Response> {
        let mut req = self.streaming.get(url.clone());
        for (k, v) in headers {
            req = req.header(*k, v);
        }
        let resp = req.send().map_err(SourceError::from)?;
        check_status(resp.status())?;
        Ok(resp)
    }
}

fn check_status(status: reqwest::StatusCode) -> Result<()> {
    if status.is_success() {
        return Ok(());
    }
    if status.is_redirection() {
        // 重定向被策略耗尽后，最后响应可能是 3xx。
        return Err(SourceError::RedirectLimit);
    }
    Err(SourceError::InvalidResponse {
        reason: format!("HTTP {status}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_agent_shape() {
        assert!(USER_AGENT.starts_with("ifuyo Aura/"));
        assert!(!USER_AGENT.contains("Mozilla"), "不得伪装浏览器");
        assert!(!USER_AGENT.contains("curl"));
    }

    #[test]
    fn client_builds_with_default_config() {
        let client = HttpClient::new(
            HttpConfig::default(),
            Arc::new(PlatformBlocklist::embedded().unwrap()),
        );
        assert!(client.is_ok());
    }
}
