//! CacheManager（§26/§27/§55–§62/§70–§77/§103）：统一策略门 +
//! 存储编排 + integrity/recovery + LRU/pin/容量。
//!
//! 策略硬门禁（§7 CacheWriteGuard 语义）：所有持久写入必须经
//! [`CacheManager::put_chunk`] —— Forbidden 直接拒绝（任何路径零写盘），
//! SessionOnly 只进内存，Allowed 才落盘落库。live 流二次检查（§75）。

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use aura_sources::model::CachePolicy;

use crate::db::{CacheChunkRow, CacheDb, CacheEntryRow};
use crate::error::{CacheError, Result};
use crate::keys;
use crate::store::{sha256_hex, ChunkStore, CHUNK_SIZE};
use aura_testkit::faultfs::{ChunkFs, RealFs};

/// 注入时钟（§105：测试 VirtualClock；生产真实时钟）。
pub type NowFn = Arc<dyn Fn() -> i64 + Send + Sync>;

/// 默认容量（§55）：2 GiB。
pub const DEFAULT_CAPACITY_BYTES: u64 = 2 << 30;

/// SessionOnly 内存上限（§132 有界）：32 MiB。
pub const SESSION_CAP_BYTES: usize = 32 << 20;

/// access-time 批量落库阈值（§62：不逐字节写 SQLite）。
const TOUCH_BATCH: usize = 64;

/// Cache 配置（§8：路径由应用层提供，不硬编码 %APPDATA%）。
#[derive(Clone)]
pub struct CacheConfig {
    pub root: PathBuf,
    pub capacity_bytes: u64,
    pub now_ms: NowFn,
}

impl std::fmt::Debug for CacheConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("CacheConfig")
            .field("root", &self.root)
            .field("capacity_bytes", &self.capacity_bytes)
            .finish_non_exhaustive()
    }
}

impl CacheConfig {
    /// 生产默认：真实时钟 + 2 GiB。
    #[must_use]
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            capacity_bytes: DEFAULT_CAPACITY_BYTES,
            now_ms: Arc::new(|| {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
                    .unwrap_or(0)
            }),
        }
    }
}

/// 策略路由（§79）：调用方据此选择数据通路。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Routing {
    /// Allowed：持久缓存读写。
    Persistent,
    /// SessionOnly：仅内存。
    Session,
    /// Forbidden：完全绕过 cache 层（调用方直接用 M4 source 流）。
    Bypassed,
}

/// §25 输出。
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct IntegrityReport {
    pub entries_checked: u64,
    pub chunks_checked: u64,
    pub corrupted: u64,
    pub missing: u64,
    pub orphaned: u64,
    pub repaired: u64,
}

/// 新 entry 的声明（ensure_entry 输入）。
#[derive(Debug, Clone)]
pub struct EntryDeclaration {
    pub content_key: String,
    pub source_kind: String,
    pub url: String,
    pub content_length: Option<u64>,
    pub mime: Option<String>,
    pub codec: Option<String>,
    pub seekable: bool,
    /// §75：live 流即使被错误声明也拒绝持久化。
    pub live: bool,
    pub policy: CachePolicy,
    pub etag: Option<String>,
    pub last_modified: Option<String>,
}

/// chunk 写入的 lease 注册表（§61 in-flight protection）。
#[derive(Default)]
struct Leases(Mutex<std::collections::HashSet<(String, u64)>>);

impl Leases {
    fn acquire(&self, ck: &str, idx: u64) -> bool {
        self.0
            .lock()
            .expect("leases lock")
            .insert((ck.to_string(), idx))
    }

    fn release(&self, ck: &str, idx: u64) {
        self.0
            .lock()
            .expect("leases lock")
            .remove(&(ck.to_string(), idx));
    }

    fn held(&self, ck: &str, idx: u64) -> bool {
        self.0
            .lock()
            .expect("leases lock")
            .contains(&(ck.to_string(), idx))
    }
}

/// in-flight fetch 注册表（§53/§95/§136）：FetchKey = (content_key, chunk_index)。
/// 同 key 的并发拉取共享一次下载 —— 后到者等 Condvar 并读缓存。
#[derive(Default)]
struct Inflight {
    map: InflightMap,
}

/// 共享等待槽。
type InflightSlot = Arc<(Mutex<u32>, Condvar)>;
type InflightMap = Mutex<HashMap<(String, u64), InflightSlot>>;

