# Whole-App Port Map — `djp-sitrep`

- **Goal**: Standalone pure-Rust HTTP server (`rust/app`) replacing Node.js `server.js` + 27 `src/*.js` modules while preserving 100% UI (`public/`) and API/config/auth drop-in compatibility with Proxmox LXC CT 104.
- **Branch**: `feat/rust-port`
- **ADRs**: [DECISIONS.md](DECISIONS.md) (ADR-001 through ADR-006)
- **Glossary**: [GLOSSARY.md](GLOSSARY.md)
- **Research**: [research/franc.md](research/franc.md)
- **Out of Scope**: `public/` (served unchanged), `src/client/` & `scripts/` (client build tooling stays JS), `threads_worker.py` & `ffmpeg` (invoked as subprocesses per ADR-004).

## Progress Summary
- **Front Door**: `done` (`rust/app` binary `djp-sitrep`, Axum 0.8 + Tokio, **272/272** HTTP cases passing; `GET /api/pajak/uptime` wired & live-verified in pure Rust with 0 fallback)
- **Fallback Rate**: `100.0%` on recorded static/deterministic baseline (`GET /api/pajak/uptime` now served natively by Rust; fallback drops as Wave 2–5 routes wire in)
- **Current Wave**: **Wave 1 complete (7/7 gated)** -> ready for **Wave 2** (Core Domain Engines: `security`, `geocode`, `indonesian-nlp`, `pajak-cluster`, `social-cluster`, `assessment`)

## Wave Plan & Unit Roster

### Wave 0 — Foundation, Recording & Front Door
| Unit / Milestone | Routes / Scope | State | Notes |
| :--- | :--- | :--- | :--- |
| `app-cases` (HTTP recording) | 19 core/security/error routes + 260 semantic traps (`279` total) | `done` | Zero noise warnings (`timestamps` + `epoch_ms` normalizers); 279/279 original pass |
| `rust/app` (Axum front door) | Binary `djp-sitrep` reverse-proxying Node.js | `done` | 279/279 replay pass |

### Wave 1 — Leaf Pure-Logic & Lightweight Services (Zero Internal Deps)
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `metrics` | `src/metrics.js` | 120 | `done` | `gate` PASS, 1/1 diff, 7/7 `cargo test`, `MetricsService` struct ready |
| `xpool` | `src/xpool.js` | 103 | `done` | `gate` PASS, 150/150 diff, `buildXAccounts` **1.90x**, 8/8 `cargo test`, `XPool` struct ready |
| `uptime-service` | `src/uptime-service.js` | 116 | `done` | `gate` PASS, 212/212 diff, 8/8 `cargo test`, async `UptimeService` **wired** to `GET /api/pajak/uptime` |
| `evmerge` | `src/evmerge.js` | 176 | `done` | `gate` PASS, 297/297 diff (100% cov), `mergeEvents` **10.48x faster**, 13/13 `cargo test` |
| `feedtext` | `src/feedtext.js` | 308 | `done` | `gate` PASS, 406/406 diff (99% cov), `descFrom` **1.32x**, 5/5 `cargo test` |
| `nlp-adapter` | `src/nlp-adapter.js` | 139 | `done` | `gate` PASS, 54/54 diff, 5/5 `cargo test`, `NlpAdapter` struct ready |
| `social-diagnostics` | `src/social-diagnostics.js` | 216 | `done` | `gate` PASS, 90/90 diff, `evaluateCorpus` **7.14x**, 6/6 `cargo test`, `Diagnostics` struct ready |

### Wave 2 — Core Domain Engines (Geo, NLP, Security, Clustering)
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `security` | `src/security.js` | 1254 | `open` | `scrypt` auth, HMAC sessions, DNS-pinned SSRF `safeFetch` |
| `geocode` | `src/geocode.js` | 649 | `open` | Loads `geo/gazetteer.json` & `geo/coord-fixes.json` |
| `indonesian-nlp` | `src/indonesian-nlp.js` | 1413 | `open` | Uses `whatlang` (ADR-003) + `geocode` |
| `pajak-cluster` | `src/pajak-cluster.js` | 428 | `open` | 6-dimension tax clustering (`nasional`/`daerah`), has tests |
| `social-cluster` | `src/social-cluster.js` | 501 | `open` | Social topic clustering, has tests |
| `assessment` | `src/assessment.js` | 553 | `open` | Incident & situation scoring |

