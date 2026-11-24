//! Async `CopernicusProxy` for Copernicus Sentinel-2 satellite imagery.
//! Connects to Copernicus Data Space Ecosystem (CDSE) / Sentinel Hub APIs.

#![forbid(unsafe_code)]

use chrono::Utc;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};

pub const TOKEN_URL: &str =
    "https://identity.dataspace.copernicus.eu/auth/realms/CDSE/protocol/openid-connect/token";
pub const PROCESS_URL: &str = "https://sh.dataspace.copernicus.eu/api/v1/process";

pub const MIN_ZOOM: i64 = 6;
pub const MAX_ZOOM: i64 = 12;
pub const TILE_PX: i64 = 512;
pub const WORLD_HALF: f64 = 20037508.342789244;
pub const DEFAULT_CACHE_TTL_MS: u64 = 86_400_000; // 24h
pub const DEFAULT_DAILY_CAP: u64 = 250;
pub const DEFAULT_CONCURRENCY: usize = 4;

pub const TRUE_COLOR_EVALSCRIPT: &str = r#"//VERSION=3
function setup() {
  return { input: ["B02", "B03", "B04"], output: { bands: 4, sampleType: "AUTO" } };
}
function evaluatePixel(s) {
  if (s.B02 === 0 && s.B03 === 0 && s.B04 === 0) return [0, 0, 0, 0];
  const g = v => Math.min(1, Math.pow(Math.max(0, 2.2 * v), 0.8));
  return [g(s.B04), g(s.B03), g(s.B02), 1];
}"#;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Bounds {
    pub min_lon: f64,
    pub min_lat: f64,
    pub max_lon: f64,
    pub max_lat: f64,
}

pub const ID_BOUNDS: Bounds = Bounds {
    min_lon: 94.5,
    min_lat: -11.5,
    max_lon: 141.5,
    max_lat: 6.5,
};

pub fn tile_to_bbox3857_coords(z: f64, x: f64, y: f64) -> [f64; 4] {
    let n = 2.0_f64.powf(z);
    let span = (2.0 * WORLD_HALF) / n;
    let min_x = -WORLD_HALF + x * span;
    let max_x = min_x + span;
    let max_y = WORLD_HALF - y * span;
    let min_y = max_y - span;
    [min_x, min_y, max_x, max_y]
}

pub fn tile_to_bbox4326_coords(z: f64, x: f64, y: f64) -> [f64; 4] {
    let n = 2.0_f64.powf(z);
    let lon_of = |col: f64| (col / n) * 360.0 - 180.0;
    let lat_of = |row: f64| {
        ((std::f64::consts::PI * (1.0 - (2.0 * row) / n))
            .sinh()
            .atan()
            * 180.0)
            / std::f64::consts::PI
    };
    [lon_of(x), lat_of(y + 1.0), lon_of(x + 1.0), lat_of(y)]
}

