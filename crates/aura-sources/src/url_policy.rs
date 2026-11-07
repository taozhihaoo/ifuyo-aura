//! URL 校验与私网策略（§14/§15/§17/§18）。
//! 每次网络动作（初始请求、每一跳重定向、HLS segment——未来）都必须重过这里。

use std::net::IpAddr;

use crate::error::{Result, SourceError};
use crate::platform_block::PlatformBlocklist;

/// 允许的 scheme（§14：只允许 http/https）。
pub const ALLOWED_SCHEMES: [&str; 2] = ["http", "https"];

/// 私网/本地地址策略（§17）：默认拒绝；用户明确确认局域网来源后由应用层放开。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NetworkPolicy {
    /// `true` = 允许访问私网/本地地址（用户一次性确认后）。
    pub allow_private: bool,
}

impl NetworkPolicy {
    #[must_use]
    pub const fn deny_private() -> Self {
        Self {
            allow_private: false,
        }
    }

    #[must_use]
    pub const fn allow_private() -> Self {
        Self {
            allow_private: true,
        }
    }
}

/// URL 全面校验（§13 resolver order 的第 2 步，先于一切网络动作）：
/// parse → scheme 白名单 → authority 完整性 → 凭据拒绝（§15）→
/// 封锁表（§3，先于网络）→ 私网策略（§17）。
///
/// # Errors
/// 各失败原因见 [`SourceError`] 变体。
pub fn validate_url(
    raw: &str,
    policy: &NetworkPolicy,
    blocklist: &PlatformBlocklist,
) -> Result<url::Url> {
    let parsed = url::Url::parse(raw.trim()).map_err(|_| SourceError::InvalidUrl {
        reason: "not a valid URL".into(),
    })?;

    let scheme = parsed.scheme().to_ascii_lowercase();
    if !ALLOWED_SCHEMES.contains(&scheme.as_str()) {
        return Err(SourceError::InvalidUrl {
            reason: format!("scheme {scheme:?} not allowed (http/https only)"),
        });
    }

    let Some(host) = parsed.host_str() else {
        return Err(SourceError::InvalidUrl {
            reason: "missing host".into(),
        });
    };
    if host.is_empty() {
        return Err(SourceError::InvalidUrl {
            reason: "empty host".into(),
        });
    }

    // §15：带凭据的 URL 默认拒绝，不进日志/UI/存储。
    if !parsed.username().is_empty() || parsed.password().is_some() {
        return Err(SourceError::InvalidUrl {
            reason: "credential-bearing URLs are rejected".into(),
        });
    }

    // §3：封锁平台在一切网络动作之前拒绝。
    if let Some(err) = blocklist_reject(host, blocklist) {
        return Err(err);
    }

    // §17：私网/本地地址。
    if is_private_host(host) && !policy.allow_private {
        return Err(SourceError::PrivateNetworkBlocked {
            host: host.to_string(),
        });
    }

    Ok(parsed)
}

fn blocklist_reject(host: &str, blocklist: &PlatformBlocklist) -> Option<SourceError> {
    blocklist
        .match_host(host)
        .map(|_| blocklist.reject_host(host))
}

/// host 字符串级私网判定（§18 的第一道：IP 字面量 + 常见本地名）。
/// DNS 解析后的 IP 复检见 `is_ip_allowed`（http 客户端的自定义解析器）。
#[must_use]
pub fn is_private_host(host: &str) -> bool {
    let host = host.trim_end_matches('.');
    let lower = host.to_ascii_lowercase();
    if lower == "localhost" || lower.ends_with(".localhost") || lower.ends_with(".local") {
        return true;
    }
    if let Some(ip_str) = lower.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
        return parse_ip_private(ip_str);
    }
    parse_ip_private(host)
}

fn parse_ip_private(s: &str) -> bool {
    match s.parse::<IpAddr>() {
        Ok(ip) => is_ip_private(&ip),
        Err(_) => false, // 域名：字符串层不判私网（IP 级在 DNS 解析后复检）
    }
}

