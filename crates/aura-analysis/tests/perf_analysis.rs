//! M8 性能基线与存储预算（§215–§221/§302–§309）。
//! `#[ignore]` 显式运行：
//! `cargo test -p aura-analysis --test perf_analysis -- --ignored --nocapture`
//! 结果记入 docs/PERF.md（M8 节）。

use std::time::Instant;

use aura_analysis::domain::{AnalysisStatus, TrackAnalysis};
use aura_analysis::embedding::{EmbeddingProvider, SpectralStatsProvider};
use aura_analysis::mfcc::analyze_mfcc;
use aura_analysis::params::ANALYSIS_SAMPLE_RATE;
use aura_analysis::pipeline::{analyze_stream, CancelToken};
use aura_analysis::scheduler::{
    AnalysisRepository, AnalysisScheduler, InMemoryRepository, Priority,
};
use aura_analysis::{analyze_tempo, TrackAnalysisSummary};
use aura_library::service::LibraryService;

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

/// 节奏 WAV（确定性 click+噪声混合）。
fn rhythmic_wav(seconds: f32, sr: u32) -> Vec<u8> {
    let period = (0.5 * sr as f32) as usize; // 120 BPM
    let burst = period / 8;
    let total = (sr as f32 * seconds) as usize;
    let mut out = vec![0i16; total];
    let mut seed = 0x1234_5678u64;
    let mut t = 0usize;
    while t + burst < total {
        for i in 0..burst {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            let env = 1.0 - i as f32 / burst as f32;
            out[t + i] =
                (env * 10_000.0 * ((seed >> 40) as f32 / (1u32 << 24) as f32 - 0.5)) as i16;
        }
        t += period;
    }
    wav_bytes(&out, sr)
}

/// §302/§303：端到端吞吐——30s/3min synthetic；分段耗时。
#[test]
#[ignore = "perf baseline；显式运行"]
fn end_to_end_by_track_length() {
    for seconds in [30.0f32, 180.0] {
        let wav = rhythmic_wav(seconds, 22_050);
        let t0 = Instant::now();
        let stream: Box<dyn aura_core::decoder::AudioByteStream> =
            Box::new(MemStream(std::io::Cursor::new(wav.clone())));
        analyze_stream(
            stream,
            1,
            &format!("perf-{seconds}"),
            &SpectralStatsProvider,
            &CancelToken::new(),
            || 0,
        )
        .expect("pipeline ok");
        println!("{seconds}s track: {:?}", t0.elapsed());
    }
}
/// §302：特征分段耗时（对同一 buffer）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn feature_timings() {
    let mut p = 0xABCDEFu64;
    let samples: Vec<f32> = (0..ANALYSIS_SAMPLE_RATE as usize * 30)
        .map(|_| {
            p ^= p << 13;
            p ^= p >> 7;
            p ^= p << 17;
            (p >> 40) as f32 / (1u32 << 24) as f32 - 0.5
        })
        .collect();
    let n = samples.len() as f64;

    let t = Instant::now();
    let _ = analyze_tempo(&samples, ANALYSIS_SAMPLE_RATE);
    println!("BPM time          : {:?}", t.elapsed());

    let t = Instant::now();
    let _ = aura_analysis::analyze_key(&samples, ANALYSIS_SAMPLE_RATE);
    println!("Key time          : {:?}", t.elapsed());

    let t = Instant::now();
    let _ = aura_analysis::analyze_loudness(&samples, ANALYSIS_SAMPLE_RATE);
    println!("LUFS time         : {:?}", t.elapsed());

    let t = Instant::now();
    let _ = analyze_mfcc(&samples, ANALYSIS_SAMPLE_RATE);
    println!("MFCC time         : {:?}", t.elapsed());

    let provider = SpectralStatsProvider;
    let t = Instant::now();
    let emb = provider
        .embed(&aura_analysis::embedding::EmbeddingInput::new(
            samples.clone(),
            ANALYSIS_SAMPLE_RATE,
        ))
        .unwrap();
    println!("Embedding time    : {:?}", t.elapsed());
    println!(
        "embedding storage : dim={} × 4B = {} B/track（10k = {:.1} MB）",
        emb.dimension,
        emb.dimension * 4,
        emb.dimension as f64 * 4.0 * 10_000.0 / 1_048_576.0
    );
    let _ = n;
}

