//! M3 性能基线（§78）：扫描 1000 文件 / 搜索 10k 记录 / metadata 提取 /
//! 缩略图 / DB 批量写入。全部 `#[ignore]`，用
//! `cargo test -p aura-library --test perf_baseline -- --ignored --nocapture`
//! 显式运行；结果记录于 docs/PERF.md。

use std::fs;
use std::path::{Path, PathBuf};
use std::time::Instant;

use aura_library::query::{Page, SortKey};
use aura_library::service::LibraryService;
use aura_library::types::CancelToken;
use lofty::config::WriteOptions;
use lofty::file::TaggedFileExt;
use lofty::prelude::*;
use lofty::tag::{Tag, TagType};

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures")
        .join(name)
}

fn temp_dir(tag: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!("aura-m3-perf-{tag}-{}", std::process::id()));
    let _ = fs::remove_dir_all(&p);
    fs::create_dir_all(&p).unwrap();
    p
}

/// 生成一个有独立内容（标签不同 → 字节不同）的合法 FLAC。
fn make_track(dir: &Path, i: usize) -> PathBuf {
    let p = dir.join(format!("track_{i:04}.flac"));
    fs::copy(fixture("sine_8k.flac"), &p).unwrap();
    let mut tagged = lofty::read_from_path(&p).unwrap();
    let mut tag = Tag::new(TagType::VorbisComments);
    tag.set_title(format!("Song {i:04}"));
    tag.set_artist(format!("Artist {:02}", i % 50));
    tag.set_album(format!("Album {:03}", i % 100));
    tagged.insert_tag(tag);
    tagged.save_to_path(&p, WriteOptions::default()).unwrap();
    p
}

/// §78 Scan：1000 文件扫描。
#[test]
#[ignore = "perf baseline；显式运行"]
fn scan_1000_files() {
    let root = temp_dir("scan1000");
    let t0 = Instant::now();
    for i in 0..1000 {
        make_track(&root, i);
    }
    let gen = t0.elapsed();

    let mut svc = LibraryService::open_in_memory().unwrap();
    svc.add_root(&root, 1).unwrap();
    let t1 = Instant::now();
    let r = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let scan = t1.elapsed();

    let t2 = Instant::now();
    let r2 = svc.scan_root(&root, &CancelToken::new(), None).unwrap();
    let rescan = t2.elapsed();

    println!("== scan 1000 files ==");
    println!("fixture generation : {gen:?} (1000 tagged flac)");
    println!(
        "initial scan       : {scan:?} ({:.0} files/s), imported={}",
        1000.0 / scan.as_secs_f64(),
        r.files_imported
    );
    println!(
        "incremental rescan : {rescan:?} (skipped={})",
        r2.files_skipped
    );
    assert_eq!(r.files_imported, 1000);
    assert_eq!(r2.files_skipped, 1000);
    let _ = fs::remove_dir_all(&root);
}

/// §78 Search：10k 记录下的搜索延迟。
#[test]
#[ignore = "perf baseline；显式运行"]
fn search_10k_records() {
    let svc = LibraryService::open_in_memory().unwrap();
    let t0 = Instant::now();
    {
        let tx = svc.database().connection().unchecked_transaction().unwrap();
        tx.execute(
            "INSERT INTO library_roots(id, path, date_added_ms) VALUES (1, 'C:/m', 0)",
            [],
        )
        .unwrap();
        for i in 0..10_000i64 {
            let title = format!("Song {i:05}");
            let artist = format!("Artist {:03}", i % 200);
            let album = format!("Album {:04}", i % 500);
            // 直接 DB 写（含 FTS 同步的等价路径：borrow repo 层）。
            aura_library::db::repo::upsert_track(
                &tx,
                aura_library::types::RootId(1),
                std::path::Path::new(&format!("C:/m/{i}.flac")),
                &parsed_for(&title, &artist, &album),
                &identity_for(i, &title, &artist),
                None,
                1000,
                1000,
                i,
            )
            .unwrap();
        }
        tx.commit().unwrap();
    }
    let insert = t0.elapsed();
    println!("== search 10k records ==");
    println!(
        "bulk insert        : {insert:?} ({:.0} rows/s)",
        10_000.0 / insert.as_secs_f64()
    );

    // 查询延迟：50 次混合查询。
    let queries = [
        "song 00500".to_string(),
        "artist 123".to_string(),
        "album 0450".to_string(),
        "nightfall".to_string(),
        "song 09999".to_string(),
    ];
    let mut latencies = Vec::new();
    for q in &queries {
        let t = Instant::now();
        let hits = svc.search_tracks(q, Page::new(50, 0)).unwrap();
        latencies.push(t.elapsed());
        let _ = hits;
    }
    let avg: f64 = latencies.iter().map(|d| d.as_secs_f64()).sum::<f64>() / latencies.len() as f64;
    let worst = latencies.iter().max().unwrap();
    println!("search avg         : {:.2} ms", avg * 1000.0);
    println!("search worst       : {worst:?}");

    // list_tracks 全量分页。
    let t = Instant::now();
    let mut total = 0;
    let mut offset = 0;
    loop {
        let page = svc
            .list_tracks(SortKey::Title, Page::new(500, offset))
            .unwrap();
        let n = page.len();
        total += n;
        offset += 500;
        if n < 500 {
            break;
        }
    }
    println!("list 10k paginated : {:?} ({total} rows)", t.elapsed());
    assert_eq!(total, 10_000);
}

