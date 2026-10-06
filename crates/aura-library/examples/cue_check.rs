//! M15a 诊断：load_for_audio 对真实 fixture 的行为。
use std::path::Path;

fn main() {
    let p = std::env::args().nth(1).expect("usage: cue_check <audio>");
    let sheet = aura_library::cue::load_for_audio(Path::new(&p));
    match sheet {
        Some(s) => {
            println!(
                "file={:?} album={:?} subtracks={}",
                s.file_name,
                s.album,
                s.subtracks.len()
            );
            for t in &s.subtracks {
                println!("  {:02} {:?} start={}", t.number, t.title, t.start_seconds);
            }
        }
        None => println!("None（未找到同名 .cue 或子轨 <2）"),
    }
}
