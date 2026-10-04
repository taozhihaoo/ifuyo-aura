//! M9 Music Map IPC（§228/§229/§230/§141/§161，D-116）：状态握手、
//! 概览、视口批量节点、簇、邻居、单曲 detail 与 build 请求/取消。
//!
//! - §234：先 status（dataset_hash/状态）再取节点数据；首次 status
//!   触发一次 restore 评估（dataset 未变 → 免重算恢复 §174）。
//! - §141/§229/§230：视口批量查询（单次请求；limit 有界 §232）。
//! - §158/§160：hover/hit-testing 全在前端本地，不产生 IPC；IPC 只
//!   发生在批量取数与真选中（§159）。
//! - §165/§166：同 dataset 构建去重；generation 保证旧 build 不覆盖
//!   新 build（§167 迟到者丢弃）。
//! - §186：持久化 temp + 原子替换；损坏 → 清除 + Stale（§187），
//!   不触碰 M3 库（§185）。
//! - §236/§237/§238：构建仅用户显式触发，后台线程执行，不阻塞播放。
//! - §239：绝不使用未完成的 embedding 构建假图（只取 Completed 行）。
//! - 安全：TrackId 参数化；数据全部来自 M3/M8 数据库，路径不经前端。

use std::collections::{BTreeMap, HashMap};
use std::sync::{Arc, Mutex};

use aura_analysis::domain::AnalysisStatus;
use aura_analysis::TrackAnalysisSummary;
use aura_map::clustering::MapCluster;
use aura_map::dataset::{MapDataset, MapEntry};
use aura_map::graph::MapGraph;
use aura_map::layout::MapPosition;
use aura_map::persistence::PersistedMap;
use aura_map::{BuildGeneration, BuildParams, MapBuildError, MapBuildStatus};
use serde::Serialize;
use tauri::State;

use crate::ipc::{from_err, AppError, AppState, CmdResult};

/// 单次视口返回上限（§232 有界内存）。
const NODE_LIMIT_MAX: usize = 20_000;
const NODE_LIMIT_DEFAULT: usize = 5_000;

/// §122/§124/§212–§214：节点渲染/hover/detail 所需元数据——只含
/// M3/M8 真实存在的字段；缺失 = None（UI 隐藏/中性显示 §216）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapNodeDto {
    pub track_id: i64,
    pub x: f32,
    pub y: f32,
    pub cluster_id: usize,
    pub title: Option<String>,
    pub artist: Option<String>,
    pub album: Option<String>,
    pub genre: Option<String>,
    pub bpm: Option<f32>,
    pub musical_key: Option<String>,
    pub energy: Option<f32>,
    pub brightness: Option<f32>,
    pub integrated_lufs: Option<f32>,
}

/// §140/§193：簇概览（centroid/radius 来自布局，可能为 None）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapClusterDto {
    pub cluster_id: usize,
    pub count: usize,
    pub centroid: Option<[f32; 2]>,
    pub radius: Option<f32>,
}

/// §234 状态握手（status 真实反映 §161；无 fake progress §162）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapStatusDto {
    pub status: MapBuildStatus,
    pub dataset_hash: Option<String>,
    pub track_count: usize,
    pub cluster_count: usize,
    pub build_id: u64,
    /// §471/§473：上一次成功构建的真实用时（ms；None = 未构建）。
    pub build_duration_ms: Option<u64>,
    pub message: Option<String>,
}

/// §155 有限包围盒（min_x/min_y/max_x/max_y；空图 → 全零，§139 由
/// 前端 fit 处理 0 节点）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapOverviewDto {
    pub dataset_hash: String,
    pub track_count: usize,
    pub isolated_count: usize,
    pub cluster_count: usize,
    pub bounds: [f32; 4],
    pub clusters: Vec<MapClusterDto>,
    /// §247–§249：真实覆盖计数（unmapped 绝不悄悄排除）。
    pub coverage: MapCoverageDto,
    /// §250：mapped 曲目的真实去重 artist/album 数（M3 来源）。
    pub artists: usize,
    pub albums: usize,
}

