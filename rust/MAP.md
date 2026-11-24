# Whole-App Port Map — `djp-sitrep`

- **Goal**: Standalone pure-Rust HTTP server (`rust/app`) replacing Node.js `server.js` + 27 `src/*.js` modules while preserving 100% UI (`public/`) and API/config/auth drop-in compatibility with Proxmox LXC CT 104.
- **Branch**: `feat/rust-port`
- **ADRs**: [DECISIONS.md](DECISIONS.md) (ADR-001 through ADR-006)
- **Glossary**: [GLOSSARY.md](GLOSSARY.md)
- **Research**: [research/franc.md](research/franc.md)
- **Out of Scope**: `public/` (served unchanged), `src/client/` & `scripts/` (client build tooling stays JS), `threads_worker.py` & `ffmpeg` (invoked as subprocesses per ADR-004).

## Progress Summary
- **Front Door**: `done` (`rust/app` binary `djp-sitrep`, Axum 0.8 + Tokio, **272/272** HTTP cases passing; global Rust `AuthManager` + `RateLimiter` middleware + **23 native Rust routes** wired: `GET /api/pajak/uptime`, `GET /api/auth/me`, `POST /api/auth/login`, `POST /api/auth/logout`, `GET /api/nodes`, `GET /api/geo/labels`, `GET /api/exa/status`, `POST /api/pajak/summarize`, `POST /api/pajak/brief`, `GET /api/copernicus/status`, `GET /api/copernicus/tile/{*path}`, `GET /api/tomtom/status`, `GET /api/tomtom/flow`, `GET /api/tomtom/incidents`, `GET /api/tomtom/route`, `GET /tomtom/tile/{*path}`, `GET /api/market`, `GET /api/closes`, `GET /api/news`, `GET /api/breaking`, `GET /api/incidents`, `GET /api/situation`, `GET /api/pajak/clusters`)
- **Fallback Rate**: **`95.2%` (`259/272`)** — down from `97.4%`
- **Current Wave**: **Wave 3 complete (18/18 crates gated across Waves 1–3, 8,883/8,883 diff cases, 156 cargo tests)** -> ready for **Wave 4** (Reader, Social & Geo-Sensors)

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
| `security` | `src/security.js` | 1254 | `done` | `gate` PASS, 782/782 diff, `rdPrivateIP` **3.09x**, `publicURL` **1.69x**, 9/9 `cargo test`, **wired** (Axum middleware + `/api/auth/*`) |
| `geocode` | `src/geocode.js` | 649 | `done` | `gate` PASS, 264/264 diff (99% cov), `locate` **3.72x**, 6/6 `cargo test`, **wired** (`/api/nodes`, `/api/geo/labels`) |
| `indonesian-nlp` | `src/indonesian-nlp.js` | 1413 | `done` | `gate` PASS, 3,475/3,475 diff (94% cov, `whatlang` ADR-003), `isGenericLocationPhrase` **10.21x**, 8/8 `cargo test` |
| `pajak-cluster` | `src/pajak-cluster.js` | 428 | `done` | `gate` PASS, 91/91 diff (100% cov), `classifyTaxScope` **2.67x**, `classifyPajakThemes` **1.68x**, 7/7 `cargo test` |
| `social-cluster` | `src/social-cluster.js` | 501 | `done` | `gate` PASS, 978/978 diff (100% cov), `deduplicatePosts` **2.75x**, `collapseSocialFeed` **2.64x**, 7/7 `cargo test` |
| `assessment` | `src/assessment.js` | 553 | `done` | `gate` PASS, 429/429 diff, `classifyScope` **4.30x**, 9/9 `cargo test`, `assess_item` in `service.rs` |

### Wave 3 — HTTP Services (Tax AI, News, Market, Traffic, Satellite)
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `exa-summary` | `src/exa-summary.js` | 480 | `done` | `gate` PASS, 259/259 diff (100%), `isValidPublicHttpUrl` **10.03x**, 9/9 `cargo test`, full async `ExaSummaryService` in `src/service.rs`, wired to `/api/exa/status`, `/api/pajak/summarize`, `/api/pajak/brief` |
| `copernicus` | `src/copernicus.js` | 421 | `done` | `gate` PASS, 646/646 diff (100%), 12/12 `cargo test`, Sentinel-2 WGS84/WebMercator bbox & tile proxy in `src/service.rs`, wired to `/api/copernicus/status`, `/api/copernicus/tile/*` |
| `tomtom` | `src/tomtom.js` | 198 | `done` | `gate` PASS, 420/420 diff (100%), `ttHostOK` **11.93x**, `haversine` **1.43x**, 10/10 `cargo test`, `TomTomService` in `src/service.rs`, wired to `/api/tomtom/status`, `/api/tomtom/flow`, `/api/tomtom/incidents`, `/api/tomtom/route`, `/tomtom/tile/*` |
| `market` | `src/market.js` | 382 | `done` | `gate` PASS, 248/248 diff (100%), `downsampleCloses` **2.17x**, `cnbcSymbolFor` **1.33x**, 8/8 `cargo test`, `MarketService` in `src/service.rs`, wired to `/api/market`, `/api/closes` |
| `news-service` | `src/news-service.js` | 1153 | `done` | `gate` PASS, 81/81 diff (100%), `isBannedHostUrl` **4.40x**, 19/19 `cargo test`, `NewsService` in `src/service.rs`, wired to `/api/news`, `/api/breaking`, `/api/incidents`, `/api/situation`, `/api/pajak/clusters` |

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
6. **Avoid `rustAnythingWireInit` `ctor` collision between sibling N-API crates**: Never add one `rust-anything` cdylib crate as a Cargo path dep inside another `rust-anything` cdylib crate; mount sibling pure-Rust modules directly via `#[path = "../../<crate>/src/service.rs"] mod ...;` instead. (Inside the pure binary `rust/app`, Cargo path deps work cleanly without collision.)
7. **Authoritative Rate Limiter in Axum Middleware**: `rust/app/src/middleware.rs` owns the single `RateLimiter` + `AuthManager` layer so native Rust routes and proxied Node routes share identical `x-ratelimit-remaining` counters.

## Open Questions
- None (all ADR-001..ADR-006 approved on 2026-11-24).
