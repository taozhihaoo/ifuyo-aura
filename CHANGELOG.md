# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — M3: Music Library / Metadata / SQLite / Search / Artwork (2026-10-04)

- `aura-library` crate: local directory scanner (iterative, cancellable, per-file error tolerant, no-reparse-follow policy, root boundary via canonicalize), extension filter (.flac/.mp3/.ogg/.oga/.wav/.m4a/.alac) with content-probe rejection of fake audio.
- Content identity: size + head/tail 1MiB SipHash-2-4-128 fingerprint; rename/move keep TrackId, tag-only edits keep TrackId via audio-signature resolution, audio replacement creates a new identity; orphan re-link covers move+edit; stats survive deletion (D-037..D-039).
- SQLite (rusqlite bundled, WAL): schema v1 (tracks/track_paths/artists/albums/genres/artworks/playback_stats/library_roots + track_analysis placeholder + playlists/smart_playlists/queue models), transactional `user_version` migrations with fresh/upgrade/rerun/failure-rollback/reopen/WAL/corruption tests.
- FTS5 search: pre-tokenized CJK single+sliding-bigram indexing (voiced-kana NFKD trap fixed), quoted-prefix query builder (injection-proof), bm25 weighted ranking with deterministic tie-break; filter expression AST + parser (`field:value`, `>`, `<`, `min-max`, analysis fields => MissingData, no panic).
- Artwork: embedded extraction (front-cover first, byte-signature MIME sniff, 8MB guard), content-hash dedup, 64/256 thumbnails via deterministic box-filter, atomic tmp+rename writes, decode memory limits (hostile-PNG tested), corrupt-cache invalidate/rebuild.
- Incremental scan: size+mtime skip (40x faster rescan), idempotent triple-scan, per-file transaction commits, cancellation at entry boundary, structured ScanResult/ScanProgress; LibraryService with injected clock.
- Watcher: notify adapter → deterministic coalescer (500ms window, Removed>Renamed>Changed fold, injected clock) → rescan candidates; no DB writes in watcher path; real-watcher integration tests with bounded condition wait.
- Query API: list/get/search/filter/artwork/artists/albums, 6 sort keys with NULLS LAST + id tie-break, limit/offset pagination, playback stats record/read, `integrity_check` diagnostics.
- [B] playlist models: manual playlists (stable ordering, position compaction, cascade delete), smart playlists (executable query storage, no fabricated analysis), queue + command log with undo (Clear undo explicitly a no-op).
- Tests: 110 aura-library tests (was 1). PERF.md M3 baselines (release): scan 1000 @ 3484 files/s, incremental rescan 25ms, 10k-row search avg 1.1ms, identity hash 1.49ms/5MB, thumbnail 2.46ms.

### Added — M2: Playback Quality (2026-10-03)

- Sample-level gapless playback: single-pipeline sequential decode with 3s next-track predecode window, `TrackFrameRange` accounting, no generation bump at natural track boundaries.
- Crossfade 0–12s with equal-power curves (cos/sin), frame-based model, gapless-priority policy, degenerate-overlap downgrade; seek/stop/load cancel transitions.
- Loudness normalization consuming `TrackLoudness` metadata (target −16 LUFS), true-peak-safe gain clamping, tanh soft limiter (last line of defense, ≤ 1.0 dBFS).
- [B] 10-band RBJ parametric EQ (Peak/Shelf), preamp ±12dB, 6 pure-data presets, lock-free atomic snapshot bus (50×AtomicU32 + even/odd seq, tear detection).
- Output device hotplug state machine: safe pause on removal, position preservation, format reconfiguration on restore, no auto-play, stream-error counting.
- Zero-alloc proof upgraded: full DSP chain (volume+loudness+EQ+limiter) callback path, separate test binary to avoid parallel-counter interference; resampler moved to `process_chunk_into` (into-buffer, no per-chunk allocation).
- Tests: 108 workspace green (was 70). PERF.md: full-DSP callback avg 44.6µs / p95 49.3µs / p99 54.6µs per 1024-frame chunk (0.21% of 21.3ms budget); 1h-equivalent soak PASS.

