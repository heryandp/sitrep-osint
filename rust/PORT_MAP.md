# Port map: djp-sitrep

27 of 47 units have a Rust port; no original changed since it was recorded.

## Ports

| Unit | Crate | In Rust | Kept in the original | Cases | Last diff | Original |
| --- | --- | --- | --- | ---: | ---: | --- |
| `src/article-extract.js` | `rust/article-extract-rs` | `isChallengeHtml`, `decodeEntities`, `rdClean`, `rdInline`, `rdHost`, `rdAbs`, `parseAttrs`, `pickBestSrcset`, `rdMeta`, `rdLd`, `rdName`, `findMatchingClose`, `stripElementsByTag`, `stripJunkContainers`, `stripCardLinks`, `stripLinkBlocks`, `extractContainers`, `rdBlocks`, `rdVideos`, `rdOnTopic`, `topicTokens`, `pruneBlocks`, `computeTextQuality`, `cleanArticleTitle`, `rdExtract`, `rdJinaMd` | - | 3099 | 3099/3099 | unchanged |
| `src/article-media.js` | `rust/article-media-rs` | `canonicalArticleUrl`, `regDomain`, `resolveUrl`, `upgradeImage`, `imageKey`, `dirKey`, `rdSmallHint`, `rdJunkUrl`, `classifyCandidateUrl`, `classifyCandidateAttrs`, `imageSize`, `inspectImageBuffer`, `validateImageMetrics`, `extractHtmlImageCandidates`, `extractJinaImageCandidates`, `extractFeedImageCandidates`, `feedImage`, `isHijackedHtml`, `resolveArticleMedia` | - | 1886 | 1886/1886 | unchanged |
| `src/assessment.js` | `rust/assessment-rs` | `isHistorical`, `norm`, `clauses`, `classifyScope` | `assess` | 429 | 429/429 | unchanged |
| `src/cctv-catalog.js` | `rust/cctv-catalog-rs` | `createCctvCatalog` | - | 12 | 12/12 | unchanged |
| `src/cctv-gateway.js` | `rust/cctv-gateway-rs` | `createGateway`, `makeLane`, `makeBucket`, `makeLru` | - | 533 | 533/533 | unchanged |
| `src/copernicus.js` | `rust/copernicus-rs` | `tileToBbox3857`, `tileToBbox4326`, `tileIntersectsBounds` | `createCopernicusProxy` | 646 | 646/646 | unchanged |
| `src/evmerge.js` | `rust/evmerge-rs` | `mergeEvents`, `breakNorm`, `hazardOf`, `repScore`, `evHash` | - | 297 | 297/297 | unchanged |
| `src/exa-summary.js` | `rust/exa-summary-rs` | `isValidPublicHttpUrl`, `extractKeyEntities`, `buildFallbackSummary`, `parseExaSummaryObject` | `createExaSummaryService` | 259 | 259/259 | unchanged |
| `src/feedtext.js` | `rust/feedtext-rs` | `decodeEntities`, `htmlToText`, `descFrom`, `newsEnglish`, `cleanTitle` | - | 406 | 406/406 | unchanged |
| `src/geo-sensors.js` | `rust/geo-sensors-rs` | `createGeoSensors`, `parseMagma`, `csvLine`, `parseCsv`, `parseCapField`, `parsePolygon` | - | 166 | 166/166 | unchanged |
| `src/geocode.js` | `rust/geocode-rs` | `locate`, `norm`, `queryMapLabels` | - | 264 | 264/264 | unchanged |
| `src/indonesian-nlp.js` | `rust/indonesian-nlp-rs` | `decodeBasicEntities`, `normalizeIndonesian`, `normalizePhraseText`, `segmentHashtag`, `isHistoricalClause`, `isGenericLocationPhrase`, `analyzePost`, `extractImpact`, `extractAffectedEntity`, `extractEventTime`, `metaphorBlocksTopic` | - | 3475 | 3475/3475 | unchanged |
| `src/market.js` | `rust/market-rs` | `downsampleCloses`, `cnbcSymbolFor` | `createMarketService` | 248 | 248/248 | unchanged |
| `src/metrics.js` | `rust/metrics-rs` | `createMetricsService` | - | 1 | 1/1 | unchanged |
| `src/news-service.js` | `rust/news-service-rs` | `cleanPub`, `isBannedHostUrl`, `isBannedNewsItem`, `breakNorm`, `wibDayStartMs` | `createNewsService` | 81 | 81/81 | unchanged |
| `src/nlp-adapter.js` | `rust/nlp-adapter-rs` | `getConfig`, `redactPII` | `createNlpAdapter` | 54 | 54/54 | unchanged |
| `src/pajak-cluster.js` | `rust/pajak-cluster-rs` | `isTaxText`, `classifyTaxScope`, `classifyPajakThemes`, `buildPajakClusters` | - | 91 | 91/91 | unchanged |
| `src/reader-service.js` | `rust/reader-service-rs` | `createReaderService` | - | 12 | 12/12 | unchanged |
| `src/security.js` | `rust/security-rs` | `rdIPv4Bytes`, `rdPrivateBytes`, `rdPrivateIP`, `publicURL`, `verifyPassword`, `parseUsersSpec`, `hasRole`, `rateKey`, `getClientIp`, `isHttpsReq`, `isSameOriginPost` | `createHttpClient`, `loadTrustedRoots`, `hashPassword`, `createAuthManager`, `createRateLimiter`, `runIsolated`, `createFatalGuard` | 782 | 782/782 | unchanged |
| `src/social-cluster.js` | `rust/social-cluster-rs` | `classifySourceProvenance`, `stripBoilerplateForSimilarity`, `similarityTokens`, `isNearDuplicate`, `deduplicatePosts`, `clusterIncidents`, `matchPostToNews`, `eventSpecificTokens`, `collapseSocialFeed` | `jaccardSimilarity` | 978 | 978/978 | unchanged |
| `src/social-diagnostics.js` | `rust/social-diagnostics-rs` | `createDiagnostics`, `evaluateCorpus` | - | 90 | 90/90 | unchanged |
| `src/social-history.js` | `rust/social-history-rs` | `keyOf`, `tsOf`, `tag`, `retag`, `snapshot` | - | 545 | 545/545 | unchanged |
| `src/social-service.js` | `rust/social-service-rs` | `createSocialService` | - | 12 | 12/12 | unchanged |
| `src/social.js` | `rust/social-rs` | `socAssess`, `socTopics`, `socAdhoc`, `socKeepX`, `socKeepTh`, `socBalance`, `socQueries`, `socHot`, `socNonLatin`, `socVi`, `socHashJunk`, `astroturfSig`, `socForeignLang`, `socKeywordMine`, `explainPost` | `socPhraseScan` | 1320 | 1320/1320 | unchanged |
| `src/tomtom.js` | `rust/tomtom-rs` | `haversine`, `roundGrid`, `roundGridStr`, `ttHostOK` | `createTomTomService` | 420 | 420/420 | unchanged |
| `src/uptime-service.js` | `rust/uptime-service-rs` | `normalizeCheckResult` | `createUptimeService` | 212 | 212/212 | unchanged |
| `src/xpool.js` | `rust/xpool-rs` | `buildXAccounts`, `formatWibTime` | `createXPool` | 150 | 150/150 | unchanged |

