//! SPSC ring buffer 测试矩阵（M1 §13，单线程确定性，无 sleep）。

use aura_core::audio::buffer::ring_buffer;
use aura_core::types::{Channels, Generation, SampleRate};

fn stereo() -> Channels {
    Channels::new(2).unwrap()
}

/// 生成确定性交错 PCM：sample[i] = (i + offset) as f32（自校验用 ramp）。
fn ramp(n: usize, offset: usize) -> Vec<f32> {
    (0..n).map(|i| (i + offset) as f32).collect()
}

#[test]
fn empty_buffer_reports_zero_available() {
    let (mut p, mut c) = ring_buffer(stereo(), 8 * stereo().get() as usize).unwrap();
    assert_eq!(c.available_frames(), 0);
    let mut out = vec![f32::NAN; 8];
    assert_eq!(c.pop_frames(&mut out), 0, "underrun：空缓冲弹出 0 帧");
    assert_eq!(p.free_frames(), 8);
    assert_eq!(p.push_frames(&ramp(8 * 2, 0)), 8);
    assert_eq!(c.available_frames(), 8);
}

#[test]
fn full_buffer_rejects_partial_overrun() {
    let (mut p, mut c) = ring_buffer(stereo(), 4 * stereo().get() as usize).unwrap();
    // overrun：请求写入 6 帧，容量只有 4 帧 → 只写入 4 帧（部分拒绝）
    assert_eq!(p.push_frames(&ramp(6 * 2, 0)), 4);
    assert_eq!(p.free_frames(), 0);
    // 已写入数据必须完好（不覆盖、不损坏）
    let mut out = vec![0.0; 4 * 2];
    assert_eq!(c.pop_frames(&mut out), 4);
    assert_eq!(out, ramp(8, 0), "overrun 时已写入数据不得损坏");
}

#[test]
fn single_frame_write_read() {
    let (mut p, mut c) = ring_buffer(stereo(), 4 * 2).unwrap();
    assert_eq!(p.push_frames(&[1.0, 2.0]), 1);
    assert_eq!(c.available_frames(), 1);
    let mut out = [0.0f32; 2];
    assert_eq!(c.pop_frames(&mut out), 1);
    assert_eq!(out, [1.0, 2.0]);
}

#[test]
fn large_block_write_read() {
    let (mut p, mut c) = ring_buffer(stereo(), 64 * 2).unwrap();
    let block = ramp(64 * 2, 0);
    assert_eq!(p.push_frames(&block), 64);
    let mut out = vec![0.0; 64 * 2];
    assert_eq!(c.pop_frames(&mut out), 64);
    assert_eq!(out, block);
}

#[test]
fn wraparound_write_read_across_internal_boundary() {
    // 容量 4 帧；反复 3 帧读写，推进内部游标越过环形边界
    let (mut p, mut c) = ring_buffer(stereo(), 4 * 2).unwrap();
    let mut seq = 0usize;
    for round in 0..7u64 {
        let data = ramp(3 * 2, seq);
        assert_eq!(p.push_frames(&data), 3, "round {round}");
        let mut out = vec![0.0f32; 3 * 2];
        assert_eq!(c.pop_frames(&mut out), 3);
        assert_eq!(out, data, "跨边界数据必须原样 FIFO");
        seq += 3 * 2;
    }
}

#[test]
fn cross_boundary_large_read_after_partial_pop() {
    let (mut p, mut c) = ring_buffer(stereo(), 8 * 2).unwrap();
    assert_eq!(p.push_frames(&ramp(8 * 2, 0)), 8);
    // 先弹出 1 帧使游标错位，再一次性大块读剩余（跨内部边界）
    let mut one = [0.0f32; 2];
    assert_eq!(c.pop_frames(&mut one), 1);
    let mut rest = vec![0.0f32; 7 * 2];
    assert_eq!(c.pop_frames(&mut rest), 7);
    assert_eq!(rest, ramp(14, 2));
}

/// SPSC 违约由类型系统阻止：句柄不可 Clone、各只有一份。
/// 运行时断言 Send（可移交线程）；独占所有权用移动语义证明。
#[test]
fn spsc_ownership_contract() {
    fn assert_send<T: Send>() {}
    let (p, c) = ring_buffer(stereo(), 2 * 2).unwrap();
    assert_send::<aura_core::audio::buffer::ProducerHandle>();
    assert_send::<aura_core::audio::buffer::ConsumerHandle>();
    // 独占所有权：移入块填充，再移出继续使用（不可 Clone 即无法违约）
    let p = {
        let mut taken = p;
        taken.push_frames(&ramp(2, 0));
        taken
    };
    assert_eq!(p.capacity_frames(), 2);
    assert_eq!(c.available_frames(), 1);
}

#[test]
fn long_loop_wraparound_integrity() {
    let (mut p, mut c) = ring_buffer(stereo(), 16 * 2).unwrap();
    let mut seq = 0usize;
    for _ in 0..10_000 {
        let data = ramp(5 * 2, seq);
        assert_eq!(p.push_frames(&data), 5);
        let mut out = vec![0.0f32; 5 * 2];
        assert_eq!(c.pop_frames(&mut out), 5);
        assert_eq!(out, data);
        seq += 5 * 2;
    }
}