/// §247–§249：库 → 地图覆盖（每轨恰入一桶；数字全部来自本次采集）。
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapCoverageDto {
    pub total_tracks: usize,
    pub mapped_tracks: usize,
    pub unmapped_tracks: usize,
    pub stale_tracks: usize,
    pub unsupported_tracks: usize,
    pub missing_embedding_tracks: usize,
}

/// §126：邻居列表（真实 M9 top-K；UI 命名 "Similar Tracks" §127）。
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MapNeighborDto {
    pub track_id: i64,
    pub similarity: f32,
    pub title: Option<String>,
    pub artist: Option<String>,
}

/// 快照期节点元数据（与冻结 dataset 同源；构建线程内组装）。
#[derive(Debug, Clone)]
struct NodeMeta {
    title: Option<String>,
    artist: Option<String>,
    album: Option<String>,
    genre: Option<String>,
    bpm: Option<f32>,
    musical_key: Option<String>,
    energy: Option<f32>,
    brightness: Option<f32>,
    integrated_lufs: Option<f32>,
}

/// 内存快照（§65 derived data：全部可由 M3/M8 重建）。
pub struct MapSnapshot {
    pub dataset: MapDataset,
    pub graph: MapGraph,
    pub clusters: Vec<MapCluster>,
    pub positions: BTreeMap<i64, MapPosition>,
    metas: HashMap<i64, NodeMeta>,
    cluster_of: HashMap<i64, usize>,
    pub dataset_hash: String,
    pub coverage: MapCoverageDto,
    pub artists: usize,
    pub albums: usize,
}

impl MapSnapshot {
    #[allow(clippy::too_many_arguments)]
    fn assemble(
        dataset: MapDataset,
        graph: MapGraph,
        clusters: Vec<MapCluster>,
        positions: BTreeMap<i64, MapPosition>,
        metas: HashMap<i64, NodeMeta>,
        base_coverage: MapCoverageDto,
        collected_entries: usize,
    ) -> Self {
        let mut cluster_of = HashMap::new();
        for c in &clusters {
            for id in &c.track_ids {
                cluster_of.insert(*id, c.cluster_id);
            }
        }
        // §248/§249：mapped 以 dataset 实际收录为准；采集与收录的差 =
        // 不兼容嵌入（§247 unsupported model 语义），unmapped 全量如实。
        let mut coverage = base_coverage;
        coverage.mapped_tracks = dataset.entries.len();
        coverage.unsupported_tracks += collected_entries.saturating_sub(coverage.mapped_tracks);
        coverage.unmapped_tracks = coverage.total_tracks.saturating_sub(coverage.mapped_tracks);
        // §250：mapped 曲目的去重 artist/album（真实频率来源 M3）。
        let mut artists = std::collections::HashSet::new();
        let mut albums = std::collections::HashSet::new();
        for e in &dataset.entries {
            if let Some(m) = metas.get(&e.track_id) {
                if let Some(a) = &m.artist {
                    artists.insert(a.clone());
                }
                if let Some(al) = &m.album {
                    albums.insert(al.clone());
                }
            }
        }
        Self {
            cluster_of,
            dataset_hash: dataset.dataset_hash.clone(),
            dataset,
            graph,
            clusters,
            positions,
            metas,
            coverage,
            artists: artists.len(),
            albums: albums.len(),
        }
    }

    fn bounds(&self) -> [f32; 4] {
        let mut it = self.positions.values();
        let Some(p) = it.next() else {
            return [0.0; 4];
        };
        let mut b = [p.x, p.y, p.x, p.y];
        for p in it {
            b[0] = b[0].min(p.x);
            b[1] = b[1].min(p.y);
            b[2] = b[2].max(p.x);
            b[3] = b[3].max(p.y);
        }
        b
    }

