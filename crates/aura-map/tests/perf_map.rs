//! M9 性能基线（§44/§58/§76–§78）。
//! `#[ignore]` 显式运行：
//! `cargo test -p aura-map --test perf_map -- --ignored --nocapture`

use std::time::Instant;

use aura_analysis::domain::Embedding;
use aura_analysis::domain::EMBEDDING_MODEL_ID;
use aura_map::dataset::MapEntry;
use aura_map::{build_knn_graph, compute_layout, detect_clusters, validate_graph, MapDataset};

fn emb(v: Vec<f32>) -> Embedding {
    Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
}

/// 确定性合成数据集：4 个方向簇（4000 tracks，34 维 = M8 嵌入实际维度）。
fn synthetic(n: usize) -> MapDataset {
    let dirs: [[f32; 34]; 4] = {
        let mut d = [[0.0f32; 34]; 4];
        for (i, dd) in d.iter_mut().enumerate() {
            dd[i * 8] = 1.0;
        }
        d
    };
    let mut seed = 0x9E37_79B9_7F4A_7C15u64;
    let entries: Vec<MapEntry> = (0..n)
        .map(|i| {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let jitter = (seed >> 44) as f32 * 0.002;
            let dir = dirs[i % 4];
            let v: Vec<f32> = dir.iter().map(|d| d + jitter).collect();
            MapEntry {
                track_id: i as i64,
                content_identity: format!("c{i}"),
                analysis_key: format!("k{i}"),
                embedding: emb(v),
            }
        })
        .collect();
    MapDataset::build(entries, None).unwrap()
}

#[test]
#[ignore = "perf baseline；显式运行"]
fn map_build_perf_4k_nodes() {
    let t0 = Instant::now();
    let ds = synthetic(4_000);
    println!("dataset build     : {:?}", t0.elapsed());

    let t = Instant::now();
    let g = build_knn_graph(&ds, 12, 0.0).unwrap();
    println!(
        "kNN graph (N=4k)  : {:?} (edges={})",
        t.elapsed(),
        g.edges.len()
    );
    assert!(g.edges.len() <= ds.entries.len() * 32, "§58 边预算");

    let t = Instant::now();
    let clusters = detect_clusters(&g, 20);
    println!(
        "clustering        : {:?} (clusters={})",
        t.elapsed(),
        clusters.clusters.len()
    );

    let t = Instant::now();
    let layout = compute_layout(&ds, &g).unwrap();
    println!("layout (150 iter) : {:?}", t.elapsed());
    assert!(validate_graph(&g, 12).is_ok());
    assert_eq!(layout.positions.len(), 4_000);
}
