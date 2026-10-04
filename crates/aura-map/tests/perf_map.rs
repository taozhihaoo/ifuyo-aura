//! M9 性能基线（§44/§58/§76–§78 + 下篇 §295–§298/§474–§478）。
//! `#[ignore]` 显式运行：
//! `cargo test -p aura-map --test perf_map -- --ignored --nocapture`

use std::time::Instant;

use aura_analysis::domain::{Embedding, EMBEDDING_MODEL_ID};
use aura_map::clustering::detect_clusters;
use aura_map::dataset::{MapDataset, MapEntry};
use aura_map::persistence::{persist, restore_map};
use aura_map::query::{neighbors, viewport_nodes};
use aura_map::{build_knn_graph, compute_layout, layout_seeded, validate_graph};

fn emb(v: Vec<f32>) -> Embedding {
    Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
}

/// 确定性合成数据集：4 个方向簇（34 维 = M8 嵌入实际维度；xorshift
/// 确定性微扰 §270/§387——非生产随机源）。
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

/// §297 全链分段 + §291/§295/§478 内存核算。
fn bench_full(n: usize) {
    println!("==== N={n} ====");
    let dim = 34usize;

    let t0 = Instant::now();
    let ds = synthetic(n);
    println!("dataset build     : {:?}", t0.elapsed());

    let t = Instant::now();
    let g = build_knn_graph(&ds, 12, 0.0).unwrap();
    println!(
        "kNN graph (exact) : {:?} (edges={})",
        t.elapsed(),
        g.edges.len()
    );
    assert!(g.edges.len() <= ds.entries.len() * 24, "§58 边预算");

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
    assert_eq!(layout.positions.len(), n);

    let t = Instant::now();
    let persisted = persist(&ds, &g, &clusters.clusters, &layout.positions);
    println!("persist (serde)   : {:?}", t.elapsed());

    let t = Instant::now();
    let restored = restore_map(&persisted, &ds.dataset_hash, &ds).unwrap();
    println!("restore (no recompute): {:?}", t.elapsed());
    assert_eq!(restored.positions, layout.positions);

    // §298 查询：top-k / viewport。
    let probe = ds.entries[n / 2].track_id;
    let t = Instant::now();
    let rounds = 100;
    for _ in 0..rounds {
        let _ = neighbors(&ds, &g, probe, 8).unwrap();
    }
    println!(
        "top-k neighbor x{rounds}  : {:?} ({:?}/call)",
        t.elapsed(),
        t.elapsed() / rounds
    );

    let t = Instant::now();
    let _ = viewport_nodes(&layout.positions, -0.5, -0.5, 0.5, 0.5);
    println!("viewport query    : {:?}", t.elapsed());

    // §277 增量：旧位置作种子（40 迭代）。
    let t = Instant::now();
    let _ = layout_seeded(&ds, &g, Some(&layout.positions), 40).unwrap();
    println!("incremental relax (40 iter, all seeded): {:?}", t.elapsed());

    // §291/§295/§478 内存核算（结构账面；不含 allocator 开销）。
    let vectors = n * dim * 4;
    let edges = g.edges.len() * 20; // (i64,i64,f32)
    let positions = n * 8; // (i64 key + 2×f32)≈16，保守按 BTreeMap 值 8
    let clusters_mem = clusters.clusters.len() * 24 + g.nodes.len() * 4;
    println!(
        "memory account    : vectors={vectors}B ({:.1}MB) edges={edges}B positions≈{positions}B clusters≈{clusters_mem}B",
        vectors as f64 / 1_048_576.0
    );
}

#[test]
#[ignore = "perf baseline；显式运行"]
fn map_build_perf_4k_nodes() {
    bench_full(4_000);
}

#[test]
#[ignore = "perf §476；显式运行"]
fn map_build_perf_1k_nodes() {
    bench_full(1_000);
}

#[test]
#[ignore = "perf §476；显式运行"]
fn map_build_perf_10k_nodes() {
    bench_full(10_000);
}

#[test]
#[ignore = "§290/§393：超 25k 节点明确报错（B 级边界，非伪造）"]
fn large_dataset_100k_rejected() {
    let n = 25_001;
    let ds = synthetic(n);
    // §44/§46：exact scan 超 25k → DatasetTooLarge（ANN=B 级未引入）。
    let err = build_knn_graph(&ds, 4, 0.0).unwrap_err();
    assert!(
        matches!(err, aura_map::SimilarityError::DatasetTooLarge { .. }),
        "超预算必须显式报错：{err:?}"
    );
    println!(
        "25k+ dataset → graph rejected（DatasetTooLarge）；B 级 ANN/progressive 未引入，如实拒绝"
    );
}