    fn node_dto(&self, track_id: i64, pos: MapPosition) -> MapNodeDto {
        let m = self.metas.get(&track_id);
        MapNodeDto {
            track_id,
            x: pos.x,
            y: pos.y,
            cluster_id: self.cluster_of.get(&track_id).copied().unwrap_or(0),
            title: m.and_then(|m| m.title.clone()),
            artist: m.and_then(|m| m.artist.clone()),
            album: m.and_then(|m| m.album.clone()),
            genre: m.and_then(|m| m.genre.clone()),
            bpm: m.and_then(|m| m.bpm),
            musical_key: m.and_then(|m| m.musical_key.clone()),
            energy: m.and_then(|m| m.energy),
            brightness: m.and_then(|m| m.brightness),
            integrated_lufs: m.and_then(|m| m.integrated_lufs),
        }
    }
}

struct ActiveBuild {
    build_id: u64,
    /// §165 去重键：请求时冻结的 dataset hash。
    dataset_hash: String,
    cancel: Arc<aura_analysis::pipeline::CancelToken>,
}

struct Inner {
    status: MapBuildStatus,
    message: Option<String>,
    snapshot: Option<Arc<MapSnapshot>>,
    persisted: Option<PersistedMap>,
    active: Option<ActiveBuild>,
    last_build_id: u64,
    /// §471：上次成功构建真实用时（ms）。
    last_build_ms: Option<u64>,
    /// 本运行已做过一次 restore 评估（避免每次 status 重扫库）。
    assessed: bool,
}

/// Map 会话宿主（应用层数据集/构建编排；aura-map 保持纯库）。
pub struct MapHost {
    inner: Arc<Mutex<Inner>>,
    generation: Arc<BuildGeneration>,
    data_dir: std::path::PathBuf,
}

fn lock_err() -> AppError {
    from_err("MAP_STATE", "map state lock poisoned", true)
}

fn store_err(e: aura_analysis::store::StoreError) -> AppError {
    from_err("MAP_DB", e, true)
}

impl MapHost {
    /// 打开：读取持久化 map.json（§174）；损坏 → 清除（§187）。
    ///
    /// # Errors
    /// 目录不可用 → [`String`]。
    pub fn open(data_dir: &std::path::Path) -> Result<Self, String> {
        let path = data_dir.join("map").join("map.json");
        let persisted = match std::fs::read(&path) {
            Ok(bytes) => match serde_json::from_slice::<PersistedMap>(&bytes) {
                Ok(p) => Some(p),
                Err(_) => {
                    let _ = std::fs::remove_file(&path);
                    None
                }
            },
            Err(_) => None,
        };
        Ok(Self {
            inner: Arc::new(Mutex::new(Inner {
                status: MapBuildStatus::Pending,
                message: None,
                snapshot: None,
                persisted,
                active: None,
                last_build_id: 0,
                last_build_ms: None,
                assessed: false,
            })),
            generation: Arc::new(BuildGeneration::new()),
            data_dir: data_dir.to_path_buf(),
        })
    }

    fn map_json_path(&self) -> std::path::PathBuf {
        self.data_dir.join("map").join("map.json")
    }

    /// 惰性评估：dataset 未变 → restore；失效/缺失 → Stale/Pending。
    /// 每运行只做一次（后续 status 命中缓存/构建状态）。
    fn ensure_snapshot(
        &self,
        library: &crate::library::LibraryState,
    ) -> Result<Option<Arc<MapSnapshot>>, AppError> {
        {
            let inner = self.inner.lock().map_err(|_| lock_err())?;
            if let Some(s) = &inner.snapshot {
                return Ok(Some(Arc::clone(s)));
            }
            if inner.assessed || inner.active.is_some() {
                return Ok(None);
            }
        }
        let collected = collect_library(library)?;
        let Ok(ds) = MapDataset::build(collected.entries, None) else {
            // 无已分析数据（§239）：保持 Pending，等分析完成。
            let mut inner = self.inner.lock().map_err(|_| lock_err())?;
            inner.assessed = true;
            inner.status = MapBuildStatus::Pending;
            inner.message = Some("no analyzed tracks with embeddings yet".into());
            return Ok(None);
        };
        let mut inner = self.inner.lock().map_err(|_| lock_err())?;
        inner.assessed = true;
        if let Some(s) = &inner.snapshot {
            return Ok(Some(Arc::clone(s)));
        }
        let Some(p) = inner.persisted.take() else {
            return Ok(None); // 从未构建：Pending（等用户首次 Build §236）
        };
        match aura_map::persistence::restore_map(&p, &ds.dataset_hash, &ds) {
            Ok(output) => {
                let snap = Arc::new(MapSnapshot::assemble(
                    output.dataset,
                    output.graph,
                    output.clusters,
                    output.positions,
                    collected.metas,
                    collected.coverage,
                    collected.entries_len,
                ));
                inner.status = MapBuildStatus::Ready;
                inner.message = None;
                inner.snapshot = Some(Arc::clone(&snap));
                Ok(Some(snap))
            }
            Err(reason) => {
                // §175/§187：失效/损坏 → 删除重建（derived data）。
                let _ = std::fs::remove_file(self.map_json_path());
                inner.status = MapBuildStatus::Stale;
                inner.message = Some(reason);
                Ok(None)
            }
        }
    }
}

