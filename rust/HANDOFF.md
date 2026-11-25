# Session Handoff — `djp-sitrep` Whole-App Rust Port

- **Date**: 2026-11-25
- **Branch**: `feat/rust-port` (baseline commit on `main`: `459bdd1`)
- **Current Status**: **ALL WAVES COMPLETE (Waves 0, 1, 2, 3, 4a, 4b, 5) — 27/27 Crates Gated, 16,468/16,468 Diff Cases (100.0%), 264 Tests Passing, 0.0% Fallback Rate, Standalone Binary Built (`rust/app/target/release/djp-sitrep`), 9.2MB RSS**.
- **Completed Crates & Waves**:
  - **Wave 0**: ADR-001..006, HTTP baseline (272 cases), Axum front door skeleton.
  - **Wave 1 (7/7)**: `evmerge-rs` (10.48x), `feedtext-rs` (1.32x), `uptime-service-rs`, `xpool-rs` (1.90x), `social-diagnostics-rs` (7.14x), `nlp-adapter-rs`, `metrics-rs`.
  - **Wave 2 (6/6)**: `security-rs` (3.09x, global middleware), `geocode-rs` (3.72x), `social-cluster-rs` (2.75x), `pajak-cluster-rs` (2.67x), `indonesian-nlp-rs` (10.21x, `whatlang` ADR-003), `assessment-rs` (4.30x).
  - **Wave 3 (5/5)**: `exa-summary-rs` (10.03x), `copernicus-rs`, `tomtom-rs` (11.93x), `market-rs` (1.70x), `news-service-rs` (4.40x).
  - **Wave 4a (4/4)**: `article-extract-rs` (100%), `article-media-rs` (100%), `social-rs` (100%), `geo-sensors-rs` (1.96x).
  - **Wave 4b (3/3)**: `reader-service-rs` (397.5x), `social-history-rs` (100%), `social-service-rs` (438.4x).
  - **Wave 5 (2/2 + server shell + release binary)**: `cctv-catalog-rs` (4.26x), `cctv-gateway-rs` (6.00x), static file serving via `ServeDir`, standalone release binary (27MB, 9.2MB RSS, 0.0% Node fallback), and deployment script `rust/cutover.sh`.
- **Next Step**: Review with user, execute cutover to Proxmox LXC CT 104 when authorized (ADR-006).

## Key Files
- Plan & Unit Claims: [MAP.md](MAP.md) (includes 7 Porting Rules Learned)
- Measured State: [PORT_MAP.md](PORT_MAP.md)
- Architecture Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..ADR-006)
- Domain Glossary: [GLOSSARY.md](GLOSSARY.md)
- Dependency Research: [research/franc.md](research/franc.md)
- Latest Replay Report: [app-cases/replay-report.md](app-cases/replay-report.md)
- Axum Front Door Crate: [app/Cargo.toml](app/Cargo.toml)