fn parsed_for(title: &str, artist: &str, album: &str) -> aura_library::metadata::ParsedFile {
    let mut m = aura_library::types::TrackMetadata::default();
    m.title = Some(title.to_string());
    m.artist = Some(artist.to_string());
    m.album = Some(album.to_string());
    m.duration_frames = Some(2000);
    m.sample_rate = Some(8000);
    m.channels = Some(1);
    m.codec = Some("flac".into());
    m.container = Some("flac".into());
    aura_library::metadata::ParsedFile {
        metadata: m,
        artwork: None,
        artwork_unavailable: None,
    }
}

fn identity_for(i: i64, title: &str, artist: &str) -> aura_library::types::TrackIdentity {
    let fp = aura_library::identity::fingerprint128(format!("{i}:{title}:{artist}").as_bytes());
    aura_library::types::TrackIdentity {
        identity_key: aura_library::identity::identity_key(1000, &fp),
        audio_sig: "flac/8000/1/2000".to_string(),
    }
}

/// §78：metadata 提取 + 内容身份（含 5MB 文件的 head/tail 采样）。
#[test]
#[ignore = "perf baseline；显式运行"]
fn metadata_and_identity_extraction() {
    let dir = temp_dir("meta");
    let p = make_track(&dir, 1);

    let t = Instant::now();
    for _ in 0..1000 {
        let _ = aura_library::metadata::parse_file(&p).unwrap();
    }
    let parse = t.elapsed();
    println!("== metadata extraction ==");
    println!(
        "parse_file 1000x   : {parse:?} ({:.1} us/file)",
        parse.as_secs_f64() * 1e6 / 1000.0
    );

    // 5MB 文件（> 2MiB → head/tail 1MiB 采样）。
    let big = dir.join("big.flac");
    let base = fs::read(fixture("sine_8k.flac")).unwrap();
    let mut data = Vec::with_capacity(5 << 20);
    while data.len() < 5 << 20 {
        data.extend_from_slice(&base);
    }
    fs::write(&big, &data).unwrap();
    let t = Instant::now();
    for _ in 0..100 {
        let _ = aura_library::identity::content_identity(&big, "flac/8000/1/2000".into()).unwrap();
    }
    let hash = t.elapsed();
    println!(
        "identity 5MB 100x  : {hash:?} ({:.2} ms/file, 2MiB 采样)",
        hash.as_secs_f64() * 10.0
    );
    let _ = fs::remove_dir_all(&dir);
}

/// §78：缩略图生成。
#[test]
#[ignore = "perf baseline；显式运行"]
fn thumbnail_generation() {
    use aura_library::artwork::render_thumbnail_png;
    let img = image::RgbaImage::from_fn(512, 512, |x, y| {
        image::Rgba([
            u8::try_from(x % 256).unwrap(),
            u8::try_from(y % 256).unwrap(),
            128,
            255,
        ])
    });
    use image::ImageEncoder;
    let mut src = Vec::new();
    image::codecs::png::PngEncoder::new(std::io::Cursor::new(&mut src))
        .write_image(img.as_raw(), 512, 512, image::ExtendedColorType::Rgba8)
        .unwrap();

    let t = Instant::now();
    for _ in 0..100 {
        let _ = render_thumbnail_png(&src, 256).unwrap();
    }
    let d = t.elapsed();
    println!("== artwork thumbnails ==");
    println!(
        "512→256 100x       : {d:?} ({:.2} ms/张)",
        d.as_secs_f64() * 10.0
    );
}
