//! The Rust port of `src/metrics.js`.
//!
//! Exposes `create_metrics_service` for the N-API bridge and the stateful `MetricsService` API
//! for direct use by `rust/app`.

#![forbid(unsafe_code)]

mod pinned;

use std::collections::{BTreeMap, VecDeque};

use serde_json::{Value, json};

/// Upper bounds (ms) for latency histogram buckets (`src/metrics.js:5`).
pub const LAT_BUCKETS: [u64; 7] = [5, 15, 50, 150, 500, 1500, 5000];

/// Fixed-bucket latency histogram.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LatHist {
    pub buckets: [u64; LAT_BUCKETS.len() + 1],
    pub count: u64,
    pub sum_ms: f64,
}

impl LatHist {
    pub fn new() -> Self {
        Self::default()
    }

    fn to_json_value(&self) -> Value {
        json!({ "buckets": self.buckets, "count": self.count, "sumMs": crate::js::number(self.sum_ms) })
    }
}

/// Creates a new empty latency histogram (`src/metrics.js:8`).
pub fn new_lat_hist() -> LatHist {
    LatHist::new()
}

/// Records a latency sample in milliseconds (`src/metrics.js:11`).
pub fn add_lat_sample(h: &mut LatHist, ms: f64) {
    let v = if ms.is_nan() || ms <= 0.0 { 0.0 } else { ms };
    h.count += 1;
    h.sum_ms += v;
    let bi = LAT_BUCKETS
        .iter()
        .position(|&b| v <= b as f64)
        .unwrap_or(LAT_BUCKETS.len());
    h.buckets[bi] += 1;
}

/// Estimates quantile `q` (e.g. `0.5`, `0.95`, `0.99`) from a histogram (`src/metrics.js:21`).
pub fn quantile_from_hist(h: &LatHist, q: f64) -> u64 {
    if h.count == 0 {
        return 0;
    }
    let target = (h.count as f64 * q).ceil();
    let mut cum = 0_u64;
    for (i, &bound) in LAT_BUCKETS.iter().enumerate() {
        cum += h.buckets[i];
        if (cum as f64) >= target {
            return bound;
        }
    }
    LAT_BUCKETS[LAT_BUCKETS.len() - 1]
}

/// One 1-second bucket in the rolling egress byte window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BytesWindowEntry {
    pub sec: i64,
    pub bytes: u64,
}

/// Per-route request metrics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RouteMetric {
    pub count: u64,
    pub err: u64,
    pub cache_hits: u64,
    pub bytes_out: u64,
    pub lat: LatHist,
}

/// Per-upstream provider metrics.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct UpstreamMetric {
    pub count: u64,
    pub err: u64,
    pub lat: LatHist,
}

/// Stateful metrics collector (`src/metrics.js:4`).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct MetricsService {
    pub started_at_ms: i64,
    pub req_total: u64,
    pub rejected_429: u64,
    pub rejected_403: u64,
    pub rejected_503: u64,
    pub outbound_blocked: u64,
    pub proxy_warning: bool,
    pub cache_hits: u64,
    pub bytes_out: u64,
    pub bytes_window: VecDeque<BytesWindowEntry>,
    pub latency: LatHist,
    pub routes: BTreeMap<String, RouteMetric>,
    pub upstream: BTreeMap<String, UpstreamMetric>,
}

impl MetricsService {
    pub fn new() -> Self {
        Self::with_started_at(pinned::now_ms() as i64)
    }

    pub fn with_started_at(started_at_ms: i64) -> Self {
        Self {
            started_at_ms,
            ..Self::default()
        }
    }

    /// Adds outbound bytes using the current clock (`src/metrics.js:46`).
    pub fn add_bytes_out(&mut self, n: u64) {
        self.add_bytes_out_at(n, pinned::now_ms() as i64);
    }

    /// Adds outbound bytes at an explicit millisecond timestamp.
    pub fn add_bytes_out_at(&mut self, n: u64, now_ms: i64) {
        if n == 0 {
            return;
        }
        self.bytes_out += n;
        let sec = now_ms.div_euclid(1000);
        if let Some(last) = self.bytes_window.back_mut()
            && last.sec == sec
        {
            last.bytes += n;
        } else {
            self.bytes_window
                .push_back(BytesWindowEntry { sec, bytes: n });
            while self.bytes_window.len() > 15 {
                self.bytes_window.pop_front();
            }
        }
    }

    /// Rolling 10-second average egress bytes/sec (`src/metrics.js:58`).
    pub fn origin_bytes_per_sec(&self) -> u64 {
        self.origin_bytes_per_sec_at(pinned::now_ms() as i64)
    }

    /// Rolling 10-second average egress bytes/sec at an explicit millisecond timestamp.
    pub fn origin_bytes_per_sec_at(&self, now_ms: i64) -> u64 {
        let cutoff = now_ms.div_euclid(1000) - 9;
        let sum: u64 = self
            .bytes_window
            .iter()
            .filter(|b| b.sec >= cutoff)
            .map(|b| b.bytes)
            .sum();
        let span = (now_ms - self.started_at_ms).div_euclid(1000).clamp(1, 10) as u64;
        crate::js::math_round(sum as f64 / span as f64) as u64
    }

