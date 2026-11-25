# Session Handoff — `djp-sitrep` Whole-App Rust Port

- **Date**: 2026-11-25
- **Branch**: `feat/rust-port` (baseline commit on `main`: `459bdd1`)
- **Current Status**: **Wave 4a complete (22/22 crates gated across Waves 1–4a, 15,354/15,354 diff cases passing, 191/191 `cargo test`s passing)** + global Rust `AuthManager` & `RateLimiter` middleware + **35 native Rust HTTP routes** live in `rust/app`.
- **Completed Crates**:
  - **Wave 1 (7/7)**: `evmerge-rs` (10.48x), `feedtext-rs` (1.32x), `uptime-service-rs` (wired `/api/pajak/uptime`), `xpool-rs` (1.90x), `social-diagnostics-rs` (7.14x), `nlp-adapter-rs`, `metrics-rs`.
  - **Wave 2 (6/6)**: `security-rs` (782/782 diff, 3.09x, wired middleware + `/api/auth/*`), `geocode-rs` (264/264 diff, 3.72x, wired `/api/nodes`, `/api/geo/labels`), `social-cluster-rs` (978/978 diff, 2.75x), `pajak-cluster-rs` (91/91 diff, 2.67x), `indonesian-nlp-rs` (3,475/3,475 diff, `whatlang` ADR-003, 10.21x), `assessment-rs` (429/429 diff, 4.30x).
  - **Wave 3 (5/5)**: `exa-summary-rs` (259/259 diff, 10.03x, wired `/api/exa/status`, `/api/pajak/summarize`, `/api/pajak/brief`), `copernicus-rs` (646/646 diff, wired `/api/copernicus/status`, `/api/copernicus/tile/*`), `tomtom-rs` (420/420 diff, 11.93x, wired `/api/tomtom/*`, `/tomtom/tile/*`), `market-rs` (248/248 diff, 1.70x, wired `/api/market`, `/api/closes`), `news-service-rs` (81/81 diff, 4.40x, wired `/api/news`, `/api/breaking`, `/api/incidents`, `/api/situation`, `/api/pajak/clusters`).
  - **Wave 4a (4/4)**: `article-extract-rs` (3,099/3,099 diff, 100%), `article-media-rs` (1,886/1,886 diff, 100%), `social-rs` (1,320/1,320 diff, 100%), `geo-sensors-rs` (166/166 diff, 100%, 1.96x, wired 12 routes: `/api/flights`, `/api/ships`, `/api/transit`, `/api/disaster`, `/api/weather`, `/api/wind`, `/api/fires`, `/api/sipongi`, `/api/satellite`, `/api/bmkg-modelrun`, `/api/alerts`, `/api/radar`).
- **Next Step**: **Wave 4b — Data Services & Pipelines (`reader-service`, `social-history`, `social-service`)**

## Key Files
- Plan & Unit Claims: [MAP.md](MAP.md) (includes 7 Porting Rules Learned)
- Measured State: [PORT_MAP.md](PORT_MAP.md)
- Architecture Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..ADR-006)
- Domain Glossary: [GLOSSARY.md](GLOSSARY.md)
- Dependency Research: [research/franc.md](research/franc.md)
- Latest Replay Report: [app-cases/replay-report.md](app-cases/replay-report.md)
- Axum Front Door Crate: [app/Cargo.toml](app/Cargo.toml)
