//! Region 流式生成（§116–§125）：分区域独立生成 + 后台线程 + 取消 +
//! 有界内存缓存。
//!
//! - RegionCoord：64 m 网格（§117）；区域 seed = hash(world seed + 坐标)
//!   （§118），同 world+region 恒同内容（§119）。
//! - 生成在 worker 线程执行（§121），不阻塞调用方帧；状态机
//!   Pending/Ready/Failed/Cancelled（§123）；关闭/换世界时全部取消
//!   （§124）。
//! - 内存 LRU 有界（§125 B 级）；与 M5 网络缓存无关（§126 不写库）。

use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{mpsc, Arc, Mutex};

use serde::{Deserialize, Serialize};

use crate::biome::Biome;
use crate::genome::Genome;
use crate::seed::{stable_hash64, DreamscapeSeed};
use crate::world::{PlacedObject, World, WORLD_BOUND};

/// 区域边长（米；非 GRID_CELL——区域是生成单元，GRID_CELL 是碰撞单元）。
pub const REGION_SIZE: f32 = 64.0;
/// 内存缓存上限（§125 bounded）。
pub const REGION_CACHE_MAX: usize = 16;

/// 区域坐标（§117）。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegionCoord {
    pub x: i32,
    pub z: i32,
}

impl RegionCoord {
    /// 世界坐标 → 区域坐标。
    #[must_use]
    pub fn of(x: f32, z: f32) -> Self {
        Self {
            x: (x / REGION_SIZE).floor() as i32,
            z: (z / REGION_SIZE).floor() as i32,
        }
    }

    /// 区域原点（世界坐标）。
    #[must_use]
    pub fn origin(self) -> (f32, f32) {
        (self.x as f32 * REGION_SIZE, self.z as f32 * REGION_SIZE)
    }

    /// 区域稳定 seed（§118：world_seed + 坐标派生）。
    #[must_use]
    pub fn region_seed(self, world: &DreamscapeSeed) -> u64 {
        stable_hash64(
            b"region-seed/v1",
            &[
                world.value.to_le_bytes().as_slice(),
                self.x.to_le_bytes().as_slice(),
                self.z.to_le_bytes().as_slice(),
            ]
            .concat(),
        )
    }
}

/// 区域内容：区域内的对象子集 + 地形参数（同 world+region 恒同，§119）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RegionContent {
    pub coord: RegionCoord,
    pub region_seed: u64,
    pub terrain_amplitude: f32,
    pub terrain_frequency: f32,
    pub objects: Vec<PlacedObject>,
}

/// 生成状态机（§123）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenerationStatus {
    Pending,
    Ready,
    Failed,
    Cancelled,
}

/// 后台生成结果。
#[derive(Debug, Clone, PartialEq)]
pub struct RegionResult {
    pub coord: RegionCoord,
    pub epoch: u64,
    pub status: GenerationStatus,
    pub content: Option<RegionContent>,
}

struct Job {
    coord: RegionCoord,
    epoch: u64,
}

/// 有界 LRU 区域缓存（§125；纯内存，与 M5 无关 §126）。
#[derive(Default)]
pub struct RegionCache {
    map: HashMap<RegionCoord, RegionContent>,
    order: Vec<RegionCoord>,
}

impl RegionCache {
    pub fn get(&mut self, coord: RegionCoord) -> Option<&RegionContent> {
        if self.map.contains_key(&coord) {
            self.order.retain(|c| *c != coord);
            self.order.push(coord);
        }
        self.map.get(&coord)
    }

    pub fn put(&mut self, content: RegionContent) {
        let coord = content.coord;
        if self.map.contains_key(&coord) {
            self.order.retain(|c| *c != coord);
        }
        self.order.push(coord);
        self.map.insert(coord, content);
        while self.map.len() > REGION_CACHE_MAX {
            let evict = self.order.remove(0);
            self.map.remove(&evict);
        }
    }

