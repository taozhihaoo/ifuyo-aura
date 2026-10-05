# M12 Security Threat Model（§5/§50）

> 信任边界与资产分类。标记：`UNTRUSTED`（外部输入）/ `TRUSTED`（应用内
> 权威）/ `PRIVILEGED`（高权限能力）/ `NETWORK`（出网面）/ `USER DATA`
>（用户数据）。

## 架构与边界（§50）

```text
React / WebView [UNTRUSTED 输入面]
       │ Typed IPC（tauri 宏 = 类型化反序列化；50 个命令全部强类型参数）
       ▼
Tauri Commands [PRIVILEGED 边界：唯一高权限入口]
       │
       ├── Library (SQLite)  [USER DATA；参数化 SQL §372]
       ├── Playback Authority [TRUSTED；M10 §51 不得破坏]
       ├── Sources / Cache   [NETWORK；SSRF 策略 M4/§65]
       ├── Analysis          [USER DATA 派生]
       ├── Dreamscape        [本地程序化；零网络]
       ├── Music Map         [derived data]
       ├── Windows Integration [PRIVILEGED：SMTC/托盘]
       └── Updater           [PRIVILEGED + NETWORK；签名验证 §64]
```

原则：**Frontend 不拥有 application authority**（§50）——所有播放/
库/更新决定都在 Rust 应用层。

## 信任边界与威胁面（STRIDE 摘要）

| 边界 | 输入 | 威胁 | 缓解 | 验证 |
| --- | --- | --- | --- | --- |
| WebView→IPC | typed 命令参数 | 权限提升/注入 | 50 命令全强类型；capabilities 最小集 §62；无 `*` 授权 | tests/security_m12 + 人工审计 |
| 网络→Resolver | URL/ICY/流字节 | SSRF/凭据泄露/重定向 | M4 scheme 白名单+封锁表+私网策略+凭据拒绝 | aura-sources url_policy 测试+security_m12 |
| 流→Metadata/Artwork | 标签/封面字节 | 崩溃/超大资源 | normalize+limit（M3）；sanitize（M10 §133） | sanitize 测试 |
| 远程→Updater | manifest/artifact | 伪造更新/RCE | minisign 签名验证（插件 Rust 侧）+版本比较+HTTPS-only（prod） | M11 dry-run+降级拒绝单测 |
| 文件系统 | 用户路径 | 穿越/任意读写 | M3 identity（路径非身份）；artwork 仅 app_data 字节流（§236 无用户路径注入） | installer/smoke+路径拒绝测试 |
| 第二实例 | argv | 任意命令 | 仅 focus，argv 丢弃（§231） | 单测 |
| SMTC/系统 | 媒体按钮/seek | 伪造事件 | 参数化进 Playback Authority；seek 验证（§69） | m9_gate/build 测试 |

## 不变量（§51/§52/§109）

1. Playback Authority 唯一（UI/Mini/Tray/SMTC/Updater 全为 command/adapter/observer）。
2. 安全代码不进实时 audio callback（M12 未改 audio 路径）。
3. Remote manifest 永远走 validate→version→signature→artifact，不存在"下载即执行"。
4. Frontend 不持有任何 secret（§63 审计通过）。
