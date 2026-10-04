//! ifuyo Aura Music Map 内核（M9 上篇，§0–108）。
//!
//! 职责（§13）：M8 Analysis Dataset → Compatibility Check → Vector
//! Index → Similarity → kNN Graph → Clusters → Spatial Layout →
//! Map Dataset。不含 audio playback/WebGL/Tauri/HTTP/Dreamscape。
//!
//! 确定性合同（§7/§26/§27/§100）：同 dataset + 同算法版本 + 同参数 →
//! 同图/同聚类/同位置。空间关系全部来自真实 M8 embedding + 相似度
//! 度量 + 布局算法（§7 禁 random/hash 位置）。

pub mod build;
pub mod clustering;
pub mod dataset;
pub mod graph;
pub mod layout;
pub mod persistence;
pub mod query;
pub mod similarity;

pub use build::{
    build_map, choose_build_kind, freeze_snapshot, BuildGeneration, BuildKind, BuildParams,
    MapBuildError, MapBuildOutput, MapBuildStatus, FULL_REBUILD_THRESHOLD,
};
pub use clustering::{attach_spatial_stats, detect_clusters, MapCluster, CLUSTER_ALGO_VERSION};
pub use dataset::{DatasetError, MapDataset, MapEntry, DATASET_ALGO_VERSION, MAP_ALGO_VERSION};
pub use graph::{build_knn_graph, validate_graph, MapEdge, MapGraph, MAX_EDGES_FACTOR};
pub use layout::{
    compute_layout, neighbor_centroid, MapPosition, LAYOUT_ALGO_VERSION, MAX_ITERATIONS, MAX_NODES,
};
pub use similarity::{top_k_neighbors, SimilarityError, DEFAULT_K, MAX_K};