/// IP 级私网判定（§17/§18）。
#[must_use]
pub fn is_ip_private(ip: &IpAddr) -> bool {
    match ip {
        IpAddr::V4(v4) => {
            let o = v4.octets();
            matches!(
                o,
                [0, ..]                                   // 0.0.0.0/8 "this network"
                | [10, ..]                                // 10/8
                | [127, ..]                               // loopback
                | [169, 254, ..]                          // link-local
                | [172, 16..=31, ..]                      // 172.16/12
                | [192, 168, ..]                          // 192.168/16
                | [100, 64..=127, ..]                     // CGNAT 100.64/10
                | [198, 18..=19, ..]                      // benchmark
                | [224..=239, ..]                         // multicast
                | [240..=255, ..] // reserved/broadcast
            )
        }
        IpAddr::V6(v6) => {
            // IPv4-mapped / IPv4-compatible 形式把 IPv4 地址塞进 IPv6 字面量
            // （例如 `::ffff:127.0.0.1`）。它的 seg[0] 是 0，既不是 ::1 也
            // 不落在 fc00::/7，直接放行会让 §17/§18 的私网门禁被绕过
            // （SSRF）。提取内嵌 IPv4 后递归走同一套 V4 判定。
            if let Some(v4) = v6.to_ipv4() {
                return is_ip_private(&IpAddr::V4(v4));
            }
            let seg = v6.segments();
            v6.is_loopback()
                || v6.is_unspecified()
                || (seg[0] & 0xfe00) == 0xfc00   // fc00::/7 unique local
                || (seg[0] & 0xffc0) == 0xfe80   // fe80::/10 link local
                || (seg[0] & 0xff00) == 0xff00 // multicast
                // 可封装 IPv4 的过渡前缀：保守拒绝，避免把内网地址藏进
                // 隧道地址绕过复检（正常公网 IPv6 不使用这些前缀）。
                || seg[0] == 0x2002                              // 6to4 2002::/16
                || (seg[0] == 0x2001 && seg[1] == 0x0000)       // Teredo 2001:0::/32
                || (seg[0] == 0x0064 && seg[1] == 0xff9b && seg[2] == 0x0000) // NAT64
        }
    }
}