pub fn tile_intersects_bounds_coords(z: f64, x: f64, y: f64, bounds: &Bounds) -> bool {
    let [min_lon, min_lat, max_lon, max_lat] = tile_to_bbox4326_coords(z, x, y);
    max_lon >= bounds.min_lon
        && min_lon <= bounds.max_lon
        && max_lat >= bounds.min_lat
        && min_lat <= bounds.max_lat
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CopernicusConfig {
    pub client_id: String,
    pub client_secret: String,
    pub cache_dir: PathBuf,
    pub quota_file: Option<PathBuf>,
    pub daily_cap: u64,
    pub month_cap: u64,
    pub concurrency: usize,
    pub cache_ttl_ms: u64,
    pub max_cache_bytes: u64,
    pub lookback_days: u32,
    pub mem_cap: usize,
}

impl Default for CopernicusConfig {
    fn default() -> Self {
        Self {
            client_id: String::new(),
            client_secret: String::new(),
            cache_dir: PathBuf::from("data/s2-cache"),
            quota_file: Some(PathBuf::from("data/s2-quota.json")),
            daily_cap: DEFAULT_DAILY_CAP,
            month_cap: 9500,
            concurrency: DEFAULT_CONCURRENCY,
            cache_ttl_ms: DEFAULT_CACHE_TTL_MS,
            max_cache_bytes: 256 * 1024 * 1024,
            lookback_days: 45,
            mem_cap: 64,
        }
    }
}

#[derive(Debug, Clone)]
pub struct TileResponse {
    pub status: u16,
    pub buf: Vec<u8>,
    pub content_type: String,
    pub cache_state: String,
    pub etag: Option<String>,
    pub stale: bool,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProxyStatus {
    pub ok: bool,
    pub configured: bool,
    pub day_key: String,
    pub month_key: String,
    pub daily_used: u64,
    pub daily_cap: u64,
    pub month_used: u64,
    pub month_cap: u64,
    pub min_zoom: i64,
    pub max_zoom: i64,
    pub tile_size: i64,
    pub cache_ttl_hours: f64,
    pub lookback_days: u32,
    pub cache_entries: usize,
    pub cache_bytes: u64,
    pub cooldown_sec: u64,
}

#[derive(Clone)]
struct CacheEntry {
    buf: Vec<u8>,
    at: Instant,
    etag: String,
}

struct TokenState {
    token: String,
    expires_at: Instant,
}

#[derive(Default, Serialize, Deserialize)]
struct QuotaState {
    day_key: String,
    daily_used: u64,
    month_key: String,
    month_used: u64,
}

struct InnerState {
    mem_cache: HashMap<String, CacheEntry>,
    mem_order: Vec<String>,
    token: TokenState,
    cooldown_until: Option<Instant>,
    quota: QuotaState,
}

pub struct CopernicusProxy {
    config: CopernicusConfig,
    client: Client,
    semaphore: Arc<Semaphore>,
    state: Mutex<InnerState>,
}

fn sha256_hex(input: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input);
    format!("{:x}", hasher.finalize())
}

fn make_etag(buf: &[u8]) -> String {
    let hash = sha256_hex(buf);
    format!("W/\"s2-{}\"", &hash[..14.min(hash.len())])
}

impl CopernicusProxy {
    pub fn new(config: CopernicusConfig) -> Self {
        let concurrency = config.concurrency.max(1);
        let now_utc = Utc::now();
        let day_key = now_utc.format("%Y-%m-%d").to_string();
        let month_key = now_utc.format("%Y-%m").to_string();

        let quota = if let Some(ref qp) = config.quota_file {
            std::fs::read_to_string(qp)
                .ok()
                .and_then(|s| serde_json::from_str::<QuotaState>(&s).ok())
                .unwrap_or_else(|| QuotaState {
                    day_key: day_key.clone(),
                    daily_used: 0,
                    month_key: month_key.clone(),
                    month_used: 0,
                })
        } else {
            QuotaState {
                day_key,
                daily_used: 0,
                month_key,
                month_used: 0,
            }
        };

        let past = Instant::now()
            .checked_sub(Duration::from_secs(3600))
            .unwrap_or_else(Instant::now);

        Self {
            semaphore: Arc::new(Semaphore::new(concurrency)),
            client: Client::builder()
                .timeout(Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            config,
            state: Mutex::new(InnerState {
                mem_cache: HashMap::new(),
                mem_order: Vec::new(),
                token: TokenState {
                    token: String::new(),
                    expires_at: past,
                },
                cooldown_until: None,
                quota,
            }),
        }
    }

    pub fn is_configured(&self) -> bool {
        !self.config.client_id.trim().is_empty() && !self.config.client_secret.trim().is_empty()
    }

    fn tile_cache_key(z: i64, x: i64, y: i64) -> String {
        format!("{z}_{x}_{y}")
    }

    fn tile_disk_path(&self, z: i64, x: i64, y: i64) -> PathBuf {
        let raw_key = format!("{z}/{x}/{y}");
        let hash = sha256_hex(raw_key.as_bytes());
        self.config.cache_dir.join(format!(
            "{}_{}.png",
            Self::tile_cache_key(z, x, y),
            &hash[..10]
        ))
    }

    async fn read_disk_tile(&self, z: i64, x: i64, y: i64) -> Option<(Vec<u8>, String)> {
        let fp = self.tile_disk_path(z, x, y);
        let legacy_fp = self.config.cache_dir.join(format!("{z}_{x}_{y}.png"));

        let target = if fp.exists() {
            fp
        } else if legacy_fp.exists() {
            legacy_fp
        } else {
            return None;
        };

        let bytes = tokio::fs::read(&target).await.ok()?;
        if bytes.is_empty() {
            return None;
        }
        let etag = make_etag(&bytes);
        Some((bytes, etag))
    }

    async fn write_disk_tile(&self, z: i64, x: i64, y: i64, bytes: &[u8]) {
        let fp = self.tile_disk_path(z, x, y);
        if let Some(parent) = fp.parent() {
            let _ = tokio::fs::create_dir_all(parent).await;
        }
        let tmp = fp.with_extension("tmp");
        if tokio::fs::write(&tmp, bytes).await.is_ok() {
            let _ = tokio::fs::rename(&tmp, &fp).await;
        }
    }

    fn rollover_quota(state: &mut InnerState) {
        let now_utc = Utc::now();
        let cur_day = now_utc.format("%Y-%m-%d").to_string();
        let cur_month = now_utc.format("%Y-%m").to_string();

        if state.quota.day_key != cur_day {
            state.quota.day_key = cur_day;
            state.quota.daily_used = 0;
        }
        if state.quota.month_key != cur_month {
            state.quota.month_key = cur_month;
            state.quota.month_used = 0;
        }
    }

    pub async fn status(&self) -> ProxyStatus {
        let mut st = self.state.lock().await;
        Self::rollover_quota(&mut st);

        let now = Instant::now();
        let cooldown_sec = st
            .cooldown_until
            .filter(|&c| c > now)
            .map(|c| (c - now).as_secs())
            .unwrap_or(0);

        let (cache_entries, cache_bytes) = self.scan_cache_stats().await;

        ProxyStatus {
            ok: true,
            configured: self.is_configured(),
            day_key: st.quota.day_key.clone(),
            month_key: st.quota.month_key.clone(),
            daily_used: st.quota.daily_used,
            daily_cap: self.config.daily_cap,
            month_used: st.quota.month_used,
            month_cap: self.config.month_cap,
            min_zoom: MIN_ZOOM,
            max_zoom: MAX_ZOOM,
            tile_size: TILE_PX,
            cache_ttl_hours: self.config.cache_ttl_ms as f64 / 3_600_000.0,
            lookback_days: self.config.lookback_days,
            cache_entries,
            cache_bytes,
            cooldown_sec,
        }
    }

    async fn scan_cache_stats(&self) -> (usize, u64) {
        let mut count = 0;
        let mut total_bytes = 0;
        if let Ok(mut dir) = tokio::fs::read_dir(&self.config.cache_dir).await {
            while let Ok(Some(entry)) = dir.next_entry().await {
                if let Ok(meta) = entry.metadata().await
                    && meta.is_file()
                {
                    count += 1;
                    total_bytes += meta.len();
                }
            }
        }
        (count, total_bytes)
    }

    async fn get_token(&self) -> Result<String, String> {
        let now = Instant::now();
        {
            let st = self.state.lock().await;
            if !st.token.token.is_empty() && now + Duration::from_secs(60) < st.token.expires_at {
                return Ok(st.token.token.clone());
            }
        }

        let body = [
            ("grant_type", "client_credentials"),
            ("client_id", self.config.client_id.as_str()),
            ("client_secret", self.config.client_secret.as_str()),
        ];

        let res = self
            .client
            .post(TOKEN_URL)
            .form(&body)
            .send()
            .await
            .map_err(|e| format!("CDSE auth network error: {e}"))?;

        let status = res.status();
        if !status.is_success() {
            let code = status.as_u16();
            if code == 401 || code == 403 || code == 429 {
                let mut st = self.state.lock().await;
                st.cooldown_until = Some(Instant::now() + Duration::from_secs(15 * 60));
            }
            return Err(format!("CDSE auth gagal ({code})"));
        }

        #[derive(Deserialize)]
        struct TokenResp {
            access_token: String,
            expires_in: Option<u64>,
        }

        let json = res
            .json::<TokenResp>()
            .await
            .map_err(|e| format!("CDSE auth parse error: {e}"))?;

        if json.access_token.is_empty() {
            return Err("CDSE token kosong".into());
        }

        let ttl = json.expires_in.unwrap_or(600).max(120);
        let mut st = self.state.lock().await;
        st.token = TokenState {
            token: json.access_token.clone(),
            expires_at: Instant::now() + Duration::from_secs(ttl),
        };

        Ok(json.access_token)
    }

    async fn fetch_upstream(&self, z: i64, x: i64, y: i64) -> Result<Vec<u8>, (u16, String)> {
        let _permit = self
            .semaphore
            .acquire()
            .await
            .map_err(|_| (500, "semaphore closed".into()))?;

        let token = self.get_token().await.map_err(|e| (502, e))?;

        let now_utc = Utc::now();
        let start_utc = now_utc - chrono::Duration::days(self.config.lookback_days as i64);

        let bbox = tile_to_bbox3857_coords(z as f64, x as f64, y as f64);
        let payload = json!({
            "input": {
                "bounds": {
                    "properties": { "crs": "http://www.opengis.net/def/crs/EPSG/0/3857" },
                    "bbox": bbox,
                },
                "data": [{
                    "type": "sentinel-2-l2a",
                    "dataFilter": {
                        "timeRange": {
                            "from": start_utc.to_rfc3339(),
                            "to": now_utc.to_rfc3339(),
                        },
                        "mosaickingOrder": "leastCC",
                    }
                }],
            },
            "output": {
                "width": TILE_PX,
                "height": TILE_PX,
                "responses": [{ "identifier": "default", "format": { "type": "image/png" } }],
            },
            "evalscript": TRUE_COLOR_EVALSCRIPT,
        });

        let res = self
            .client
            .post(PROCESS_URL)
            .bearer_auth(token)
            .json(&payload)
            .send()
            .await
            .map_err(|e| (502, format!("CDSE process network error: {e}")))?;

        let code = res.status().as_u16();
        if code == 401 {
            let mut st = self.state.lock().await;
            st.token.token.clear();
            return Err((401, "CDSE token ditolak (401)".into()));
        }
        if code == 429 || code == 402 {
            let mut st = self.state.lock().await;
            st.cooldown_until = Some(Instant::now() + Duration::from_secs(15 * 60));
            return Err((code, format!("CDSE kuota/rate limit ({code})")));
        }
        if !res.status().is_success() {
            return Err((code, format!("CDSE HTTP {code}")));
        }

        let bytes = res
            .bytes()
            .await
            .map_err(|e| (502, format!("CDSE bytes error: {e}")))?
            .to_vec();

        if bytes.is_empty() {
            return Err((502, "CDSE tile kosong".into()));
        }

        Ok(bytes)
    }

    pub async fn fetch_tile(&self, z: i64, x: i64, y: i64) -> TileResponse {
        let max_coord = if (0..=18).contains(&z) {
            (1 << z) - 1
        } else {
            -1
        };
        if !(0..=18).contains(&z) || x < 0 || y < 0 || x > max_coord || y > max_coord {
            return TileResponse {
                status: 400,
                buf: b"koordinat tile tidak valid".to_vec(),
                content_type: "text/plain".into(),
                cache_state: "BAD-COORD".into(),
                etag: None,
                stale: false,
                error: None,
            };
        }

        if !(MIN_ZOOM..=MAX_ZOOM).contains(&z)
            || !tile_intersects_bounds_coords(z as f64, x as f64, y as f64, &ID_BOUNDS)
        {
            return TileResponse {
                status: 204,
                buf: Vec::new(),
                content_type: "image/png".into(),
                cache_state: "OUT-OF-ROI".into(),
                etag: None,
                stale: false,
                error: None,
            };
        }

        if !self.is_configured() {
            return TileResponse {
                status: 503,
                buf: b"Copernicus CDSE credentials belum dikonfigurasi".to_vec(),
                content_type: "text/plain".into(),
                cache_state: "UNCONFIGURED".into(),
                etag: None,
                stale: false,
                error: None,
            };
        }

        let key = Self::tile_cache_key(z, x, y);
        let now = Instant::now();
        let ttl = Duration::from_millis(self.config.cache_ttl_ms);

        // 1. Memory cache check
        {
            let st = self.state.lock().await;
            if let Some(entry) = st.mem_cache.get(&key)
                && now.duration_since(entry.at) < ttl
            {
                let entry = entry.clone();
                return TileResponse {
                    status: 200,
                    buf: entry.buf,
                    content_type: "image/png".into(),
                    cache_state: "HIT-MEM".into(),
                    etag: Some(entry.etag),
                    stale: false,
                    error: None,
                };
            }
        }

        // 2. Disk cache check
        let disk_hit = self.read_disk_tile(z, x, y).await;
        if let Some((ref buf, ref etag)) = disk_hit {
            let mut st = self.state.lock().await;
            Self::put_mem_cache(
                &mut st,
                key.clone(),
                buf.clone(),
                etag.clone(),
                self.config.mem_cap,
            );
            return TileResponse {
                status: 200,
                buf: buf.clone(),
                content_type: "image/png".into(),
                cache_state: "HIT-DISK".into(),
                etag: Some(etag.clone()),
                stale: false,
                error: None,
            };
        }

        // 3. Quota check
        {
            let mut st = self.state.lock().await;
            Self::rollover_quota(&mut st);
            let in_cooldown = st.cooldown_until.is_some_and(|c| c > now);
            let daily_exhausted =
                self.config.daily_cap > 0 && st.quota.daily_used >= self.config.daily_cap;
            let month_exhausted =
                self.config.month_cap > 0 && st.quota.month_used >= self.config.month_cap;

            if in_cooldown || daily_exhausted || month_exhausted {
                if let Some((buf, etag)) = disk_hit {
                    return TileResponse {
                        status: 200,
                        buf,
                        content_type: "image/png".into(),
                        cache_state: "STALE-CAP".into(),
                        etag: Some(etag),
                        stale: true,
                        error: None,
                    };
                }
                return TileResponse {
                    status: 204,
                    buf: Vec::new(),
                    content_type: "image/png".into(),
                    cache_state: "CAP-EXHAUSTED".into(),
                    etag: None,
                    stale: false,
                    error: None,
                };
            }

            // Reserve quota slot
            st.quota.daily_used += 1;
            st.quota.month_used += 1;
        }

        // 4. Upstream fetch
        match self.fetch_upstream(z, x, y).await {
            Ok(buf) => {
                let etag = make_etag(&buf);
                self.write_disk_tile(z, x, y, &buf).await;
                {
                    let mut st = self.state.lock().await;
                    Self::put_mem_cache(
                        &mut st,
                        key,
                        buf.clone(),
                        etag.clone(),
                        self.config.mem_cap,
                    );
                }
                TileResponse {
                    status: 200,
                    buf,
                    content_type: "image/png".into(),
                    cache_state: "MISS-FETCH".into(),
                    etag: Some(etag),
                    stale: false,
                    error: None,
                }
            }
            Err((_, err_msg)) => {
                // Refund quota
                {
                    let mut st = self.state.lock().await;
                    if st.quota.daily_used > 0 {
                        st.quota.daily_used -= 1;
                    }
                    if st.quota.month_used > 0 {
                        st.quota.month_used -= 1;
                    }
                }
                if let Some((buf, etag)) = disk_hit {
                    TileResponse {
                        status: 200,
                        buf,
                        content_type: "image/png".into(),
                        cache_state: "STALE-ERR".into(),
                        etag: Some(etag),
                        stale: true,
                        error: Some(err_msg),
                    }
                } else {
                    TileResponse {
                        status: 204,
                        buf: Vec::new(),
                        content_type: "image/png".into(),
                        cache_state: "ERR".into(),
                        etag: None,
                        stale: false,
                        error: Some(err_msg),
                    }
                }
            }
        }
    }

    fn put_mem_cache(st: &mut InnerState, key: String, buf: Vec<u8>, etag: String, cap: usize) {
        let entry = CacheEntry {
            buf,
            at: Instant::now(),
            etag,
        };
        st.mem_cache.insert(key.clone(), entry);
        st.mem_order.retain(|k| k != &key);
        st.mem_order.push(key);
        while st.mem_order.len() > cap {
            let oldest = st.mem_order.remove(0);
            st.mem_cache.remove(&oldest);
        }
    }
}