### Wave 3 — HTTP Services (Tax AI, News, Market, Traffic, Satellite)
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `exa-summary` | `src/exa-summary.js` | 480 | `open` | `/api/exa/status`, `/api/pajak/summarize`, `/api/pajak/brief`, disk cache, has tests |
| `news-service` | `src/news-service.js` | 1153 | `open` | 67 RSS feeds, `/api/news`, `/api/breaking` |
| `market` | `src/market.js` | 376 | `open` | `/api/market`, `/api/closes` |
| `tomtom` | `src/tomtom.js` | 206 | `open` | `/api/tomtom/*`, `/tomtom/tile/*` |
| `copernicus` | `src/copernicus.js` | 421 | `open` | `/api/copernicus/*` |

### Wave 4 — Reader, Social & Geo-Sensors
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `article-extract` | `src/article-extract.js` | 1094 | `open` | HTML extraction (`scraper` + `fancy-regex`) |
| `article-media` | `src/article-media.js` | 919 | `open` | OG/media extraction |
| `reader-service` | `src/reader-service.js` | 1979 | `open` | `/api/read`, `/api/thumb`, `/api/desc`, Firecrawl/Jina fallback |
| `social` | `src/social.js` | 734 | `open` | Classifier & scoring |
| `social-history` | `src/social-history.js` | 105 | `open` | Rolling social history |
| `social-service` | `src/social-service.js` | 1405 | `open` | X GraphQL `SearchByRawQuery` + `threads_worker.py` subprocess |
| `geo-sensors` | `src/geo-sensors.js` | 675 | `open` | AISStream WS, flights, BMKG, NASA FIRMS, Sipongi |

### Wave 5 — CCTV Gateway, Server Shell & Lift
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `cctv-catalog` | `src/cctv-catalog.js` | 505 | `open` | `/api/cams`, `/api/cctv`, `/api/tma` |
| `cctv-gateway` | `src/cctv-gateway.js` | 723 | `open` | `/hls`, `/vhs`, `/img`, `/api/cctv/snap` (`ffmpeg`) |
| `server-shell` | `server.js` | 1957 | `open` | Wire remaining routes (`/api/ping`, `/api/auth/*`, `/api/metrics`, `/api/incidents`, `/api/situation`, static `public/`) |
| `app lift` | `rust/app` | — | `open` | Remove Node fallback, verify 100% replay & 0% fallback, release binary |

## Porting Rules Learned (Wave 1 -> Wave 2+)
1. **`fancy-regex` case-insensitive `\1` trap**: `fancy-regex` 0.14 matches backreference `\1` case-sensitively even under `(?i)`. Expand small tag alternations into linear-time `regex::Regex` instead.
2. **Distinguishing JS `undefined` vs `null`**: Use `serde_json::Value` (not `Option<Value>`) + `rust-anything refresh` when `undefined` triggers a default fallback while `null` throws or behaves differently.
3. **JS ASCII `\b` / `\w` semantics**: Use `(?-u:\b)` and `[A-Za-z0-9_]` in Rust regexes when matching JS `\b`/`\w` around accented Latin characters.
4. **Keeping Rust service structs safe from `refresh`**: Place idiomatic async/stateful structs in `src/service.rs` imported via `#[path = "service.rs"] pub mod service;` inside `src/port.rs` so `rust-anything refresh` never overwrites `lib.rs`.
5. **Root shim imports in `tests/*.test.js`**: Several tests `require('../<mod>')` instead of `../src/<mod>`; supply `--inputs <extra.jsonl>` alongside `--from-tests` to guarantee 95–100% coverage.

## Open Questions
- None (all ADR-001..ADR-006 approved on 2026-11-24).
