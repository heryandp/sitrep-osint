# App replay

Build: `rust/app/target/debug/djp-sitrep`

Passed: 270/272

Fallback rate: 95.2% (259/272 cases answered by the original)

## Failing cases

### copernicus-status

Arguments: `GET /api/copernicus/status`

body:

```diff
- {"ok":true,"configured":false,"dayKey":"2026-11-24","monthKey":"2026-11","dailyUsed":0,"dailyCap":1200,"monthUsed":0,"monthCap":9500,"minZoom":6,"maxZoom":12,"tileSize":512,"cacheTtlHours":24,"lookbackDays":45,"cacheEntries":0,"cacheBytes":0,"cooldownSec":0}
+ {"ok":true,"configured":false,"dayKey":"2026-11-25","monthKey":"2026-11","dailyUsed":0,"dailyCap":1200,"monthUsed":0,"monthCap":9500,"minZoom":6,"maxZoom":12,"tileSize":512,"cacheTtlHours":24,"lookbackDays":45,"cacheEntries":0,"cacheBytes":0,"cooldownSec":0}
```

### exa-status

Arguments: `GET /api/exa/status`

body:

```diff
- {"ok":true,"configured":true,"dailyUsed":5,"dailyCap":150,"dailyRemaining":145,"dailyDate":"2026-11-24","cacheEntries":5,"cooldownUntil":0}
+ {"ok":true,"configured":true,"dailyUsed":0,"dailyCap":150,"dailyRemaining":150,"dailyDate":"2026-11-25","cacheEntries":0,"cooldownUntil":0}
```