/// 冻结采集（§243）：library 全量 + M8 Completed 行（嵌入/特征）。
struct Collection {
    entries: Vec<MapEntry>,
    metas: HashMap<i64, NodeMeta>,
    /// §247/§248：采集期真实计数（mapped 之后在 assemble 修正）。
    coverage: MapCoverageDto,
    /// 采集到的嵌入条数（assemble 用它算不兼容差值）。
    entries_len: usize,
}

fn collect_library(library: &crate::library::LibraryState) -> Result<Collection, AppError> {
    let service = library
        .service
        .lock()
        .map_err(|_| from_err("LIBRARY_DB_UNAVAILABLE", "lock", true))?;
    let tracks = service
        .list_tracks(
            aura_library::query::SortKey::Title,
            aura_library::query::Page::new(i64::MAX, 0),
        )
        .map_err(|e| from_err("LIBRARY_DB_UNAVAILABLE", e, true))?;
    let conn = service.database().connection();
    let mut entries = Vec::new();
    let mut metas = HashMap::new();
    let mut coverage = MapCoverageDto::default();
    for t in tracks {
        coverage.total_tracks += 1;
        let analysis =
            aura_analysis::store::AnalysisStore::load_by_track(conn, t.id.0).map_err(store_err)?;
        let Some(a) = analysis else {
            coverage.missing_embedding_tracks += 1; // 未分析（§247 missing）
            continue;
        };
        // §239：只用 Completed 行——未完成的 embedding 不进图。
        match a.status {
            AnalysisStatus::Stale => {
                coverage.stale_tracks += 1;
                continue;
            }
            AnalysisStatus::Unsupported => {
                coverage.unsupported_tracks += 1;
                continue;
            }
            AnalysisStatus::Completed => {}
            _ => {
                coverage.missing_embedding_tracks += 1;
                continue;
            }
        }
        let Some(embedding) = a.embedding.clone() else {
            coverage.missing_embedding_tracks += 1;
            continue;
        };
        let s = TrackAnalysisSummary::from(&a);
        metas.insert(
            t.id.0,
            NodeMeta {
                title: t.title.clone(),
                artist: t.artist.clone(),
                album: t.album.clone(),
                genre: t.genre.clone(),
                bpm: s.bpm,
                musical_key: s.musical_key,
                energy: s.energy,
                brightness: s.brightness,
                integrated_lufs: s.integrated_lufs,
            },
        );
        entries.push(MapEntry {
            track_id: t.id.0,
            content_identity: t.identity_key.clone(),
            analysis_key: a.analysis_key.clone(),
            embedding,
        });
    }
    let entries_len = entries.len();
    Ok(Collection {
        entries,
        metas,
        coverage,
        entries_len,
    })
}

/// §186 temp + 原子替换。
fn write_atomic(path: &std::path::Path, persisted: &PersistedMap) {
    let Some(dir) = path.parent() else { return };
    let _ = std::fs::create_dir_all(dir);
    let Ok(json) = serde_json::to_vec_pretty(persisted) else {
        return;
    };
    let tmp = path.with_extension("json.tmp");
    if std::fs::write(&tmp, &json).is_ok() {
        let _ = std::fs::rename(&tmp, path);
    }
}

