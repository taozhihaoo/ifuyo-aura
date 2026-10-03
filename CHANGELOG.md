# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added — M6 上篇: Desktop UI Foundation / Tauri Shell / Now Playing / Library / Command Palette / Aurora (2026-10-05)

- Tauri 2 desktop shell: frameless window with custom title bar + window controls (minimal capabilities, no shell permission, strict CSP), 1180×760/min 900×600.
- Rust application layer (src-tauri): AudioHandle driver thread owning the M1 Engine (fill+process loop, 10 Hz snapshots) with real CPAL output via new `detach_consumer_for_external_output` (D-064); LibraryService integration (list/search/scan with progress events + cancellation); app settings JSON v1 with backward-compatible defaults.
- UI (React 18 + TS + Zustand + Vite): split domain/UI stores (ui/player/library/settings), typed IPC client with DTO guards (no `any`), i18n zh-CN/en with fallback chain, UI scale 90–140% via root font-size, global shortcuts (Ctrl+K/Space/Ctrl+F/Esc, typing-aware).
- Now Playing: real playback state via snapshots + rAF position interpolation, seek disabled when non-seekable, volume → M2 smoothing, play/pause/stop controls.
- Library: real FTS5 search with 250ms debounce, @tanstack/react-virtual list (TrackId stable keys), sort → M3 query API, scan progress + cancel (real CancelToken), structured empty/error/loading states.
- Command Palette: declarative command registry (S99), fuzzy match, keyboard-only UX with focus restore, Ctrl+K global.
- Queue drawer: application-layer in-memory queue referencing TrackId (S48–50).
- Mini Player: separate always-on-top frameless WebviewWindow sharing the single audio engine (S109–113/157).
- Aurora: single-canvas Three.js-free raw WebGL GLSL visual (brand-palette derived), playing/idle FPS throttle (30/8), reduced-motion support, stable per-track visual seed, WebGL-failure CSS fallback (S69), resolution cap + resize debounce.
- Tests: 16 vitest (stores/i18n/commands/DTO guards/seed), Rust shell compiles clean; workspace 339 passed / 0 failed (M1–M5 regression intact).

### Added — M5: Persistent Network Cache / Range Reader / LRU / Pin / Recovery (2026-10-04)

- `aura-cache` crate: CachePolicy hard gate (Forbidden zero-write structural guarantee, SessionOnly bounded 32MiB memory, Allowed persistent), content keys via normalized-URL SipHash-2-4-128 (query-preserving, fixed-depth path mapping, secret redaction).
- 256KiB chunk storage: temp-write → verify → atomic rename → SHA-256 → transactional DB record; FaultFS injection (write/truncate/rename) tests; crash-window analysis (orphan cleanup via integrity pass).
- SQLite cache index: independent of library DB (isolation tested), WAL, entries/chunks/pins schema v1, batched access-time updates (S62), LRU ordering with deterministic tie-break.
- CachedRangeReader: cache-first Read+Seek; per-chunk Range fetch with Content-Range validation; 200→non-range degradation to bounded sequential fill; seek limited to filled range; offline cached reads + Network error on miss (no fake EOS); conditional revalidation (If-None-Match/LM, 304=Validated, content-change invalidates, offline=Stale); 3-attempt exponential backoff with injectable jitter/sleeper.
- Fetch deduplication: in-flight registry keyed (content_key, chunk_index); leader persists in critical section; waiter reads cache. Bounded prefetch (2 chunks ahead) + explicit prefetch_ahead with seek-epoch cancellation.
- LRU eviction: capacity counts valid chunk bytes only (default 2GiB, configurable); skips pinned + in-flight leases; CacheFullPinned when all pinned; pin survives restart.
- aura-core ByteStream abstraction (Send+Sync) via aura-sources (S122 — aura-cache free of reqwest types).
- Tests: 52 aura-cache tests (workspace ≈345) incl. FaultFS recovery matrix, crash scenarios, concurrency, soak (60 seek+evict+refetch iterations). PERF.md M5: chunk read 1423 MB/s, write+db 244 MB/s, SHA-256 2429 MB/s, lookup 6.7µs.

### Added — M4: Source System / Local / Direct HTTP / Internet Radio / ICY / Radio Browser (2026-10-04)

- `aura-sources` crate: unified Source model (SourceKind/CachePolicy/Resolved/ResolvedLicense/Attribution/StreamMetadataEvent/SourceState), `resolve_input` with §13 ordering (local → URL validation → blocklist-before-any-request → network probe), Local Source reusing aura-library metadata.
- Network security: URL scheme whitelist, credential-URL rejection, per-hop redirect revalidation (≤5), custom DNS resolver with per-IP private-network check (DNS-rebind-proof), private-network policy with explicit allow knob, platform blocklist (config/unsupported_platforms.json) with zero-request DNS-spy proof, bounded response bodies, redacted URL logging.
- HttpClient: reqwest blocking + rustls, descriptive UA `ifuyo Aura/<version>`, connect/metadata-total/stream-idle timeout split, fake std-TCP HTTP test server with Range support.
- ICY: pure metadata parser (UTF-8-safe, malformed-block drop) + arbitrary-boundary IcyReader stripper + StreamTitle split + event channel; protocol cap 4080 bytes.
- NetworkAudioStream: bounded pipe (finite 1MiB, live ≤30s by bitrate) + feeder thread with ownership-handoff cancellation, consumer-side idle timeout, Range re-request seek (stale-generation pollution fixed), backpressure.
- Internet Radio: live/non-seekable/Forbidden semantics, zero-disk-write tests, live-EOF semantics.
- Radio Browser: /json/servers discovery (no hardcoded server), production-random + test-injected mirror selectors, bounded fallback, tolerant station parser, url_resolved preferred with recorded field, codec support filter, click reporting default-OFF (zero requests), SourceUnavailable offline semantics.
- aura-core: AudioByteStream trait + MediaSource adapter + StreamingDecoder (symphonia probe, default registry); symphonia `mp3` feature enabled (default features exclude it); ResolvedStream.byte_size wired for Range seek; engine integration tests (direct MP3 → Engine PCM non-silent, seek-then-decode, ICY stream decode).
- Tests: 85 aura-sources tests (workspace ≈300). PERF.md M4: ICY strip 1818 MB/s, URL validation 0.96µs, local resolve 65.9µs, live buffer caps.

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