impl Inflight {
    /// 进入：返回 true = 本调用负责下载；false = 已有人在下载（等待方）。
    fn enter(&self, ck: &str, idx: u64) -> (bool, Arc<(Mutex<u32>, Condvar)>) {
        let mut map = self.map.lock().expect("inflight lock");
        let key = (ck.to_string(), idx);
        if let Some(slot) = map.get(&key) {
            return (false, Arc::clone(slot));
        }
        let slot = Arc::new((Mutex::new(1), Condvar::new()));
        map.insert(key, Arc::clone(&slot));
        (true, slot)
    }

    /// 负责方完成：计数清零 + 唤醒等待方并移除。
    /// （等待方以 `count==0` 为出边条件——只 notify 不清零会让等待方
    /// 空转到 30s deadline，M6 门禁实测暴露。）
    fn finish(&self, ck: &str, idx: u64) {
        let mut map = self.map.lock().expect("inflight lock");
        if let Some(slot) = map.remove(&(ck.to_string(), idx)) {
            if let Ok(mut count) = slot.0.lock() {
                *count = 0;
            }
            slot.1.notify_all();
        }
    }
}

/// 缓存管理器。线程安全（多 reader 共享）。
pub struct CacheManager {
    config: CacheConfig,
    db: Mutex<CacheDb>,
    store: ChunkStore,
    leases: Leases,
    /// SessionOnly 内存存储（§5）。
    session: Mutex<SessionStore>,
    session_bytes: AtomicU64,
    /// §62 access-time 聚合。
    pending_touches: Mutex<Vec<(String, u64, i64)>>,
    inflight: Inflight,
    closed: AtomicBool,
}

#[derive(Default)]
struct SessionStore {
    map: HashMap<(String, u64), Vec<u8>>,
    order: Vec<(String, u64)>, // 插入序（FIFO 淘汰，够简单可靠）
}

impl CacheManager {
    /// 打开/创建（§26）。DB 打不开 → 备份后重建空索引（§27，不删 chunks；
    /// chunks 变 orphan 由 integrity 处理）。
    ///
    /// # Errors
    /// 目录创建失败/重建仍失败。
    pub fn open(config: CacheConfig) -> Result<Self> {
        Self::open_with_fs(config, Arc::new(RealFs))
    }

    /// 注入 Fs（测试 FaultFS）。
    ///
    /// # Errors
    /// 同 [`CacheManager::open`]。
    pub fn open_with_fs(config: CacheConfig, fs: Arc<dyn ChunkFs>) -> Result<Self> {
        fs.create_dir_all(&config.root)?;
        let db_path = config.root.join("cache.db");
        let db = match CacheDb::open(&db_path) {
            Ok(db) => db,
            Err(e) => {
                // §27：保留损坏 DB 备份，重建空索引（绝不碰 library DB ——
                // 本模块根本不知道 library 路径，结构上隔离 §28）。
                let backup = config
                    .root
                    .join(format!("cache.db.corrupt-{}", (config.now_ms)().max(0)));
                let _ = std::fs::rename(&db_path, &backup);
                for suffix in ["-wal", "-shm"] {
                    let _ = std::fs::rename(
                        config.root.join(format!("cache.db{suffix}")),
                        config.root.join(format!("cache.db{suffix}.corrupt")),
                    );
                }
                CacheDb::open(&db_path).map_err(|_| e)?;
                CacheDb::open(&db_path)?
            }
        };
        let store = ChunkStore::new(&config.root, fs)?;
        Ok(Self {
            config,
            db: Mutex::new(db),
            store,
            leases: Leases::default(),
            session: Mutex::new(SessionStore::default()),
            session_bytes: AtomicU64::new(0),
            pending_touches: Mutex::new(Vec::new()),
            inflight: Inflight::default(),
            closed: AtomicBool::new(false),
        })
    }

    /// §79：policy → 路由。未知/显式 policy 决定。
    #[must_use]
    pub fn routing(policy: CachePolicy) -> Routing {
        match policy {
            CachePolicy::Allowed => Routing::Persistent,
            CachePolicy::SessionOnly => Routing::Session,
            CachePolicy::Forbidden => Routing::Bypassed,
        }
    }

    fn now(&self) -> i64 {
        (self.config.now_ms)()
    }

