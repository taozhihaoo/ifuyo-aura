//! chunk 文件存储（§9/§10/§14/§18/§20/§118/§119）：临时写 → flush →
//! 原子 rename → DB 事务记录。路径全部由 `keys` 固定映射生成
//! （无用户字符串拼接，§116）；temp 只在 cache 根内 `tmp/`（§118）；
//! rename 前后都在 cache 根内（§119）。

use std::path::{Path, PathBuf};
use std::sync::Arc;

use sha2::{Digest, Sha256};

use crate::error::{CacheError, Result};
use crate::keys;
use aura_testkit::faultfs::ChunkFs;

/// 固定 chunk 大小（§10）：256 KiB。全模块唯一常量。
pub const CHUNK_SIZE: usize = 256 * 1024;

/// chunk 文件记录（写入成功后返回）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredChunk {
    pub content_key: String,
    pub chunk_index: u64,
    pub size: u64,
    pub sha256: String,
}

/// chunk 存储。`fs` 可注入（测试 FaultFS；生产 RealFs）。
pub struct ChunkStore {
    root: PathBuf,
    fs: Arc<dyn ChunkFs>,
}

impl ChunkStore {
    /// 打开/创建（§26：目录自动创建）。
    ///
    /// # Errors
    /// 目录创建失败（权限/满盘）→ [`CacheError`]。
    pub fn new(root: impl Into<PathBuf>, fs: Arc<dyn ChunkFs>) -> Result<Self> {
        let root = root.into();
        fs.create_dir_all(&root.join("chunks"))?;
        fs.create_dir_all(&root.join("tmp"))?;
        Ok(Self { root, fs })
    }

    #[must_use]
    pub fn root(&self) -> &Path {
        &self.root
    }

    /// chunk 绝对路径（root + 固定映射；并在根内校验，§119）。
    fn chunk_path(&self, content_key: &str, chunk_index: u64) -> Result<PathBuf> {
        let rel = keys::chunk_rel_path(content_key, chunk_index)?;
        let p = self.root.join(rel);
        if !p.starts_with(&self.root) {
            return Err(CacheError::InvalidKey {
                reason: "chunk path escaped cache root".into(),
            });
        }
        Ok(p)
    }

    fn temp_path(&self, content_key: &str, chunk_index: u64) -> Result<PathBuf> {
        let ck = keys::is_valid_content_key(content_key)
            .then_some(content_key)
            .ok_or_else(|| CacheError::InvalidKey {
                reason: "bad content key".into(),
            })?;
        let p = self.root.join(format!("tmp/{ck}-{chunk_index:08}.tmp"));
        if !p.starts_with(&self.root) {
            return Err(CacheError::InvalidKey {
                reason: "temp path escaped cache root".into(),
            });
        }
        Ok(p)
    }

    /// §20 原子写：tmp 写入 → 大小校验 → rename → 返回记录。
    /// DB 事务由调用方在成功后执行（先文件后元数据；崩溃时文件成为
    /// orphan，integrity 回收 —— D-058）。
    ///
    /// # Errors
    /// 写入失败（含 FaultFS 注入）/ rename 失败 / 大小不符。
    pub fn write_chunk(
        &self,
        content_key: &str,
        chunk_index: u64,
        bytes: &[u8],
    ) -> Result<StoredChunk> {
        if bytes.len() > CHUNK_SIZE {
            return Err(CacheError::Corrupt {
                what: format!("chunk {chunk_index} exceeds CHUNK_SIZE"),
            });
        }
        let tmp = self.temp_path(content_key, chunk_index)?;
        let dst = self.chunk_path(content_key, chunk_index)?;

        self.fs.write_file(&tmp, bytes)?;
        // 写后校验（§65 Truncation 仿真必须在此被抓住）。
        let written = self.fs.file_size(&tmp)?;
        if written != bytes.len() as u64 {
            let _ = self.fs.remove_file(&tmp);
            return Err(CacheError::Corrupt {
                what: format!("temp chunk truncated: {written} != {}", bytes.len()),
            });
        }
        if let Err(e) = self.fs.rename(&tmp, &dst) {
            let _ = self.fs.remove_file(&tmp); // rename 失败不残留 temp（§20）
            return Err(e.into());
        }
        // rename 后再校验目标存在与大小（Windows rename 原子；防御式）。
        if !self.fs.exists(&dst) || self.fs.file_size(&dst)? != bytes.len() as u64 {
            return Err(CacheError::Corrupt {
                what: "chunk file missing after rename".into(),
            });
        }
        Ok(StoredChunk {
            content_key: content_key.to_string(),
            chunk_index,
            size: bytes.len() as u64,
            sha256: sha256_hex(bytes),
        })
    }

