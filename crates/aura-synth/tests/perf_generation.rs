//! Dreamscape 生成链性能基线（§243/§244）：seed→genome、genome+env→光场、
//! 光场序列化/反序列化/hash、快照保存/装载。
//! `#[ignore]` 显式运行：`cargo test -p aura-synth --test perf_generation -- --ignored --nocapture`
//! 结果记入 docs/PERF.md（M7 节）。

use std::time::Instant;

use aura_synth::biome::Biome;
use aura_synth::env::EnvironmentState;
use aura_synth::genome::{Genome, GenomeField};
use aura_synth::manifest::LightField;
use aura_synth::seed::{world_identity, DreamscapeSeed};
use aura_synth::snapshot::DreamscapeSession;

#[test]
#[ignore = "perf baseline；显式运行"]
fn generation_pipeline() {
    let seed = DreamscapeSeed::from_input("perf");

    // seed → genome（×1000）。
    let t = Instant::now();
    for _ in 0..1000 {
        let _ = Genome::from_seed(&seed);
    }
    println!("seed->genome        : {:?}/call", t.elapsed() / 1000);

    let genome = Genome::from_seed(&seed);
    let biome = Biome::select(&seed, &genome);
    let env = EnvironmentState::initial(&seed, 1000);
    let wid = world_identity(&seed, 1, biome.kind.id().as_bytes());

    // genome + env → 光场（×1000；默认与极端密度两档）。
    for (tag, g) in [
        ("default", genome),
        ("dense", {
            let mut g = genome;
            g.set(GenomeField::FloraDensity, 1.0);
            g.set(GenomeField::TerrainScale, 4.0);
            g
        }),
    ] {
        let t = Instant::now();
        for _ in 0..1000 {
            let _ = LightField::new(wid, &seed, &g, &biome, &env);
        }
        println!("light field ({tag})   : {:?}/call", t.elapsed() / 1000);
    }

    // 光场序列化 / 反序列化 / hash（×1000）。
    let field = LightField::new(wid, &seed, &genome, &biome, &env);
    let t = Instant::now();
    for _ in 0..1000 {
        let _ = serde_json::to_vec(&field).unwrap();
    }
    let ser = t.elapsed() / 1000;
    let bytes = serde_json::to_vec(&field).unwrap();
    let t = Instant::now();
    for _ in 0..1000 {
        let _: LightField = serde_json::from_slice(&bytes).unwrap();
    }
    let de = t.elapsed() / 1000;
    let t = Instant::now();
    for _ in 0..1000 {
        let _ = field.scene_hash();
    }
    println!("field serialize   : {ser:?}/call ({} bytes)", bytes.len());
    println!("field deserialize : {de:?}/call");
    println!("field scene_hash  : {:?}/call", t.elapsed() / 1000);

    // 快照保存（capture+seal+JSON）/ 装载（parse+validate+restore）（×200）。
    let mut session = DreamscapeSession::new(seed, 0);
    session.advance(20_000);
    let t = Instant::now();
    for _ in 0..200 {
        let mut snap = session.snapshot();
        snap.seal();
        let _ = serde_json::to_vec(&snap).unwrap();
    }
    println!("snapshot save     : {:?}/call", t.elapsed() / 200);
    let snap = session.snapshot();
    let json = serde_json::to_vec(&snap).unwrap();
    let t = Instant::now();
    for _ in 0..200 {
        let back: aura_synth::snapshot::DreamscapeSnapshot = serde_json::from_slice(&json).unwrap();
        let _ = back.validate();
        let _ = DreamscapeSession::restore(&back).unwrap();
    }
    println!("snapshot load     : {:?}/call", t.elapsed() / 200);
}