    /// 声明/刷新一个 Allowed entry（§17 字段；§40/§41 ETag/LM 存证）。
    /// Session/Forbidden 不产生任何 DB 行（§70/§71）。
    ///
    /// # Errors
    /// Forbidden/Session 传入 → [`CacheError::PolicyViolation`]；SQL 失败。
    pub fn ensure_entry(&self, d: &EntryDeclaration) -> Result<()> {
        if d.policy != CachePolicy::Allowed {
            return Err(CacheError::PolicyViolation {
                reason: "ensure_entry is only for Allowed policy".into(),
            });
        }
        if d.live {
            // §75：live 流双重检查，绝不持久化。
            return Err(CacheError::PolicyViolation {
                reason: "live streams are never persisted".into(),
            });
        }
        let now = self.now();
        let redacted = keys::redact_url(&d.url);
        let url_hash = keys::content_key(&d.url)?;
        self.db
            .lock()
            .expect("db lock")
            .upsert_entry(&CacheEntryRow {
                content_key: d.content_key.clone(),
                source_kind: d.source_kind.clone(),
                url_hash,
                redacted_url: redacted,
                content_length: d.content_length,
                mime: d.mime.clone(),
                codec: d.codec.clone(),
                seekable: d.seekable,
                complete: false,
                etag: d.etag.clone(),
                last_modified: d.last_modified.clone(),
                policy: "Allowed".into(),
                created_at_ms: now,
                last_accessed_ms: now,
                total_bytes: 0,
            })
    }

    /// entry 快照（reader revalidation 用）。
    #[must_use]
    pub fn entry_snapshot(&self, content_key: &str) -> Option<crate::db::CacheEntryRow> {
        self.db
            .lock()
            .expect("db lock")
            .get_entry(content_key)
            .unwrap_or(None)
    }

    /// §43：内容变更 → 作废该 key 全部 chunks（文件 + 行），保留 entry
    /// 行与 pin（重填后自然恢复）。
    pub fn invalidate_content(&self, content_key: &str) {
        let rows = self
            .db
            .lock()
            .expect("db lock")
            .list_chunks(content_key)
            .unwrap_or_default();
        let ndb = self.db.lock().expect("db lock");
        for r in &rows {
            let _ = ndb.delete_chunk(content_key, r.chunk_index);
        }
        drop(ndb);
        let _ = self.store.remove_entry_files(content_key);
    }

    /// 更新 entry 的 validator（revalidation 后）。
    pub fn update_validators(
        &self,
        content_key: &str,
        etag: Option<String>,
        last_modified: Option<String>,
    ) {
        if let Ok(tx) = self.db.lock().expect("db lock").transaction() {
            let _ = tx.execute(
                "UPDATE cache_entries SET etag = ?2, last_modified = ?3 WHERE content_key = ?1",
                rusqlite::params![content_key, etag, last_modified],
            );
            let _ = tx.commit();
        }
    }