## Whole app

| Glue | Runs |
| --- | --- |
| `rust/app/src/units/cctv_catalog.rs` | Rust only |
| `rust/app/src/units/cctv_gateway.rs` | Rust only |
| `rust/app/src/units/copernicus.rs` | Rust only |
| `rust/app/src/units/exa_summary.rs` | Rust only |
| `rust/app/src/units/geo_sensors.rs` | Rust only |
| `rust/app/src/units/geocode.rs` | Rust only |
| `rust/app/src/units/market.rs` | Rust only |
| `rust/app/src/units/news_service.rs` | Rust only |
| `rust/app/src/units/reader_service.rs` | Rust only |
| `rust/app/src/units/security.rs` | Rust only |
| `rust/app/src/units/social_service.rs` | Rust only |
| `rust/app/src/units/tomtom.rs` | Rust only |
| `rust/app/src/units/uptime_service.rs` | Rust only |

## Not ported yet

In the order `analyze` suggests.

| Unit | Lines | Exports | Readiness |
| --- | ---: | ---: | ---: |
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
| `scripts/` | 489 | 0 | 35 |
| `server.js` | 1957 | 5 | 0 |
| `src/client/scripts/` | 10212 | 0 | 0 |

Regenerate with `rust-anything map --write`. Commit this file: when the original changes upstream, the next map marks every port whose recording it outdated.