    /// Normalizes a route path with cardinality capping (`src/metrics.js:66`).
    pub fn normalize_metric_route(&self, route: &str) -> String {
        if route.is_empty() {
            return "other".to_string();
        }
        if is_hashed_asset_route(route) {
            return "/asset.js".to_string();
        }
        if route.starts_with("/tomtom/tile/") {
            return "/tomtom/tile/*".to_string();
        }
        if self.routes.contains_key(route) || self.routes.len() < 80 {
            return route.to_string();
        }
        "other".to_string()
    }

    /// Increments total request and rejection counters (`src/metrics.js:73`).
    pub fn metric_inc(&mut self, _route: &str, status: u16) {
        self.req_total += 1;
        match status {
            429 => self.rejected_429 += 1,
            403 => self.rejected_403 += 1,
            503 => self.rejected_503 += 1,
            _ => {}
        }
    }

    /// Records latency, bytes, and per-route stats for one request (`src/metrics.js:79`).
    pub fn record_request_metric(
        &mut self,
        route: &str,
        status: u16,
        ms: f64,
        cache_hit: bool,
        bytes_out: u64,
    ) {
        self.record_request_metric_at(
            route,
            status,
            ms,
            cache_hit,
            bytes_out,
            pinned::now_ms() as i64,
        );
    }

    /// Records request metrics at an explicit millisecond timestamp.
    pub fn record_request_metric_at(
        &mut self,
        route: &str,
        status: u16,
        ms: f64,
        cache_hit: bool,
        bytes_out: u64,
        now_ms: i64,
    ) {
        add_lat_sample(&mut self.latency, ms);
        self.add_bytes_out_at(bytes_out, now_ms);
        let key = self.normalize_metric_route(route);
        let r = self.routes.entry(key).or_default();
        r.count += 1;
        r.err += u64::from(status >= 400);
        r.cache_hits += u64::from(cache_hit);
        r.bytes_out += bytes_out;
        add_lat_sample(&mut r.lat, ms);
    }

    /// Records an upstream provider call with 32-provider cap (`src/metrics.js:94`).
    pub fn record_upstream(&mut self, provider: &str, ok: bool, ms: f64) {
        let raw = if provider.is_empty() {
            "other"
        } else {
            provider
        };
        let key = if raw.is_ascii() {
            raw[..raw.len().min(24)].to_string()
        } else {
            crate::utf16::slice(&crate::utf16::units(raw), 0, 24)
        };
        if !self.upstream.contains_key(&key) && self.upstream.len() >= 32 {
            return;
        }
        let u = self.upstream.entry(key).or_default();
        u.count += 1;
        u.err += u64::from(!ok);
        add_lat_sample(&mut u.lat, ms);
    }

    /// Serializes the service in the exact shape recorded from JS `createMetricsService()`.
    pub fn to_js_snapshot(&self) -> Value {
        let bytes_window: Vec<Value> = self
            .bytes_window
            .iter()
            .map(|b| json!({ "sec": b.sec, "bytes": b.bytes }))
            .collect();
        let routes: Vec<Value> = self
            .routes
            .iter()
            .map(|(k, r)| {
                json!([k, {
                    "count": r.count, "err": r.err, "cacheHits": r.cache_hits,
                    "bytesOut": r.bytes_out, "lat": r.lat.to_json_value(),
                }])
            })
            .collect();
        let upstream: Vec<Value> = self
            .upstream
            .iter()
            .map(|(k, u)| json!([k, { "count": u.count, "err": u.err, "lat": u.lat.to_json_value() }]))
            .collect();
        json!({
            "LAT_BUCKETS": LAT_BUCKETS,
            "eventLoopDelay": {},
            "newLatHist": { "$function": "newLatHist" },
            "addLatSample": { "$function": "addLatSample" },
            "quantileFromHist": { "$function": "quantileFromHist" },
            "METRICS": {
                "startedAt": self.started_at_ms,
                "reqTotal": self.req_total,
                "rejected429": self.rejected_429,
                "rejected403": self.rejected_403,
                "rejected503": self.rejected_503,
                "outboundBlocked": self.outbound_blocked,
                "proxyWarning": self.proxy_warning,
                "cacheHits": self.cache_hits,
                "bytesOut": self.bytes_out,
                "bytesWindow": bytes_window,
                "latency": self.latency.to_json_value(),
                "routes": { "$map": routes },
                "upstream": { "$map": upstream },
            },
            "addBytesOut": { "$function": "addBytesOut" },
            "originBytesPerSec": { "$function": "originBytesPerSec" },
            "normalizeMetricRoute": { "$function": "normalizeMetricRoute" },
            "metricInc": { "$function": "metricInc" },
            "recordRequestMetric": { "$function": "recordRequestMetric" },
            "recordUpstream": { "$function": "recordUpstream" },
        })
    }
}