/// 日志用的 URL 脱敏（§96）：scheme://host/path，去 query/fragment/userinfo。
#[must_use]
pub fn redact_url(raw: &str) -> String {
    match url::Url::parse(raw) {
        Ok(u) => {
            let host = u.host_str().unwrap_or("");
            let port = u.port().map(|p| format!(":{p}")).unwrap_or_default();
            let path = {
                let p = u.path();
                if p.is_empty() {
                    "/".to_string()
                } else {
                    p.to_string()
                }
            };
            format!("{}://{}{}{}", u.scheme(), host, port, path)
        }
        Err(_) => "<unparsed url>".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bl() -> PlatformBlocklist {
        PlatformBlocklist::embedded().unwrap()
    }

    fn ok(raw: &str) -> url::Url {
        validate_url(raw, &NetworkPolicy::deny_private(), &bl()).unwrap()
    }

    fn err(raw: &str) -> SourceError {
        validate_url(raw, &NetworkPolicy::deny_private(), &bl()).unwrap_err()
    }

    /// §14：scheme 白名单。
    #[test]
    fn schemes_whitelisted() {
        assert_eq!(ok("http://example.com/a.mp3").scheme(), "http");
        assert_eq!(ok("https://example.com/a.mp3").scheme(), "https");
        for bad in [
            "ftp://example.com/a",
            "file:///C:/music/a.flac",
            "data:audio/mpeg;base64,AAAA",
            "javascript:alert(1)",
            "blob:https://example.com/x",
            "ws://example.com/",
            "wss://example.com/",
            "gopher://example.com/",
        ] {
            let e = err(bad);
            assert!(matches!(e, SourceError::InvalidUrl { .. }), "{bad}: {e:?}");
        }
    }

    #[test]
    fn malformed_authority_rejected() {
        // 注：`https:///path` 会被 url crate 规范化为 host="path"（WHATWG 行为），
        // 仍是合法 http(s) URL，走完整校验链，不构成绕过。
        for bad in ["http://", "not a url", "http:// ", "", "https://"] {
            assert!(
                matches!(err(bad), SourceError::InvalidUrl { .. }),
                "{bad:?}"
            );
        }
    }

    /// §15：credential URL 拒绝。
    #[test]
    fn credential_urls_rejected() {
        let e = err("https://user:password@example.com/audio.mp3");
        assert!(matches!(e, SourceError::InvalidUrl { .. }));
        let e = err("https://api:key@example.com/x");
        assert!(matches!(e, SourceError::InvalidUrl { .. }));
        let e = err("https://user@example.com/x");
        assert!(matches!(e, SourceError::InvalidUrl { .. }));
    }

    /// §3：封锁平台先于一切网络动作。
    #[test]
    fn blocked_platform_rejected() {
        let e = err("https://www.youtube.com/watch?v=x");
        assert!(matches!(e, SourceError::UnsupportedPlatform { .. }));
        let e = err("http://music.163.com/song");
        assert!(matches!(e, SourceError::UnsupportedPlatform { .. }));
    }

    /// §17：私网默认拒绝；显式放开后允许（局域网自建服务器）。
    #[test]
    fn private_network_policy() {
        for host in [
            "127.0.0.1",
            "10.1.2.3",
            "192.168.1.10",
            "172.16.0.1",
            "172.31.255.255",
            "169.254.1.1",
            "[::1]",
            "[fe80::1]",
            "[fd12::1]",
            "localhost",
            "nas.local",
            "myserver.localhost",
            "100.64.0.1",
            "0.0.0.0",
            "224.0.0.1",
        ] {
            let raw = format!("http://{host}:8000/stream");
            let e = err(&raw);
            assert!(
                matches!(e, SourceError::PrivateNetworkBlocked { .. }),
                "{host}: {e:?}"
            );
        }
        // 172.32 不在 172.16/12。
        assert!(validate_url("http://172.32.0.1/s", &NetworkPolicy::deny_private(), &bl()).is_ok());
        // 用户确认后允许。
        let u = validate_url(
            "http://192.168.1.10:8384/stream",
            &NetworkPolicy::allow_private(),
            &bl(),
        )
        .unwrap();
        assert_eq!(u.host_str(), Some("192.168.1.10"));
    }

    /// §18 的字符串层不误伤公网域名（IP 复检在解析器）。
    #[test]
    fn public_domains_pass() {
        assert!(ok("https://example.com/stream.mp3").host_str().is_some());
        assert_eq!(
            ok("https://stream.radio.example:8000/live").port(),
            Some(8000)
        );
    }

    #[test]
    fn redaction_strips_query_and_userinfo() {
        let r = redact_url("https://user:pw@example.com/path?a=secret&token=x#frag");
        assert_eq!(r, "https://example.com/path");
        assert!(!r.contains("secret"));
    }

    /// §18 SSRF 回归：IPv4-mapped / IPv4-compatible IPv6 字面量必须按内嵌
    /// IPv4 判定，不能因为 seg[0]==0 就绕过私网门禁。
    #[test]
    fn ipv4_mapped_ipv6_privates_are_rejected() {
        for ip in [
            "::ffff:127.0.0.1",
            "::ffff:10.0.0.1",
            "::ffff:192.168.1.1",
            "::ffff:169.254.169.254",
            "::127.0.0.1",
        ] {
            let parsed: std::net::IpAddr = ip.parse().expect("parse ip");
            assert!(is_ip_private(&parsed), "{ip} must be treated as private");
        }
        // URL host 字符串层（带方括号）同样拒绝。
        assert!(is_private_host("[::ffff:127.0.0.1]"));
    }

    /// 公网 mapped 地址不能误伤。
    #[test]
    fn ipv4_mapped_ipv6_public_still_allowed() {
        let parsed: std::net::IpAddr = "::ffff:8.8.8.8".parse().expect("parse ip");
        assert!(!is_ip_private(&parsed));
    }

    /// 可封装 IPv4 的过渡前缀保守拒绝，防止内网地址藏进隧道。
    #[test]
    fn ipv4_tunnel_prefixes_are_rejected() {
        for ip in [
            "2002:0a00:0001::1",    // 6to4
            "2001:0000:1234::1",    // Teredo
            "64:ff9b::192.168.1.1", // NAT64
        ] {
            let parsed: std::net::IpAddr = ip.parse().expect("parse ip");
            assert!(is_ip_private(&parsed), "{ip} must be rejected");
        }
    }
}
