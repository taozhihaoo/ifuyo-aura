# Security

> Security baseline for ifuyo Aura. This document is maintained in English; it is the
> source of truth for the project's security posture. Where the implementation and this
> document disagree, the implementation must be fixed.

## Reporting a vulnerability

This project is pre-alpha. Please report security issues through **GitHub Security
Advisories** (the repository's *Security* tab), not in a public issue.

Include: affected version, reproduction steps, impact assessment, and a suggested fix if
you have one. We will acknowledge receipt, triage, fix and release — without a fixed SLA.

**In scope:** IPC privilege escalation, path traversal, SSRF, update-signature bypass,
secret leakage, client-side denial of service.

**Out of scope:** social engineering, attacks requiring physical access, and dev-only
tooling issues (for example the Vite dev server).

Only the latest tagged release (`app-v*`) receives security fixes.

## Secret handling

- The repository must never contain real API keys, passwords, tokens, certificate private
  keys or signing private keys.
- Pre-commit audit list: `api_key`, `apikey`, `client_secret`, `password`, `token`,
  `private_key`, `BEGIN PRIVATE KEY`, `.env`.
- Signing and update private keys exist only in CI secrets (see
  [CODE_SIGNING_POLICY.md](CODE_SIGNING_POLICY.md)).
- Application credentials go only to Windows Credential Manager (see [PRIVACY.md](PRIVACY.md)).

## Dependency review

Before adding a dependency, confirm: necessity, license (GPL/AGPL/LGPL are rejected),
maintenance status, Windows compatibility, CI buildability and architectural coupling.
`cargo deny` and the frontend license check run in CI.

## Unsafe Rust review

- Audio callbacks must not allocate, lock, log, perform syscalls, block, do I/O or panic
  (Charter §8.2).
- Every `unsafe` block must document its invariants, be covered by tests, and be
  traceable in review. The goal is to minimise unsafe surface (ring buffers, SIMD and a
  few other hot spots only).

## IPC validation

- Tauri commands are allowlisted; capabilities are minimal.
- Every IPC input is validated (type, range, length). Untrusted input never reaches the
  domain layer directly.
- CSP forbids remote scripts, styles, fonts and images.
- Persisted settings are normalised on both read and write, so a caller cannot bypass
  per-command clamping by saving the whole settings object.

## Path traversal protection

User-supplied paths are normalised and confined to their authorised root. File and
directory access must not escape that root via `..`, symlinks or UNC forms.

## Network and URL validation

### URL policy

- Only `http` and `https` schemes. `file:`, `ftp:`, `data:`, `javascript:`, `blob:`,
  `ws:` and similar are rejected.
- The host must exist and be non-empty; malformed WHATWG-style authorities go through the
  full validation chain.
- URLs carrying credentials (`user:password@host`) are rejected by default (§15) and are
  never written to logs, UI or storage.

### Redirects

At most 5 hops. **Every hop** re-runs the full scheme / credentials / blocked-platform /
private-network checks (§16) — a legal first hop does not make the destination legal
(covered by tests).

### SSRF and private networks

- At the host-string level, `localhost`, `*.localhost`, `*.local`, plus IPv4/IPv6 private,
  loopback, link-local, CGNAT and multicast literals are rejected by default. The
  application layer may allow LAN sources after explicit user confirmation
  (`NetworkPolicy::allow_private`).
- **DNS re-check** (anti-rebinding, §18): a custom resolver validates every resolved IP
  against the private-network rules and fails the whole resolution on a hit — no usable
  address is handed to the connection layer. Covered by DNS-rebinding simulation tests.
- No intranet probing or localhost scanning: the source layer performs no active scanning.

### Unsupported platforms

A hit in `config/unsupported_platforms.json` is rejected before resolution, with **zero
requests** (DNS-spy tests assert zero resolutions, §125).

### Response size limits

- Text responses (JSON/RSS/m3u8/ICY headers): 8 MiB cap (64 KiB for server lists), with
  both a `Content-Length` pre-check and a read-side limit (§21).
- Audio streams use bounded memory buffers (finite: 1 MiB; live: rolling ≤30 s), which
  structurally excludes unbounded accumulation (§29/§100).

### ICY limits

The metadata block protocol is capped at 4080 bytes; malformed or non-UTF-8 blocks are
dropped while audio continues (§33/§36).

### XML safety

M4-A contains no XML parser (RSS is grade B). Any future implementation must disable
external entities; XXE must remain impossible.

### Cancellation

All network requests are cancellable; consumers return immediately after cancellation and
feeder threads do not become orphans (D-052, covered by tests).

### Logging redaction

`tracing` output is filtered before writing: passwords, tokens, API keys, credentials and
full URL query strings (including keys) are redacted. URL logs contain only
`scheme://host/path`; userinfo and query are stripped (`redact_url`, §96).

## Desktop shell (Tauri)

### Capabilities

Minimal set: `core:default` + `core:window:{minimize,toggle-maximize,close,start-dragging,show}`
+ `core:webview:{allow-create-webview-window,allow-get-all-webviews}` + `dialog:default`
(D-066/D-076). No shell, filesystem, updater or HTTP wildcard permissions; no arbitrary
navigation. `create-webview-window` is called only by application code with a fixed URL
(`index.html?view=mini`). Applies to the `main` and `mini` windows.

### CSP

`default-src 'self'; img-src 'self' data:; style-src 'self'; script-src 'self'; font-src 'self'; connect-src ipc: http://ipc.localhost`.
Artwork is delivered as `data:` URLs over an IPC base64 channel, never as filesystem paths
(D-067). Zero remote CDNs or fonts (§25 audit: no hits).

### Asset loading

All assets are embedded (custom-protocol build, D-075); no remote URLs or remote fonts.

### IPC validation

Command arguments are strongly typed (DTO mirrors; serde deserialisation failure rejects).
Local paths are used only for in-library queries (`track_id_by_path`, exact match against
an indexed path). Artwork never passes a path (D-067). Open-URL goes through the M4
validation chain (§193/§278); the frontend performs no `fetch`.

### External links

The UI renders no external links; no `shell` or open capability exists, so the app cannot
launch a browser or execute programs.

### Filesystem access

Validation happens on the Rust side (§192): playback checks existence and probes the
decoder through the M4 local source; scan directories come from the native dialog at
directory granularity (§272 — no pre-scan of system directories). No filesystem wildcard.

### Production DevTools

DevTools are not explicitly enabled (Tauri v2 disables them in release builds). No debug
logging is emitted (`grep` finds no `console.`/`dbg!`/`println!` in non-test code under
`ui/src` or `src-tauri/src`).

### Credential and secret audit (§300)

Full-tree scan for `api_key|apikey|password|client_secret|Authorization|Cookie|BEGIN PRIVATE KEY`
over `src`, `crates`, `ui/src` and `src-tauri`: hits are limited to credential *rejection
logic* and its tests (`url_policy`). No `.env`; no hard-coded secrets. `VisualTelemetry` is
in-process render data, not network telemetry (the name is historical; PRIVACY.md explains).

## Range cache

- Content keys are fixed-length hex derived from the URL; chunk paths are
  `chunks/<prefix>/<key>/<index>.bin` with fixed depth and width and no user-controlled
  strings. Every path is checked to remain under the cache root (§119).
- Invalidating one entry removes only that entry's directory. The entry directory returned
  by `entry_rel_dir` includes the full content key, so two entries sharing a two-character
  prefix can never delete each other's chunks (regression test:
  `remove_entry_files_is_scoped_to_one_content_key`).