### Added — M1: Audio Core (2026-10-03)

- `aura-core` audio engine: decoder abstraction + real FLAC path (symphonia-bundle-flac), mono→stereo channel map, rubato sinc resampler (44.1k↔48k matrix tested), frame-based SPSC ring buffer with generation-aware stale-sample rejection, playback state machine (Stopped/Paused/Playing), frame-authoritative position, volume with 10ms per-sample gain smoothing, underrun silence+counter, `AudioError` model.
- Deterministic testing base: NullOutput capture, `render_for_frames`, counting-allocator zero-allocation proof for the callback path (tests/zero_alloc.rs).
- Real FLAC decode tests against a committed 21KB libsndfile-generated fixture (decode+seek, sample-exact).
- CPAL real output adapter skeleton (default device, f32/i16, safe error callback).
- `docs/PERF.md`: measured baselines (ring 5×10⁹ fps, FLAC decode 3668× RT, resample 285× RT, callback 5.1µs/1024-frame chunk) on Ryzen 7 7700 / rustc 1.99.0.

### Added — M0: Project Foundation (2026-10-03)

- Rust toolchain pinned: stable `x86_64-pc-windows-msvc` (rustc 1.99.0), MSRV verified at **1.87** against the real dependency tree (`rust-toolchain.toml`, DECISIONS D-011).
- M1 audio dependency lock in `aura-core`: symphonia 0.6.1, cpal 0.18.2, rubato 5.0.1, rtrb 0.4.0 (DECISIONS D-013); `cargo deny` licenses/bans/sources green across 127 packages.
- `src-tauri` wired into the Cargo workspace (D-010 fulfilled); placeholder brand-color icon set generated via `@tauri-apps/cli icon` (official logo lands by M6).
- Minimal typed IPC contract: `app_info` command with brand data baked in from `brand.json`, TS mirror in `ui/src/ipc/`, contract tests on both sides.
- `aura-testkit` substance: `VirtualClock` (deterministic clock), `DeterministicRng` (SplitMix64), synthetic PCM generators (sine/silence/DC), `NullOutput` with frame/underrun accounting (FaultFS deferred to M3/M5).

## [0.1.0] - 2026-10-02 — Phase 0: Commercial Foundation Freeze

### Added

- Project identity: `brand.json` as single brand source of truth (name `ifuyo Aura`, vendor `ifuyo · 伊芙游`).
- Governance documentation: README (bilingual), ARCHITECTURE, DECISIONS (D-001…D-012), PROGRESS, SOURCES, CONTENT_POLICY, PRIVACY, BRAND, SECURITY, CODE_SIGNING_POLICY, THIRD_PARTY_LICENSES.
- Dual licensing: `LICENSE-MIT` OR `LICENSE-APACHE`.
- `config/unsupported_platforms.json`: unsupported platform registry (reject-before-request).
- Cargo workspace skeleton with 7 crates: `aura-core`, `aura-sources`, `aura-cache`, `aura-library`, `aura-analysis`, `aura-synth`, `aura-testkit` (dependency-free, skeleton only).
- Rust quality configs: `rustfmt.toml`, `clippy.toml`, `deny.toml`.
- Frontend scaffold (`ui/`): Vite + React 18 + TypeScript, design tokens (`ui/src/design/tokens.ts`), generated CSS variables (`ui/src/generated/`), token generator/validator (`scripts/generate-tokens.mjs`), eslint flat config, vitest.
- Tauri 2 scaffold (`src-tauri/`), not yet wired into the workspace (DECISIONS D-010).
- CI skeleton (`.github/workflows/ci.yml`) reserving all milestone gates.

### Notes

- Product renamed from `IFUYO Music` to `ifuyo Aura` on 2026-10-02 (charter change, see DECISIONS D-001).
- No M1+ functionality is included in this release.
