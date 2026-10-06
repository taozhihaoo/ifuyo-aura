//! M13.3 fixture 工具：向音频文件写入 USLT 歌词标签（gen-fixtures.py 调用）。
//! 用应用同款 lofty 写入——与读取路径保证往返一致。
//! 用法：cargo run -p aura-library --example set_uslt -- <file> <lyrics>
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{ItemKey, ItemValue, TagItem};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: set_uslt <file> <lyrics>");
    let lyrics = args.next().expect("usage: set_uslt <file> <lyrics>");
    let mut tagged = lofty::read_from_path(&path).expect("read");
    let has_primary = tagged.primary_tag().is_some();
    let tag = if has_primary {
        tagged.primary_tag_mut().expect("primary")
    } else {
        tagged.first_tag_mut().expect("first")
    };
    tag.insert(TagItem::new(
        ItemKey::Lyrics,
        ItemValue::Text(lyrics.clone()),
    ));
    tagged
        .save_to_path(&path, WriteOptions::default())
        .expect("save");
    println!("USLT written: {path}");
}
