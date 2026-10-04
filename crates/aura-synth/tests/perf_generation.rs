//! Dreamscape 生成链性能基线（§243/§244）：seed→genome、genome→world、
//! manifest 序列化/反序列化/hash、快照保存/装载。
//! `#[ignore]` 显式运行：`cargo test -p aura-synth --test perf_generation -- --ignored --nocapture`
//! 结果记入 docs/PERF.md（M7 节）。

use std::time::Instant;

use aura_synth::biome::Biome;
use aura_synth::env::EnvironmentState;
use aura_synth::genome::Genome;
use aura_synth::manifest::SceneManifest;
use aura_synth::seed::{world_identity, DreamscapeSeed};
use aura_synth::snapshot::DreamscapeSession;
use aura_synth::world::World;

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

    // genome → world（×100；§244 各档：默认/极端密度）。
    let biome = Biome::select(&seed, &genome);
    for (tag, g) in [
        ("default", genome),
        ("dense", {
            let mut g = genome;
            g.set(GenomeField::FloraDensity, 1.0);
            g
        }),
    ] {
        let t = Instant::now();
        for _ in 0..100 {
            let _ = World::generate(&seed, &g, &biome);
        }
        let world = World::generate(&seed, &g, &biome);
        println!(
            "world gen ({tag})     : {:?}/call (objects={}, landmarks={})",
            t.elapsed() / 100,
            world.objects.len(),
            world.landmarks.len()
        );
    }

    // manifest 序列化 / 反序列化 / hash（×500）。
    let env = EnvironmentState::initial(&seed, 1000);
    let world = World::generate(&seed, &genome, &biome);
    let wid = world_identity(&seed, 1, biome.kind.id().as_bytes());
    let manifest = SceneManifest::from_world(wid, &world, &env, biome.effective_mist_bias());
    let t = Instant::now();
    for _ in 0..500 {
        let _ = serde_json::to_vec(&manifest).unwrap();
    }
    let ser = t.elapsed() / 500;
    let bytes = serde_json::to_vec(&manifest).unwrap();
    let t = Instant::now();
    for _ in 0..500 {
        let _: SceneManifest = serde_json::from_slice(&bytes).unwrap();
    }
    let de = t.elapsed() / 500;
    let t = Instant::now();
    for _ in 0..500 {
        let _ = manifest.scene_hash();
    }
    println!("manifest serialize  : {ser:?}/call ({} bytes)", bytes.len());
    println!("manifest deserialize: {de:?}/call");
    println!("manifest scene_hash : {:?}/call", t.elapsed() / 500);

    // 快照保存（capture+seal+JSON）/ 装载（parse+validate+restore）（×200）。
    let mut session = DreamscapeSession::new(seed, 0);
    session.advance(20_000);
    let t = Instant::now();
    for _ in 0..200 {
        let mut snap = session.snapshot();
        snap.seal();
        let _ = serde_json::to_vec(&snap).unwrap();
    }
    println!("snapshot save       : {:?}/call", t.elapsed() / 200);
    let snap = session.snapshot();
    let json = serde_json::to_vec(&snap).unwrap();
    let t = Instant::now();
    for _ in 0..200 {
        let back: aura_synth::snapshot::DreamscapeSnapshot = serde_json::from_slice(&json).unwrap();
        let _ = back.validate();
        let _ = DreamscapeSession::restore(&back).unwrap();
    }
    println!("snapshot load       : {:?}/call", t.elapsed() / 200);
}

use aura_synth::genome::GenomeField;
