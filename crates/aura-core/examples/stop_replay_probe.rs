//! M12 诊断：Stop→Play 重播泄漏探针（真实文件）。
//! 运行：cargo run -p aura-core --example stop_replay_probe -- <音频路径>
//! 比较：(1) 首次播放的 PCM (2) Stop→Play 后重播的 PCM。
//! 若重播开头出现中断处的音频（非文件开头），两条捕获会不一致。

use aura_core::types::AudioCommand;
use aura_core::{Channels, Engine, SampleRate};

fn main() {
    let path = std::env::args()
        .nth(1)
        .expect("usage: stop_replay_probe <file>");
    let run = |label: &str, stop_in_middle: bool| -> Vec<f32> {
        let (mut engine, mut poster) = Engine::new(
            SampleRate::new(48_000).unwrap(),
            Channels::new(2).unwrap(),
            16_384,
        )
        .unwrap();
        let bytes = std::fs::read(&path).expect("read file");
        println!("[{label}] file {} bytes", bytes.len());
        let byte_stream = aura_core::decoder::streaming::MemoryStream::new(bytes, true);
        let hint = path.rsplit('.').next().map(str::to_ascii_lowercase);
        let decoder =
            aura_core::decoder::StreamingDecoder::open(Box::new(byte_stream), hint.as_deref())
                .expect("open decoder");
        let duration = aura_core::decoder::Decoder::duration_frames(&decoder);
        println!("[{label}] duration_frames={duration:?}");
        engine.load_track(Box::new(decoder)).expect("load");
        poster.post(AudioCommand::Play).unwrap();
        let _ = engine.render_for_frames(48_000 * 3); // 播 3 秒
        if stop_in_middle {
            poster.post(AudioCommand::Stop).unwrap();
            let _ = engine.render_for_frames(48); // 回调继续走几块（模拟停止瞬间）
            poster.post(AudioCommand::Play).unwrap();
        }
        engine.clear_capture();
        let _ = engine.render_for_frames(2_000);
        engine.capture().to_vec()
    };

    let fresh = run("fresh", false);
    let replay = run("stop+play", true);

    // 基准 A：解码器直接 seek(0) 解码（等同重播应有的内容）。
    let direct_zero = {
        let bytes = std::fs::read(&path).unwrap();
        let mut d = aura_core::decoder::StreamingDecoder::open(
            Box::new(aura_core::decoder::streaming::MemoryStream::new(
                bytes, true,
            )),
            Some("mp3"),
        )
        .unwrap();
        use aura_core::decoder::Decoder as _;
        let mut out = aura_core::PcmBuffer::new();
        let mut data: Vec<f32> = Vec::new();
        while data.len() < 2000 {
            out.clear();
            let _ = d.decode_next(&mut out).unwrap();
            data.extend(out.take_data());
        }
        data[..2000].to_vec()
    };
    // 基准 B：从 stop 位置（≈3s=144000 帧）直接解码（若是续播则匹配）。
    let direct_cont = {
        let bytes = std::fs::read(&path).unwrap();
        let mut d = aura_core::decoder::StreamingDecoder::open(
            Box::new(aura_core::decoder::streaming::MemoryStream::new(
                bytes, true,
            )),
            Some("mp3"),
        )
        .unwrap();
        use aura_core::decoder::Decoder as _;
        d.seek_to_frame(144_000).unwrap();
        let mut out = aura_core::PcmBuffer::new();
        let mut data: Vec<f32> = Vec::new();
        while data.len() < 2000 {
            out.clear();
            let _ = d.decode_next(&mut out).unwrap();
            data.extend(out.take_data());
        }
        data[..2000].to_vec()
    };
    let cmp = |name: &str, a: &[f32], b: &[f32]| {
        let diffs = a
            .iter()
            .zip(b.iter())
            .filter(|(x, y)| (**x - **y).abs() > 1e-3)
            .count();
        println!("compare {name}: diffs={diffs}/{}", a.len());
        diffs
    };
    let d_zero = cmp("replay vs direct_seek0", &replay, &direct_zero);
    let d_cont = cmp("replay vs direct_cont(144000)", &replay, &direct_cont);
    cmp("direct_seek0 vs direct_cont", &direct_zero, &direct_cont);
    let _ = (d_zero, d_cont);
    let n = fresh.len().min(replay.len());
    let mut diff_at = None;
    for i in 0..n {
        if (fresh[i] - replay[i]).abs() > 1e-3 {
            diff_at = Some(i);
            break;
        }
    }
    match diff_at {
        None => println!("RESULT: 重播与首次播放 PCM 一致（前 {n} 样本）——无泄漏"),
        Some(i) => {
            println!(
                "RESULT: LEAK——首个差异在样本 {i}（帧 {}）：fresh={:.4} replay={:.4}",
                i / 2,
                fresh[i],
                replay[i]
            );
            // 找重新对齐点：replay 从某偏移起与 fresh 逐样本一致（窗口匹配）。
            let win = 200usize;
            let mut align = None;
            'outer: for off in 0..(n - win) {
                for k in 0..win {
                    if (fresh[k] - replay[off + k]).abs() > 1e-3 {
                        continue 'outer;
                    }
                }
                align = Some(off);
                break;
            }
            match align {
                Some(off) => println!(
                    "RESULT: replay 与 fresh[0..] 在 replay 偏移 {off}（帧 {}，≈{:.3}s）处重新对齐 → 泄漏长度 ≈ {off} 样本",
                    off / 2,
                    off as f64 / 96_000.0
                ),
                None => println!("RESULT: replay 在整个窗口内与 fresh 无对齐（可能整体为续播/其它内容）"),
            }
            let s: Vec<f32> = replay[..12.min(replay.len())].to_vec();
            println!("replay 开头 12 样本: {s:?}");
        }
    }
}
