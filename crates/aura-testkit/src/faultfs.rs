//! FaultFS（M0 §计划的测试基建，M5 实装）：真实文件系统 + 可注入故障。
//!
//! 用途：cache 原子写的 write failure / truncation / rename failure 注入
//! （M5 §65/§101）。确定性：故障在显式 `arm()` 之后按配置触发，测试可复现。
//!
//! aura-cache 定义 `ChunkFs` 操作集；本模块提供该 trait 的故障实现与
//! 真实实现（`RealFs`），生产路径零 trait 开销由调用方决定。

use std::path::{Path, PathBuf};
use std::sync::Mutex;

/// 文件系统操作集（cache 存储依赖的最小面）。
pub trait ChunkFs: Send + Sync {
    /// 原子性由调用方负责：`write_file` 只写临时文件。
    fn write_file(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()>;
    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()>;
    fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>>;
    fn remove_file(&self, path: &Path) -> std::io::Result<()>;
    fn exists(&self, path: &Path) -> bool;
    fn file_size(&self, path: &Path) -> std::io::Result<u64>;
    fn create_dir_all(&self, path: &Path) -> std::io::Result<()>;
    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()>;
    /// 目录一级列表（integrity 扫描用）。
    fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>>;
}

/// 真实文件系统实现。
#[derive(Debug, Default, Clone, Copy)]
pub struct RealFs;

impl ChunkFs for RealFs {
    fn write_file(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, bytes)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        if let Some(parent) = to.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::rename(from, to)
    }

    fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        std::fs::read(path)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        path.exists()
    }

    fn file_size(&self, path: &Path) -> std::io::Result<u64> {
        Ok(std::fs::metadata(path)?.len())
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(path)
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        std::fs::remove_dir_all(path)
    }

    fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        Ok(std::fs::read_dir(path)?
            .filter_map(|e| e.ok())
            .map(|e| e.path())
            .collect())
    }
}

/// 注入的故障类型。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Fault {
    /// write_file 直接失败（权限/满盘仿真）。
    WriteFailure,
    /// write_file 成功但**截断**到指定字节数（少写仿真，§65 Truncation）。
    Truncate { keep_bytes: usize },
    /// rename 失败（§65 Rename failure）。
    RenameFailure,
}

/// FaultFS：包装 [`RealFs`]，`arm()` 的故障在**下一次**匹配操作触发一次
/// （触发后自动解除 —— 每次只注入一个故障，确定性最好）。
#[derive(Debug)]
pub struct FaultFS {
    inner: RealFs,
    armed: Mutex<Option<Fault>>,
    /// 触发记录（测试断言"故障确实被消费"）。
    fired: Mutex<Vec<Fault>>,
}

impl FaultFS {
    #[must_use]
    pub fn new() -> Self {
        Self {
            inner: RealFs,
            armed: Mutex::new(None),
            fired: Mutex::new(Vec::new()),
        }
    }

    /// 预埋下一次匹配的故障。
    pub fn arm(&self, fault: Fault) {
        *self.armed.lock().expect("armed lock") = Some(fault);
    }

    /// 已触发的故障列表（快照）。
    #[must_use]
    pub fn fired(&self) -> Vec<Fault> {
        self.fired.lock().expect("fired lock").clone()
    }

    fn take_fault(&self, kind: fn(&Fault) -> bool) -> Option<Fault> {
        let mut armed = self.armed.lock().expect("armed lock");
        if armed.as_ref().is_some_and(kind) {
            armed.take()
        } else {
            None
        }
    }

    fn record(&self, f: Fault) {
        self.fired.lock().expect("fired lock").push(f);
    }
}

impl Default for FaultFS {
    fn default() -> Self {
        Self::new()
    }
}

impl ChunkFs for FaultFS {
    fn write_file(&self, path: &Path, bytes: &[u8]) -> std::io::Result<()> {
        match self.take_fault(|f| matches!(f, Fault::WriteFailure | Fault::Truncate { .. })) {
            Some(Fault::WriteFailure) => {
                let f = Fault::WriteFailure;
                self.record(f);
                return Err(std::io::Error::other("FaultFS: injected write failure"));
            }
            Some(Fault::Truncate { keep_bytes }) => {
                let cut = keep_bytes.min(bytes.len());
                self.record(Fault::Truncate { keep_bytes });
                return RealFs.write_file(path, &bytes[..cut]);
            }
            _ => {}
        }
        self.inner.write_file(path, bytes)
    }

    fn rename(&self, from: &Path, to: &Path) -> std::io::Result<()> {
        if let Some(f @ Fault::RenameFailure) =
            self.take_fault(|f| matches!(f, Fault::RenameFailure))
        {
            self.record(f);
            return Err(std::io::Error::other("FaultFS: injected rename failure"));
        }
        self.inner.rename(from, to)
    }

    fn read_file(&self, path: &Path) -> std::io::Result<Vec<u8>> {
        self.inner.read_file(path)
    }

    fn remove_file(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_file(path)
    }

    fn exists(&self, path: &Path) -> bool {
        self.inner.exists(path)
    }

    fn file_size(&self, path: &Path) -> std::io::Result<u64> {
        self.inner.file_size(path)
    }

    fn create_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.inner.create_dir_all(path)
    }

    fn remove_dir_all(&self, path: &Path) -> std::io::Result<()> {
        self.inner.remove_dir_all(path)
    }

    fn list_dir(&self, path: &Path) -> std::io::Result<Vec<PathBuf>> {
        self.inner.list_dir(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn write_failure_injected_once() {
        let fs = FaultFS::new();
        let dir = std::env::temp_dir().join(format!("faultfs-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("a.bin");

        fs.arm(Fault::WriteFailure);
        assert!(fs.write_file(&p, b"data").is_err());
        assert_eq!(fs.fired().len(), 1);

        // 故障一次性：后续调用恢复正常。
        assert!(fs.write_file(&p, b"data").is_ok());
        assert_eq!(fs.read_file(&p).unwrap(), b"data");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncation_writes_partial() {
        let fs = FaultFS::new();
        let dir = std::env::temp_dir().join(format!("faultfs-tr-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let p = dir.join("t.bin");

        fs.arm(Fault::Truncate { keep_bytes: 3 });
        fs.write_file(&p, b"abcdef").unwrap();
        assert_eq!(fs.read_file(&p).unwrap(), b"abc");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn rename_failure_injected_once() {
        let fs = FaultFS::new();
        let dir = std::env::temp_dir().join(format!("faultfs-rn-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let a = dir.join("a");
        let b = dir.join("b");
        std::fs::write(&a, b"x").unwrap();

        fs.arm(Fault::RenameFailure);
        assert!(fs.rename(&a, &b).is_err());
        assert!(a.exists(), "源文件未被破坏");
        assert!(fs.rename(&a, &b).is_ok(), "第二次恢复正常");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn armed_fault_mismatches_op_stays_armed() {
        let fs = FaultFS::new();
        let dir = std::env::temp_dir().join(format!("faultfs-mm-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();

        fs.arm(Fault::RenameFailure);
        // 先做一次 write：不消费 rename 故障。
        let p = dir.join("w");
        fs.write_file(&p, b"x").unwrap();
        // rename 仍然被注入。
        let q = dir.join("q");
        assert!(fs.rename(&p, &q).is_err());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