fn snapshot_or_err(
    host: &MapHost,
    state: &State<'_, AppState>,
) -> Result<Arc<MapSnapshot>, AppError> {
    host.ensure_snapshot(&state.library)?.ok_or_else(|| {
        from_err(
            "MAP_NOT_READY",
            "map is not built yet (see get_map_status)",
            true,
        )
    })
}

/// §234：状态握手（首次调用触发一次 restore 评估）。
#[tauri::command]
pub fn get_map_status(state: State<'_, AppState>) -> CmdResult<MapStatusDto> {
    let host = &state.map;
    // 评估失败（Stale/Pending）不是命令错误——状态里如实报告。
    let _ = host.ensure_snapshot(&state.library)?;
    let inner = host.inner.lock().map_err(|_| lock_err())?;
    Ok(match &inner.snapshot {
        Some(s) => MapStatusDto {
            status: inner.status,
            dataset_hash: Some(s.dataset_hash.clone()),
            track_count: s.dataset.entries.len(),
            cluster_count: s.clusters.len(),
            build_id: inner.last_build_id,
            build_duration_ms: inner.last_build_ms,
            message: inner.message.clone(),
        },
        None => MapStatusDto {
            status: inner.status,
            dataset_hash: None,
            track_count: 0,
            cluster_count: 0,
            build_id: inner.last_build_id,
            build_duration_ms: inner.last_build_ms,
            message: inner.message.clone(),
        },
    })
}

/// §144/§229/§230：概览 + 全簇（打开 Map 时一次）。
#[tauri::command]
pub fn get_map_overview(state: State<'_, AppState>) -> CmdResult<MapOverviewDto> {
    let host = &state.map;
    let snap = snapshot_or_err(host, &state)?;
    Ok(MapOverviewDto {
        dataset_hash: snap.dataset_hash.clone(),
        track_count: snap.dataset.entries.len(),
        isolated_count: snap.graph.isolated.len(),
        cluster_count: snap.clusters.len(),
        bounds: snap.bounds(),
        coverage: snap.coverage,
        artists: snap.artists,
        albums: snap.albums,
        clusters: snap
            .clusters
            .iter()
            .map(|c: &MapCluster| MapClusterDto {
                cluster_id: c.cluster_id,
                count: c.track_ids.len(),
                centroid: c.centroid.map(|(x, y)| [x, y]),
                radius: c.radius,
            })
            .collect(),
    })
}

/// §141/§229/§230：视口批量节点（非法视口 → 全图缺省，不崩溃 §192）。
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub fn get_map_nodes(
    state: State<'_, AppState>,
    min_x: Option<f32>,
    min_y: Option<f32>,
    max_x: Option<f32>,
    max_y: Option<f32>,
    limit: Option<usize>,
) -> CmdResult<Vec<MapNodeDto>> {
    let host = &state.map;
    let snap = snapshot_or_err(host, &state)?;
    let (min_x, max_x) = norm_axis(min_x, max_x);
    let (min_y, max_y) = norm_axis(min_y, max_y);
    let limit = limit.unwrap_or(NODE_LIMIT_DEFAULT).clamp(1, NODE_LIMIT_MAX);
    Ok(
        aura_map::query::viewport_nodes(&snap.positions, min_x, min_y, max_x, max_y)
            .into_iter()
            .take(limit)
            .map(|(id, p)| snap.node_dto(id, p))
            .collect(),
    )
}

/// 非法/NaN 轴 → 全图 [-1,1]（§192/§181 精神）。
fn norm_axis(min: Option<f32>, max: Option<f32>) -> (f32, f32) {
    let (a, b) = (min.unwrap_or(-1.0), max.unwrap_or(1.0));
    if !a.is_finite() || !b.is_finite() || a > b {
        return (-1.0, 1.0);
    }
    (a.clamp(-1.0, 1.0), b.clamp(-1.0, 1.0))
}

