# Domain Glossary — `djp-sitrep` Rust Port

Shared mapping between original Node.js domain concepts and Rust module/type names so every unit crate uses consistent terminology.

| Original Term (JS) | Meaning | Rust Target Name |
| :--- | :--- | :--- |
| `SITREP` / `CFG` | Runtime configuration loaded from `config.json` + `.env` | `SitrepConfig` |
| `AUTH` (`publicMode`, `userMode`, `token`) | Deployment auth modes (`SITREP_PUBLIC`, `SITREP_USERS`, `SITREP_AUTH_SECRET`) | `AuthMode`, `AuthState`, `SessionUser` |
| `safeFetch` / `ssrfGuard` | DNS-pinned SSRF-safe outbound HTTP fetcher (`src/security.js`) | `security::safe_fetch` |
| `evmerge` / `mergeEvents` | News & social incident deduplication / cluster merger | `evmerge::merge_events` |
| `PajakCluster` / `6 Dimensi` | Tax news clustering across 6 dimensions (`ideologi`, `politik`, `sosbud`, `ekonomi`, `hukum`, `teknologi`) & scope (`nasional`, `daerah`) | `pajak_cluster::{ClusterResult, DimId, TaxScope}` |
| `ExaSummary` / `EXA_DAILY_CAP` | Exa AI summarizer & daily brief generator with disk cache + extractive fallback | `exa_summary::{SummaryService, SummaryCache}` |
| `UptimeCheck` / `DJP_UPTIME` | Website availability checks via `isdownapi.com` for DJP & Kemenkeu domains | `uptime_service::{UptimeService, DomainStatus}` |
| `Gazetteer` / `GEO.locate` | Indonesian administrative location geocoder (`prov`, `kab`, `kec`, `desa`, `poi`) | `geocode::{Gazetteer, GeoMatch, Precision}` |
| `IndonesianNLP` / `analyzePost` | Normalization, foreign-language filter, intent/hazard/event frame scoring | `indonesian_nlp::{PostAnalysis, LangProfile}` |
| `Assessment` / `assessIncident` | Incident severity, corroboration, and situation summary scoring | `assessment::{IncidentAssessment, SituationSummary}` |
| `SocialCluster` / `SocialSvc` | X (`SearchByRawQuery`) & Threads (`threads_worker.py`) ingest + clustering | `social_cluster`, `social_service`, `social` |
| `ReaderSvc` / `ArticleExtract` | In-app article reader, HTML sanitizer, Firecrawl/Jina fallback, media extractor | `reader_service`, `article_extract`, `article_media` |
| `GeoSensors` | Flights, AISStream ships, BMKG disaster/weather/radar, NASA FIRMS fires, Sipongi | `geo_sensors` |
| `CctvGateway` / `GW` | HLS/VHS/IMG proxy, `ffmpeg` snapshot worker, RUM telemetry | `cctv_gateway`, `cctv_catalog` |
| `Metrics` | Per-route latency histograms, cache hit counters, byte/error metrics | `metrics::MetricsService` |
| `XPool` | Multi-account X cookie rotation & cooldown pool | `xpool::XPool` |