    #[must_use]
    pub fn len(&self) -> usize {
        self.map.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// 只读命中检查（不改 LRU 序）。
    #[must_use]
    pub fn contains(&self, coord: RegionCoord) -> bool {
        self.map.contains_key(&coord)
    }
}

/// 区域流式生成器：worker 线程 + 非阻塞 request/poll + 代际取消
/// （§120–§124）。
pub struct RegionStreamer {
    job_tx: Option<mpsc::Sender<Job>>,
    res_rx: Mutex<mpsc::Receiver<RegionResult>>,
    epoch: Arc<AtomicU64>,
    shutdown: Arc<AtomicBool>,
    worker: Option<std::thread::JoinHandle<()>>,
    cache: Mutex<RegionCache>,
}

impl RegionStreamer {
    /// 启动（world 参数快照进 worker；换世界 = drop 后重建，§124 精神）。
    #[must_use]
    pub fn start(seed: DreamscapeSeed, genome: Genome, biome: Biome) -> Self {
        let (job_tx, job_rx) = mpsc::channel::<Job>();
        let (res_tx, res_rx) = mpsc::channel::<RegionResult>();
        let epoch = Arc::new(AtomicU64::new(1));
        let shutdown = Arc::new(AtomicBool::new(false));
        let epoch2 = Arc::clone(&epoch);
        let shutdown2 = Arc::clone(&shutdown);
        let world = World::generate(&seed, &genome, &biome);
        let worker = std::thread::Builder::new()
            .name("aura-dream-region".into())
            .spawn(move || {
                while !shutdown2.load(Ordering::Relaxed) {
                    // 有界等待：收到任务或停机标志才动。
                    let job = match job_rx.recv_timeout(std::time::Duration::from_millis(50)) {
                        Ok(j) => j,
                        Err(mpsc::RecvTimeoutError::Timeout) => continue,
                        Err(mpsc::RecvTimeoutError::Disconnected) => break,
                    };
                    if shutdown2.load(Ordering::Relaxed)
                        || job.epoch != epoch2.load(Ordering::Relaxed)
                    {
                        // 取消（§124）：陈旧代任务直接回报 Cancelled。
                        let _ = res_tx.send(RegionResult {
                            coord: job.coord,
                            epoch: job.epoch,
                            status: GenerationStatus::Cancelled,
                            content: None,
                        });
                        continue;
                    }
                    // 同 world+region 恒同（§119）；worker 内纯计算（§121/§122：
                    // 不触碰任何 WebGL 对象）。
                    let content = generate_region(&world, job.coord);
                    let status = if content.is_some() {
                        GenerationStatus::Ready
                    } else {
                        GenerationStatus::Failed
                    };
                    let _ = res_tx.send(RegionResult {
                        coord: job.coord,
                        epoch: job.epoch,
                        status,
                        content,
                    });
                }
            });
        Self {
            job_tx: Some(job_tx),
            res_rx: Mutex::new(res_rx),
            epoch,
            shutdown,
            worker: worker.ok(),
            cache: Mutex::new(RegionCache::default()),
        }
    }

    /// 请求区域（非阻塞；入队即 Pending；已缓存则跳过 §125）。
    pub fn request(&self, coord: RegionCoord) {
        if let Ok(cache) = self.cache.lock() {
            if cache.contains(coord) {
                return;
            }
        }
        if let Some(tx) = &self.job_tx {
            let _ = tx.send(Job {
                coord,
                epoch: self.epoch.load(Ordering::Relaxed),
            });
        }
    }

    /// 非阻塞收割结果（不等待；Ready 结果进缓存，§125）。
    pub fn poll(&self) -> Option<RegionResult> {
        let rx = self.res_rx.lock().ok()?;
        let result = rx.try_recv().ok()?;
        if result.status == GenerationStatus::Ready {
            if let Some(content) = &result.content {
                if let Ok(mut cache) = self.cache.lock() {
                    cache.put(content.clone());
                }
            }
        }
        Some(result)
    }

    /// 取消全部在途任务（§124）：epoch 前移，陈旧结果将被丢弃/回报取消。
    pub fn cancel_all(&self) {
        self.epoch.fetch_add(1, Ordering::Relaxed);
    }

    /// 收割时丢弃陈旧代结果（配合 [`Self::cancel_all`]）。
    pub fn poll_current(&self, current_epoch: u64) -> Option<RegionResult> {
        let r = self.poll()?;
        if r.epoch != current_epoch {
            return None; // 陈旧代：静默丢弃（调用方视角 = 已取消）
        }
        Some(r)
    }

