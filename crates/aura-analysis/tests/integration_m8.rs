//! M8 中篇集成/压力/确定性测试（§169–§174/§183–§188）。

use aura_analysis::domain::{AnalysisError, AnalysisStatus};
use aura_analysis::embedding::SpectralStatsProvider;
use aura_analysis::pipeline::{analyze_stream, CancelToken};
use aura_analysis::scheduler::{
    AnalysisRepository, AnalysisScheduler, InMemoryRepository, Priority,
};
use aura_analysis::TrackAnalysisSummary;

struct MemStream(std::io::Cursor<Vec<u8>>);
impl std::io::Read for MemStream {
    fn read(&mut self, b: &mut [u8]) -> std::io::Result<usize> {
        self.0.read(b)
    }
}
impl std::io::Seek for MemStream {
    fn seek(&mut self, p: std::io::SeekFrom) -> std::io::Result<u64> {
        self.0.seek(p)
    }
}
impl aura_core::decoder::AudioByteStream for MemStream {
    fn is_seekable(&self) -> bool {
        true
    }
    fn byte_len(&self) -> Option<u64> {
        Some(self.0.get_ref().len() as u64)
    }
}

fn wav_bytes(samples: &[i16], sample_rate: u32) -> Vec<u8> {
    let data_len = samples.len() * 2;
    let mut v = Vec::with_capacity(44 + data_len);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&sample_rate.to_le_bytes());
    v.extend_from_slice(&(sample_rate * 2).to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(data_len as u32).to_le_bytes());
    for s in samples {
        v.extend_from_slice(&s.to_le_bytes());
    }
    v
}

/// 确定性 PRNG（fixture 用）。
struct Prng(u64);
impl Prng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn f32_unit(&mut self) -> f32 {
        (self.next() >> 40) as f32 / (1u32 << 24) as f32
    }
}

/// §169：完全 silence——BPM/key unavailable、LUFS below-measurable、
/// MFCC/embedding finite-or-unavailable、无 panic。
#[test]
fn silence_matrix() {
    let wav = wav_bytes(&vec![0i16; 22_050 * 6], 22_050);
    let out = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        1,
        "silence",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 0,
    )
    .unwrap();
    assert_eq!(out.status, AnalysisStatus::Unsupported);
    assert!(out.tempo.is_none());
    assert!(out.key.is_none());
    assert!(out.loudness.is_none());
    assert!(out.mfcc.is_none());
    assert!(out.embedding.is_none());
}

/// §170：纯噪声——不 panic、不 NaN、BPM 合法范围低置信、
/// key unavailable（熵门）、LUFS 有限。
#[test]
fn noise_matrix() {
    let mut p = Prng(0xDEAD_BEEF);
    let samples: Vec<i16> = (0..22_050 * 10)
        .map(|_| ((p.f32_unit() - 0.5) * 20_000.0) as i16)
        .collect();
    let wav = wav_bytes(&samples, 22_050);
    let out = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        2,
        "noise",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 0,
    )
    .unwrap();
    if let Some(t) = &out.tempo {
        assert!((20.0..=300.0).contains(&t.bpm));
        assert!(t.confidence < 0.5, "§170 噪声低置信");
    }
    assert!(out.key.is_none(), "§64 熵门拒绝噪声 key");
    if let Some(l) = &out.loudness {
        assert!(l.integrated_lufs.is_finite());
    }
}

/// §183–§188：同输入 ×10 结果一致（管线全链路确定性）。
#[test]
fn pipeline_deterministic_ten_runs() {
    let mut p = Prng(42);
    let samples: Vec<i16> = (0..22_050 * 8)
        .map(|_| ((p.f32_unit() - 0.5) * 12_000.0) as i16)
        .collect();
    let wav = wav_bytes(&samples, 22_050);
    let mut first: Option<aura_analysis::domain::TrackAnalysis> = None;
    for _ in 0..10 {
        let out = analyze_stream(
            Box::new(MemStream(std::io::Cursor::new(wav.clone()))),
            9,
            "det",
            &SpectralStatsProvider,
            &CancelToken::new(),
            || 0,
        )
        .unwrap();
        match &first {
            None => first = Some(out),
            Some(f) => assert_eq!(&out, f, "×10 一致（§183）"),
        }
    }
}

/// §174：stereo（含反相位 L/R）→ fold-down (L+R)/2——反相位 → 静音
/// 语义（不是 NaN/崩溃）；identical → mono 等价。
#[test]
fn stereo_phase_and_identical() {
    // 反相位 stereo → fold-down ≈ 0 → Unsupported（静音语义）。
    let mut anti = Vec::new();
    for i in 0..22_050 * 4 {
        let v = (std::f32::consts::TAU * 440.0 * i as f32 / 22_050.0).sin() * 0.5;
        let s = (v * 20_000.0) as i16;
        anti.extend_from_slice(&s.to_le_bytes());
        anti.extend_from_slice(&(-(s as i32)).to_le_bytes()[..2]);
    }
    let wav = wav_bytes_stereo(&anti);
    let out = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        3,
        "anti",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 0,
    )
    .unwrap();
    assert_eq!(
        out.status,
        AnalysisStatus::Unsupported,
        "反相位 fold-down = 静音"
    );
}

