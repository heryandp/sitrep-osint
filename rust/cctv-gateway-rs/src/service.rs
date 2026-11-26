//! Core CCTV streaming gateway service.
//!
//! Provides concurrency limiting, rate limiting, segment & playlist caching,
//! upstream request coalescing, snapshot generation via ffmpeg/direct fallback,
//! RUM telemetry ingestion, and dead camera tracking.

use bytes::Bytes;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};
use std::collections::{HashMap, VecDeque};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, LazyLock, Mutex};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Semaphore, broadcast};
use url::Url;

static RE_PANTAU: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(^|\.)(pantaucctv\.com|cctvnusantara\.com|99apps\.id)$").expect("valid regex"));
static RE_MOLECOOL: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^cctv\.molecool\.id$").expect("valid regex"));
static RE_KOTANG: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(^|\.)dpuprkotang\.info$").expect("valid regex"));
static RE_TK: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(^|\.)tangerangkab\.go\.id$").expect("valid regex"));
static RE_TK_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)/video/([^/]+)/").expect("valid regex"));
static RE_BALI: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)(^|\.)baliprov\.go\.id$").expect("valid regex"));
static RE_BALI_PATH: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)/cctv/([^/]+)/").expect("valid regex"));
static RE_DIRECT_IMG: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)/api/frame\.jpe?g|/preview\.jpe?g|\.(?:jpe?g|png|webp)(?:\?|#|$)")
        .expect("valid regex")
});
static RE_DSDA: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(^|\.)dsdajakarta\.id$|(^|\.)dsdadki\.web\.id$").expect("valid regex")
});
static RE_HOST_VALID: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[a-zA-Z0-9.:-]+$").expect("valid regex"));
static RE_URI_ATTR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r#"URI="([^"]+)""#).expect("valid regex"));
static RE_TARGET_DUR: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#EXT-X-TARGETDURATION:(\d+)").expect("valid regex"));

fn current_time_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

// ---------------------------------------------------------------------------
// Rate Limiter: Token Bucket
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
struct BucketEntry {
    tokens: f64,
    last_update_ms: u64,
}

#[derive(Debug, Clone)]
pub struct TokenBucket {
    rate: f64,
    burst: f64,
    entries: HashMap<String, BucketEntry>,
    order: VecDeque<String>,
}

