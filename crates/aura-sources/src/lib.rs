//! ifuyo Aura 来源系统（M4）：Source trait、本地/直链 HTTP/网络电台来源、
//! ICY 元数据、Radio Browser 目录、URL/私网/平台封锁策略。
//!
//! 职责边界（§7/§55/§59/§93/§128）：Input → Detection → Resolution →
//! Resolved(+License/Attribution/CachePolicy)。不负责 audio output /
//! persistent cache（M5）/ UI / SQLite 直写。

pub mod error;
pub mod http;
pub mod icy;
pub mod model;
pub mod platform_block;
pub mod url_policy;
