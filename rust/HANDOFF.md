# Session Handoff — `djp-sitrep` Whole-App Rust Port

- **Date**: 2026-11-24
- **Branch**: `feat/rust-port` (baseline commit on `main`: `459bdd1`)
- **Current Status**: **Wave 0 complete** (Foundation, `franc` research, HTTP recording, Axum front door).
- **Replay Status**: `279/279` passed (`100%`), Fallback rate: `100.0%` (`279/279`).
- **Next Wave**: **Wave 1** — 7 leaf modules (`src/metrics.js`, `src/xpool.js`, `src/uptime-service.js`, `src/evmerge.js`, `src/feedtext.js`, `src/nlp-adapter.js`, `src/social-diagnostics.js`). Recommended first unit: `src/evmerge.js` or `src/uptime-service.js` (has live HTTP route `/api/pajak/uptime` + unit tests).

## Key Files
- Plan & Unit Claims: [MAP.md](MAP.md)
- Measured State: [PORT_MAP.md](PORT_MAP.md)
- Architecture Decisions: [DECISIONS.md](DECISIONS.md) (ADR-001..ADR-006)
- Domain Glossary: [GLOSSARY.md](GLOSSARY.md)
- Dependency Research: [research/franc.md](research/franc.md)
- Latest Replay Report: [app-cases/replay-report.md](app-cases/replay-report.md)
- Axum Front Door Crate: [app/Cargo.toml](app/Cargo.toml)

## Next Session Instructions
1. Ensure `$HOME/.cargo/bin` is on `PATH` (`rustc 1.100.0`, `rust-anything 3.4.0`).
2. Follow `erkamyaman/rust-anything` `SKILL.md` + `whole-app.md`.
3. Claim Wave 1 units in `rust/MAP.md`, run `rust-anything bridge . <unit>` -> `record --from-tests` -> port `src/port.rs` -> `diff` -> `bench` -> `gate` -> `app wire` (where HTTP routes apply) -> `app replay`.
4. Do NOT touch Proxmox LXC CT `104` until cutover approval.
