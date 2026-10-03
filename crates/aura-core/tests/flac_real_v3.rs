//! 真实 FLAC 解码测试（M1 §41/§51-1）。
//!
//! 夹具：`fixtures/flac_mono_48k_440hz_1s.flac`（21KB，libsndfile 参考编码器生成，
//! 48kHz / mono / 16bit / 440Hz 正弦 1 秒；自生成、可公开分发、license 无争议）。
//! 输入 PCM 为确定性公式，测试端可精确重算（FLAC 无损 → 逐样本一致）。

use aura_core::{Decoder, FlacDecoder};

const FLAC: &[u8] = include_bytes!("fixtures/flac_mono_48k_440hz_1s.flac");
const SAMPLE_RATE: u32 = 48_000;
const FRAMES: usize = 48_000;

/// 夹具的原始输入公式（与生成脚本一致）：int16 截断量化。
fn expected_i16(frame: usize) -> i16 {
    let v = (std::f64::consts::TAU * 440.0 * frame as f64 / f64::from(SAMPLE_RATE)).sin() * 16384.0;
    v as i16
}

fn expected_f32(frame: usize) -> f32 {
    f32::from(expected_i16(frame)) / 32_768.0
}

#[test]
fn real_flac_decodes_with_matching_frame_count_and_samples() {
    let mut decoder = FlacDecoder::from_bytes(FLAC.to_vec()).expect("夹具必须可解析");
    assert_eq!(decoder.format().sample_rate.get(), SAMPLE_RATE);
    assert_eq!(decoder.format().channels.get(), 1);
    assert_eq!(decoder.duration_frames(), Some(FRAMES as u64));

    let mut out = aura_core::PcmBuffer::new();
    let mut total = 0u64;
    loop {
        let n = decoder.decode_next(&mut out).expect("decode ok");
        if n == 0 {
            break;
        }
        total += n;
    }
    assert_eq!(total, FRAMES as u64, "解码帧数与夹具一致（无损）");
    assert_eq!(out.frames(), FRAMES as u64);

    for (i, d) in out.data().iter().enumerate() {
        let expect = expected_f32(i);
        assert!(
            (d - expect).abs() <= 2.0 / 32_768.0,
            "sample {i}: decoded {d} vs expected {expect}"
        );
    }
}

#[test]
fn real_flac_seek_reads_from_target() {
    let mut decoder = FlacDecoder::from_bytes(FLAC.to_vec()).expect("夹具必须可解析");
    let target = 30_000usize;
    decoder.seek_to_frame(target as u64).expect("seek ok");

    let mut out = aura_core::PcmBuffer::new();
    let n = decoder.decode_next(&mut out).expect("decode ok");
    assert!(n >= 1);
    let first = out.data()[0];
    let expect = expected_f32(target);
    assert!(
        (first - expect).abs() <= 2.0 / 32_768.0,
        "seek 后首帧 {first} 应为帧 {target} 的值 {expect}"
    );
}