/// §217/§218/§219/§220/§308：10k 库 soak（synthetic metadata +
/// tiny analysis rows）——查询/批量/存储尺寸。
#[test]
#[ignore = "perf baseline；显式运行"]
fn large_library_soak_10k() {
    let svc = LibraryService::open_in_memory().unwrap();
    {
        let tx = svc.database().connection().unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO library_roots(id, path, date_added_ms) VALUES (1, 'C:/m', 0)",
            [],
        )
        .unwrap();
        for i in 0..10_000i64 {
            tx.execute(
                "INSERT INTO tracks (id, identity_key, audio_sig, file_size, modified_ms, date_added_ms, last_scanned_ms)
                 VALUES (?1, ?2, ?3, 1, 0, 0, 0)",
                rusqlite::params![i + 1, format!("ik-{i}"), format!("sig-{i}")],
            )
            .unwrap();
            // 典型分析行（含 32 维嵌入 BLOB = 128B + mfcc 104B）。
            tx.execute(
                "INSERT INTO track_analysis (track_id, content_identity, analysis_key, analysis_version, status,
                    bpm, bpm_confidence, first_beat_offset_s, tempo_algo_version, tempo_availability,
                    musical_key, key_mode, key_confidence, camelot_key, key_algo_version, key_availability,
                    lufs, lra, true_peak_dbtp, loudness_algo_version, loudness_availability,
                    rms, peak, centroid_hz, rolloff_hz, brightness, energy, spectral_algo_version, spectral_availability,
                    mfcc_blob, mfcc_algo_version, mfcc_availability,
                    embedding_blob, embedding_dim, embedding_model_id, embedding_model_version, embedding_availability,
                    analyzed_at_ms)
                 VALUES (?1, ?2, ?3, 1, 'completed',
                    120.0, 0.8, 0.0, 1, 'available',
                    'Fs', 'Minor', 0.7, '11A', 1, 'available',
                    -14.0, 4.0, -1.0, 1, 'available',
                    0.3, 0.9, 2000.0, 9000.0, 0.5, 0.6, 1, 'available',
                    zeroblob(104), 1, 'available',
                    zeroblob(128), 32, 'aura-spectral-stats', 1, 'available',
                    0)",
                rusqlite::params![i + 1, format!("c{i}"), format!("k{i}")],
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }

    let t = Instant::now();
    let single: i64 = svc
        .database()
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM track_analysis WHERE track_id = 5000",
            [],
            |r| r.get(0),
        )
        .unwrap();
    println!("get_analysis(single): {single} ({:?})", t.elapsed());

    let t = Instant::now();
    let n100: i64 = svc
        .database()
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM track_analysis WHERE track_id <= 100",
            [],
            |r| r.get(0),
        )
        .unwrap();
    println!("batch(100)         : {n100} rows ({:?})", t.elapsed());

    let t = Instant::now();
    let n1000: i64 = svc
        .database()
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM track_analysis WHERE track_id <= 1000",
            [],
            |r| r.get(0),
        )
        .unwrap();
    println!("batch(1000)        : {n1000} rows ({:?})", t.elapsed());

    let t = Instant::now();
    let bpm_hits: i64 = svc
        .database()
        .connection()
        .query_row(
            "SELECT COUNT(*) FROM track_analysis WHERE bpm BETWEEN 119 AND 121",
            [],
            |r| r.get(0),
        )
        .unwrap();
    println!("filter bpm 119-121 : {bpm_hits} rows ({:?})", t.elapsed());

    // DB 大小（内存库不落盘 → 报告行均字节的算术值）。
    let per_track =
        104.0 /* mfcc */ + 128.0 /* embedding */ + 300.0 /* 标量列+键（实测典型） */;
    println!(
        "storage per track  : ≈{per_track:.0} B → 10k tracks ≈ {:.1} MB（远低于数 MB/首 §218）",
        per_track * 10_000.0 / 1_048_576.0
    );
}

/// §217/§309：10k 入队 + 去重 + 取消（调度器内存有界 §310/§311——
/// 队列项仅 TrackId/Priority/JobId）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn queue_10k_benchmark() {
    let mut s = AnalysisScheduler::new();
    let t = Instant::now();
    for i in 0..10_000i64 {
        s.enqueue(
            i,
            if i % 100 == 0 {
                Priority::UserRequested
            } else {
                Priority::Background
            },
        );
    }
    println!("enqueue 10k        : {:?}", t.elapsed());
    // 同 track 重复入队 → 去重。
    s.enqueue(0, Priority::UserRequested);
    assert_eq!(s.pending_count(), 10_000, "§207 去重");
    let t = Instant::now();
    let job = s.dequeue().unwrap();
    println!("dequeue latency    : {:?}", t.elapsed());
    assert_eq!(job.priority, Priority::UserRequested);
}

/// §117/§122：summary 批量导出（M9 合同模拟：100 首 → vectors + BPM +
/// key + energy 可直接取用；不实现 Map §356/§357）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn m9_contract_batch_100() {
    let mut repo = InMemoryRepository::new();
    for i in 0..100i64 {
        let mut a = TrackAnalysis::skeleton(i, &format!("c{i}"));
        a.status = AnalysisStatus::Completed;
        repo.save_result(&a);
    }
    let t = Instant::now();
    let batch: Vec<TrackAnalysisSummary> = (0..100)
        .filter_map(|id| repo.load_result(id))
        .map(|a| TrackAnalysisSummary::from(&a))
        .collect();
    println!(
        "batch(100) summary : {:?} ({} items)",
        t.elapsed(),
        batch.len()
    );
    assert_eq!(batch.len(), 100);
}