fn is_hashed_asset_route(route: &str) -> bool {
    let Some(rest) = route.strip_prefix('/').and_then(|s| s.strip_suffix(".js")) else {
        return false;
    };
    let Some((stem, hash)) = rest.split_once('.') else {
        return false;
    };
    matches!(stem, "signals" | "wxicons" | "wire-core" | "cctv-core")
        && hash.len() == 8
        && hash.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f'))
}

/// Port of `createMetricsService` (`src/metrics.js:4`).
pub fn create_metrics_service() -> Result<Value, String> {
    Ok(MetricsService::new().to_js_snapshot())
}

#[cfg(test)]
mod tests {
    use super::*;

    const T0: i64 = 1_700_000_000_000;

    #[test]
    fn histogram_buckets_and_quantiles() {
        let mut h = new_lat_hist();
        assert_eq!(quantile_from_hist(&h, 0.5), 0);
        let samples = [
            -5.0,
            f64::NAN,
            0.0,
            5.0,
            5.1,
            15.0,
            50.0,
            150.0,
            500.0,
            1500.0,
            5000.0,
            9999.0,
        ];
        for ms in samples {
            add_lat_sample(&mut h, ms);
        }
        assert_eq!((h.count, h.buckets), (12, [4, 2, 1, 1, 1, 1, 1, 1]));
        assert!((h.sum_ms - 17224.1).abs() < 1e-9);
        assert_eq!(quantile_from_hist(&h, 0.5), 15);
        assert_eq!(quantile_from_hist(&h, 0.95), 5000);
        assert_eq!(quantile_from_hist(&h, 0.99), 5000);
        assert_eq!(quantile_from_hist(&h, 0.0), 5);
    }

    #[test]
    fn bytes_window_and_origin_bps() {
        let mut svc = MetricsService::with_started_at(T0);
        svc.add_bytes_out_at(0, T0);
        assert!(svc.bytes_out == 0 && svc.bytes_window.is_empty());
        for sec in 0..20_i64 {
            svc.add_bytes_out_at(100, T0 + sec * 1000);
            svc.add_bytes_out_at(50, T0 + sec * 1000 + 500);
        }
        assert_eq!((svc.bytes_out, svc.bytes_window.len()), (3000, 15));
        assert_eq!(svc.bytes_window.front().map(|b| b.sec), Some(1_700_000_005));
        assert_eq!(svc.origin_bytes_per_sec_at(T0 + 19_500), 150);

        let mut early = MetricsService::with_started_at(T0);
        early.add_bytes_out_at(5, T0 + 200);
        assert_eq!(early.origin_bytes_per_sec_at(T0 + 2_000), 3);
    }

    #[test]
    fn route_normalization_and_cap() {
        let mut svc = MetricsService::with_started_at(T0);
        assert_eq!(svc.normalize_metric_route(""), "other");
        for p in [
            "/signals.0123abcd.js",
            "/wxicons.deadbeef.js",
            "/wire-core.00000000.js",
            "/cctv-core.ffffffff.js",
        ] {
            assert_eq!(svc.normalize_metric_route(p), "/asset.js");
        }
        assert_eq!(
            svc.normalize_metric_route("/signals.DEADBEEF.js"),
            "/signals.DEADBEEF.js"
        );
        assert_eq!(
            svc.normalize_metric_route("/tomtom/tile/1/2/3.png"),
            "/tomtom/tile/*"
        );

        for i in 0..80 {
            svc.record_request_metric_at(&format!("/r/{i}"), 200, 10.0, false, 10, T0);
        }
        assert_eq!(svc.routes.len(), 80);
        assert_eq!(svc.normalize_metric_route("/r/0"), "/r/0");
        assert_eq!(svc.normalize_metric_route("/r/new"), "other");
        assert_eq!(
            svc.normalize_metric_route("/signals.abcdef01.js"),
            "/asset.js"
        );
    }

    #[test]
    fn counters_requests_and_upstream_cap() {
        let mut svc = MetricsService::with_started_at(T0);
        for status in [200, 403, 429, 503, 500] {
            svc.metric_inc("/api/news", status);
        }
        assert_eq!(
            (
                svc.req_total,
                svc.rejected_403,
                svc.rejected_429,
                svc.rejected_503
            ),
            (5, 1, 1, 1)
        );

        svc.record_request_metric_at("/api/news", 500, 120.0, true, 512, T0 + 1000);
        let r = &svc.routes["/api/news"];
        assert_eq!((r.count, r.err, r.cache_hits, r.bytes_out), (1, 1, 1, 512));

        svc.record_upstream("", false, 40.0);
        assert_eq!(svc.upstream["other"].err, 1);
        svc.record_upstream("abcdefghijklmnopqrstuvwxyz12345", true, 12.0);
        assert!(svc.upstream.contains_key("abcdefghijklmnopqrstuvwx"));
        for i in 0..40 {
            svc.record_upstream(&format!("p{i}"), true, 5.0);
        }
        assert_eq!(svc.upstream.len(), 32);
        svc.record_upstream("other", true, 5.0);
        assert_eq!(svc.upstream["other"].count, 2);
    }
}
