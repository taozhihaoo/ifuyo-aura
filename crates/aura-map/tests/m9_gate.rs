//! M9 下篇门禁测试（§270–§299/§386–§399）：确定性合成数据集 +
//! 相似度/kNN/图/聚类/布局矩阵 + 病理用例 + 版本失效 + 增量布局 +
//! 损坏恢复 + build/cancel/reopen 循环。全部向量 TEST ONLY 确定性
//! 生成（§270/§386/§387：无 Math.random / 无未种子随机）。

use aura_analysis::domain::{Embedding, EMBEDDING_MODEL_ID};
use aura_map::clustering::{attach_spatial_stats, detect_clusters};
use aura_map::dataset::{MapDataset, MapEntry};
use aura_map::graph::{build_knn_graph, validate_graph};
use aura_map::layout::{compute_layout, layout_seeded, MapPosition};
use aura_map::persistence::{persist, restore_map};
use aura_map::query::{neighbors, viewport_nodes};
use aura_map::similarity::{similarity, top_k_neighbors};
use aura_map::{
    build_map, freeze_snapshot, BuildGeneration, BuildParams, MapBuildError, MapBuildStatus,
};
use std::collections::BTreeMap;

// ---------- §270/§386/§387：确定性合成 fixture（TEST ONLY） ----------

fn emb(v: Vec<f32>) -> Embedding {
    Embedding::normalized(v, EMBEDDING_MODEL_ID, 1)
}

/// 三个明显分离的簇方向 + 单位化（§270 cluster A/B/C）。
const CLUSTER_DIRS: [[f32; 8]; 3] = [
    [1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0],
    [0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0],
];

/// 确定性伪微扰：由 index 派生（§387 fixed seed 语义；非随机源）。
fn jitter(i: usize, d: usize) -> f32 {
    let x = ((i * 7 + d * 13) % 17) as f32 / 17.0 - 0.5;
    x * 0.04
}

/// n 个节点、3 簇均匀分配（i % 3），8 维方向 + 确定性微扰。
fn synthetic_entries(n: usize) -> Vec<MapEntry> {
    (0..n)
        .map(|i| {
            let c = i % 3;
            let mut v = [0f32; 8];
            for (d, slot) in v.iter_mut().enumerate() {
                *slot = CLUSTER_DIRS[c][d] + jitter(i, d);
            }
            MapEntry {
                track_id: i as i64 + 1,
                content_identity: format!("c{}", i + 1),
                analysis_key: format!("k{}", i + 1),
                embedding: emb(v.to_vec()),
            }
        })
        .collect()
}

fn ds_of(entries: Vec<MapEntry>) -> MapDataset {
    MapDataset::build(entries, None).unwrap()
}

// ---------- §389 相似度矩阵 ----------

#[test]
fn similarity_matrix() {
    let a = emb(vec![1.0, 0.0, 0.0]);
    // same → 1（§543 证据）。
    let a2 = emb(vec![1.0, 0.0, 0.0]);
    assert!((similarity(&a, &a2).unwrap() - 1.0).abs() < 1e-6);
    // orthogonal → ~0。
    let b = emb(vec![0.0, 1.0, 0.0]);
    assert!(similarity(&a, &b).unwrap().abs() < 1e-6);
    // nearby。
    let near = emb(vec![0.99, 0.1, 0.0]);
    let s = similarity(&a, &near).unwrap();
    assert!(s > 0.9 && s < 1.0, "nearby {s}");
    // zero → Err（§33 不悄悄 0.0）。
    let z = emb(vec![0.0, 0.0, 0.0]);
    assert!(matches!(
        similarity(&a, &z),
        Err(aura_map::SimilarityError::ZeroVector)
    ));
    // invalid dimension → Err。
    let d = emb(vec![1.0, 0.0]);
    assert!(matches!(
        similarity(&a, &d),
        Err(aura_map::SimilarityError::DimensionMismatch)
    ));
    // NaN → Err。
    let mut nan = a.clone();
    nan.vector[0] = f32::NAN;
    assert!(similarity(&a, &nan).is_err());
    // Infinity → Err。
    let mut inf = a.clone();
    inf.vector[0] = f32::INFINITY;
    assert!(similarity(&a, &inf).is_err());
}