#[test]
fn high_frequency_small_ops() {
    let (mut p, mut c) = ring_buffer(stereo(), 2 * 2).unwrap();
    let mut value = 0.0f32;
    for _ in 0..50_000 {
        assert_eq!(p.push_frames(&[value, value]), 1);
        let mut out = [0.0f32; 2];
        assert_eq!(c.pop_frames(&mut out), 1);
        assert_eq!(out[0], value);
        value += 1.0;
    }
}

#[test]
fn underrun_returns_partial_and_keeps_pipeline_consistent() {
    let (mut p, mut c) = ring_buffer(stereo(), 8 * 2).unwrap();
    assert_eq!(p.push_frames(&ramp(2 * 3, 0)), 3);
    let mut out = vec![0.0f32; 2 * 8];
    let got = c.pop_frames(&mut out);
    assert_eq!(got, 3, "underrun：只返回可用的帧数");
    assert_eq!(&out[..6], ramp(6, 0));
    // 之后 producer 恢复写入，管线可继续
    assert_eq!(p.push_frames(&ramp(2 * 2, 6)), 2);
    assert_eq!(c.available_frames(), 2);
}

#[test]
fn clear_via_generation_bump_discards_stale() {
    let (mut p, mut c) = ring_buffer(stereo(), 8 * 2).unwrap();
    assert_eq!(p.push_frames(&ramp(8 * 2, 0)), 8);
    let stale_gen = c.read_generation();
    assert_eq!(stale_gen, Generation::FIRST);

    // seek 场景：producer 先进入新代。旧代数据仍占满缓冲（producer 无清除权），
    // 写入被背压拒绝（D-022）——这是设计行为，不是丢数据。
    let new_gen = p.bump_generation();
    assert_eq!(new_gen, Generation::FIRST.next());
    assert_eq!(
        p.push_frames(&ramp(8, 1000)),
        0,
        "旧代占满时新数据被背压拒绝"
    );

    // consumer 首次按代读取：整块丢弃旧代残留，返回 0（输出侧补静音）
    let mut seen = stale_gen;
    let mut out = vec![0.0f32; 2 * 4];
    assert_eq!(
        c.pop_frames_until_generation(&mut out, &mut seen),
        0,
        "切换代的首读=丢弃"
    );
    assert_eq!(seen, new_gen);
    assert_eq!(c.available_frames(), 0, "旧代已全部丢弃");

    // 背压解除，新代数据写入并原样可达
    assert_eq!(p.push_frames(&ramp(8, 1000)), 4);
    assert_eq!(c.pop_frames(&mut out), 4);
    assert_eq!(out, ramp(8, 1000), "新代数据必须原样可达");
}

#[test]
fn multiple_generations_only_discard_once_per_change() {
    let (mut p, mut c) = ring_buffer(stereo(), 8 * 2).unwrap();
    let mut seen = c.read_generation();
    let mut out = vec![0.0f32; 8];

    // 先写入 2 帧旧代（gen1）数据
    assert_eq!(p.push_frames(&ramp(4, 0)), 2);
    assert_eq!(c.available_frames(), 2);

    // 第一代切换：bump 后旧代 2 帧被精确标记；新代数据可以紧随其后写入
    assert_eq!(p.bump_generation(), Generation::FIRST.next());
    assert_eq!(p.push_frames(&ramp(4, 100)), 2, "新代数据不受旧代影响");
    // consumer 首次按代读取：只丢弃旧代 2 帧，返回 0（输出侧补静音）
    assert_eq!(
        c.pop_frames_until_generation(&mut out, &mut seen),
        0,
        "第一代切换丢弃"
    );
    assert_eq!(c.available_frames(), 2, "新代数据必须保留");
    assert_eq!(c.pop_frames(&mut out), 2);
    assert_eq!(&out[..4], ramp(4, 100), "第一代切换后新代数据原样");

    // 第二代切换：同上
    let g3 = p.bump_generation();
    assert_eq!(g3.get(), 3);
    assert_eq!(p.push_frames(&ramp(4, 200)), 2);
    assert_eq!(
        c.pop_frames_until_generation(&mut out, &mut seen),
        0,
        "第二代切换丢弃"
    );
    assert_eq!(c.available_frames(), 2);
    assert_eq!(c.pop_frames(&mut out), 2);
    assert_eq!(&out[..4], ramp(4, 200), "第二代数据原样");

    // 未切换代时正常读取，不再丢弃
    assert_eq!(p.push_frames(&ramp(4, 300)), 2);
    assert_eq!(c.pop_frames_until_generation(&mut out, &mut seen), 2);
    assert_eq!(&out[..4], ramp(4, 300), "同代连续读取不再丢弃");
}

#[test]
fn capacity_must_be_frame_aligned() {
    let sr = SampleRate::new(48_000).unwrap();
    let _ = sr; // 帧对齐与采样率无关，占位以示 API 使用面
    assert!(ring_buffer(stereo(), 3).is_err(), "容量必须是声道整数倍");
    assert!(ring_buffer(stereo(), 0).is_err());
    assert!(ring_buffer(stereo(), 2 * 16).is_ok());
}
