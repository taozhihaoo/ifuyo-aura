# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
