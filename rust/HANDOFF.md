# Session Handoff — `djp-sitrep` Whole-App Rust Port

- **Date**: 2026-11-24
- **Branch**: `feat/rust-port` (baseline commit on `main`: `459bdd1`)
- **Current Status**: **Wave 1 complete (7/7 leaf crates gated & passing 100% diff)** + first native Rust route (`GET /api/pajak/uptime`) wired into `rust/app`.
- **Replay Status**: `272/272` passed (`100%`); `GET /api/pajak/uptime` live-verified with 0% fallback.
- **Completed Crates (Wave 1)**:
  - `rust/evmerge-rs` (297/297 diff, **10.48x faster**, 13/13 `cargo test`, gate PASS)
  - `rust/feedtext-rs` (406/406 diff, `descFrom` 1.32x, 5/5 `cargo test`, gate PASS)
  - `rust/uptime-service-rs` (212/212 diff, 8/8 `cargo test`, gate PASS, **wired** to `GET /api/pajak/uptime`)
  - `rust/xpool-rs` (150/150 diff, `buildXAccounts` 1.90x, 8/8 `cargo test`, gate PASS)
  - `rust/social-diagnostics-rs` (90/90 diff, `evaluateCorpus` 7.14x, 6/6 `cargo test`, gate PASS)
  - `rust/nlp-adapter-rs` (54/54 diff, 5/5 `cargo test`, gate PASS)
  - `rust/metrics-rs` (1/1 diff, 7/7 `cargo test`, gate PASS)
- **Next Wave**: **Wave 2 — Core Domain Engines** (`src/security.js`, `src/geocode.js`, `src/indonesian-nlp.js`, `src/pajak-cluster.js`, `src/social-cluster.js`, `src/assessment.js`).
  - Note dependency ordering inside Wave 2: `security` + `geocode` + `social-cluster` can run in Wave 2a in parallel; then `indonesian-nlp` + `pajak-cluster` (depend on `geocode`) + `assessment` in Wave 2b!

## Key Files
- Plan & Unit Claims: [MAP.md](MAP.md) (includes Wave 1 Porting Rules Learned)
- Measured State: [PORT_MAP.md](PORT_MAP.md)
- Architecture Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..ADR-006)
- Domain Glossary: [GLOSSARY.md](GLOSSARY.md)
- Dependency Research: [research/franc.md](research/franc.md)
- Latest Replay Report: [app-cases/replay-report.md](app-cases/replay-report.md)
- Axum Front Door Crate: [app/Cargo.toml](app/Cargo.toml)
