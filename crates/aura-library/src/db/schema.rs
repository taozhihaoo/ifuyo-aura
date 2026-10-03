//! SQLite schema（§21/§22/§56）。每版一条迁移脚本，只追加不修改；
//! 破坏性变更必须新增版本（§83：禁止删库重建）。

/// v1：初始 schema（tracks/track_paths/artists/albums/genres/artworks/
/// playback_stats/library_roots + FTS5 + track_analysis 占位 +
/// playlists/smart_playlists/queue 持久化模型 [B]）。
pub const V1: &str = r#"
CREATE TABLE library_roots (
    id            INTEGER PRIMARY KEY,
    path          TEXT NOT NULL UNIQUE,
    date_added_ms INTEGER NOT NULL
);

CREATE TABLE artists (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    norm TEXT NOT NULL UNIQUE
);

CREATE TABLE albums (
    id        INTEGER PRIMARY KEY,
    title     TEXT NOT NULL,
    norm      TEXT NOT NULL,
    artist_id INTEGER REFERENCES artists(id)
);
CREATE UNIQUE INDEX albums_uniq ON albums(norm, IFNULL(artist_id, 0));

CREATE TABLE genres (
    id   INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    norm TEXT NOT NULL UNIQUE
);

CREATE TABLE artworks (
    id           INTEGER PRIMARY KEY,
    content_hash TEXT NOT NULL UNIQUE,
    mime         TEXT NOT NULL,
    byte_size    INTEGER NOT NULL,
    data         BLOB NOT NULL
);

CREATE TABLE tracks (
    id              INTEGER PRIMARY KEY,
    identity_key    TEXT NOT NULL UNIQUE,
    audio_sig       TEXT NOT NULL,
    title           TEXT,
    norm_title      TEXT,
    artist_id       INTEGER REFERENCES artists(id),
    album_id        INTEGER REFERENCES albums(id),
    genre_id        INTEGER REFERENCES genres(id),
    album_artist_id INTEGER REFERENCES artists(id),
    year            INTEGER,
    track_number    INTEGER,
    track_total     INTEGER,
    disc_number     INTEGER,
    disc_total      INTEGER,
    duration_frames INTEGER NOT NULL DEFAULT 0,
    sample_rate     INTEGER NOT NULL DEFAULT 0,
    channels        INTEGER NOT NULL DEFAULT 0,
    codec           TEXT,
    container       TEXT,
    artwork_id      INTEGER REFERENCES artworks(id),
    file_size       INTEGER NOT NULL,
    modified_ms     INTEGER NOT NULL,
    date_added_ms   INTEGER NOT NULL,
    last_scanned_ms INTEGER NOT NULL,
    is_active       INTEGER NOT NULL DEFAULT 1
);
CREATE INDEX tracks_active ON tracks(is_active);

CREATE TABLE track_paths (
    id            INTEGER PRIMARY KEY,
    track_id      INTEGER NOT NULL REFERENCES tracks(id) ON DELETE CASCADE,
    root_id       INTEGER NOT NULL REFERENCES library_roots(id),
    path          TEXT NOT NULL UNIQUE,
    is_active     INTEGER NOT NULL DEFAULT 1,
    first_seen_ms INTEGER NOT NULL,
    last_seen_ms  INTEGER NOT NULL
);
CREATE INDEX track_paths_track ON track_paths(track_id);
CREATE INDEX track_paths_root ON track_paths(root_id);

CREATE TABLE playback_stats (
    track_id            INTEGER PRIMARY KEY REFERENCES tracks(id),
    play_count          INTEGER NOT NULL DEFAULT 0,
    skip_count          INTEGER NOT NULL DEFAULT 0,
    total_played_frames INTEGER NOT NULL DEFAULT 0,
    last_played_at_ms   INTEGER
);

CREATE TABLE track_analysis (
    track_id         INTEGER PRIMARY KEY REFERENCES tracks(id),
    analysis_version INTEGER,
    bpm              REAL,
    bpm_confidence   REAL,
    musical_key      TEXT,
    camelot_key      TEXT,
    lufs             REAL,
    true_peak        REAL,
    energy           REAL,
    brightness       REAL,
    embedding        BLOB,
    analyzed_at_ms   INTEGER
);

CREATE VIRTUAL TABLE tracks_fts USING fts5(
    track_id UNINDEXED,
    title,
    artist,
    album,
    album_artist,
    genre,
    path
);

CREATE TABLE playlists (
    id               INTEGER PRIMARY KEY,
    name             TEXT NOT NULL UNIQUE,
    sort             TEXT,
    date_created_ms  INTEGER NOT NULL,
    date_modified_ms INTEGER NOT NULL
);

CREATE TABLE playlist_tracks (
    playlist_id INTEGER NOT NULL REFERENCES playlists(id) ON DELETE CASCADE,
    position    INTEGER NOT NULL,
    track_id    INTEGER NOT NULL REFERENCES tracks(id),
    PRIMARY KEY (playlist_id, position)
);

CREATE TABLE smart_playlists (
    id          INTEGER PRIMARY KEY REFERENCES playlists(id) ON DELETE CASCADE,
    query       TEXT NOT NULL,
    sort        TEXT,
    limit_count INTEGER
);

CREATE TABLE queue_items (
    id             INTEGER PRIMARY KEY,
    position       INTEGER NOT NULL UNIQUE,
    track_id       INTEGER NOT NULL REFERENCES tracks(id),
    source_context TEXT NOT NULL DEFAULT '{}'
);

CREATE TABLE queue_log (
    id      INTEGER PRIMARY KEY,
    ts_ms   INTEGER NOT NULL,
    op      TEXT NOT NULL,
    payload TEXT NOT NULL
);
"#;
