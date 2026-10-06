//! M15a 诊断：走应用真实 open_decoder 路径探测新格式。
//! 用法：cargo run -p aura-shell --bin fmt_probe -- <file>...
use aura_shell_lib::audio::open_decoder;

fn probe(path: &str) {
    match open_decoder(std::path::Path::new(path)) {
        Err(e) => println!("{path}: OPEN FAIL: {e}"),
        Ok((mut decoder, duration)) => {
            let f = decoder.format();
            println!(
                "{path}: open OK rate={} ch={} frames={:?} dur={duration:?}",
                f.sample_rate.get(),
                f.channels.get(),
                decoder.duration_frames(),
            );
            // M15a 诊断：复刻 apply_stop 的 seek_to_frame(0)——验证
            // symphonia 各格式 seek(0) 后解码是否仍然健康。
            match decoder.seek_to_frame(0) {
                Ok(()) => println!("  seek(0) OK"),
                Err(e) => println!("  seek(0) ERR: {e}"),
            }
            let mut decoded = 0u64;
            let mut buf = aura_core::PcmBuffer::new();
            for i in 0..10 {
                match decoder.decode_next(&mut buf) {
                    Ok(0) => {
                        println!("  packet{i}: EOF at 0 frames (decoded {decoded})");
                        return;
                    }
                    Ok(n) => decoded += n,
                    Err(e) => {
                        println!("  packet{i}: decode err {e} (decoded {decoded})");
                        return;
                    }
                }
            }
            println!("{path}: decode OK {decoded} frames");
        }
    }
}

fn n_frames_zero(n: u64) -> bool {
    n == 0
}

fn main() {
    for p in std::env::args().skip(1) {
        probe(&p);
    }
}
