# Session Handoff — `djp-sitrep` Whole-App Rust Port

- **Date**: 2026-11-24
- **Branch**: `feat/rust-port` (baseline commit on `main`: `459bdd1`)
- **Current Status**: **Wave 3 complete (18/18 crates gated across Waves 1–3, 8,883/8,883 diff cases passing, 156/156 `cargo test`s passing)** + global Rust `AuthManager` & `RateLimiter` middleware + **23 native Rust HTTP routes** live in `rust/app`.
- **Replay Status**: **`272/272` passed (`100%`)**, **Fallback rate: `95.2%` (`259/272`)** — down from `97.4%`.
- **Completed Crates**:
  - **Wave 1 (7/7)**: `evmerge-rs` (10.48x), `feedtext-rs` (1.32x), `uptime-service-rs` (wired `/api/pajak/uptime`), `xpool-rs` (1.90x), `social-diagnostics-rs` (7.14x), `nlp-adapter-rs`, `metrics-rs`.
  - **Wave 2 (6/6)**: `security-rs` (782/782 diff, 3.09x, wired middleware + `/api/auth/*`), `geocode-rs` (264/264 diff, 3.72x, wired `/api/nodes`, `/api/geo/labels`), `social-cluster-rs` (978/978 diff, 2.75x), `pajak-cluster-rs` (91/91 diff, 2.67x), `indonesian-nlp-rs` (3,475/3,475 diff, `whatlang` ADR-003, 10.21x), `assessment-rs` (429/429 diff, 4.30x).
  - **Wave 3 (5/5)**: `exa-summary-rs` (259/259 diff, 10.03x, wired `/api/exa/status`, `/api/pajak/summarize`, `/api/pajak/brief`), `copernicus-rs` (646/646 diff, wired `/api/copernicus/status`, `/api/copernicus/tile/*`), `tomtom-rs` (420/420 diff, 11.93x, wired `/api/tomtom/*`, `/tomtom/tile/*`), `market-rs` (248/248 diff, 1.70x, wired `/api/market`, `/api/closes`), `news-service-rs` (81/81 diff, 4.40x, wired `/api/news`, `/api/breaking`, `/api/incidents`, `/api/situation`, `/api/pajak/clusters`).
- **Next Wave**: **Wave 4 — Reader, Social & Geo-Sensors (7 parallel crates in 2 batches)**:
  - Batch 4a: `src/article-extract.js`, `src/article-media.js`, `src/social.js`, `src/geo-sensors.js`
  - Batch 4b: `src/reader-service.js`, `src/social-history.js`, `src/social-service.js`

## Key Files
- Plan & Unit Claims: [MAP.md](MAP.md) (includes 7 Porting Rules Learned)
- Measured State: [PORT_MAP.md](PORT_MAP.md)
- Architecture Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..ADR-006)
- Domain Glossary: [GLOSSARY.md](GLOSSARY.md)
- Dependency Research: [research/franc.md](research/franc.md)
- Latest Replay Report: [app-cases/replay-report.md](app-cases/replay-report.md)
- Axum Front Door Crate: [app/Cargo.toml](app/Cargo.toml)
