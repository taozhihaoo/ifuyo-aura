//! 病态种子 / 非法输入 stress & fuzz（§249–§253/§168/§169）：
//! 任意输入不 panic；极端 genome 下光场参数仍有限；光场清单极小。

use aura_synth::biome::Biome;
use aura_synth::env::EnvironmentState;
use aura_synth::genome::{Genome, GenomeField};
use aura_synth::manifest::LightField;
use aura_synth::seed::{world_identity, DreamscapeSeed};
use aura_synth::snapshot::DreamscapeSession;

/// §249 pathological genomes：极端 genome 参数仍全部有限、序列化仍极小。
#[test]
fn stress_pathological_genomes_bounded() {
    for i in 0..30u64 {
        let seed = DreamscapeSeed::from_input(&format!("stress{i}"));
        let mut g = Genome::from_seed(&seed);
        g.set(
            GenomeField::FloraDensity,
            if i % 2 == 0 { 1.0 } else { 0.0 },
        );
        g.set(GenomeField::TerrainScale, 4.0);
        g.set(GenomeField::MistDensity, 1.0);
        g.set(GenomeField::WaterPresence, 1.0);
        g.set(GenomeField::ColorTemperature, 1.0);
        let biome = Biome::select(&seed, &g);
        let env = EnvironmentState::initial(&seed, i * 999);
        let wid = world_identity(&seed, 1, biome.kind.id().as_bytes());
        let field = LightField::new(wid, &seed, &g, &biome, &env);
        for v in field.params.as_scalars().into_iter().chain([
            field.light.sun_intensity,
            field.light.ambient_intensity,
            field.light.horizon_glow,
            field.atmosphere.fog,
            field.atmosphere.wind,
            field.atmosphere.mist_bias,
            field.palette.base_hue,
            field.palette.saturation,
            field.palette.luminance,
        ]) {
            assert!(v.is_finite(), "§249 极端 genome 下参数必须有限");
        }
        let json = serde_json::to_string(&field).unwrap();
        assert!(
            json.len() < 4096,
            "§115 光场清单必须极小：{} bytes",
            json.len()
        );
        assert!(field.scene_hash() != 0, "hash 必须有效");
    }
}

/// §250 invalid seeds：empty/huge/unicode/emoji/whitespace/separators
/// —— 全部不 panic 且确定性。
#[test]
fn invalid_seed_inputs_never_panic() {
    let inputs = [
        "",
        " ",
        "\t\n",
        "a",
        &"x".repeat(100_000),
        "日本語のシード",
        "🌍🚀",
        "DS-",
        "DS-!!!!-!!!!-!!!!",
        "DS-00000-0000-0000",
        "-----",
        "::::",
        "null",
        "\0\0\0",
        "mix 世界 🌍 and ascii",
    ];
    for input in inputs {
        let s1 = DreamscapeSeed::from_input(input);
        let s2 = DreamscapeSeed::from_input(input);
        assert_eq!(s1, s2, "任意输入确定性");
        let g = Genome::from_seed(&s1);
        let b = Biome::select(&s1, &g);
        let env = EnvironmentState::initial(&s1, 0);
        let field = LightField::new(1, &s1, &g, &b, &env);
        assert_eq!(
            field.scene_hash(),
            LightField::new(1, &s2, &g, &b, &env).scene_hash()
        );
        let session = DreamscapeSession::new(s1, 0);
        let _ = session.export_seed();
        let _ = session.field();
    }
}

/// §251 fuzz/property：任意 UTF-8 串 → seed → 完整生成链不 panic。
#[test]
fn fuzz_seed_parser_property() {
    // 确定性伪随机（测试内自带；不依赖外部 fuzz 框架 §182 轻量原则）。
    let mut state = 0x1234_5678_9abc_def0u64;
    let mut next = move || {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        state
    };
    for round in 0..500u64 {
        let len = (next() % 64) as usize;
        let bytes: Vec<u8> = (0..len)
            .map(|_| {
                // 混合可打印 ASCII / 高位字节（会产生合法 UTF-8 随机串）。
                if next() % 3 == 0 {
                    0x80 + (next() % 0x40) as u8
                } else {
                    (next() % 95 + 32) as u8
                }
            })
            .collect();
        let Ok(input) = std::str::from_utf8(&bytes) else {
            continue;
        };
        let seed = DreamscapeSeed::from_input(input);
        let g = Genome::from_seed(&seed);
        let b = Biome::select(&seed, &g);
        let env = EnvironmentState::initial(&seed, round);
        let field = LightField::new(1, &seed, &g, &b, &env);
        assert!(field.params.as_scalars().iter().all(|v| v.is_finite()));
        if round % 50 == 0 {
            let _ = field.scene_hash();
        }
    }
}

/// §252 fuzz snapshot parser：malformed JSON/bytes 不 panic。
#[test]
fn fuzz_snapshot_parser_never_panics() {
    let malformed: Vec<Vec<u8>> = vec![
        b"".to_vec(),
        b"{".to_vec(),
        b"null".to_vec(),
        b"[]".to_vec(),
        br#"{"snapshot_version":1}"#.to_vec(),
        b"\xff\xfe\x00trunc".to_vec(),
        b"{\"snapshot_version\":99}".to_vec(),
        b"{\"snapshot_version\":1,\"integrity\":123}".to_vec(),
        serde_json::json!({"snapshot_version": 1, "world_id": "not-a-number"})
            .to_string()
            .into_bytes(),
        serde_json::json!({"snapshot_version": 1, "genome": {"terrain_scale": "NaN"}})
            .to_string()
            .into_bytes(),
    ];
    for payload in &malformed {
        let parsed: Result<aura_synth::snapshot::DreamscapeSnapshot, _> =
            serde_json::from_slice(payload);
        if let Ok(snap) = parsed {
            let _ = snap.validate(); // 合法 JSON 也要过校验门
        }
    }
}