    /// §7 统一写入门。
    ///
    /// - `Forbidden` → `PolicyViolation`（任何执行路径零持久写，§6）；
    /// - `SessionOnly` → 有界内存（§5/§132）；
    /// - `Allowed` → 容量检查（§59）→ LRU 腾位（§55/§60）→ 原子写（§20）
    ///   → sha 校验（§19/§24）→ DB 记录（§64）。
    ///
    /// 返回是否真正持久化（Session = false）。
    ///
    /// # Errors
    /// 见上；live 流非法声明同样在此拒绝（§75 双保险）。
    pub fn put_chunk(
        &self,
        policy: CachePolicy,
        content_key: &str,
        chunk_index: u64,
        bytes: &[u8],
        live: bool,
    ) -> Result<bool> {
        if self.closed.load(Ordering::Relaxed) {
            return Err(CacheError::Cancelled);
        }
        // CHUNK_SIZE 上限只约束持久层；SessionOnly 内存块不受限（§5）。
        if policy == CachePolicy::Allowed && bytes.len() > CHUNK_SIZE {
            return Err(CacheError::Corrupt {
                what: "chunk exceeds CHUNK_SIZE".into(),
            });
        }
        match policy {
            CachePolicy::Forbidden => {
                // §6/§75：任何代码路径零持久写。
                return Err(CacheError::PolicyViolation {
                    reason: "Forbidden source must never touch persistent cache".into(),
                });
            }
            CachePolicy::SessionOnly => {
                if live {
                    return Err(CacheError::PolicyViolation {
                        reason: "live streams are never cached".into(),
                    });
                }
                self.session_put(content_key, chunk_index, bytes)?;
                return Ok(false);
            }
            CachePolicy::Allowed => {}
        }
        if live {
            return Err(CacheError::PolicyViolation {
                reason: "live streams are never persisted".into(),
            });
        }

        // §61：in-flight lease（防下载中被 LRU 删除）。
        if !self.leases.acquire(content_key, chunk_index) {
            return Err(CacheError::Cancelled); // 同 chunk 并发写：调用方走 dedup
        }
        let _guard = LeaseGuard(&self.leases, content_key, chunk_index);

        // §55/§59：容量检查 + LRU 腾位（pinned/lease 跳过；腾不动 → CacheFullPinned）。
        self.evict_for_capacity(bytes.len() as u64)?;

        let now = self.now();
        let stored = self.store.write_chunk(content_key, chunk_index, bytes)?;
        // §24：写后 sha 校验（新数据必须可校验）。
        let expect = sha256_hex(bytes);
        if stored.sha256 != expect {
            let _ = self.store.remove_chunk(content_key, chunk_index);
            return Err(CacheError::Corrupt {
                what: "post-write sha mismatch".into(),
            });
        }
        self.db
            .lock()
            .expect("db lock")
            .insert_chunk(&CacheChunkRow {
                content_key: content_key.to_string(),
                chunk_index,
                size: stored.size,
                sha256: stored.sha256,
                created_at_ms: now,
                last_accessed_ms: now,
            })?;
        Ok(true)
    }

    /// §79 读：Allowed 查盘；SessionOnly 查内存；Forbidden 恒 None。
    #[must_use]
    pub fn get_chunk(
        &self,
        policy: CachePolicy,
        content_key: &str,
        chunk_index: u64,
    ) -> Option<Vec<u8>> {
        match policy {
            CachePolicy::Forbidden => None,
            CachePolicy::SessionOnly => self.session_get(content_key, chunk_index),
            CachePolicy::Allowed => {
                if self.closed.load(Ordering::Relaxed) {
                    return None;
                }
                let bytes = self.store.read_chunk(content_key, chunk_index).ok()?;
                // §62：内存聚合，批量落库。
                let mut t = self.pending_touches.lock().expect("touch lock");
                t.push((content_key.to_string(), chunk_index, self.now()));
                if t.len() >= TOUCH_BATCH {
                    let batch: Vec<_> = t.drain(..).collect();
                    drop(t);
                    let _ = self.db.lock().expect("db lock").touch_chunks(&batch);
                }
                Some(bytes)
            }
        }
    }

    /// §53/§95/§136：dedup 拉取 —— 同 FetchKey 并发时只有一方下载。
    /// `fetch` 只在负责方执行；等待方在完成后从缓存读取。
    ///
    /// # Errors
    /// 负责方的 fetch 失败原样上抛（等待方随后自行重试读取/拉取）。
    pub fn dedup_fetch(
        &self,
        content_key: &str,
        chunk_index: u64,
        fetch: impl FnOnce() -> Result<Vec<u8>>,
    ) -> Result<Vec<u8>> {
        let (leader, slot) = self.inflight.enter(content_key, chunk_index);
        if leader {
            let out = fetch();
            self.inflight.finish(content_key, chunk_index);
            out
        } else {
            // §95：等负责方完成（count==0 即完成；有界 30s 防失控）。
            let (lock, cv) = &*slot;
            let guard = lock.lock().expect("slot lock");
            let deadline = std::time::Instant::now() + Duration::from_secs(30);
            let mut guard = guard;
            loop {
                if *guard == 0 || std::time::Instant::now() > deadline {
                    break;
                }
                let (g, _) = cv
                    .wait_timeout(guard, Duration::from_millis(50))
                    .expect("slot cv");
                guard = g;
            }
            // 负责方的 fetch 闭包负责持久化（reader 路径如此）；
            // 等待方从缓存读取。
            self.get_chunk(CachePolicy::Allowed, content_key, chunk_index)
                .ok_or_else(|| CacheError::Network {
                    reason: "shared fetch did not produce chunk".into(),
                })
        }
    }

    /// in-flight 数（诊断/测试）。
    #[must_use]
    pub fn inflight_count(&self) -> usize {
        self.inflight.map.lock().expect("inflight lock").len()
    }

