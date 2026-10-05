//! M12 §38/§60：应用层安全回归矩阵（P0 边界集中回归）。
//! 覆盖：URL/SSF 拒绝、路径/命令验证、元数据 sanitize、播放输入上限、
//! 更新降级拒绝语义引用、诊断不泄密。全部确定性、无网络（§57）。
#![cfg(test)]

use aura_sources::resolver;

fn resolve_err(input: &str) -> String {
    let blocklist =
        std::sync::Arc::new(aura_sources::platform_block::PlatformBlocklist::embedded().unwrap());
    let client = aura_sources::http::HttpClient::new(
        aura_sources::http::HttpConfig {
            policy: aura_sources::url_policy::NetworkPolicy::allow_private(),
            ..aura_sources::http::HttpConfig::default()
        },
        blocklist.clone(),
    )
    .expect("http client");
    match resolver::resolve_input(input, &client, &blocklist) {
        Ok(_) => "RESOLVED".to_string(),
        Err(e) => format!("{e:?}"),
    }
}

/// §38 Network：localhost / 私网特殊端点 / 凭据 URL / 非 http(s) scheme /
/// 超长 URL —— resolver 必须拒绝或按 M4 白名单语义处理。
#[test]
fn network_malicious_inputs_rejected() {
    // 凭据 URL（§38 credential URL）。
    let cred = resolve_err("https://user:pass@example.com/a.mp3");
    assert!(
        cred != "RESOLVED" || cred.contains("credential"),
        "凭据 URL 应显式拒绝: {cred}"
    );
    // 非 http(s) scheme。
    for bad in [
        "ftp://example.com/a.mp3",
        "file:///C:/x.mp3",
        "javascript:alert(1)",
    ] {
        let r = resolve_err(bad);
        assert!(r != "RESOLVED", "{bad} 不应解析成功");
    }
    // 超长 URL（§54 上限语义）。
    let long = format!("http://example.com/{}", "a".repeat(8192));
    let r = resolve_err(&long);
    assert!(r != "RESOLVED", "超长 URL 应拒绝: {r}");
}

/// §38 Filesystem：本地路径穿越/UNC/非法盘符 —— resolver Local 分支
/// 必须存在性/扩展名校验（§192），不存在即拒绝。
#[test]
fn filesystem_malicious_paths_rejected() {
    for bad in [
        "C:\\..\\..\\Windows\\System32\\cmd.exe",
        "C:\\nonexistent\\nope.flac",
        "\\\\evil\\share\\x.mp3",
        "C:\\x.txt",
    ] {
        let r = resolve_err(bad);
        assert!(r != "RESOLVED", "{bad} 不应解析成功");
    }
}

/// §38 Metadata/IPC：sanitize 上限与控制字符（M10 已测，此处回归锚定
/// §60 matrix M12 行）。
#[test]
fn metadata_sanitize_regression() {
    let long = "曲".repeat(1000);
    assert_eq!(
        aura_shell_lib::integration::sanitize(Some(&long), 256)
            .unwrap()
            .chars()
            .count(),
        256
    );
    assert_eq!(
        aura_shell_lib::integration::sanitize(Some("ok\u{7}text"), 64).unwrap(),
        "oktext"
    );
    assert_eq!(aura_shell_lib::integration::sanitize(Some("  "), 10), None);
}

/// §38 Privacy：M10 集成诊断不含 secret 字段（§56 安全事件只记
/// kind/reason/subsystem）。
#[test]
fn integration_diagnostics_no_secrets() {
    let d = aura_shell_lib::integration::IntegrationDiagnostics {
        smtc_available: true,
        smtc_state: "playing".into(),
        last_revision: 3,
        last_error: Some("smtc: hr".into()),
        tray_initialized: true,
    };
    let json = serde_json::to_string(&d).unwrap();
    for bad in ["key", "token", "password", "secret", "credential"] {
        assert!(
            !json.to_lowercase().contains(&format!("\"{bad}")),
            "{bad} 出现"
        );
    }
}
