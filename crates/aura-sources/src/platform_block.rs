//! 不支持平台注册表（§3）：`config/unsupported_platforms.json` 为唯一事实。
//! 命中即拒绝 —— 不得发出任何网络请求（DNS/HTTP/redirect 都不允许，§125）。

use serde::Deserialize;

use crate::error::{Result, SourceError};

/// 编译期内嵌注册表（仓库 `config/unsupported_platforms.json`），
/// 运行时可整体替换（测试/未来用户配置）。
pub const EMBEDDED_PLATFORMS_JSON: &str =
    include_str!("../../../config/unsupported_platforms.json");

#[derive(Debug, Clone, Deserialize)]
struct PlatformsDoc {
    policy: PolicyDoc,
    platforms: Vec<PlatformEntry>,
}

#[derive(Debug, Clone, Deserialize)]
struct PolicyDoc {
    // on_match 语义固定为 reject_without_request（§3）；字段仅为文档自校验保留。
    #[serde(default)]
    #[allow(dead_code)]
    on_match: String,
    #[serde(default)]
    user_message: String,
}

#[derive(Debug, Clone, Deserialize)]
struct PlatformEntry {
    id: String,
    #[serde(default)]
    #[allow(dead_code)]
    name: String,
    domains: Vec<String>,
}

/// 封锁表。命中匹配在 resolve 阶段、任何网络动作之前执行。
#[derive(Debug, Clone)]
pub struct PlatformBlocklist {
    platforms: Vec<PlatformEntry>,
    user_message: String,
}

impl PlatformBlocklist {
    /// 内嵌注册表（编译期固化）。
    ///
    /// # Errors
    /// 内嵌 JSON 损坏（理论不可能；仓库 CI 测试锁定）。
    pub fn embedded() -> Result<Self> {
        Self::from_json(EMBEDDED_PLATFORMS_JSON)
    }

    /// 从 JSON 构建（测试/替换用）。
    ///
    /// # Errors
    /// JSON 非法时返回 [`SourceError::ParseError`]。
    pub fn from_json(json: &str) -> Result<Self> {
        let doc: PlatformsDoc =
            serde_json::from_str(json).map_err(|e| SourceError::ParseError {
                reason: format!(
                    "unsupported_platforms.json: {}",
                    crate::error::short(&e.to_string())
                ),
            })?;
        let user_message = if doc.policy.user_message.is_empty() {
            "该平台不受支持，请使用本地文件或开放来源。".to_string()
        } else {
            doc.policy.user_message
        };
        Ok(Self {
            platforms: doc.platforms,
            user_message,
        })
    }

    /// 命中检查：host 精确等于注册域名或是其子域（大小写不敏感）。
    #[must_use]
    pub fn match_host(&self, host: &str) -> Option<&str> {
        let host = host.to_ascii_lowercase();
        let host = host.strip_suffix('.').unwrap_or(&host);
        self.platforms.iter().find_map(|p| {
            p.domains
                .iter()
                .any(|d| {
                    let d = d.trim_start_matches('.').to_ascii_lowercase();
                    host == d
                        || host
                            .strip_suffix(&d)
                            .is_some_and(|rest| rest.ends_with('.'))
                })
                .then_some(p.id.as_str())
        })
    }

    /// 命中时的用户可读消息（来自注册表 policy）。
    #[must_use]
    pub fn user_message(&self) -> &str {
        &self.user_message
    }

    /// 命中并构造拒绝错误（§3 的固定出口）。
    pub fn reject_host(&self, host: &str) -> SourceError {
        let platform = self.match_host(host).unwrap_or("unknown").to_string();
        SourceError::UnsupportedPlatform {
            platform,
            message: self.user_message.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_registry_loads_and_has_charter_platforms() {
        let bl = PlatformBlocklist::embedded().unwrap();
        for host in [
            "youtube.com",
            "www.youtube.com",
            "youtu.be",
            "open.spotify.com",
            "music.163.com",
            "y.qq.com",
            "music.apple.com",
            "tidal.com",
            "b23.tv",
        ] {
            assert!(bl.match_host(host).is_some(), "{host} 应被封锁");
        }
    }

    #[test]
    fn similar_but_unrelated_hosts_not_blocked() {
        let bl = PlatformBlocklist::embedded().unwrap();
        // 前缀歧义不算子域（notyoutube.com 不含 youtube.com 作为域后缀）。
        assert!(bl.match_host("notyoutube.com").is_none());
        assert!(bl.match_host("example.com").is_none());
        assert!(bl.match_host("spotify.com.evil.test").is_none());
    }

    #[test]
    fn reject_uses_registry_message() {
        let bl = PlatformBlocklist::embedded().unwrap();
        let err = bl.reject_host("music.163.com");
        match err {
            SourceError::UnsupportedPlatform { platform, message } => {
                assert_eq!(platform, "netease-music");
                assert!(message.contains("本地文件"));
            }
            other => panic!("wrong error: {other:?}"),
        }
    }

    #[test]
    fn case_and_trailing_dot_insensitive() {
        let bl = PlatformBlocklist::embedded().unwrap();
        assert!(bl.match_host("MUSIC.163.COM.").is_some());
    }

    #[test]
    fn malformed_json_is_parse_error() {
        assert!(PlatformBlocklist::from_json("{ not json").is_err());
    }
}
