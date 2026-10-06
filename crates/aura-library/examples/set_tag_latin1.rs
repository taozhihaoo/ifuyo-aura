//! M15a fixture 工具：以 Latin-1 形态写入 GBK 原字节的标签文本
//! （模拟老 MP3 的 ID3v2.3 encoding=0 乱码场景；gen-fixtures.py 调用）。
//! 用法：cargo run -p aura-library --example set_tag_latin1 -- <file> <text>
use encoding_rs::GBK;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{ItemKey, ItemValue, TagItem};

fn main() {
    let mut args = std::env::args().skip(1);
    let path = args.next().expect("usage: set_tag_latin1 <file> <text>");
    let text = args.next().expect("usage: set_tag_latin1 <file> <text>");
    // GBK 原字节 → Latin-1 字符串（字节透明映射）
    let (bytes, _, _) = GBK.encode(&text);
    let mojibake: String = bytes.iter().map(|&b| b as char).collect();

    let mut tagged = lofty::read_from_path(&path).expect("read");
    let tag = if tagged.primary_tag().is_some() {
        tagged.primary_tag_mut().expect("primary")
    } else {
        tagged.first_tag_mut().expect("first")
    };
    tag.insert(TagItem::new(ItemKey::TrackTitle, ItemValue::Text(mojibake)));
    tagged
        .save_to_path(&path, WriteOptions::default())
        .expect("save");
    println!("Latin-1(GBK) title written: {path}");
}