/// §228：全簇（颜色/图例/fit cluster §140/§211）。
#[tauri::command]
pub fn get_map_clusters(state: State<'_, AppState>) -> CmdResult<Vec<MapClusterDto>> {
    let host = &state.map;
    let snap = snapshot_or_err(host, &state)?;
    Ok(snap
        .clusters
        .iter()
        .map(|c: &MapCluster| MapClusterDto {
            cluster_id: c.cluster_id,
            count: c.track_ids.len(),
            centroid: c.centroid.map(|(x, y)| [x, y]),
            radius: c.radius,
        })
        .collect())
}

/// §126/§159：真选中时取 top-K 邻居（复用 M9 内核，k 有界）。
#[tauri::command]
pub fn get_map_neighbors(
    state: State<'_, AppState>,
    track_id: i64,
    k: Option<usize>,
) -> CmdResult<Vec<MapNeighborDto>> {
    let host = &state.map;
    let snap = snapshot_or_err(host, &state)?;
    let k = k.unwrap_or(8).clamp(1, 50);
    let ranked = aura_map::query::neighbors(&snap.dataset, &snap.graph, track_id, k)
        .map_err(|e| from_err("MAP_NEIGHBORS", e, true))?;
    Ok(ranked
        .into_iter()
        .map(|(id, sim)| {
            let m = snap.metas.get(&id);
            MapNeighborDto {
                track_id: id,
                similarity: sim,
                title: m.and_then(|m| m.title.clone()),
                artist: m.and_then(|m| m.artist.clone()),
            }
        })
        .collect())
}

/// §124：单曲 detail（来自快照缓存，不触发分析 §125）。
#[tauri::command]
pub fn get_map_track(state: State<'_, AppState>, track_id: i64) -> CmdResult<Option<MapNodeDto>> {
    let host = &state.map;
    let snap = snapshot_or_err(host, &state)?;
    Ok(snap
        .positions
        .get(&track_id)
        .map(|p| snap.node_dto(track_id, *p)))
}