impl TokenBucket {
    pub fn new(rate: f64, burst: f64) -> Self {
        Self {
            rate,
            burst,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    fn ensure(&mut self, key: &str, now_ms: u64) -> &mut BucketEntry {
        if !self.entries.contains_key(key) {
            if self.entries.len() >= 5000
                && let Some(oldest) = self.order.pop_front()
            {
                self.entries.remove(&oldest);
            }
            self.order.push_back(key.to_string());
            self.entries.insert(
                key.to_string(),
                BucketEntry {
                    tokens: self.burst,
                    last_update_ms: now_ms,
                },
            );
        }
        // INFALLIBLE: key was just ensured to exist above
        self.entries
            .get_mut(key)
            .expect("key exists in token bucket")
    }

    pub fn take(&mut self, key: &str, cost: f64, now_ms: u64) -> bool {
        let burst = self.burst;
        let rate = self.rate;
        let b = self.ensure(key, now_ms);
        let elapsed_sec = (now_ms.saturating_sub(b.last_update_ms)) as f64 / 1000.0;
        b.tokens = (b.tokens + elapsed_sec * rate).min(burst);
        b.last_update_ms = now_ms;
        if b.tokens < cost {
            false
        } else {
            b.tokens -= cost;
            true
        }
    }

    pub fn charge(&mut self, key: &str, cost: f64, now_ms: u64) {
        let burst = self.burst;
        let rate = self.rate;
        let b = self.ensure(key, now_ms);
        let elapsed_sec = (now_ms.saturating_sub(b.last_update_ms)) as f64 / 1000.0;
        b.tokens = (b.tokens + elapsed_sec * rate).min(burst) - cost;
        b.last_update_ms = now_ms;
    }

    pub fn ok(&self, key: &str, now_ms: u64) -> bool {
        match self.entries.get(key) {
            None => true,
            Some(b) => {
                let elapsed_sec = (now_ms.saturating_sub(b.last_update_ms)) as f64 / 1000.0;
                (b.tokens + elapsed_sec * self.rate).min(self.burst) > 0.0
            }
        }
    }

    pub fn size(&self) -> usize {
        self.entries.len()
    }
}

// ---------------------------------------------------------------------------
// LRU Cache with TTL and Byte Cap
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct LruEntry<V> {
    pub value: V,
    pub size: usize,
    pub expire_at_ms: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LruStats {
    pub entries: usize,
    pub bytes: usize,
}

#[derive(Debug, Clone)]
pub struct LruCache<V> {
    max_bytes: usize,
    current_bytes: usize,
    map: HashMap<String, LruEntry<V>>,
    order: VecDeque<String>,
}

impl<V: Clone> LruCache<V> {
    pub fn new(max_bytes: usize) -> Self {
        Self {
            max_bytes,
            current_bytes: 0,
            map: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    pub fn get(&mut self, key: &str, now_ms: u64) -> Option<V> {
        let entry = self.map.get(key)?;
        if now_ms > entry.expire_at_ms {
            self.delete(key);
            return None;
        }
        // Refresh position in order
        if let Some(pos) = self.order.iter().position(|x| x == key) {
            self.order.remove(pos);
            self.order.push_back(key.to_string());
        }
        self.map.get(key).map(|e| e.value.clone())
    }

    pub fn set(&mut self, key: String, val: V, size: usize, ttl_ms: u64, now_ms: u64) {
        if size > self.max_bytes {
            return;
        }
        self.delete(&key);
        self.current_bytes += size;
        self.order.push_back(key.clone());
        self.map.insert(
            key,
            LruEntry {
                value: val,
                size,
                expire_at_ms: now_ms + ttl_ms,
            },
        );
        while self.current_bytes > self.max_bytes && !self.order.is_empty() {
            if let Some(oldest) = self.order.pop_front()
                && let Some(e) = self.map.remove(&oldest)
            {
                self.current_bytes = self.current_bytes.saturating_sub(e.size);
            }
        }
    }

    pub fn delete(&mut self, key: &str) {
        if let Some(e) = self.map.remove(key) {
            self.current_bytes = self.current_bytes.saturating_sub(e.size);
            if let Some(pos) = self.order.iter().position(|x| x == key) {
                self.order.remove(pos);
            }
        }
    }

    pub fn stats(&self) -> LruStats {
        LruStats {
            entries: self.map.len(),
            bytes: self.current_bytes,
        }
    }
}

// ---------------------------------------------------------------------------
// Concurrency Lane
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct LaneStats {
    pub active: usize,
    pub queued: usize,
    #[serde(rename = "maxQueue")]
    pub max_queue: Option<usize>,
    pub hosts: usize,
    pub cancelled: usize,
}

pub struct ConcurrencyLane {
    max_active: usize,
    per_host: usize,
    max_queue: usize,
    active: AtomicUsize,
    cancelled: AtomicUsize,
    host_counts: Arc<Mutex<HashMap<String, usize>>>,
    global_sem: Arc<Semaphore>,
}

impl ConcurrencyLane {
    pub fn new(max_active: usize, per_host: usize, max_queue: usize) -> Self {
        Self {
            max_active,
            per_host,
            max_queue,
            active: AtomicUsize::new(0),
            cancelled: AtomicUsize::new(0),
            host_counts: Arc::new(Mutex::new(HashMap::new())),
            global_sem: Arc::new(Semaphore::new(max_active)),
        }
    }

    pub async fn run<F, T>(&self, host: &str, f: F) -> Result<T, (u16, &'static str)>
    where
        F: std::future::Future<Output = T>,
    {
        // Check per-host limit
        {
            let per = self
                .host_counts
                .lock()
                .map_err(|_| (500, "mutex poisoned"))?;
            let count = per.get(host).copied().unwrap_or(0);
            if count >= self.per_host {
                // If max queue reached
                if self.active.load(Ordering::Relaxed) >= self.max_queue {
                    return Err((503, "antrean gateway penuh"));
                }
            }
        }

        let _permit = self
            .global_sem
            .acquire()
            .await
            .map_err(|_| (500, "semaphore closed"))?;

        {
            let mut per = self
                .host_counts
                .lock()
                .map_err(|_| (500, "mutex poisoned"))?;
            *per.entry(host.to_string()).or_insert(0) += 1;
            self.active.fetch_add(1, Ordering::Relaxed);
        }

        let result = f.await;

        {
            if let Ok(mut per) = self.host_counts.lock() {
                if let Some(c) = per.get_mut(host) {
                    *c = c.saturating_sub(1);
                    if *c == 0 {
                        per.remove(host);
                    }
                }
                self.active.fetch_sub(1, Ordering::Relaxed);
            }
        }

        Ok(result)
    }

    pub fn max_active(&self) -> usize {
        self.max_active
    }

    pub fn stats(&self) -> LaneStats {
        let hosts = self.host_counts.lock().map(|m| m.len()).unwrap_or(0);
        LaneStats {
            active: self.active.load(Ordering::Relaxed),
            queued: 0,
            max_queue: Some(self.max_queue),
            hosts,
            cancelled: self.cancelled.load(Ordering::Relaxed),
        }
    }
}

// ---------------------------------------------------------------------------
// Telemetry & Reporting Types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RumEvent {
    pub h: String,
    pub t: Option<f64>,
    pub f: Option<bool>,
    pub s: Option<f64>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct RumHostStats {
    pub ok: u64,
    pub fail: u64,
    pub stalls: u64,
    pub ttff_p50: Option<u64>,
    pub ttff_p95: Option<u64>,
}

#[derive(Debug, Clone, Default)]
struct InternalRumHost {
    ok: u64,
    fail: u64,
    stalls: u64,
    ttff: Vec<u64>,
}

#[derive(Debug, Clone, Default)]
struct CameraReport {
    fails: HashMap<String, u64>,
    ok_at: u64,
}

// ---------------------------------------------------------------------------
// Gateway Configuration & Statistics
// ---------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct GatewayOptions {
    pub max_active: usize,
    pub per_host: usize,
    pub max_queue: usize,
    pub snap_active: usize,
    pub snap_max_queue: usize,
    pub seg_cache_bytes: usize,
    pub seg_ttl_ms: u64,
    pub seg_cache_max: usize,
    pub seg_max: usize,
    pub pl_min: u64,
    pub pl_max: u64,
    pub req_rate: f64,
    pub req_burst: f64,
    pub byte_rate: f64,
    pub byte_burst: f64,
    pub global_byte_rate: f64,
    pub global_byte_burst: f64,
    pub snap_ttl_ms: u64,
    pub upstream_ms: u64,
    pub allow_cors: bool,
    pub allow_public_cache: bool,
    pub ffmpeg: Option<String>,
}

impl Default for GatewayOptions {
    fn default() -> Self {
        Self {
            max_active: 48,
            per_host: 8,
            max_queue: 64,
            snap_active: 12,
            snap_max_queue: 64,
            seg_cache_bytes: 64 << 20,
            seg_ttl_ms: 30000,
            seg_cache_max: 16 << 20,
            seg_max: 25 << 20,
            pl_min: 500,
            pl_max: 2500,
            req_rate: 40.0,
            req_burst: 200.0,
            byte_rate: 3.0 * (1 << 20) as f64,
            byte_burst: 120.0 * (1 << 20) as f64,
            global_byte_rate: 45.0 * (1 << 20) as f64,
            global_byte_burst: 180.0 * (1 << 20) as f64,
            snap_ttl_ms: 90000,
            upstream_ms: 25000,
            allow_cors: true,
            allow_public_cache: false,
            ffmpeg: None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GatewayStats {
    pub req: u64,
    pub hit: u64,
    pub coalesced: u64,
    pub upstream: u64,
    pub rl: u64,
    pub global_rl: u64,
    pub bytes_out: u64,
    pub errors: u64,
    pub aborted: u64,
    pub rejected503: u64,
    pub lane: LaneStats,
    pub snap_lane: LaneStats,
    pub cache: LruStats,
    pub snap_cache: LruStats,
    pub inflight: usize,
    pub reports: usize,
}

// ---------------------------------------------------------------------------
// Stateful CctvGateway Service
// ---------------------------------------------------------------------------

type FlightSender = broadcast::Sender<Result<Bytes, (u16, String)>>;

pub struct CctvGateway {
    pub options: GatewayOptions,
    pub lane: ConcurrencyLane,
    pub snap_lane: ConcurrencyLane,
    req_rl: Mutex<TokenBucket>,
    byte_rl: Mutex<TokenBucket>,
    byte_global: Mutex<TokenBucket>,
    rep_rl: Mutex<TokenBucket>,
    rum_rl: Mutex<TokenBucket>,
    cache: Mutex<LruCache<Bytes>>,
    snap_cache: Mutex<LruCache<Bytes>>,
    reports: Mutex<HashMap<String, CameraReport>>,
    rum: Mutex<HashMap<String, InternalRumHost>>,
    inflight: Mutex<HashMap<String, FlightSender>>,
    ffmpeg_detected: AtomicBool,
    ffmpeg_checked: AtomicBool,
    // Metrics
    m_req: AtomicU64,
    m_hit: AtomicU64,
    m_coalesced: AtomicU64,
    m_upstream: AtomicU64,
    m_rl: AtomicU64,
    m_global_rl: AtomicU64,
    m_bytes_out: AtomicU64,
    m_errors: AtomicU64,
    m_aborted: AtomicU64,
    m_rejected503: AtomicU64,
}

impl CctvGateway {
    pub fn new(options: GatewayOptions) -> Self {
        let lane = ConcurrencyLane::new(options.max_active, options.per_host, options.max_queue);
        let snap_lane = ConcurrencyLane::new(
            options.snap_active,
            options.snap_active,
            options.snap_max_queue,
        );
        let req_rl = Mutex::new(TokenBucket::new(options.req_rate, options.req_burst));
        let byte_rl = Mutex::new(TokenBucket::new(options.byte_rate, options.byte_burst));
        let byte_global = Mutex::new(TokenBucket::new(
            options.global_byte_rate,
            options.global_byte_burst,
        ));
        let rep_rl = Mutex::new(TokenBucket::new(1.0, 30.0));
        let rum_rl = Mutex::new(TokenBucket::new(0.5, 10.0));
        let cache = Mutex::new(LruCache::new(options.seg_cache_bytes));
        let snap_cache = Mutex::new(LruCache::new(24 << 20));

        Self {
            options,
            lane,
            snap_lane,
            req_rl,
            byte_rl,
            byte_global,
            rep_rl,
            rum_rl,
            cache,
            snap_cache,
            reports: Mutex::new(HashMap::new()),
            rum: Mutex::new(HashMap::new()),
            inflight: Mutex::new(HashMap::new()),
            ffmpeg_detected: AtomicBool::new(false),
            ffmpeg_checked: AtomicBool::new(false),
            m_req: AtomicU64::new(0),
            m_hit: AtomicU64::new(0),
            m_coalesced: AtomicU64::new(0),
            m_upstream: AtomicU64::new(0),
            m_rl: AtomicU64::new(0),
            m_global_rl: AtomicU64::new(0),
            m_bytes_out: AtomicU64::new(0),
            m_errors: AtomicU64::new(0),
            m_aborted: AtomicU64::new(0),
            m_rejected503: AtomicU64::new(0),
        }
    }

    /// Compute canonical coalescing key for URL.
    pub fn canonical_key(raw: &str) -> String {
        let Ok(u) = Url::parse(raw) else {
            return raw.to_string();
        };
        let host = u.host_str().unwrap_or("").to_lowercase();

        if RE_PANTAU.is_match(&host) {
            if let Some((_, id)) = u.query_pairs().find(|(k, _)| k == "id") {
                return format!("pc:{}", id.to_ascii_lowercase());
            }
            if u.query_pairs().any(|(k, _)| k == "t") {
                let t_val = u
                    .query_pairs()
                    .find(|(k, _)| k == "t")
                    .map(|(_, v)| v.to_string())
                    .unwrap_or_default();
                let b64 = t_val.split('.').next().unwrap_or("");
                if !b64.is_empty()
                    && let Ok(json_str) = Self::decode_base64url(b64)
                    && let Ok(val) = serde_json::from_str::<serde_json::Value>(&json_str)
                {
                    let inner = val
                        .get("u")
                        .or_else(|| val.get("url"))
                        .or_else(|| val.get("stream"))
                        .and_then(|v| v.as_str());
                    if let Some(inner_url) = inner
                        && let Ok(iu) = Url::parse(inner_url)
                    {
                        let i_host = iu.host_str().unwrap_or("").to_lowercase();
                        return format!("pc:{}{}", i_host, iu.path());
                    }
                }
                let mut u_clean = u.clone();
                let filtered_pairs: Vec<(String, String)> = u_clean
                    .query_pairs()
                    .filter(|(k, _)| k != "t")
                    .map(|(k, v)| (k.to_string(), v.to_string()))
                    .collect();
                u_clean.set_query(None);
                for (k, v) in filtered_pairs {
                    u_clean.query_pairs_mut().append_pair(&k, &v);
                }
                let query_part = u_clean
                    .query()
                    .map(|q| format!("?{}", q))
                    .unwrap_or_default();
                return format!("pc:{}{}{}", host, u_clean.path(), query_part);
            }
        }

        if RE_MOLECOOL.is_match(&host) {
            return format!("mc:{}", u.path());
        }

        if RE_KOTANG.is_match(&host) {
            let src = u
                .query_pairs()
                .find(|(k, _)| k == "src")
                .map(|(_, v)| v.to_lowercase())
                .unwrap_or_default();
            return format!("kt:{}:{}", u.path().to_lowercase(), src);
        }

        if RE_TK.is_match(&host) {
            if let Some(caps) = RE_TK_PATH.captures(u.path())
                && let Some(m) = caps.get(1)
            {
                return format!("tk:{}", m.as_str().to_lowercase());
            }
            return format!("tk:{}", u.path().to_lowercase());
        }

        if RE_BALI.is_match(&host) {
            if let Some(caps) = RE_BALI_PATH.captures(u.path())
                && let Some(m) = caps.get(1)
            {
                return format!("bali:{}", m.as_str().to_lowercase());
            }
            return format!("bali:{}", u.path().to_lowercase());
        }

        raw.to_string()
    }

    fn decode_base64url(s: &str) -> Result<String, ()> {
        let mut base64 = s.replace('-', "+").replace('_', "/");
        while !base64.len().is_multiple_of(4) {
            base64.push('=');
        }
        let bytes = Self::base64_decode_bytes(&base64).ok_or(())?;
        String::from_utf8(bytes).map_err(|_| ())
    }

    fn base64_decode_bytes(s: &str) -> Option<Vec<u8>> {
        const TABLE: [i8; 256] = {
            let mut t = [-1i8; 256];
            let mut i = 0usize;
            while i < 26 {
                t[(b'A' + i as u8) as usize] = i as i8;
                t[(b'a' + i as u8) as usize] = (i + 26) as i8;
                i += 1;
            }
            let mut j = 0usize;
            while j < 10 {
                t[(b'0' + j as u8) as usize] = (j + 52) as i8;
                j += 1;
            }
            t[b'+' as usize] = 62;
            t[b'/' as usize] = 63;
            t
        };
        let bytes = s.as_bytes();
        let mut out = Vec::with_capacity((bytes.len() * 3) / 4);
        let mut buf = 0u32;
        let mut bits = 0;
        for &b in bytes {
            if b == b'=' {
                break;
            }
            let val = TABLE[b as usize];
            if val < 0 {
                continue;
            }
            buf = (buf << 6) | (val as u32);
            bits += 6;
            if bits >= 8 {
                bits -= 8;
                out.push((buf >> bits) as u8);
            }
        }
        Some(out)
    }

    /// Resolve direct JPEG snapshot bypass URL if available.
    pub fn resolve_direct_snap_url(raw: &str) -> Option<String> {
        let Ok(mut u) = Url::parse(raw) else {
            return None;
        };
        let host = u.host_str().unwrap_or("").to_lowercase();

        if RE_KOTANG.is_match(&host) {
            u.set_path("/api/frame.jpeg");
            let filtered: Vec<(String, String)> = u
                .query_pairs()
                .filter(|(k, _)| k != "t" && k != "v")
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            u.set_query(None);
            for (k, v) in filtered {
                u.query_pairs_mut().append_pair(&k, &v);
            }
            return Some(u.to_string());
        }

        if RE_DIRECT_IMG.is_match(u.path()) {
            let filtered: Vec<(String, String)> = u
                .query_pairs()
                .filter(|(k, _)| k != "t" && k != "v")
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect();
            u.set_query(None);
            for (k, v) in filtered {
                u.query_pairs_mut().append_pair(&k, &v);
            }
            return Some(u.to_string());
        }

        if RE_DSDA.is_match(&host) {
            return Some(u.to_string());
        }

        None
    }

    /// Rewrite playlist URLs to proxy through the given px prefix closure.
    pub fn rewrite_playlist(txt: &str, final_url: &str, px: impl Fn(&str) -> String) -> String {
        let base_url = Url::parse(final_url).ok();
        let mut out = Vec::new();

        for line in txt.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                out.push(line.to_string());
                continue;
            }
            if trimmed.starts_with('#') {
                let rewritten = RE_URI_ATTR.replace_all(trimmed, |caps: &regex::Captures| {
                    let uri = &caps[1];
                    let resolved = base_url
                        .as_ref()
                        .and_then(|b| b.join(uri).ok())
                        .map(|u| u.to_string())
                        .unwrap_or_else(|| uri.to_string());
                    format!(r#"URI="{}""#, px(&resolved))
                });
                out.push(rewritten.to_string());
            } else {
                let resolved = base_url
                    .as_ref()
                    .and_then(|b| b.join(trimmed).ok())
                    .map(|u| u.to_string())
                    .unwrap_or_else(|| trimmed.to_string());
                out.push(px(&resolved));
            }
        }

        out.join("\n")
    }

    /// Compute TTL for HLS playlist in milliseconds.
    pub fn playlist_ttl(txt: &str, min_ms: u64, max_ms: u64) -> u64 {
        if txt.contains("#EXT-X-ENDLIST") {
            return 30000;
        }
        let target_dur = RE_TARGET_DUR
            .captures(txt)
            .and_then(|c| c.get(1))
            .and_then(|m| m.as_str().parse::<u64>().ok())
            .unwrap_or(4);
        (target_dur * 250).clamp(min_ms, max_ms)
    }

    /// Sanitize media content-type.
    pub fn sanitize_media_ct(ct: &str, url: &str) -> String {
        let low = ct
            .split(';')
            .next()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if low.is_empty() || low == "text/vnd.trolltech.linguist" || low == "text/plain" {
            return "video/mp2t".to_string();
        }
        let url_low = url.to_ascii_lowercase();
        if (url_low.contains(".ts?") || url_low.ends_with(".ts") || url_low.contains(".ts#"))
            && (low.starts_with("text/") || low == "application/octet-stream")
        {
            return "video/mp2t".to_string();
        }
        if url_low.contains(".m4s?") || url_low.ends_with(".m4s") || url_low.contains(".m4s#") {
            return "video/iso.segment".to_string();
        }
        if low.starts_with("text/html")
            || low.starts_with("application/xhtml")
            || low.starts_with("image/svg")
            || low.starts_with("text/javascript")
            || low.starts_with("application/javascript")
        {
            return "application/octet-stream".to_string();
        }
        low
    }

    /// Dead cam report submission.
    pub fn report(&self, url: &str, ok: bool, ip: &str, is_known: impl Fn(&str) -> bool) -> bool {
        if url.is_empty() || url.len() > 500 {
            return false;
        }
        let now = current_time_ms();
        {
            let mut rl = self.rep_rl.lock().unwrap_or_else(|p| p.into_inner());
            if !rl.take(if ip.is_empty() { "x" } else { ip }, 1.0, now) {
                return false;
            }
        }
        if !is_known(url) {
            return false;
        }

        let mut reports = self.reports.lock().unwrap_or_else(|p| p.into_inner());
        let rep = reports.entry(url.to_string()).or_default();
        if ok {
            rep.ok_at = now;
            rep.fails.clear();
        } else {
            let hash = Self::hash_ip(ip);
            rep.fails.insert(hash, now);
        }

        if reports.len() > 5000 {
            reports.retain(|_, v| {
                let max_fail = v.fails.values().copied().max().unwrap_or(0);
                let last = v.ok_at.max(max_fail);
                now.saturating_sub(last) <= 3_600_000
            });
        }
        true
    }

    /// Check if a camera stream has been reported dead (>=2 distinct reporting IPs within 20 mins).
    pub fn reported_dead(&self, url: &str) -> bool {
        let reports = self.reports.lock().unwrap_or_else(|p| p.into_inner());
        let Some(rep) = reports.get(url) else {
            return false;
        };
        let now = current_time_ms();
        const DEAD_WINDOW: u64 = 1_200_000;
        let mut count = 0;
        for &t in rep.fails.values() {
            if now.saturating_sub(t) < DEAD_WINDOW && t > rep.ok_at {
                count += 1;
            }
        }
        count >= 2
    }

    fn hash_ip(ip: &str) -> String {
        let mut hasher = Sha1::new();
        hasher.update(ip.as_bytes());
        let result = hasher.finalize();
        let hex = format!("{:x}", result);
        hex[..12.min(hex.len())].to_string()
    }

    /// Ingest RUM telemetry events.
    pub fn rum_ingest(
        &self,
        events: &[RumEvent],
        ip: &str,
        is_known_host: impl Fn(&str) -> bool,
    ) -> usize {
        let now = current_time_ms();
        {
            let mut rl = self.rum_rl.lock().unwrap_or_else(|p| p.into_inner());
            if !rl.take(if ip.is_empty() { "x" } else { ip }, 1.0, now) {
                return 0;
            }
        }

        let mut rum = self.rum.lock().unwrap_or_else(|p| p.into_inner());
        let mut accepted = 0;

        for e in events.iter().take(100) {
            if e.h.is_empty() || e.h.len() > 120 || !RE_HOST_VALID.is_match(&e.h) {
                continue;
            }
            if !is_known_host(&e.h) {
                continue;
            }
            if !rum.contains_key(&e.h) && rum.len() >= 300 {
                continue;
            }
            let s = rum.entry(e.h.clone()).or_default();
            if e.f.unwrap_or(false) {
                s.fail += 1;
            } else if let Some(t) = e.t
                && t.is_finite()
                && (0.0..300000.0).contains(&t)
            {
                s.ok += 1;
                s.ttff.push(t.round() as u64);
                if s.ttff.len() > 300 {
                    s.ttff.remove(0);
                }
            }
            if let Some(st) = e.s
                && st.is_finite()
                && (0.0..1000.0).contains(&st)
                && st > 0.0
            {
                s.stalls += st as u64;
            }
            accepted += 1;
        }

        accepted
    }

    /// Calculate RUM stats with p50 and p95 quantiles.
    pub fn rum_stats(&self) -> HashMap<String, RumHostStats> {
        let rum = self.rum.lock().unwrap_or_else(|p| p.into_inner());
        let mut out = HashMap::new();

        for (host, s) in rum.iter() {
            let p50 = Self::quantile(&s.ttff, 0.5);
            let p95 = Self::quantile(&s.ttff, 0.95);
            out.insert(
                host.clone(),
                RumHostStats {
                    ok: s.ok,
                    fail: s.fail,
                    stalls: s.stalls,
                    ttff_p50: p50,
                    ttff_p95: p95,
                },
            );
        }

        out
    }

    fn quantile(arr: &[u64], p: f64) -> Option<u64> {
        if arr.is_empty() {
            return None;
        }
        let mut sorted = arr.to_vec();
        sorted.sort_unstable();
        let idx = ((p * sorted.len() as f64).floor() as usize).min(sorted.len() - 1);
        Some(sorted[idx])
    }

    /// Asynchronously check if ffmpeg is available.
    pub async fn detect_ffmpeg(&self) -> bool {
        if self.ffmpeg_checked.load(Ordering::Acquire) {
            return self.ffmpeg_detected.load(Ordering::Acquire);
        }
        let bin = self
            .options
            .ffmpeg
            .clone()
            .or_else(|| std::env::var("FFMPEG_PATH").ok())
            .unwrap_or_else(|| "ffmpeg".to_string());

        let ok = match tokio::process::Command::new(&bin)
            .arg("-version")
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .status()
            .await
        {
            Ok(status) => status.success(),
            Err(_) => false,
        };

        self.ffmpeg_detected.store(ok, Ordering::Release);
        self.ffmpeg_checked.store(true, Ordering::Release);
        ok
    }

    /// Extract single video frame as JPEG from pipe buffer using ffmpeg subprocess.
    pub async fn extract_snapshot_ffmpeg(&self, input: &[u8]) -> Result<Bytes, String> {
        let bin = self
            .options
            .ffmpeg
            .clone()
            .or_else(|| std::env::var("FFMPEG_PATH").ok())
            .unwrap_or_else(|| "ffmpeg".to_string());

        let mut child = tokio::process::Command::new(&bin)
            .args([
                "-loglevel",
                "error",
                "-i",
                "pipe:0",
                "-frames:v",
                "1",
                "-vf",
                "scale=360:-2",
                "-q:v",
                "7",
                "-f",
                "mjpeg",
                "pipe:1",
            ])
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .spawn()
            .map_err(|e| format!("failed to spawn ffmpeg: {}", e))?;

        if let Some(mut stdin) = child.stdin.take() {
            use tokio::io::AsyncWriteExt;
            let _ = stdin.write_all(input).await;
            let _ = stdin.shutdown().await;
        }

        let run = async {
            let output = child
                .wait_with_output()
                .await
                .map_err(|e| format!("ffmpeg execution error: {}", e))?;
            if !output.status.success() || output.stdout.is_empty() {
                return Err("ffmpeg kosong".to_string());
            }
            if output.stdout.len() > (1 << 20) {
                return Err("ffmpeg output too large".to_string());
            }
            Ok(Bytes::from(output.stdout))
        };

        tokio::time::timeout(Duration::from_secs(9), run)
            .await
            .map_err(|_| "ffmpeg timeout".to_string())?
    }

    /// Extract MJPEG frame from buffer.
    pub fn mjpeg_extract_frame(buf: &[u8]) -> Option<Bytes> {
        let start = buf.windows(2).position(|w| w == [0xff, 0xd8])?;
        let end = buf[start + 2..]
            .windows(2)
            .position(|w| w == [0xff, 0xd9])
            .map(|p| start + 2 + p)?;
        if end + 2 - start >= 1024 {
            Some(Bytes::copy_from_slice(&buf[start..=end + 1]))
        } else {
            None
        }
    }

    /// Execute an upstream flight with coalescing.
    pub async fn coalesced_fetch<F, Fut>(
        &self,
        key: &str,
        fetcher: F,
    ) -> Result<Bytes, (u16, String)>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<Bytes, (u16, String)>>,
    {
        let rx = {
            let mut map = self.inflight.lock().unwrap_or_else(|p| p.into_inner());
            if let Some(tx) = map.get(key) {
                self.m_coalesced.fetch_add(1, Ordering::Relaxed);
                Some(tx.subscribe())
            } else {
                let (tx, _) = broadcast::channel(1);
                map.insert(key.to_string(), tx);
                None
            }
        };

        if let Some(mut rx) = rx {
            match rx.recv().await {
                Ok(res) => return res,
                Err(_) => return Err((502, "leader failed".to_string())),
            }
        }

        // Leader flight
        self.m_upstream.fetch_add(1, Ordering::Relaxed);
        let result = fetcher().await;

        let mut map = self.inflight.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(tx) = map.remove(key) {
            let _ = tx.send(result.clone());
        }

        result
    }

    /// Get cached payload from segment cache.
    pub fn get_cached(&self, key: &str) -> Option<Bytes> {
        let now = current_time_ms();
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        if let Some(data) = cache.get(key, now) {
            self.m_hit.fetch_add(1, Ordering::Relaxed);
            Some(data)
        } else {
            None
        }
    }

    /// Put payload in segment cache.
    pub fn set_cached(&self, key: &str, val: Bytes, ttl_ms: u64) {
        let now = current_time_ms();
        let mut cache = self.cache.lock().unwrap_or_else(|p| p.into_inner());
        cache.set(key.to_string(), val.clone(), val.len(), ttl_ms, now);
    }

    /// Get snapshot payload from cache.
    pub fn get_snap_cached(&self, key: &str) -> Option<Bytes> {
        let now = current_time_ms();
        let mut snap = self.snap_cache.lock().unwrap_or_else(|p| p.into_inner());
        snap.get(key, now)
    }

    /// Put snapshot payload in cache.
    pub fn set_snap_cached(&self, key: &str, val: Bytes, ttl_ms: u64) {
        let now = current_time_ms();
        let mut snap = self.snap_cache.lock().unwrap_or_else(|p| p.into_inner());
        snap.set(key.to_string(), val.clone(), val.len(), ttl_ms, now);
    }

    /// Record outbound bytes.
    pub fn charge_bytes(&self, ip: &str, count: usize) {
        self.m_bytes_out.fetch_add(count as u64, Ordering::Relaxed);
        let now = current_time_ms();
        let mut byte_rl = self.byte_rl.lock().unwrap_or_else(|p| p.into_inner());
        byte_rl.charge(ip, count as f64, now);
        let mut byte_global = self.byte_global.lock().unwrap_or_else(|p| p.into_inner());
        byte_global.charge("_", count as f64, now);
    }

    /// Check request rate limiter for client IP.
    pub fn rate_limit_req(&self, ip: &str) -> bool {
        let now = current_time_ms();
        let mut rl = self.req_rl.lock().unwrap_or_else(|p| p.into_inner());
        let key = if ip.is_empty() { "x" } else { ip };
        if rl.take(key, 1.0, now) {
            true
        } else {
            self.m_rl.fetch_add(1, Ordering::Relaxed);
            false
        }
    }

    /// Overall gateway statistics.
    pub fn stats(&self) -> GatewayStats {
        let lane_stats = self.lane.stats();
        let snap_lane_stats = self.snap_lane.stats();
        let cache_stats = self.cache.lock().map(|c| c.stats()).unwrap_or_default();
        let snap_cache_stats = self
            .snap_cache
            .lock()
            .map(|c| c.stats())
            .unwrap_or_default();
        let inflight_count = self.inflight.lock().map(|m| m.len()).unwrap_or(0);
        let reports_count = self.reports.lock().map(|m| m.len()).unwrap_or(0);

        GatewayStats {
            req: self.m_req.load(Ordering::Relaxed),
            hit: self.m_hit.load(Ordering::Relaxed),
            coalesced: self.m_coalesced.load(Ordering::Relaxed),
            upstream: self.m_upstream.load(Ordering::Relaxed),
            rl: self.m_rl.load(Ordering::Relaxed),
            global_rl: self.m_global_rl.load(Ordering::Relaxed),
            bytes_out: self.m_bytes_out.load(Ordering::Relaxed),
            errors: self.m_errors.load(Ordering::Relaxed),
            aborted: self.m_aborted.load(Ordering::Relaxed),
            rejected503: self.m_rejected503.load(Ordering::Relaxed),
            lane: lane_stats,
            snap_lane: snap_lane_stats,
            cache: cache_stats,
            snap_cache: snap_cache_stats,
            inflight: inflight_count,
            reports: reports_count,
        }
    }
}
