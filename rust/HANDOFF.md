# Session Handoff — `djp-sitrep` Whole-App Rust Port

- **Date**: 2026-11-24
- **Branch**: `feat/rust-port` (baseline commit on `main`: `459bdd1`)
- **Current Status**: **Wave 2 complete (13/13 crates gated across Waves 1–2, 7,229/7,229 diff cases passing, 98/98 `cargo test`s passing)** + global Rust `AuthManager` & `RateLimiter` middleware + 6 native Rust HTTP routes live in `rust/app`.
- **Replay Status**: **`272/272` passed (`100%`)**, **Fallback rate: `97.4%` (`265/272`)**.
- **Completed Crates**:
  - **Wave 1 (7/7)**: `evmerge-rs` (10.48x), `feedtext-rs` (1.32x), `uptime-service-rs` (wired `/api/pajak/uptime`), `xpool-rs` (1.90x), `social-diagnostics-rs` (7.14x), `nlp-adapter-rs`, `metrics-rs`.
  - **Wave 2 (6/6)**: `security-rs` (782/782 diff, 3.09x, wired middleware + `/api/auth/*`), `geocode-rs` (264/264 diff, 3.72x, wired `/api/nodes`, `/api/geo/labels`), `social-cluster-rs` (978/978 diff, 2.75x), `pajak-cluster-rs` (91/91 diff, 2.67x), `indonesian-nlp-rs` (3,475/3,475 diff, `whatlang` ADR-003, 10.21x), `assessment-rs` (429/429 diff, 4.30x).
- **Next Wave**: **Wave 3 — HTTP Services (5 parallel crates)**:
  - `src/exa-summary.js` (`/api/exa/status`, `/api/pajak/summarize`, `/api/pajak/brief`)
  - `src/news-service.js` (`/api/news`, `/api/breaking`, `/api/incidents`, `/api/situation`, `/api/pajak/clusters`)
  - `src/market.js` (`/api/market`, `/api/closes`)
  - `src/tomtom.js` (`/api/tomtom/*`, `/tomtom/tile/*`)
  - `src/copernicus.js` (`/api/copernicus/*`)

## Key Files
- Plan & Unit Claims: [MAP.md](MAP.md) (includes 7 Porting Rules Learned)
- Measured State: [PORT_MAP.md](PORT_MAP.md)
- Architecture Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..ADR-006)
- Domain Glossary: [GLOSSARY.md](GLOSSARY.md)
- Dependency Research: [research/franc.md](research/franc.md)
- Latest Replay Report: [app-cases/replay-report.md](app-cases/replay-report.md)
- Axum Front Door Crate: [app/Cargo.toml](app/Cargo.toml)