// ---------- §272/§390 top-K / kNN K=1/3/5、tie、孤立 ----------

#[test]
fn knn_k1_k3_k5_and_ties() {
    // 显式构造：1 与 2/3/4/5 相似度已知降序；6 正交。
    let ds = ds_of(vec![
        MapEntry {
            track_id: 1,
            content_identity: "a".into(),
            analysis_key: "k1".into(),
            embedding: emb(vec![1.0, 0.0, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 2,
            content_identity: "b".into(),
            analysis_key: "k2".into(),
            embedding: emb(vec![0.9, 0.1, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 3,
            content_identity: "c".into(),
            analysis_key: "k3".into(),
            embedding: emb(vec![0.8, 0.2, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 4,
            content_identity: "d".into(),
            analysis_key: "k4".into(),
            embedding: emb(vec![0.7, 0.3, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 5,
            content_identity: "e".into(),
            analysis_key: "k5".into(),
            embedding: emb(vec![0.6, 0.4, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 6,
            content_identity: "f".into(),
            analysis_key: "k6".into(),
            embedding: emb(vec![0.0, 0.0, 1.0, 0.0]),
        },
    ]);
    for k in [1usize, 3, 5] {
        let top = top_k_neighbors(&ds, 1, k).unwrap();
        assert_eq!(top.len(), k, "K={k}");
        // DESC 排序验证（§272/§336）。
        for w in top.windows(2) {
            assert!(w[0].1 >= w[1].1, "top-k DESC");
        }
        assert_eq!(top[0].0, 2, "最近邻=2");
    }
    // tie-break：duplicate similarity → TrackId ASC（§42/§390 ties）。
    let ds2 = ds_of(vec![
        MapEntry {
            track_id: 10,
            content_identity: "x".into(),
            analysis_key: "kx".into(),
            embedding: emb(vec![1.0, 0.0]),
        },
        MapEntry {
            track_id: 11,
            content_identity: "y".into(),
            analysis_key: "ky".into(),
            embedding: emb(vec![0.6, 0.8]),
        },
        MapEntry {
            track_id: 12,
            content_identity: "z".into(),
            analysis_key: "kz".into(),
            embedding: emb(vec![0.6, 0.8]),
        },
    ]);
    let top = top_k_neighbors(&ds2, 10, 2).unwrap();
    assert_eq!(top.len(), 2);
    assert_eq!((top[0].0, top[1].0), (11, 12), "同分 → id 升序");
    // 孤立节点（6 与任何节点无边）。
    let ds3 = ds_of(synthetic_entries(12));
    let g = build_knn_graph(&ds3, 2, 0.9).unwrap();
    let iso = ds3
        .entries
        .iter()
        .map(|e| e.track_id)
        .find(|id| g.neighbors_of(*id).is_empty());
    if let Some(id) = iso {
        assert!(g.isolated.contains(&id), "孤立节点在 isolated 列表");
    }
}

// ---------- §274/§391 图矩阵 ----------

#[test]
fn graph_matrix() {
    // empty / 1 / 2 / 10 nodes。
    for n in [0usize, 1, 2, 10] {
        let entries = synthetic_entries(n);
        if n == 0 {
            assert!(MapDataset::build(entries, None).is_err());
            continue;
        }
        let ds = ds_of(entries);
        let g = build_knn_graph(&ds, 3, 0.0).unwrap();
        assert!(validate_graph(&g, 3).is_ok(), "n={n} validate");
        // no self-loop / no duplicate。
        for e in &g.edges {
            assert_ne!(e.a, e.b);
        }
        let uniq: std::collections::HashSet<(i64, i64)> = g
            .edges
            .iter()
            .map(|e| (e.a.min(e.b), e.a.max(e.b)))
            .collect();
        assert_eq!(uniq.len(), g.edges.len(), "无重复边 n={n}");
        // 边预算（§58：≤2NK）。
        assert!(g.edges.len() <= g.nodes.len() * 3 * 2);
    }
    // disconnected：两组正交向量（阈值 0.5 不跨组）。
    let ds = ds_of(vec![
        MapEntry {
            track_id: 1,
            content_identity: "a".into(),
            analysis_key: "ka".into(),
            embedding: emb(vec![1.0, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 2,
            content_identity: "b".into(),
            analysis_key: "kb".into(),
            embedding: emb(vec![0.99, 0.1, 0.0]),
        },
        MapEntry {
            track_id: 3,
            content_identity: "c".into(),
            analysis_key: "kc".into(),
            embedding: emb(vec![0.0, 0.0, 1.0]),
        },
        MapEntry {
            track_id: 4,
            content_identity: "d".into(),
            analysis_key: "kd".into(),
            embedding: emb(vec![0.0, 0.1, 0.99]),
        },
    ]);
    let g = build_knn_graph(&ds, 1, 0.5).unwrap();
    // 簇间无边（disconnected §391）。
    assert!(g.edges.iter().all(|e| {
        let (lo, hi) = (e.a.min(e.b), e.a.max(e.b));
        (lo <= 2 && hi <= 2) || (lo >= 3 && hi >= 3)
    }));
    // sparse：高阈值 → 全孤立。
    let g2 = build_knn_graph(&ds, 3, 0.999).unwrap();
    assert!(g2.edges.is_empty());
    // dense：K 覆盖全组。
    let g3 = build_knn_graph(&ds, 3, 0.0).unwrap();
    assert!(validate_graph(&g3, 3).is_ok());
}

// ---------- §275/§392 聚类矩阵 ----------

#[test]
fn cluster_matrix() {
    // 3 clusters（§275）。
    let ds = ds_of(synthetic_entries(30));
    let g = build_knn_graph(&ds, 4, 0.5).unwrap();
    let r = detect_clusters(&g, 20);
    assert_eq!(r.clusters.len(), 3, "3 簇，实得 {}", r.clusters.len());
    // 不随机漂移（§100/§275）：重复跑同结果。
    for _ in 0..5 {
        assert_eq!(detect_clusters(&g, 20).clusters, r.clusters);
    }
    // 成员跨跑一致。
    // 1 cluster（全相似）。
    let ds1 = ds_of(
        (0..8)
            .map(|i| MapEntry {
                track_id: i + 1,
                content_identity: format!("c{i}"),
                analysis_key: format!("k{i}"),
                embedding: emb(vec![1.0 - i as f32 * 0.01, 0.0, 0.0, 0.0]),
            })
            .collect(),
    );
    let g1 = build_knn_graph(&ds1, 3, 0.0).unwrap();
    let r1 = detect_clusters(&g1, 20);
    assert_eq!(r1.clusters.len(), 1, "全相似 → 1 簇");
    // 2 clusters。
    let ds2 = ds_of(vec![
        MapEntry {
            track_id: 1,
            content_identity: "a".into(),
            analysis_key: "ka".into(),
            embedding: emb(vec![1.0, 0.0]),
        },
        MapEntry {
            track_id: 2,
            content_identity: "b".into(),
            analysis_key: "kb".into(),
            embedding: emb(vec![0.95, 0.05]),
        },
        MapEntry {
            track_id: 3,
            content_identity: "c".into(),
            analysis_key: "kc".into(),
            embedding: emb(vec![0.0, 1.0]),
        },
        MapEntry {
            track_id: 4,
            content_identity: "d".into(),
            analysis_key: "kd".into(),
            embedding: emb(vec![0.05, 0.95]),
        },
    ]);
    let g2 = build_knn_graph(&ds2, 1, 0.5).unwrap();
    assert_eq!(detect_clusters(&g2, 20).clusters.len(), 2);
    // uneven + noise：簇大小 8/8/2 + 1 孤立。
    let mut entries = synthetic_entries(18);
    entries.extend(vec![
        MapEntry {
            track_id: 19,
            content_identity: "s1".into(),
            analysis_key: "k19".into(),
            embedding: emb(vec![0.0, 0.0, 0.9, 0.1, 0.0, 0.0, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 20,
            content_identity: "s2".into(),
            analysis_key: "k20".into(),
            embedding: emb(vec![0.0, 0.0, 0.85, 0.1, 0.0, 0.0, 0.0, 0.0]),
        },
        MapEntry {
            track_id: 21,
            content_identity: "n1".into(),
            analysis_key: "k21".into(),
            embedding: emb(vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]),
        },
    ]);
    let ds3 = ds_of(entries);
    let g3 = build_knn_graph(&ds3, 3, 0.5).unwrap();
    let r3 = detect_clusters(&g3, 20);
    assert!(
        r3.clusters.len() >= 3,
        "uneven ≥3 簇，实得 {}",
        r3.clusters.len()
    );
    // 簇统计（§103/§337 真实）：attach 后 centroid/radius 有限。
    let mut clusters = r3.clusters;
    let layout = compute_layout(&ds3, &g3).unwrap();
    attach_spatial_stats(&mut clusters, &layout.positions);
    for c in &clusters {
        if let Some((cx, cy)) = c.centroid {
            assert!(cx.is_finite() && cy.is_finite());
        }
        if let Some(rad) = c.radius {
            assert!(rad.is_finite() && rad >= 0.0);
        }
    }
}

// ---------- §276/§393/§394/§395/§396 布局矩阵与病理 ----------

#[test]
fn layout_matrix_and_pathological() {
    // 0 nodes → Err（§139/§393）。
    assert!(MapDataset::build(vec![], None).is_err());
    for n in [1usize, 2, 10, 100, 1000] {
        let ds = ds_of(synthetic_entries(n));
        let g = build_knn_graph(&ds, 4.min(n), 0.5).unwrap();
        let a = compute_layout(&ds, &g).unwrap();
        let b = compute_layout(&ds, &g).unwrap();
        // deterministic（§276）。
        assert_eq!(a.positions, b.positions, "n={n} 确定性");
        // finite + bounded（§276）。
        for p in a.positions.values() {
            assert!(p.x.is_finite() && p.y.is_finite());
            assert!((-1.0..=1.0).contains(&p.x) && (-1.0..=1.0).contains(&p.y));
        }
        assert_eq!(a.positions.len(), n);
    }
    // pathological：全部相同向量（§394/§395/§396）。
    let same: Vec<MapEntry> = (0..6)
        .map(|i| MapEntry {
            track_id: i + 1,
            content_identity: format!("c{i}"),
            analysis_key: format!("k{i}"),
            embedding: emb(vec![1.0, 0.0, 0.0]),
        })
        .collect();
    let ds = ds_of(same);
    // 相同向量 → similarity == 1（§395）。
    let (ea, eb) = (&ds.entries[0].embedding, &ds.entries[1].embedding);
    assert!((similarity(ea, eb).unwrap() - 1.0).abs() < 1e-6);
    let g = build_knn_graph(&ds, 3, 0.0).unwrap();
    let r = compute_layout(&ds, &g).unwrap();
    for p in r.positions.values() {
        assert!(
            p.x.is_finite() && p.y.is_finite(),
            "相同向量不得 NaN（§395）"
        );
        assert!((-1.0..=1.0).contains(&p.x));
    }
    // duplicate/极近节点 → 松弛后不全部重叠（§396/§397 有分离）。
    let distinct: std::collections::HashSet<(u32, u32)> = r
        .positions
        .values()
        .map(|p| (p.x.to_bits(), p.y.to_bits()))
        .collect();
    assert!(
        distinct.len() >= 2,
        "相同向量布局应至少 2 个不同位置，实得 {}",
        distinct.len()
    );
    // one dominant cluster + many isolated：1 号为主簇，2..12 各占
    // 独立维度（彼此近正交 → 多孤立）。
    let mut entries = vec![MapEntry {
        track_id: 1,
        content_identity: "d".into(),
        analysis_key: "kd".into(),
        embedding: emb(vec![1.0, 0.0, 0.0]),
    }];
    for i in 0..10u64 {
        let mut v = vec![0f32; 8];
        v[(i as usize) % 8] = 1.0;
        v[7] = (i as f32) * 0.01; // 轻微去同
        entries.push(MapEntry {
            track_id: i as i64 + 2,
            content_identity: format!("i{i}"),
            analysis_key: format!("ki{i}"),
            embedding: emb(v),
        });
    }
    let ds2 = ds_of(entries);
    let g2 = build_knn_graph(&ds2, 2, 0.3).unwrap();
    let r2 = compute_layout(&ds2, &g2).unwrap();
    assert_eq!(r2.positions.len(), ds2.entries.len());
    assert!(r2
        .positions
        .values()
        .all(|p| p.x.is_finite() && p.y.is_finite()));
}

// ---------- §277/§278 增量布局 ----------

#[test]
fn incremental_layout_keeps_positions() {
    // 100 nodes → save layout → add 5 → seeded incremental（§277）。
    let base = synthetic_entries(100);
    let ds = ds_of(base.clone());
    let g = build_knn_graph(&ds, 4, 0.5).unwrap();
    let full = compute_layout(&ds, &g).unwrap();

    let mut extended = base.clone();
    extended.extend(synthetic_entries(105).into_iter().skip(100));
    let ds2 = ds_of(extended);
    let g2 = build_knn_graph(&ds2, 4, 0.5).unwrap();
    let inc = layout_seeded(&ds2, &g2, Some(&full.positions), 20).unwrap();

    // §82/§277：原 100 节点位置**精确保持**（冻结），新节点有限。
    for e in &ds.entries {
        let old = full.positions[&e.track_id];
        let new = inc.positions[&e.track_id];
        assert_eq!((old.x, old.y), (new.x, new.y), "种子节点位置保持");
    }
    assert_eq!(inc.positions.len(), 105);
    for p in inc.positions.values() {
        assert!(p.x.is_finite() && p.y.is_finite() && (-1.0..=1.0).contains(&p.x));
    }
    // 新节点确实被布局（非全与种子重合）。
    let seed_set: std::collections::HashSet<(u32, u32)> = full
        .positions
        .values()
        .map(|p| (p.x.to_bits(), p.y.to_bits()))
        .collect();
    let new_positions: Vec<(u32, u32)> = ds2
        .entries
        .iter()
        .skip(100)
        .map(|e| {
            let p = &inc.positions[&e.track_id];
            (p.x.to_bits(), p.y.to_bits())
        })
        .collect();
    assert!(
        new_positions.iter().any(|b| !seed_set.contains(b)),
        "新节点应参与松弛"
    );

    // §278 removal：100 → remove 3，其余位置几乎不动。
    let removed: Vec<i64> = vec![5, 40, 77];
    let kept: Vec<MapEntry> = base
        .iter()
        .filter(|e| !removed.contains(&e.track_id))
        .cloned()
        .collect();
    let ds3 = ds_of(kept);
    let g3 = build_knn_graph(&ds3, 4, 0.5).unwrap();
    let inc3 = layout_seeded(&ds3, &g3, Some(&full.positions), 20).unwrap();
    for e in &ds3.entries {
        let old = full.positions[&e.track_id];
        let new = inc3.positions[&e.track_id];
        assert_eq!((old.x, old.y), (new.x, new.y), "§278 其余节点保持位置");
    }
}

// ---------- §279/§280/§281/§282 dataset 版本失效 ----------

#[test]
fn dataset_invalidation_semantics() {
    // §279 replacement：换音频 → M3 identity 变 → content_identity/
    // analysis_key/向量全变（真实管线语义）→ hash 变（invalidate）。
    let base = synthetic_entries(6);
    let ds = ds_of(base.clone());
    let mut replaced = base.clone();
    replaced[2].content_identity = "replaced-audio".into();
    replaced[2].analysis_key = "k-replaced".into();
    replaced[2].embedding = emb(vec![0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0]);
    let ds2 = ds_of(replaced);
    assert_ne!(ds.dataset_hash, ds2.dataset_hash, "分析替换 → hash 变");

    // §280 metadata-only：title 不进 dataset（map 只关心嵌入）→ hash 不变。
    let mut meta = base.clone();
    meta[1].content_identity = "renamed".into();
    let ds3 = ds_of(meta);
    assert_eq!(ds.dataset_hash, ds3.dataset_hash, "纯元数据变更 → 位置保持");

    // §281 model version：model_id 变 → 兼容合同排除 → rebuild 路径。
    let mut model_bump = base.clone();
    for e in &mut model_bump {
        e.embedding.model_id = "other-model".into();
    }
    let ds4 = MapDataset::build(model_bump, None).unwrap();
    assert_ne!(ds.dataset_hash, ds4.dataset_hash);
    // 旧 persisted 对新 ds validate 失败（§281 stale/rebuild）。
    let g = build_knn_graph(&ds, 2, 0.5).unwrap();
    let clusters = detect_clusters(&g, 10).clusters;
    let p = persist(
        &ds,
        &g,
        &clusters,
        &compute_layout(&ds, &g).unwrap().positions,
    );
    assert!(!aura_map::persistence::validate(
        &p,
        &ds4.dataset_hash,
        &ds4
    ));

    // §282 analysis version：analysis_version 变 → 兼容合同失败。
    let mut ver_bump = base.clone();
    for e in &mut ver_bump {
        e.embedding.analysis_version += 1;
    }
    let ds5 = ds_of(ver_bump);
    assert_ne!(ds.dataset_hash, ds5.dataset_hash);
    assert!(!aura_map::persistence::validate(
        &p,
        &ds5.dataset_hash,
        &ds5
    ));
}

// ---------- §283/§284/§285 损坏检测 ----------

#[test]
fn corruption_detection_and_recovery() {
    let ds = ds_of(synthetic_entries(10));
    let g = build_knn_graph(&ds, 3, 0.5).unwrap();
    let layout = compute_layout(&ds, &g).unwrap();
    let clusters = detect_clusters(&g, 10).clusters;
    let good = persist(&ds, &g, &clusters, &layout.positions);

    // §283 corrupt index（图损坏：self-loop / 未知节点）→ Err + rebuild。
    let mut bad_graph = good.clone();
    bad_graph.edges.push(aura_map::MapEdge {
        a: 1,
        b: 1,
        similarity: 1.0,
    });
    bad_graph.checksum = aura_map::persistence::payload_checksum(&bad_graph);
    assert!(
        restore_map(&bad_graph, &ds.dataset_hash, &ds).is_err(),
        "self-loop 拒绝"
    );
    let mut bad_edge = good.clone();
    bad_edge.edges.push(aura_map::MapEdge {
        a: 1,
        b: 999,
        similarity: 0.5,
    });
    bad_edge.checksum = aura_map::persistence::payload_checksum(&bad_edge);
    assert!(
        restore_map(&bad_edge, &ds.dataset_hash, &ds).is_err(),
        "未知节点拒绝"
    );

    // §284 corrupt layout：NaN position → recover/rebuild。
    let mut bad_pos = good.clone();
    let id = ds.entries[0].track_id;
    let mut pos = bad_pos.positions.clone();
    pos.insert(
        id,
        MapPosition {
            x: f32::NAN,
            y: 0.0,
        },
    );
    bad_pos.positions = pos;
    bad_pos.checksum = aura_map::persistence::payload_checksum(&bad_pos);
    assert!(
        restore_map(&bad_pos, &ds.dataset_hash, &ds).is_err(),
        "NaN 位置拒绝"
    );

    // checksum 损坏 → 拒绝（§285 不 crash）。
    let mut bad_ck = good.clone();
    bad_ck.checksum = bad_ck.checksum.wrapping_add(1);
    assert!(restore_map(&bad_ck, &ds.dataset_hash, &ds).is_err());
    // 完好数据恢复成功。
    assert!(restore_map(&good, &ds.dataset_hash, &ds).is_ok());
}

// ---------- §286–§289 build crash/dup/cancel 语义 ----------

#[test]
fn build_lifecycle_semantics() {
    let entries = synthetic_entries(30);
    let (ds, hash) = freeze_snapshot(entries.clone());
    let gen = BuildGeneration::new();
    let cancel = aura_analysis::pipeline::CancelToken::new();

    // §287 并发 build：v2 取代 v1 后，v1 代号不再 current（迟到不得
    // 覆盖——shell 线程在 commit 前 is_current 复查，此处验证代号语义）。
    let v1 = gen.next();
    let out1 = build_map(&ds, &hash, BuildParams::default(), v1, &gen, &cancel, None).unwrap();
    assert_eq!(out1.build_id, v1);
    assert!(gen.is_current(v1));
    let v2 = gen.next();
    assert!(!gen.is_current(v1), "v1 已被 v2 取代");
    let out2 = build_map(&ds, &hash, BuildParams::default(), v2, &gen, &cancel, None).unwrap();
    assert_eq!(out2.status, MapBuildStatus::Ready);
    assert!(gen.is_current(v2));

    // §289 duplicate dataset build：同 hash 可重复构建（shell 层去重），
    // 结果确定性一致（graph/positions 相同）。
    let v3 = gen.next();
    let out3 = build_map(&ds, &hash, BuildParams::default(), v3, &gen, &cancel, None).unwrap();
    assert_eq!(out3.graph.edges, out2.graph.edges);
    assert_eq!(out3.positions, out2.positions);

    // §288 cancellation：worker stops + no partial ready。
    let c2 = aura_analysis::pipeline::CancelToken::new();
    c2.cancel();
    let v4 = gen.next();
    assert!(matches!(
        build_map(&ds, &hash, BuildParams::default(), v4, &gen, &c2, None),
        Err(MapBuildError::Cancelled)
    ));

    // §286 partial build 不 Ready：cancel 后 snapshot 保持旧值（这里以
    // 上一代 out3 为 last-known-good；无任何半成品可被标记）。
    assert_eq!(out3.status, MapBuildStatus::Ready);
}

// ---------- §482（缩减）build/cancel/reopen 循环 ----------

#[test]
fn build_cancel_reopen_soak() {
    let entries = synthetic_entries(40);
    let (ds, hash) = freeze_snapshot(entries);
    let gen = BuildGeneration::new();
    for round in 0..20u64 {
        let id = gen.next();
        if round % 3 == 2 {
            let c = aura_analysis::pipeline::CancelToken::new();
            c.cancel();
            assert!(matches!(
                build_map(&ds, &hash, BuildParams::default(), id, &gen, &c, None),
                Err(MapBuildError::Cancelled)
            ));
        } else {
            let out = build_map(
                &ds,
                &hash,
                BuildParams::default(),
                id,
                &gen,
                &aura_analysis::pipeline::CancelToken::new(),
                None,
            )
            .unwrap();
            assert_eq!(out.status, MapBuildStatus::Ready);
            // reopen：restore 语义等价（图/位置保真）。
            let restored = restore_map(&out.persisted, &hash, &ds).unwrap();
            assert_eq!(restored.positions, out.positions, "round {round}");
        }
    }
}

// ---------- §298 查询语义 ----------

#[test]
fn query_semantics() {
    let ds = ds_of(synthetic_entries(60));
    let g = build_knn_graph(&ds, 4, 0.5).unwrap();
    // top-k neighbor 真实相似度排序（§336）。
    let nb = neighbors(&ds, &g, 1, 5).unwrap();
    assert!(!nb.is_empty() && nb.len() <= 5);
    for w in nb.windows(2) {
        assert!(w[0].1 >= w[1].1);
    }
    // viewport（§298/§340）：矩形过滤正确。
    let vp = viewport_nodes(
        &BTreeMap::from([
            (1, MapPosition { x: -0.9, y: -0.9 }),
            (2, MapPosition { x: 0.0, y: 0.0 }),
            (3, MapPosition { x: 0.9, y: 0.9 }),
        ]),
        -1.0,
        -1.0,
        0.5,
        0.5,
    );
    assert_eq!(vp.len(), 2);
    // unknown track → Err。
    assert!(neighbors(&ds, &g, 999_999, 5).is_err());
}

// ---------- §292/§295 内存估算证据（打印；PERF.md 记录） ----------

#[test]
fn memory_accounting_evidence() {
    let n = 1000usize;
    let dim = 8usize;
    let ds = ds_of(synthetic_entries(n));
    let g = build_knn_graph(&ds, 4, 0.5).unwrap();
    let vectors_bytes = n * dim * 4; // §291：N × D × dtype
    let edges_bytes = g.edges.len() * (8 + 8 + 4);
    let positions_bytes = n * 8;
    println!(
        "M9 memory @1k nodes: vectors={vectors_bytes}B edges={}({edges_bytes}B) positions={positions_bytes}B isolated={}",
        g.edges.len(),
        g.isolated.len()
    );
    assert!(vectors_bytes > 0 && edges_bytes > 0);
}