- Integrity scans quarantine invalid directory names by path, not by re-parsing them as
  content keys (an invalid key cannot pass validation and would silently no-op).
- Range responses are validated against `Content-Range`; a `200` answer to a range request
  degrades to bounded sequential fill. Offline reads return a network error instead of a
  fake end-of-stream.

## Dreamscape

### Snapshot safety

- Stored at `app_data/dreamscape.json` (§223 — never in music folders), written atomically
  via tmp → rename (§225), sealed before write and validated after read (§300).
- Contains logical state only (§222): no credentials, `Authorization`, cookies or absolute
  paths. An integrity checksum detects tampering (§299, locked by tests).
- Corrupt or version-mismatched snapshots fall back to the default world and keep
  starting (§224). The restore command distinguishes `NO_SNAPSHOT`, `CORRUPT` and
  `VERSION_UNAVAILABLE`.

### Input and resources

- Seed parser: arbitrary UTF-8 (empty, 100 KB, Unicode, emoji, NUL, malformed DS format)
  never panics (§250/§251, 500-round fuzz).
- Snapshot parser: malformed JSON/bytes never panic (§252); unknown fields are ignored and
  missing fields default (§229/§230).
- Manifest: validation-first on the UI side (§253) — fields, numeric finiteness, instance
  count ≤2048, landmarks ≤16; out-of-range input is refused.
- No arbitrary asset paths: everything is generated procedurally (§59/§303 — zero external
  models/textures/HDRI); no network required (§301/§302, works offline).
- IPC parameters: `set_quality` allowlist; a single `advance` is capped at one logical
  year; camera values are clamped.

### Credential scan (§318)

Full-tree scan for `api_key/password/token/client_secret/Authorization/Cookie/BEGIN PRIVATE KEY`:
zero hits for Dreamscape; no `.env`.

## Analysis

- Input validation: `TrackId` is parameterised (library membership checked); batches are
  ≤500 (§241); `advance` has a per-call cap; camera values are clamped.
- Worker isolation: a panicking job is caught by `catch_unwind` and converted into an
  error (§244/§245); embedding failures are isolated (§246) and do not poison other
  features.
- Database failure semantics (§247–§250): SQLite transactions roll back; analysis failure
  does not damage M3 `tracks/playlists/stats/artwork`; `Running → Pending` recovery on
  restart (§211).
