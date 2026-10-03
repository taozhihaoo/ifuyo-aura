//! M6 UI 支撑查询性能（§202/§203）：10k 行下 library_list 分页 +
//! artwork/track_path 类点查的延迟基线。`#[ignore]` 显式运行：
//! `cargo test -p aura-shell --test perf_ui -- --ignored --nocapture`
//! 结果记入 docs/PERF.md（M6 UI 节）。

use std::time::Instant;

use aura_library::query::{Page, SortKey};
use aura_library::service::LibraryService;
use aura_library::types::{TrackIdentity, TrackMetadata};

fn parsed_for(title: &str, artist: &str, album: &str) -> aura_library::metadata::ParsedFile {
    let m = TrackMetadata {
        title: Some(title.to_string()),
        artist: Some(artist.to_string()),
        album: Some(album.to_string()),
        duration_frames: Some(2000),
        sample_rate: Some(8000),
        channels: Some(1),
        codec: Some("flac".into()),
        container: Some("flac".into()),
        ..Default::default()
    };
    aura_library::metadata::ParsedFile {
        metadata: m,
        artwork: None,
        artwork_unavailable: None,
    }
}

fn identity_for(i: i64, title: &str, artist: &str) -> TrackIdentity {
    let fp = aura_library::identity::fingerprint128(format!("{i}:{title}:{artist}").as_bytes());
    TrackIdentity {
        identity_key: aura_library::identity::identity_key(1000, &fp),
        audio_sig: "flac/8000/1/2000".to_string(),
    }
}

#[test]
#[ignore = "perf baseline；显式运行"]
fn ui_queries_10k_rows() {
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
    println!("bulk insert: {:?} (10k rows)", t0.elapsed());

    // §85：library_list 200 行/页（UI 单页渲染量），50 次翻页。
    let mut total = std::time::Duration::ZERO;
    let mut worst = std::time::Duration::ZERO;
    for page in 0..50u32 {
        let t = Instant::now();
        let rows = svc
            .list_tracks(SortKey::Title, Page::new(200, (page * 200) as i64))
            .unwrap();
        assert_eq!(rows.len(), 200);
        let d = t.elapsed();
        worst = worst.max(d);
        total += d;
    }
    println!(
        "list_tracks 200/page x50: avg {:?}, worst {worst:?}",
        total / 50
    );

    // §78：搜索（UI debounce 后单次）。
    let mut stotal = std::time::Duration::ZERO;
    let mut sworst = std::time::Duration::ZERO;
    for q in ["song 00500", "artist 123", "album 0450", "song 09999"] {
        let t = Instant::now();
        let hits = svc.search_tracks(q, Page::new(200, 0)).unwrap();
        assert!(!hits.is_empty());
        let d = t.elapsed();
        sworst = sworst.max(d);
        stotal += d;
    }
    println!("search_tracks x4: avg {:?}, worst {sworst:?}", stotal / 4);

    // §84：track_path 点查（play_track 管线）。
    let t = Instant::now();
    for i in 0..1000i64 {
        let _ = svc
            .database()
            .connection()
            .query_row(
                "SELECT path FROM track_paths WHERE track_id = ?1 AND is_active = 1 LIMIT 1",
                rusqlite::params![i + 1],
                |r| r.get::<_, String>(0),
            )
            .unwrap();
    }
    println!("track_path point x1000: {:?}", t.elapsed() / 1000);
}
