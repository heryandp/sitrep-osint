# Architecture Decision Records — `djp-sitrep` Rust Port

Source of truth for porting **SITREP // DJP OSINT Console** from Node.js to a standalone Rust server binary using `rust-anything` (`whole-app` workflow).

## ADR-001: Target Shape, Entry Points, and UI Preservation
- **Status**: accepted (2026-11-24)
- **Context**: `djp-sitrep` is an HTTP server (`server.js`, 48 HTTP routes, 27 backend modules under `src/*.js`) serving a compiled browser bundle (`public/index.html` + `public/logo-djp.svg` + `public/*`), alongside dev/build CLI scripts (`scripts/*.js`) and client sources (`src/client/`).
- **Decision**: Port the HTTP server (`server.js` + 27 backend `src/*.js` modules) into a standalone Rust server binary (`rust/app`) using `rust-anything app front-door` -> wave-by-wave `app wire` -> `app lift`. Keep `public/` untouched (served identically by Rust, and/or embedded at lift), and keep `scripts/*.js` and `src/client/` in Node.js for client asset builds.
- **Consequences**: Zero frontend UI rewrite; every existing browser feature (Night/Light theme, 3D Globe, Leaflet Map, Klastering Pajak 6 Dimensi, Exa AI Summarizer, DJP Uptime Strip, Splash Screen) works identically against the Rust server.

## ADR-002: Core Rust Runtime and Crate Stack
- **Status**: accepted (2026-11-24)
- **Context**: The server handles high-concurrency RSS/API polling, SSE/WebSocket geo-sensors, HTML article extraction, Indonesian NLP with JS regex lookarounds, and `scrypt` RBAC session auth.
- **Decision**: Standardize on `tokio` (multi-threaded async runtime), `axum` + `tower-http` (HTTP router, static files, reverse proxy front door), `reqwest` (`rustls-tls`, gzip/brotli/deflate), `serde` + `serde_json`, `fancy-regex` (only where JS lookahead/lookbehind is needed; `regex` otherwise), `scraper` (HTML DOM extraction), `quick-xml` (RSS/Atom/BMKG XML), `scrypt` + `hmac` + `sha2` (auth compatibility), and `tokio-tungstenite` (AISStream WebSocket).
- **Consequences**: Single statically-linked-TLS (`rustls`) binary with no OpenSSL system version mismatch between local Ubuntu and Proxmox LXC CT 104.

## ADR-003: Replacement for `franc` Language Detection
- **Status**: accepted (2026-11-24) — backed by [research/franc.md](research/franc.md)
- **Context**: `franc@6.2.0` is the sole npm runtime dependency in `package.json`, invoked in a single place (`src/indonesian-nlp.js:994`) exclusively to detect 11 European languages (`nld`, `deu`, `fra`, `spa`, `por`, `ita`, `rus`, `pol`, `dan`, `swe`, `nor`) when Indonesian core word hits `< 2`.
- **Decision**: Use `whatlang` (MIT, pure Rust, trigram-based, ~200 KB footprint) with explicit `min_length = 10` guard returning `"und"` and mapping `whatlang::Lang::Nob` -> `"nor"` to match `franc`'s ISO 639-3 contract.
- **Consequences**: Avoids `lingua`'s multi-megabyte model overhead while preserving identical foreign-language filtering semantics.

## ADR-004: External Subprocesses (`threads_worker.py` and `ffmpeg`)
- **Status**: accepted (2026-11-24)
- **Context**: `threads_worker.py` relies on Python `curl_cffi` browser TLS fingerprinting (JA3/BoringSSL) to scrape Threads, and `src/cctv-gateway.js` spawns `ffmpeg` for HLS/RTSP snapshot capture.
- **Decision**: Invoke `threads_worker.py` (via `.venv/bin/python3` or `python3`) and `ffmpeg` as managed async subprocesses (`tokio::process::Command`) from Rust during the whole-app port so both features work with zero regression from day one. Replacing `threads_worker.py` with a pure-Rust TLS-impersonating client (`wreq`/`rquest`) is deferred as an optional post-lift enhancement.
- **Consequences**: Guarantees 100% parity for Threads scraping and CCTV snapshots in CT 104 without risking anti-bot regressions during the core rewrite.

## ADR-005: Drop-In Compatibility & Equivalence Bar
- **Status**: accepted (2026-11-24)
- **Context**: Production Proxmox LXC CT 104 runs with `/opt/djp-sitrep/.env`, `config.json`, `geo/*.json`, and `data/*` disk caches.
- **Decision**: The Rust server MUST be 100% drop-in compatible with:
  - `.env` variables: `PORT`, `SITREP_HOST`/`BIND_HOST`, `SITREP_PUBLIC`, `SITREP_USERS` (including `scrypt$<N>$<r>$<p>$<salt>$<hash>` verification), `SITREP_AUTH_SECRET`, `AISSTREAM_API_KEY`, `NASA_FIRMS_API_KEY`, `FIRECRAWL_API_KEY`, `JINA_API_KEY`, `EXA_API_KEY`, `EXA_DAILY_CAP`, `TOMTOM_API_KEY`, `CDSE_CLIENT_ID`, `CDSE_CLIENT_SECRET`, `X_AUTH_TOKEN`, `X_CT0`, `TH_*`.
  - Data/config files: `config.json`, `geo/gazetteer.json`, `geo/coord-fixes.json`, `geo/big-villages.json`, `data/exa-summary-cache.json`, `data/cctv-*`, `data/social-*`.
  - HTTP equivalence bar: exact HTTP status codes, required headers (`content-type`, auth cookies, cache headers), and JSON schemas; dynamic timestamps (`at`, `ts`, `fetchedAt`, latencies, request IDs) and non-contractual map key order may be normalized during `app replay`.
- **Consequences**: Production cutover on CT 104 is a single systemd `ExecStart` swap with instant rollback to Node.js.

## ADR-006: Build Target, Branching, and Wave Checkpoints
- **Status**: accepted (2026-11-24)
- **Context**: Whole-app porting spans ~20k LOC across 27 modules and multiple sessions.
- **Decision**: Work cleanly on git branch `feat/rust-port` with target `x86_64-unknown-linux-gnu` (Ubuntu 24.04 local & Proxmox LXC). Execute in dependency-ordered waves, pausing at each wave checkpoint to report unit `gate` results, `app replay` pass rate, fallback rate, and update `rust/MAP.md` + `rust/HANDOFF.md`. Keep production CT 104 untouched until explicit user cutover approval.
- **Consequences**: Safe, auditable incremental progress; every commit on `feat/rust-port` keeps the server runnable and verified.