    #[must_use]
    pub fn current_epoch(&self) -> u64 {
        self.epoch.load(Ordering::Relaxed)
    }
}

impl Drop for RegionStreamer {
    fn drop(&mut self) {
        self.shutdown.store(true, Ordering::Relaxed);
        self.job_tx.take(); // 关闭通道 → worker recv 断开退出
        if let Some(h) = self.worker.take() {
            let _ = h.join();
        }
    }
}

/// 生成单个区域内容（确定性 §119；从既有世界确定性切分 + 区域种子）。
#[must_use]
pub fn generate_region(world: &World, coord: RegionCoord) -> Option<RegionContent> {
    let (ox, oz) = coord.origin();
    // 区域在有限世界内（§50）：完全出界的区域返回 Failed 内容。
    if ox > WORLD_BOUND
        || oz > WORLD_BOUND
        || ox + REGION_SIZE < -WORLD_BOUND
        || oz + REGION_SIZE < -WORLD_BOUND
    {
        return None;
    }
    let objects = world
        .objects
        .iter()
        .filter(|o| {
            let c = RegionCoord::of(o.x, o.z);
            c == coord
        })
        .copied()
        .collect();
    Some(RegionContent {
        coord,
        region_seed: coord.region_seed(&world.seed),
        terrain_amplitude: world.terrain.amplitude,
        terrain_frequency: world.terrain.frequency,
        objects,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::generator::GeneratorV1;
    use crate::genome::Genome;

    fn fixture(tag: &str) -> World {
        let seed = DreamscapeSeed::from_input(tag);
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        World::generate(&seed, &genome, &biome)
    }

    #[test]
    fn region_seed_stable_and_position_sensitive() {
        let world_seed = DreamscapeSeed::from_input("region");
        let a = RegionCoord { x: 0, z: 0 }.region_seed(&world_seed);
        let b = RegionCoord { x: 0, z: 0 }.region_seed(&world_seed);
        assert_eq!(a, b, "§118 同 world+region 同 seed");
        assert_ne!(a, RegionCoord { x: 1, z: 0 }.region_seed(&world_seed));
        assert_ne!(a, RegionCoord { x: 0, z: 1 }.region_seed(&world_seed));
    }

    #[test]
    fn region_content_deterministic_subset_of_world() {
        // §119：同 world+region 恒同；内容确为世界对象子集。
        let w = fixture("region-det");
        let c = RegionCoord { x: 0, z: 0 };
        let r1 = generate_region(&w, c).unwrap();
        let r2 = generate_region(&w, c).unwrap();
        assert_eq!(r1, r2);
        for o in &r1.objects {
            assert_eq!(RegionCoord::of(o.x, o.z), c);
            assert!(w.objects.contains(o), "子集关系");
        }
    }

    #[test]
    fn region_partition_covers_world_objects() {
        // 切分完整性：全部对象都能落在某区域。
        let w = fixture("region-partition");
        let mut total = 0;
        for x in -4..4 {
            for z in -4..4 {
                if let Some(r) = generate_region(&w, RegionCoord { x, z }) {
                    total += r.objects.len();
                }
            }
        }
        assert_eq!(total, w.objects.len(), "区域切分覆盖全部对象");
    }

    #[test]
    fn out_of_world_region_fails() {
        let w = fixture("region-oob");
        assert!(generate_region(&w, RegionCoord { x: 99, z: 99 }).is_none());
    }

    #[test]
    fn cache_bounded_lru() {
        // §125：缓存有界。
        let mut cache = RegionCache::default();
        for i in 0..32 {
            cache.put(RegionContent {
                coord: RegionCoord { x: i, z: 0 },
                region_seed: i as u64,
                terrain_amplitude: 1.0,
                terrain_frequency: 0.1,
                objects: Vec::new(),
            });
        }
        assert_eq!(cache.len(), REGION_CACHE_MAX);
        assert!(cache.get(RegionCoord { x: 31, z: 0 }).is_some(), "最新保留");
        assert!(cache.get(RegionCoord { x: 0, z: 0 }).is_none(), "最旧淘汰");
    }

    #[test]
    fn streamer_async_lifecycle() {
        // §120/§121/§123：非阻塞请求 → worker 生成 → poll 到 Ready。
        let seed = DreamscapeSeed::from_input("stream");
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        let s = RegionStreamer::start(seed, genome, biome);
        let coord = RegionCoord { x: 0, z: 0 };
        s.request(coord);
        let mut got = None;
        for _ in 0..200 {
            if let Some(r) = s.poll() {
                assert_eq!(r.coord, coord);
                assert_eq!(r.status, GenerationStatus::Ready);
                assert!(r.content.is_some());
                got = Some(r);
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        assert!(got.is_some(), "异步生成应完成");
        // 二次请求命中缓存：poll 不再产出该区域。
        s.request(coord);
        std::thread::sleep(std::time::Duration::from_millis(50));
        while let Some(r) = s.poll() {
            assert_ne!(r.coord, coord, "缓存命中不应重新生成");
        }
    }

    #[test]
    fn streamer_cancel_stale_epochs() {
        // §124：cancel_all 后陈旧代结果被丢弃。
        let seed = DreamscapeSeed::from_input("cancel");
        let genome = Genome::from_seed(&seed);
        let biome = Biome::select(&seed, &genome);
        let s = RegionStreamer::start(seed, genome, biome);
        let e0 = s.current_epoch();
        s.request(RegionCoord { x: 1, z: 1 });
        s.cancel_all();
        assert_eq!(s.current_epoch(), e0 + 1);
        // 排空：可能收到 Cancelled（陈旧代）或被 poll_current 丢弃。
        while let Some(r) = s.poll_current(s.current_epoch()) {
            assert_ne!(r.epoch, e0, "陈旧代结果不得送达");
        }
    }

    #[test]
    fn streamer_shutdown_clean() {
        // drop 即停机（§124 生命周期收尾）；重复 drop 安全。
        for _ in 0..5 {
            let seed = DreamscapeSeed::from_input("shutdown");
            let genome = Genome::from_seed(&seed);
            let biome = Biome::select(&seed, &genome);
            let s = RegionStreamer::start(seed, genome, biome);
            s.request(RegionCoord { x: 0, z: 2 });
            drop(s);
        }
    }

    #[test]
    fn generator_v1_feeds_streamer() {
        // 生成器 → 世界 → 区域（整链确定性）。
        let seed = DreamscapeSeed::from_input("chain");
        let genome = Genome::from_seed(&seed);
        let env = crate::env::EnvironmentState::initial(&seed, 0);
        let out = GeneratorV1::generate(&seed, &genome, &env);
        let c = RegionCoord { x: 0, z: 0 };
        let r = generate_region(&out.world, c).unwrap();
        assert_eq!(r.region_seed, c.region_seed(&seed));
    }
}