- No network (§328 offline audit: PASS): the analysis pipeline has no network path; models
  are in-tree code, nothing is downloaded (§204).
- BLOB limits: embedding BLOBs are checked as `dim × 4` when read back (§29); out-of-range
  reads are rejected.

## Music Map

- Input validation: all `TrackId` queries are parameterised; viewport and limit are
  clamped server-side (`limit ≤ 20 000`); non-finite or invalid viewports fall back to the
  whole map instead of crashing (§192).
- Frontend validation-first: node/status/neighbour DTOs are field-guarded; nodes with
  non-finite positions are dropped (§190/§191) so the renderer never sees NaN.
- Build-thread isolation: a background panic does not persist a snapshot (generation
  check); cancellation is idempotent.
- No network: build, query and rendering have no network path.
- Vector validation (§370): empty, wrong-dimension, non-finite or zero embeddings are
  excluded from the dataset.
- Graph validation (§367): restored graphs must pass all seven `validate_graph` checks
  (self-loops, unknown nodes, non-finite weights, duplicates, symmetry, edge budget §61);
  tests cover self-loop and unknown-node rejection.
- Dataset versioning (§281/§282): any change to `model_id/model_version/analysis_version/dim`
  changes the dataset hash, fails validation of the old persisted map and triggers a
  rebuild.
- Derived-store isolation (§185/§373): `map.json` lives in `app_data/map/`; corruption
  deletes and rebuilds it, with zero writes to M3 `tracks`.
- IPC bounds (§366): `k` clamped to 1..50, `limit` to 1..20 000, invalid viewports fall
  back to the full map; filters are structured numbers (no free-form injection surface).
- Viewport/k bounds (§364): server-side `norm_axis` clamping plus NaN fallback.
- NaN/Infinity handling (§370/§371): restore rejects non-finite positions and cluster
  statistics; the frontend drops bad nodes via per-node finite guards.
- Store recovery (§187/§285): a corrupt `map.json` is deleted at startup (locked by tests);
  state becomes `Pending`/`Stale` without crashing the app.
- Build generation (§166/§287): a late result from an old build cannot overwrite a newer
  one (semantic test).
- No network dependency (§362/§363/§490): build, query and search are fully local; search
  reuses the local M3 FTS5 index.
- Parameterised SQL (§372): every `TrackId` query binds parameters through rusqlite;
  the map store is a JSON file with no SQL.

## Windows integration

- SMTC events are untrusted (§232): seek positions are validated by the coordinator
  (finite, non-negative, seekable, clamped §69/§70); strings are length-limited and
  control characters stripped (§133–§136) before reaching the system.
- Tray (§229): stable menu IDs (§156); metadata appears only as display text and is never
  parsed as a command; locale strings are a constant table (no injection surface).
- Single instance (§230/§231): a second instance only triggers show + focus; argv/cwd are
  never parsed, routed or executed.
- No shell execution (§124): SMTC and tray perform zero PowerShell/cmd/external-executable
  calls; no external links (§125).
- Path safety (§235/§236/§48): SMTC artwork accepts only in-app artwork bytes (in-memory
  stream); there is no user-path injection surface.
- Least privilege (§123/§238): Tauri enables only the `tray-icon`/`image-png` features;
  the `windows` crate exposes only four feature areas; no global-shortcut plugin.

## Release engineering

- Update boundary (§109): remote manifest → version validation (downgrades rejected §47,
  unit-tested) → field validation §46 (unit-tested) → minisign signature verification
  (updater plugin, Rust side) → official install flow. There is no "download an exe and
  run it" path (§6/§9 — no shell).
- Updater endpoint (§9/§77): production is HTTPS-only (GitHub Releases `latest.json` §54);
  development uses a local fake server with strictly isolated configuration (a release
  overlay generated from CI secrets). The endpoint is not user-configurable.
- Secrets (§26/§28/§29): private keys and certificates live only in GitHub Secrets;
  gitleaks scans the full history in CI; `.gitleaks.toml` is a baseline with no secret
  allowlist entries.
- CI logs (§27): secrets are never echoed; artifacts, versions and signing status are
  recorded.
- Installers are currently **unsigned**; this is documented in the README and is not a
  claim of a working public update channel.

## Recent hardening

- **Settings writes are unified.** `save_settings` normalises the whole object (language,
  UI scale, crossfade, EQ preset) before persisting, takes the settings lock across
  write + cache update, and uses a unique temporary file — eliminating a concurrent-write
  race that could truncate `settings.json` or leave memory and disk out of sync.
- **Cache invalidation is entry-scoped** (see *Range cache* above), preventing silent
  deletion of unrelated cached tracks.
- **Queue and updater state machines** now advance correctly for `order` playback and
  self-transition on download progress; the updater also respects the user's
  auto-check setting and no longer throws on cancel-during-check.
- **Test harness jitter** is no longer reported as a routing failure: the fake HTTP server
  waits for a complete request head and closes the connection instead of answering `404`
  when the request has not arrived yet.
