# Whole-App Port Map — `djp-sitrep`

- **Goal**: Standalone pure-Rust HTTP server (`rust/app`) replacing Node.js `server.js` + 27 `src/*.js` modules while preserving 100% UI (`public/`) and API/config/auth drop-in compatibility with Proxmox LXC CT 104.
- **Branch**: `feat/rust-port`
- **ADRs**: [DECISIONS.md](DECISIONS.md) (ADR-001 through ADR-006)
- **Glossary**: [GLOSSARY.md](GLOSSARY.md)
- **Research**: [research/franc.md](research/franc.md)
- **Out of Scope**: `public/` (served unchanged), `src/client/` & `scripts/` (client build tooling stays JS), `threads_worker.py` & `ffmpeg` (invoked as subprocesses per ADR-004).

## Progress Summary
- **Front Door**: `done` (`rust/app` binary `djp-sitrep`, Axum 0.8 + Tokio, **279/279** HTTP cases passing)
- **Fallback Rate**: `100.0%` (`279/279` routed to Node.js original — ready to wire Wave 1 units)
- **Current Wave**: Wave 0 complete -> ready for **Wave 1**

## Wave Plan & Unit Roster

### Wave 0 — Foundation, Recording & Front Door
| Unit / Milestone | Routes / Scope | State | Notes |
| :--- | :--- | :--- | :--- |
| `app-cases` (HTTP recording) | 19 core/security/error routes + 260 semantic traps (`279` total) | `done` | Zero noise warnings (`timestamps` + `epoch_ms` normalizers); 279/279 original pass |
| `rust/app` (Axum front door) | Binary `djp-sitrep` reverse-proxying Node.js | `done` | 279/279 replay pass, 100.0% fallback rate |

### Wave 1 — Leaf Pure-Logic & Lightweight Services (Zero Internal Deps)
| Unit | Source File | LOC | State | Notes |
| :--- | :--- | ---: | :--- | :--- |
| `metrics` | `src/metrics.js` | 120 | `open` | Leaf, readiness 90 |
| `xpool` | `src/xpool.js` | 103 | `open` | Leaf, readiness 88 |
| `uptime-service` | `src/uptime-service.js` | 116 | `open` | Leaf, readiness 85, route `/api/pajak/uptime`, has tests |
| `evmerge` | `src/evmerge.js` | 176 | `open` | Leaf, priority 86, has tests |
| `feedtext` | `src/feedtext.js` | 308 | `open` | Leaf, HTML/RSS text cleaner |
| `nlp-adapter` | `src/nlp-adapter.js` | 139 | `open` | Leaf |
| `social-diagnostics` | `src/social-diagnostics.js` | 216 | `open` | Leaf, readiness 86 |

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

## Open Questions
- None (all ADR-001..ADR-006 approved on 2026-11-24).