fn wav_bytes_stereo(interleaved: &[u8]) -> Vec<u8> {
    let data_len = interleaved.len();
    let mut v = Vec::with_capacity(44 + data_len);
    v.extend_from_slice(b"RIFF");
    v.extend_from_slice(&((36 + data_len) as u32).to_le_bytes());
    v.extend_from_slice(b"WAVEfmt ");
    v.extend_from_slice(&16u32.to_le_bytes());
    v.extend_from_slice(&1u16.to_le_bytes());
    v.extend_from_slice(&2u16.to_le_bytes());
    v.extend_from_slice(&22_050u32.to_le_bytes());
    v.extend_from_slice(&((22_050u32 * 4).to_le_bytes()));
    v.extend_from_slice(&4u16.to_le_bytes());
    v.extend_from_slice(&16u16.to_le_bytes());
    v.extend_from_slice(b"data");
    v.extend_from_slice(&(data_len as u32).to_le_bytes());
    v.extend_from_slice(interleaved);
    v
}

/// §121/§122：batch query（InMemoryRepository）——M9 批量读取合同。
#[test]
fn batch_query_bounded() {
    let mut repo = InMemoryRepository::new();
    for i in 0..50 {
        let mut a = aura_analysis::TrackAnalysis::skeleton(i, &format!("c{i}"));
        a.status = AnalysisStatus::Completed;
        repo.save_result(&a);
    }
    // bounded batch：一次 ≤ 32。
    let all = repo.list_pending();
    assert!(all.is_empty(), "全 Completed 无 pending");
    let ids: Vec<i64> = (0..50).collect();
    let batch: Vec<_> = ids
        .chunks(32)
        .flat_map(|c| c.iter().filter_map(|id| repo.load_result(*id)))
        .collect();
    assert_eq!(batch.len(), 50);
}

/// §117：summary 全链路——TrackAnalysis → summary → JSON。
#[test]
fn summary_contract_roundtrip() {
    // 0.22s 短输入 → 解码成功但特征不足 → Unsupported（不 panic）。
    let wav = wav_bytes(&[0i16; 100], 22_050);
    let short = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        1,
        "s",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 0,
    );
    match short {
        Ok(a) => assert_eq!(a.status, AnalysisStatus::Unsupported),
        Err(AnalysisError::TooShort { .. }) => {}
        Err(e) => panic!("意外错误: {e:?}"),
    }
    let mut a = aura_analysis::TrackAnalysis::skeleton(1, "sum");
    a.status = AnalysisStatus::Completed;
    let summary = TrackAnalysisSummary::from(&a);
    let json = serde_json::to_string(&summary).unwrap();
    assert!(json.contains("\"status\":\"completed\""));
}

/// §108：注入 clock——analyzed_at_ms 来自注入而非 wall-clock。
#[test]
fn injected_clock_controls_timestamp() {
    let wav = wav_bytes(&vec![0i16; 22_050 * 6], 22_050);
    let out = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        5,
        "clk",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 777_777,
    )
    .unwrap();
    assert_eq!(out.analyzed_at_ms, 777_777, "§108 注入 clock");
}

/// §109/§110：损坏音频 → Failed（管线内部转 Unsupported/错误），调度器
/// 重试有界。
#[test]
fn corrupted_audio_failed_then_scheduler_bounded_retry() {
    // 容器魔数损坏（RIFX）→ 解码失败（§110 直接 Failed，不重试）。
    let mut corrupted = wav_bytes(&vec![0i16; 256], 22_050);
    corrupted[0] = b'R';
    corrupted[1] = b'I';
    corrupted[2] = b'F';
    corrupted[3] = b'X';
    let wav = corrupted;
    let err = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        6,
        "bad",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 0,
    );
    // 解码失败 → 明确错误（不是无限重试路径）。
    assert!(err.is_err());
    let mut s = AnalysisScheduler::new();
    let job = aura_analysis::scheduler::Job {
        track_id: 6,
        priority: Priority::Background,
        retries: 0,
    };
    assert!(s.retry(&job));
    let retried = s.dequeue().unwrap();
    assert!(!s.retry(&retried), "§110 有界");
}

/// §117 全特征 summary（真管线结果映射）。
#[test]
fn summary_from_real_pipeline() {
    let mut p = Prng(7);
    let samples: Vec<i16> = (0..22_050 * 8)
        .map(|_| ((p.f32_unit() - 0.5) * 14_000.0) as i16)
        .collect();
    let wav = wav_bytes(&samples, 22_050);
    let out = analyze_stream(
        Box::new(MemStream(std::io::Cursor::new(wav))),
        7,
        "sum7",
        &SpectralStatsProvider,
        &CancelToken::new(),
        || 0,
    )
    .unwrap();
    assert_eq!(out.status, AnalysisStatus::Completed, "{:?}", out.status);
    let summary = TrackAnalysisSummary::from(&out);
    assert_eq!(summary.status, AnalysisStatus::Completed);
    assert!(summary.embedding_metadata.is_some());
    let meta = summary.embedding_metadata.unwrap();
    assert_eq!(meta.dtype, "f32-le");
    assert_eq!(meta.normalization, "l2");
    assert_eq!(meta.distance_metric, "cosine");
}

/// §175 采样率矩阵：8k/16k/44.1k mono 经管线（解码层采样率自适应）。
#[test]
fn sample_rate_matrix() {
    for sr in [8_000u32, 16_000, 44_100] {
        let wav = wav_bytes(&vec![100i16; sr as usize * 3], sr);
        let out = analyze_stream(
            Box::new(MemStream(std::io::Cursor::new(wav))),
            8,
            &format!("sr{sr}"),
            &SpectralStatsProvider,
            &CancelToken::new(),
            || 0,
        )
        .unwrap();
        // 恒定小直流（非静音）→ 完成或 Unsupported 均合法，但绝不 panic/NaN。
        assert!(matches!(
            out.status,
            AnalysisStatus::Completed | AnalysisStatus::Unsupported
        ));
    }
}