/// §236：用户显式构建（同 dataset 去重 §165；generation §166）。
#[tauri::command]
pub fn request_map_build(state: State<'_, AppState>) -> CmdResult<u64> {
    let host = &state.map;
    // §243：请求时冻结 dataset（含 hash）。
    let collected = collect_library(&state.library)?;
    if collected.entries.is_empty() {
        return Err(from_err(
            "MAP_NO_DATA",
            "no analyzed tracks with embeddings yet",
            true,
        ));
    }
    let (ds, hash) = {
        let ds = MapDataset::build(collected.entries, None)
            .map_err(|e| from_err("MAP_NO_DATA", e, true))?;
        let hash = ds.dataset_hash.clone();
        (ds, hash)
    };
    let mut inner = host.inner.lock().map_err(|_| lock_err())?;
    // §168/§171/§172：变化比例选增量/全量；增量时以上一代位置为种子
    // （§82–§85/§176/§177 空间记忆），迭代预算收紧。
    let (seeds, params) = match &inner.snapshot {
        Some(snap) => {
            let old_ids: std::collections::HashSet<i64> =
                snap.dataset.entries.iter().map(|e| e.track_id).collect();
            let total = ds.entries.len();
            let added = ds
                .entries
                .iter()
                .filter(|e| !old_ids.contains(&e.track_id))
                .count();
            let removed = old_ids
                .iter()
                .filter(|id| snap.positions.contains_key(id))
                .count()
                .saturating_sub(total.saturating_sub(added));
            let changed = added.max(removed);
            match aura_map::choose_build_kind(changed, total) {
                aura_map::BuildKind::IncrementalUpdate => (
                    Some(snap.positions.clone()),
                    BuildParams {
                        max_iterations: 40,
                        ..BuildParams::default()
                    },
                ),
                aura_map::BuildKind::FullRebuild => (None, BuildParams::default()),
            }
        }
        None => (None, BuildParams::default()),
    };
    if let Some(a) = &inner.active {
        if a.dataset_hash == hash && !a.cancel.is_cancelled() {
            return Ok(a.build_id); // §165 相同 dataset 去重
        }
        a.cancel.cancel(); // §166 取代旧 build
    }
    let build_id = host.generation.next();
    let cancel = Arc::new(aura_analysis::pipeline::CancelToken::new());
    inner.status = MapBuildStatus::Building;
    // §162：真实阶段描述，不伪造百分比。
    inner.message = Some("building similarity graph…".into());
    inner.last_build_id = build_id;
    inner.active = Some(ActiveBuild {
        build_id,
        dataset_hash: hash.clone(),
        cancel: Arc::clone(&cancel),
    });
    drop(inner);

    let gen = Arc::clone(&host.generation);
    let inner_arc = Arc::clone(&host.inner);
    let metas = collected.metas;
    let coverage = collected.coverage;
    let entries_len = collected.entries_len;
    let json_path = host.map_json_path();
    std::thread::Builder::new()
        .name("map-build".into())
        .spawn(move || {
            // §238：后台低优先（独立线程；audio 线程不受影响）。
            // §471/§473：真实构建用时（不伪造）。
            let started = std::time::Instant::now();
            let out =
                aura_map::build_map(&ds, &hash, params, build_id, &gen, &cancel, seeds.as_ref());
            let build_ms = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
            let mut inner = match inner_arc.lock() {
                Ok(i) => i,
                Err(_) => return,
            };
            if !gen.is_current(build_id) {
                return; // §167 迟到者不得覆盖新 build
            }
            match out {
                Ok(o) => {
                    if o.status == MapBuildStatus::Ready {
                        let snap = Arc::new(MapSnapshot::assemble(
                            o.dataset,
                            o.graph,
                            o.clusters,
                            o.positions,
                            metas,
                            coverage,
                            entries_len,
                        ));
                        write_atomic(&json_path, &o.persisted);
                        inner.snapshot = Some(snap);
                        inner.message = None;
                        inner.last_build_ms = Some(build_ms);
                    } else {
                        // §245/§246：Stale 数据不标记 Ready。
                        inner.message = Some("dataset changed during build".into());
                    }
                    inner.status = o.status;
                }
                Err(MapBuildError::Cancelled) => {
                    inner.status = MapBuildStatus::Cancelled;
                    inner.message = Some("build cancelled".into());
                }
                Err(e) => {
                    inner.status = MapBuildStatus::Failed;
                    inner.message = Some(e.to_string());
                }
            }
            if inner
                .active
                .as_ref()
                .is_some_and(|a| a.build_id == build_id)
            {
                inner.active = None;
            }
        })
        .map_err(|e| from_err("MAP_BUILD_SPAWN", e, true))?;
    Ok(build_id)
}

/// §164：取消当前构建（构建完成后到达的取消不影响 Ready——如实）。
#[tauri::command]
pub fn cancel_map_build(state: State<'_, AppState>) -> CmdResult<()> {
    let mut inner = state.map.inner.lock().map_err(|_| lock_err())?;
    if let Some(a) = &inner.active {
        a.cancel.cancel();
        inner.status = MapBuildStatus::Cancelled;
        inner.message = Some("build cancelled".into());
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;

    /// §285/§187：损坏的 map.json → open() 不 crash、清除文件、状态 Pending。
    #[test]
    fn corrupt_map_json_removed_on_open() {
        let dir = std::env::temp_dir().join(format!("aura-map-test-{}", std::process::id()));
        let map_dir = dir.join("map");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::create_dir_all(&map_dir);
        std::fs::write(map_dir.join("map.json"), b"{ not json at all").unwrap();

        let host = MapHost::open(&dir).expect("open 不因损坏失败");
        let inner = host.inner.lock().unwrap();
        assert!(inner.persisted.is_none(), "损坏持久化被清除");
        assert_eq!(inner.status, MapBuildStatus::Pending);
        drop(inner);
        assert!(!map_dir.join("map.json").exists(), "损坏文件已删除");

        // 无文件 → 正常打开（首次运行）。
        let host2 = MapHost::open(&dir).unwrap();
        let inner2 = host2.inner.lock().unwrap();
        assert!(inner2.persisted.is_none());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
