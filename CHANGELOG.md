# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
