# Port map: djp-sitrep

13 of 47 units have a Rust port; no original changed since it was recorded.

## Ports

| Unit | Crate | In Rust | Kept in the original | Cases | Last diff | Original |
| --- | --- | --- | --- | ---: | ---: | --- |
| `src/assessment.js` | `rust/assessment-rs` | `isHistorical`, `norm`, `clauses`, `classifyScope` | `assess` | 429 | 429/429 | unchanged |
| `src/evmerge.js` | `rust/evmerge-rs` | `mergeEvents`, `breakNorm`, `hazardOf`, `repScore`, `evHash` | - | 297 | 297/297 | unchanged |
| `src/feedtext.js` | `rust/feedtext-rs` | `decodeEntities`, `htmlToText`, `descFrom`, `newsEnglish`, `cleanTitle` | - | 406 | 406/406 | unchanged |
| `src/geocode.js` | `rust/geocode-rs` | `locate`, `norm`, `queryMapLabels` | - | 264 | 264/264 | unchanged |
| `src/indonesian-nlp.js` | `rust/indonesian-nlp-rs` | `decodeBasicEntities`, `normalizeIndonesian`, `normalizePhraseText`, `segmentHashtag`, `isHistoricalClause`, `isGenericLocationPhrase`, `analyzePost`, `extractImpact`, `extractAffectedEntity`, `extractEventTime`, `metaphorBlocksTopic` | - | 3475 | 3475/3475 | unchanged |
| `src/metrics.js` | `rust/metrics-rs` | `createMetricsService` | - | 1 | 1/1 | unchanged |
| `src/nlp-adapter.js` | `rust/nlp-adapter-rs` | `getConfig`, `redactPII` | `createNlpAdapter` | 54 | 54/54 | unchanged |
| `src/pajak-cluster.js` | `rust/pajak-cluster-rs` | `isTaxText`, `classifyTaxScope`, `classifyPajakThemes`, `buildPajakClusters` | - | 91 | 91/91 | unchanged |
| `src/security.js` | `rust/security-rs` | `rdIPv4Bytes`, `rdPrivateBytes`, `rdPrivateIP`, `publicURL`, `verifyPassword`, `parseUsersSpec`, `hasRole`, `rateKey`, `getClientIp`, `isHttpsReq`, `isSameOriginPost` | `createHttpClient`, `loadTrustedRoots`, `hashPassword`, `createAuthManager`, `createRateLimiter`, `runIsolated`, `createFatalGuard` | 782 | 782/782 | unchanged |
| `src/social-cluster.js` | `rust/social-cluster-rs` | `classifySourceProvenance`, `stripBoilerplateForSimilarity`, `similarityTokens`, `isNearDuplicate`, `deduplicatePosts`, `clusterIncidents`, `matchPostToNews`, `eventSpecificTokens`, `collapseSocialFeed` | `jaccardSimilarity` | 978 | 978/978 | unchanged |
| `src/social-diagnostics.js` | `rust/social-diagnostics-rs` | `createDiagnostics`, `evaluateCorpus` | - | 90 | 90/90 | unchanged |
| `src/uptime-service.js` | `rust/uptime-service-rs` | `normalizeCheckResult` | `createUptimeService` | 212 | 212/212 | unchanged |
| `src/xpool.js` | `rust/xpool-rs` | `buildXAccounts`, `formatWibTime` | `createXPool` | 150 | 150/150 | unchanged |

## Whole app

| Glue | Runs |
| --- | --- |
| `rust/app/src/units/geocode.rs` | Rust only |
| `rust/app/src/units/security.rs` | Rust only |
| `rust/app/src/units/uptime_service.rs` | Rust only |

## Not ported yet

In the order `analyze` suggests.

| Unit | Lines | Exports | Readiness |
| --- | ---: | ---: | ---: |
| `src/social-history.js` | 105 | 5 | 79 |
| `src/tomtom.js` | 206 | 1 | 65 |
| `src/copernicus.js` | 421 | 4 | 63 |
| `article-extract.js` | 3 | 0 | 100 |
| `article-media.js` | 1 | 0 | 100 |
| `assessment.js` | 1 | 0 | 100 |
| `cctv-gateway.js` | 1 | 0 | 100 |
| `copernicus.js` | 1 | 0 | 100 |
| `evmerge.js` | 1 | 0 | 100 |
| `feedtext.js` | 1 | 0 | 100 |
| `geocode.js` | 1 | 0 | 100 |
| `indonesian-nlp.js` | 1 | 0 | 100 |
| `nlp-adapter.js` | 1 | 0 | 100 |
| `security.js` | 1 | 0 | 100 |
| `social-cluster.js` | 1 | 0 | 100 |
| `social-diagnostics.js` | 1 | 0 | 100 |
| `social-history.js` | 1 | 0 | 100 |
| `social.js` | 1 | 0 | 100 |
| `xpool.js` | 1 | 0 | 100 |
| `public/` | 537 | 0 | 50 |
| `src/cctv-catalog.js` | 505 | 1 | 38 |
| `src/exa-summary.js` | 480 | 5 | 46 |
| `scripts/` | 489 | 0 | 35 |
| `src/market.js` | 376 | 1 | 47 |
| `src/geo-sensors.js` | 675 | 1 | 31 |
| `src/article-media.js` | 919 | 19 | 24 |
| `src/social.js` | 734 | 16 | 22 |
| `src/news-service.js` | 1153 | 1 | 0 |
| `src/cctv-gateway.js` | 723 | 4 | 26 |
| `src/social-service.js` | 1405 | 1 | 0 |
| `src/article-extract.js` | 1094 | 26 | 2 |
| `src/reader-service.js` | 1979 | 1 | 0 |
| `server.js` | 1957 | 5 | 0 |
| `src/client/scripts/` | 10212 | 0 | 0 |

Regenerate with `rust-anything map --write`. Commit this file: when the original changes upstream, the next map marks every port whose recording it outdated.