    /// 读取 chunk 字节（只查存在与大小；sha 校验在 integrity pass /
    /// 怀疑场景，读路径不付 sha 成本 —— D-059）。
    ///
    /// # Errors
    /// 不存在 → [`CacheError::Corrupt`]（missing 语义，§23）。
    pub fn read_chunk(&self, content_key: &str, chunk_index: u64) -> Result<Vec<u8>> {
        let p = self.chunk_path(content_key, chunk_index)?;
        if !self.fs.exists(&p) {
            return Err(CacheError::Corrupt {
                what: format!("chunk file missing: {chunk_index}"),
            });
        }
        let size = self.fs.file_size(&p)?;
        if size > CHUNK_SIZE as u64 {
            return Err(CacheError::Corrupt {
                what: format!("chunk file oversized: {size}"),
            });
        }
        Ok(self.fs.read_file(&p)?)
    }

    /// 删除单个 chunk 文件（不存在 = Ok，幂等）。
    pub fn remove_chunk(&self, content_key: &str, chunk_index: u64) -> Result<()> {
        let p = self.chunk_path(content_key, chunk_index)?;
        if self.fs.exists(&p) {
            self.fs.remove_file(&p)?;
        }
        Ok(())
    }

    /// 删除整个条目的 chunk 目录（原子淘汰的文件侧，§60）。
    pub fn remove_entry_files(&self, content_key: &str) -> Result<()> {
        let rel = keys::entry_rel_dir(content_key)?;
        let dir = self.root.join(rel);
        if self.fs.exists(&dir) {
            self.fs.remove_dir_all(&dir)?;
        }
        Ok(())
    }

    /// 任意相对路径列表（integrity 扫描；rel 由调用方保证来自固定映射）。
    pub fn list_dir(&self, path: &Path) -> Result<Vec<PathBuf>> {
        if !self.fs.exists(path) {
            return Ok(Vec::new());
        }
        Ok(self.fs.list_dir(path)?)
    }

    /// 删除任意 cache 根内文件（integrity 修复路径；校验在根内 §119）。
    pub fn remove_file(&self, path: &Path) -> Result<()> {
        if !path.starts_with(&self.root) {
            return Err(CacheError::InvalidKey {
                reason: "path escaped cache root".into(),
            });
        }
        if self.fs.exists(path) {
            self.fs.remove_file(path)?;
        }
        Ok(())
    }

    /// chunk 相对路径列表（integrity 扫描）。
    pub fn list_entry_files(&self, content_key: &str) -> Result<Vec<PathBuf>> {
        let dir = self.root.join(keys::entry_rel_dir(content_key)?);
        if !self.fs.exists(&dir) {
            return Ok(Vec::new());
        }
        Ok(self.fs.list_dir(&dir)?)
    }

    /// 全部一级内容目录（orphan 扫描用：返回出现过的 content key 前缀目录）。
    pub fn list_prefix_dirs(&self) -> Result<Vec<PathBuf>> {
        let dir = self.root.join("chunks");
        if !self.fs.exists(&dir) {
            return Ok(Vec::new());
        }
        Ok(self.fs.list_dir(&dir)?)
    }

    /// 残留 temp 清理（§133：temp 有界；integrity/shutdown 时调用）。
    pub fn clear_tmp(&self) -> Result<usize> {
        let dir = self.root.join("tmp");
        let mut n = 0;
        for f in self.fs.list_dir(&dir).unwrap_or_default() {
            if self.fs.remove_file(&f).is_ok() {
                n += 1;
            }
        }
        Ok(n)
    }
}

/// SHA-256 hex（§19）。
#[must_use]
pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut hex = String::with_capacity(64);
    for b in digest {
        hex.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
        hex.push(char::from_digit(u32::from(b & 0xf), 16).unwrap_or('0'));
    }
    hex
}

/// 分块：把整段内容切成 chunk 序列（测试与 sequential fill 共用）。
#[must_use]
pub fn chunk_bounds(content_length: u64) -> Vec<(u64, usize)> {
    let mut out = Vec::new();
    let mut off = 0u64;
    while off < content_length {
        let len = usize::try_from((off + CHUNK_SIZE as u64).min(content_length) - off)
            .unwrap_or(CHUNK_SIZE);
        out.push((off, len));
        off += len as u64;
    }
    out
}