    /// §58：pin（Allowed entry；pinned 不被 LRU 淘汰）。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn pin(&self, content_key: &str) -> Result<()> {
        self.db
            .lock()
            .expect("db lock")
            .pin(content_key, self.now())
    }

    /// §58：unpin。
    ///
    /// # Errors
    /// SQL 失败。
    pub fn unpin(&self, content_key: &str) -> Result<()> {
        self.db.lock().expect("db lock").unpin(content_key)
    }

    #[must_use]
    pub fn is_pinned(&self, content_key: &str) -> bool {
        self.db
            .lock()
            .expect("db lock")
            .is_pinned(content_key)
            .unwrap_or(false)
    }

    /// §55/§60：LRU 淘汰到目标余量。跳过 pinned 与 in-flight lease；
    /// 全 pinned 腾不动 → [`CacheError::CacheFullPinned`]（§59，绝不删 pinned）。
    fn evict_for_capacity(&self, incoming: u64) -> Result<()> {
        let needed = self.config.capacity_bytes;
        loop {
            let used = self.total_bytes()?;
            if used + incoming <= needed {
                return Ok(());
            }
            // 一次淘汰一批（按 LRU 序）。
            let victims = self.db.lock().expect("db lock").lru_chunks(16)?;
            let mut evicted = 0u64;
            for v in victims {
                if self.leases.held(&v.content_key, v.chunk_index) {
                    continue; // §61 lease 保护
                }
                self.store.remove_chunk(&v.content_key, v.chunk_index)?;
                self.db
                    .lock()
                    .expect("db lock")
                    .delete_chunk(&v.content_key, v.chunk_index)?;
                evicted += v.size;
                if used + incoming <= needed + evicted {
                    return Ok(());
                }
            }
            if evicted == 0 {
                return Err(CacheError::CacheFullPinned);
            }
        }
    }

    /// §56：总占用 = 有效 chunk bytes（DB 统计）。
    pub fn total_bytes(&self) -> Result<u64> {
        self.db.lock().expect("db lock").total_chunk_bytes()
    }

    /// §25：integrity check。策略（D-060）：
    /// - DB 有行、文件缺失 → 删行（missing，§23；下次读触发重取）；
    /// - 文件损坏（size/sha 不符）→ 删行+删文件（§24）；
    /// - 文件存在、DB 无行 → orphan → 删文件（可重取，不留幽灵）；
    /// - dangling pin → 删 pin；
    /// - tmp 残留 → 清理（§133）。
    pub fn verify_cache(&self) -> Result<IntegrityReport> {
        let mut report = IntegrityReport::default();
        let now = self.now();
        let db = self.db.lock().expect("db lock");

        // 1) DB → 文件。
        let rows = db.list_all_chunks()?;
        report.chunks_checked = rows.len() as u64;
        report.entries_checked = db.entry_count()?;
        let mut db_keys: std::collections::HashSet<String> =
            db.list_entry_keys()?.into_iter().collect();
        for row in rows {
            match self.store.read_chunk(&row.content_key, row.chunk_index) {
                Err(_) => {
                    // missing（§23）：删行，可重取。
                    db.delete_chunk(&row.content_key, row.chunk_index)?;
                    report.missing += 1;
                    report.repaired += 1;
                }
                Ok(bytes) => {
                    let size_ok = bytes.len() as u64 == row.size;
                    let sha_ok = sha256_hex(&bytes) == row.sha256;
                    if !size_ok || !sha_ok {
                        // corrupt（§24）：删行 + 删文件，绝不播坏数据。
                        let _ = self.store.remove_chunk(&row.content_key, row.chunk_index);
                        db.delete_chunk(&row.content_key, row.chunk_index)?;
                        report.corrupted += 1;
                        report.repaired += 1;
                    }
                }
            }
            db_keys.remove(&row.content_key);
            let _ = now;
        }

        // 2) 文件 → DB：orphan 扫描（含已无 entry 的内容目录）。
        for prefix_dir in self.store.list_prefix_dirs()? {
            for content_dir in self.store.list_dir(&prefix_dir).unwrap_or_default() {
                let Some(ck) = content_dir.file_name().and_then(|n| n.to_str()) else {
                    continue;
                };
                if !keys::is_valid_content_key(ck) {
                    // §116：非法目录名（注入产物）→ 整目录 quarantine。
                    let _ = self.store.remove_entry_files(ck);
                    report.orphaned += 1;
                    continue;
                }
                for f in self.store.list_dir(&content_dir).unwrap_or_default() {
                    let Some(name) = f.file_name().and_then(|n| n.to_str()) else {
                        continue;
                    };
                    let Some(idx) = name
                        .strip_suffix(".bin")
                        .and_then(|n| n.parse::<u64>().ok())
                    else {
                        let _ = self.store.remove_file(&f); // 非法文件名 → orphan
                        report.orphaned += 1;
                        continue;
                    };
                    if db.get_chunk(ck, idx).unwrap_or(None).is_none() {
                        let _ = self.store.remove_chunk(ck, idx);
                        report.orphaned += 1;
                    }
                }
                db_keys.remove(ck);
            }
        }

        // 3) dangling pin（entry 已不存在）。
        for pk in db.list_pins()? {
            if db.get_entry(&pk).unwrap_or(None).is_none() {
                db.unpin(&pk)?;
                report.repaired += 1;
            }
        }
        let _ = db_keys; // entry 无任何 chunk 行不视为损坏（complete=false 合法）

        drop(db);
        // 4) tmp 清理（§133）。
        self.store.clear_tmp()?;
        Ok(report)
    }

    /// §62/§93：把聚合的 access-time 落库。
    pub fn flush_touches(&self) -> Result<()> {
        let batch: Vec<_> = self
            .pending_touches
            .lock()
            .expect("touch lock")
            .drain(..)
            .collect();
        if batch.is_empty() {
            return Ok(());
        }
        self.db.lock().expect("db lock").touch_chunks(&batch)
    }

    /// §93：关闭（停止接受新写入 + flush + tmp 清理）。
    ///
    /// # Errors
    /// flush 失败。
    pub fn shutdown(&self) -> Result<()> {
        self.closed.store(true, Ordering::Relaxed);
        self.flush_touches()?;
        self.store.clear_tmp()?;
        Ok(())
    }

    #[must_use]
    pub fn session_bytes(&self) -> u64 {
        self.session_bytes.load(Ordering::Relaxed)
    }

    /// entry 总数（诊断/测试）。
    #[must_use]
    pub fn entry_count(&self) -> u64 {
        self.db.lock().expect("db lock").entry_count().unwrap_or(0)
    }

    // ===== SessionOnly 内存存储（§5/§132）=====

    fn session_put(&self, ck: &str, idx: u64, bytes: &[u8]) -> Result<()> {
        let mut s = self.session.lock().expect("session lock");
        let key = (ck.to_string(), idx);
        if let Some(old) = s.map.insert(key.clone(), bytes.to_vec()) {
            self.session_bytes
                .fetch_sub(old.len() as u64, Ordering::Relaxed);
        } else {
            s.order.push(key.clone());
        }
        self.session_bytes
            .fetch_add(bytes.len() as u64, Ordering::Relaxed);

        // §132：有界 —— 超限按插入序淘汰。
        while self.session_bytes.load(Ordering::Relaxed) as usize > SESSION_CAP_BYTES {
            let Some(victim) = s.order.first().cloned() else {
                break;
            };
            if let Some(old) = s.map.remove(&victim) {
                self.session_bytes
                    .fetch_sub(old.len() as u64, Ordering::Relaxed);
            }
            s.order.remove(0);
            if victim == key {
                // 刚写入的被淘汰：容量就是不够 —— 放弃该块。
                return Err(CacheError::CacheFullPinned);
            }
        }
        Ok(())
    }

    fn session_get(&self, ck: &str, idx: u64) -> Option<Vec<u8>> {
        self.session
            .lock()
            .expect("session lock")
            .map
            .get(&(ck.to_string(), idx))
            .cloned()
    }

    /// §71：SessionOnly 不留痕（测试断言用）。
    #[must_use]
    pub fn session_contains(&self, ck: &str, idx: u64) -> bool {
        self.session
            .lock()
            .expect("session lock")
            .map
            .contains_key(&(ck.to_string(), idx))
    }
}

struct LeaseGuard<'a>(&'a Leases, &'a str, u64);

impl Drop for LeaseGuard<'_> {
    fn drop(&mut self) {
        self.0.release(self.1, self.2);
    }
}
