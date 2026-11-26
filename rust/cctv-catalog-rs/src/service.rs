//! CCTV Catalog Service for aggregating, deduplicating, probing, and caching Indonesian CCTV feeds.

#![forbid(unsafe_code)]

use futures_util::future::BoxFuture;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{SystemTime, UNIX_EPOCH};

static BPJT_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static known-valid regex pattern
    Regex::new(r"const allStreams = (\{[\s\S]*?\});\s*\n").expect("valid regex")
});

pub const ID_CITIES: &[(&str, f64, f64)] = &[
    ("Jakarta", -6.21, 106.85),
    ("Tangerang", -6.18, 106.63),
    ("Bekasi", -6.24, 106.99),
    ("Depok", -6.4, 106.82),
    ("Bogor", -6.6, 106.8),
    ("Bandung", -6.92, 107.61),
    ("Cilegon", -5.98, 106.0),
    ("Serang", -6.11, 106.15),
    ("Yogyakarta", -7.8, 110.36),
    ("Surakarta", -7.57, 110.82),
    ("Semarang", -6.97, 110.42),
    ("Magelang", -7.48, 110.4),
    ("Surabaya", -7.25, 112.75),
    ("Malang", -7.98, 112.63),
    ("Batu", -7.86, 112.52),
    ("Kediri", -7.8, 112.02),
    ("Madiun", -7.63, 111.52),
    ("Jember", -8.18, 113.71),
    ("Banyuwangi", -8.22, 114.37),
    ("Medan", 3.6, 98.67),
    ("Binjai", 3.6, 98.49),
    ("Padang", -0.95, 100.42),
    ("Padang Sidempuan", 1.28, 99.06),
    ("Pekanbaru", 0.51, 101.45),
    ("Jambi", -1.61, 103.61),
    ("Palembang", -2.99, 104.76),
    ("Bengkulu", -3.79, 102.26),
    ("Bandar Lampung", -5.45, 105.27),
    ("Prabumulih", -3.3, 103.2),
    ("Tanjung Balai", -3.02, 99.85),
    ("Batam", 1.16, 104.05),
    ("Tanjung Pinang", 0.93, 104.45),
    ("Palangkaraya", -2.2, 113.92),
    ("Banjarmasin", -3.32, 114.59),
    ("Samarinda", -0.5, 117.15),
    ("Balikpapan", -1.24, 116.83),
    ("Bontang", 1.04, 116.85),
    ("Tarakan", 3.3, 117.59),
    ("Pontianak", -0.03, 109.32),
    ("Singkawang", 0.92, 108.98),
    ("Sambas", 1.35, 109.27),
    ("Denpasar", -8.65, 115.21),
    ("Mataram", -8.58, 116.12),
    ("Kupang", -10.16, 123.6),
    ("Ende", -8.84, 121.66),
    ("Makassar", -5.15, 119.42),
    ("Parepare", -4.02, 119.87),
    ("Palopo", -3.0, 120.19),
    ("Manado", 1.47, 124.85),
    ("Kendari", -3.98, 122.51),
    ("Palangga", -5.98, 122.75),
    ("Mamuju", -2.68, 118.9),
    ("Ternate", 0.79, 127.38),
    ("Ambon", -3.7, 128.18),
    ("Jayapura", -2.53, 140.72),
    ("Merauke", -8.57, 140.4),
    ("Sorong", -0.88, 131.25),
    ("Lhokseumawe", 5.17, 97.15),
    ("Sibolga", 1.75, 98.79),
    ("Tegal", -6.87, 109.14),
    ("Purwokerto", -7.42, 109.24),
    ("Cilacap", -7.73, 109.05),
    ("Pekalongan", -6.89, 109.68),
    ("Salatiga", -7.31, 110.5),
    ("Pati", -6.75, 111.04),
    ("Rembang", -6.7, 111.34),
    ("Pacitan", -8.2, 111.18),
    ("Ngawi", -7.6, 111.47),
    ("Lumajang", -8.14, 113.23),
    ("Probolinggo", -7.75, 113.21),
    ("Pasuruan", -7.65, 112.91),
    ("Sidoarjo", -7.45, 112.72),
    ("Gresik", -7.16, 112.65),
    ("Mojokerto", -7.47, 112.43),
    ("Blitar", -8.1, 112.17),
    ("Tulungagung", -8.07, 111.9),
    ("Nganjuk", -7.6, 111.47),
    ("Magetan", -7.63, 111.28),
    ("Ponorogo", -8.05, 111.45),
    ("Trenggalek", -8.0, 111.43),
];

pub const ID_PROV: &[(&str, f64, f64, f64, f64)] = &[
    ("Banten", -5.6, -7.3, 105.1, 106.7),
    ("DKI Jakarta", -6.05, -6.4, 106.7, 107.1),
    ("Jawa Barat", -6.3, -7.8, 106.5, 109.0),
    ("DI Yogyakarta", -7.6, -8.25, 110.1, 110.9),
    ("Jawa Tengah", -6.7, -8.3, 108.9, 111.7),
    ("Jawa Timur", -6.8, -8.8, 111.4, 114.5),
    ("Bali", -8.05, -8.85, 114.35, 115.95),
    ("Nusa Tenggara", -8.3, -10.9, 115.9, 125.2),
    ("Sumatera Utara", 5.9, 0.5, 97.5, 100.5),
    ("Sumatera Barat", -0.3, -3.4, 99.4, 102.6),
    ("Riau", 0.5, -2.0, 100.8, 104.6),
    ("Jambi", -1.2, -3.0, 101.6, 104.7),
    ("Sumatera Selatan", -2.4, -4.5, 102.5, 106.2),
    ("Bengkulu", -3.0, -4.5, 100.9, 103.0),
    ("Lampung", -4.4, -6.0, 103.5, 106.1),
    ("Kepulauan Bangka Belitung", -2.0, -3.2, 104.0, 107.0),
    ("Kalimantan Barat", 0.9, -3.4, 108.7, 111.4),
    ("Kalimantan Tengah", -1.5, -3.5, 110.5, 114.5),
    ("Kalimantan Selatan", -2.9, -4.2, 114.3, 117.4),
    ("Kalimantan Timur", -0.5, 2.5, 114.5, 119.5),
    ("Kalimantan Utara", 2.0, 4.5, 116.5, 120.0),
    ("Sulawesi Utara", 0.5, 2.0, 123.5, 127.0),
    ("Sulawesi Tengah", -0.5, 1.2, 119.3, 122.0),
    ("Sulawesi Selatan", -2.5, -6.2, 118.5, 121.8),
    ("Sulawesi Tenggara", -3.3, -5.6, 121.5, 124.5),
    ("Sulawesi Barat", -0.5, -3.5, 118.8, 120.6),
    ("Maluku", -2.5, -4.5, 126.5, 131.0),
    ("Maluku Utara", -0.5, 3.0, 127.0, 129.5),
    ("Papua", -0.5, -9.0, 130.9, 141.0),
    ("Papua Barat", -0.2, -3.5, 129.0, 133.0),
];

/// Identifies Indonesian city or province region based on latitude and longitude coordinates.
pub fn id_region(la: f64, lo: f64) -> String {
    let mut best: Option<&str> = None;
    let mut bd = 60.0;
    for (n, clat, clo) in ID_CITIES {
        let d = ((la - clat) * 111.0)
            .hypot((lo - clo) * 111.0 * (la * std::f64::consts::PI / 180.0).cos());
        if d < bd {
            bd = d;
            best = Some(*n);
        }
    }
    if let Some(b) = best {
        return format!("Kota {b}");
    }
    for (n, a, b, c2, d2) in ID_PROV {
        let min_lat = a.min(*b);
        let max_lat = a.max(*b);
        let min_lon = c2.min(*d2);
        let max_lon = c2.max(*d2);
        if la >= min_lat && la <= max_lat && lo >= min_lon && lo <= max_lon {
            return n.to_string();
        }
    }
    "Lainnya".to_string()
}

pub fn norm(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

/// Normalizes stream URLs for deduplication keying.
pub fn stream_key(url_str: &str) -> String {
    let Ok(u) = url::Url::parse(url_str) else {
        return url_str.to_string();
    };
    let host = u.host_str().unwrap_or_default();
    let host_lower = host.to_ascii_lowercase();

    if (host_lower == "pantaucctv.com" || host_lower.ends_with(".pantaucctv.com")
        || host_lower == "cctvnusantara.com" || host_lower.ends_with(".cctvnusantara.com")
        || host_lower == "99apps.id" || host_lower.ends_with(".99apps.id"))
        && u.path().to_ascii_lowercase().contains("api/stream")
    {
        for (k, v) in u.query_pairs() {
            if k == "id" {
                return format!("pc:{}", v.to_ascii_lowercase());
            }
        }
    }

    if host_lower == "dpuprkotang.info" || host_lower.ends_with(".dpuprkotang.info") {
        for (k, v) in u.query_pairs() {
            if k == "src" {
                return format!("kt:{}", v.to_ascii_lowercase());
            }
        }
    }

    if host_lower == "molecool.id" || host_lower.ends_with(".molecool.id") {
        let first_seg = u
            .path()
            .split('/')
            .find(|s| !s.is_empty())
            .unwrap_or_default();
        return format!("mc:{}", first_seg.to_ascii_lowercase());
    }

    let mut q_id = None;
    for (k, v) in u.query_pairs() {
        if k == "src" || k == "id" || k == "stream" {
            q_id = Some(v.to_string());
            break;
        }
    }

    let mut dir = u.path();
    for ext in [
        "/index.m3u8",
        "/mono.m3u8",
        "/playlist.m3u8",
        "/master.m3u8",
    ] {
        if dir.to_ascii_lowercase().ends_with(ext) {
            dir = &dir[..dir.len() - ext.len()];
            break;
        }
    }
    if let Some(pos) = dir.to_ascii_lowercase().rfind("/tracks-v") {
        let rest = &dir[pos + 9..];
        if let Some(slash_idx) = rest.find('/') {
            let digits = &rest[..slash_idx];
            if digits.chars().all(|c| c.is_ascii_digit()) && !rest[slash_idx + 1..].contains('/') {
                dir = &dir[..pos];
            }
        }
    }

    let query_suffix = match q_id {
        Some(qid) => format!("?{qid}"),
        None => String::new(),
    };

    format!("{}{}{}", host, dir, query_suffix).to_ascii_lowercase()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CamItem {
    pub n: String,
    pub s: String,
    #[serde(default)]
    pub e: Option<String>,
    #[serde(default)]
    pub la: Option<f64>,
    #[serde(default)]
    pub lo: Option<f64>,
    #[serde(default)]
    pub c: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sub: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub d: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub src: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub snap: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fb: Option<Value>,
}

impl CamItem {
    pub fn new(name: impl Into<String>, stream: impl Into<String>) -> Self {
        Self {
            n: name.into(),
            s: stream.into(),
            e: Some(String::new()),
            la: None,
            lo: None,
            c: None,
            sub: None,
            d: None,
            src: None,
            snap: None,
            fb: None,
        }
    }
}

/// Deduplicates a list of cameras by stream key.
pub fn dedupe(list: &[CamItem]) -> Vec<CamItem> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for c in list {
        if c.s.is_empty() {
            continue;
        }
        let k = stream_key(&c.s);
        if seen.insert(k) {
            out.push(c.clone());
        }
    }
    out
}

/// Cleans fallback streams in camera item.
pub fn sanitize_fb(mut cam: CamItem) -> CamItem {
    let Some(ref fb_val) = cam.fb else {
        return cam;
    };
    let arr: Vec<String> = match fb_val {
        Value::Array(xs) => xs
            .iter()
            .filter_map(|v| v.as_str().map(|s| s.to_string()))
            .collect(),
        Value::String(s) => vec![s.clone()],
        _ => Vec::new(),
    };
    if arr.is_empty() {
        cam.fb = None;
        return cam;
    }
    let nk = norm(&cam.n);
    let own = stream_key(&cam.s);
    let keep: Vec<String> = arr
        .into_iter()
        .filter(|f| {
            if f.is_empty() {
                return false;
            }
            if stream_key(f) == own {
                return true;
            }
            let path = if let Ok(u) = url::Url::parse(f) {
                let full = format!(
                    "{}{}",
                    u.path(),
                    u.query().map(|q| format!("?{q}")).unwrap_or_default()
                );
                percent_encoding::percent_decode_str(&full)
                    .decode_utf8_lossy()
                    .to_string()
            } else {
                f.clone()
            };
            nk.len() >= 6 && norm(&path).contains(&nk)
        })
        .collect();

    if keep.is_empty() {
        cam.fb = None;
    } else {
        cam.fb = Some(Value::Array(keep.into_iter().map(Value::String).collect()));
    }
    cam
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CamCatalogResult {
    pub ok: bool,
    pub cams: Vec<CamItem>,
    pub bpjt_alive: usize,
    pub bpjt_total: usize,
    pub probing: bool,
    pub warming: bool,
    pub snap: bool,
    pub at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HistoryPayload {
    pub at: u64,
    pub cams: Vec<CamItem>,
    #[serde(default)]
    pub alive: Vec<String>,
    #[serde(default)]
    pub done: Vec<String>,
}

#[derive(Default)]
struct ProbeState {
    alive: HashSet<String>,
    done: HashSet<String>,
    running: bool,
    last: u64,
}

#[derive(Default)]
struct MemoEntry<T> {
    at: u64,
    val: T,
}

pub type FetchTextFn =
    Arc<dyn Fn(&str, u64) -> BoxFuture<'static, Result<String, String>> + Send + Sync + 'static>;
pub type ProbeFn =
    Arc<dyn Fn(&str, u64) -> BoxFuture<'static, Result<bool, String>> + Send + Sync + 'static>;
pub type DeadCheckFn = Arc<dyn Fn(&str) -> bool + Send + Sync + 'static>;
pub type FfmpegCheckFn = Arc<dyn Fn() -> BoxFuture<'static, bool> + Send + Sync + 'static>;
pub type IsDrainingFn = Arc<dyn Fn() -> bool + Send + Sync + 'static>;
pub type CamListFn = Arc<dyn Fn() -> Vec<CamItem> + Send + Sync + 'static>;

#[derive(Clone)]
pub struct CctvCatalogOptions {
    pub hist_dir: Option<PathBuf>,
    pub persist_hist: bool,
    pub user_agent: String,
    pub fetch_text: Option<FetchTextFn>,
    pub probe_url: Option<ProbeFn>,
    pub reported_dead: Option<DeadCheckFn>,
    pub detect_ffmpeg: Option<FfmpegCheckFn>,
    pub is_draining: Option<IsDrainingFn>,
    pub cam_list: Option<CamListFn>,
}

impl Default for CctvCatalogOptions {
    fn default() -> Self {
        Self {
            hist_dir: None,
            persist_hist: false,
            user_agent: "sitrep-cctv/1.0".to_string(),
            fetch_text: None,
            probe_url: None,
            reported_dead: None,
            detect_ffmpeg: None,
            is_draining: None,
            cam_list: None,
        }
    }
}

#[allow(dead_code)]
struct InnerState {
    last_good_cams: Vec<CamItem>,
    hist_dirty: bool,
    hist_save_at: u64,
    probe: ProbeState,
    host_cors: HashMap<String, bool>,
    src_cams: HashMap<String, Vec<CamItem>>,
    src_done: HashMap<String, bool>,
    mole_reg: HashMap<String, String>,
    mole_tok: HashMap<String, (String, u64)>,
    memo_cache: HashMap<String, MemoEntry<Vec<CamItem>>>,
    cat_cache: Option<MemoEntry<CamCatalogResult>>,
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

/// The stateful CCTV Catalog service.
#[derive(Clone)]
pub struct CctvCatalog {
    opts: CctvCatalogOptions,
    inner: Arc<RwLock<InnerState>>,
    client: reqwest::Client,
}

impl CctvCatalog {
    pub fn new(opts: CctvCatalogOptions) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(12))
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        let mut src_done = HashMap::new();
        for key in ["bpjt", "bekasi", "jogja", "molecool", "tangkab", "bali"] {
            src_done.insert(key.to_string(), false);
        }

        let catalog = Self {
            opts,
            inner: Arc::new(RwLock::new(InnerState {
                last_good_cams: Vec::new(),
                hist_dirty: false,
                hist_save_at: 0,
                probe: ProbeState::default(),
                host_cors: HashMap::new(),
                src_cams: HashMap::new(),
                src_done,
                mole_reg: HashMap::new(),
                mole_tok: HashMap::new(),
                memo_cache: HashMap::new(),
                cat_cache: None,
            })),
            client,
        };

        catalog.load_cctv_hist_sync();
        catalog
    }

    fn read_lock(&self) -> std::sync::RwLockReadGuard<'_, InnerState> {
        // INFALLIBLE: lock poisoning recovered via into_inner
        self.inner.read().unwrap_or_else(|e| e.into_inner())
    }

    fn write_lock(&self) -> std::sync::RwLockWriteGuard<'_, InnerState> {
        // INFALLIBLE: lock poisoning recovered via into_inner
        self.inner.write().unwrap_or_else(|e| e.into_inner())
    }

    fn hist_file_path(&self) -> Option<PathBuf> {
        self.opts
            .hist_dir
            .as_ref()
            .map(|d| d.join("cctv-history.json"))
    }

    fn load_cctv_hist_sync(&self) {
        let Some(path) = self.hist_file_path() else {
            return;
        };
        if !path.exists() {
            return;
        }
        let Ok(data) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(hist) = serde_json::from_str::<HistoryPayload>(&data) else {
            return;
        };
        let now = now_ms();
        if hist.at > now || now - hist.at > 7 * 86_400_000 {
            return;
        }

        let mut valid_cams = Vec::new();
        for c in hist.cams {
            if c.s.starts_with("http://") || c.s.starts_with("https://") {
                valid_cams.push(c);
            }
        }

        let mut g = self.write_lock();
        if !valid_cams.is_empty() {
            g.last_good_cams = dedupe(&valid_cams);
        }
        if now - hist.at <= 24 * 3_600_000 {
            for u in hist.alive {
                if !u.is_empty() {
                    g.probe.alive.insert(u);
                }
            }
            for u in hist.done {
                if !u.is_empty() {
                    g.probe.done.insert(u);
                }
            }
        }
    }

    /// Loads persisted CCTV history from disk into memory.
    pub fn load_cctv_hist(&self) -> Result<(), String> {
        let Some(path) = self.hist_file_path() else {
            return Ok(());
        };
        if !path.exists() {
            return Ok(());
        }
        let data =
            std::fs::read_to_string(&path).map_err(|e| format!("read history failed: {e}"))?;
        let hist = serde_json::from_str::<HistoryPayload>(&data)
            .map_err(|e| format!("parse history failed: {e}"))?;
        let now = now_ms();
        if hist.at > now || now - hist.at > 7 * 86_400_000 {
            return Ok(());
        }

        let mut valid_cams = Vec::new();
        for c in hist.cams {
            if c.s.starts_with("http://") || c.s.starts_with("https://") {
                valid_cams.push(c);
            }
        }

        let mut g = self.write_lock();
        if !valid_cams.is_empty() {
            g.last_good_cams = dedupe(&valid_cams);
        }
        if now - hist.at <= 24 * 3_600_000 {
            for u in hist.alive {
                if !u.is_empty() {
                    g.probe.alive.insert(u);
                }
            }
            for u in hist.done {
                if !u.is_empty() {
                    g.probe.done.insert(u);
                }
            }
        }
        Ok(())
    }

    /// Persists good camera streams to disk history file.
    pub fn save_cctv_hist(&self, force: bool) -> Result<(), String> {
        if !self.opts.persist_hist {
            return Ok(());
        }
        let Some(path) = self.hist_file_path() else {
            return Ok(());
        };
        let Some(parent) = path.parent() else {
            return Ok(());
        };

        let payload_str = {
            let mut g = self.write_lock();
            if g.last_good_cams.is_empty() {
                return Ok(());
            }
            if !g.hist_dirty && !force {
                return Ok(());
            }
            let now = now_ms();
            if !force && (now - g.hist_save_at < 30_000) {
                return Ok(());
            }
            g.hist_save_at = now;
            g.hist_dirty = false;

            let payload = HistoryPayload {
                at: now,
                cams: g.last_good_cams.clone(),
                alive: g.probe.alive.iter().cloned().collect(),
                done: g.probe.done.iter().cloned().collect(),
            };
            serde_json::to_string(&payload).map_err(|e| format!("serialize failed: {e}"))?
        };

        std::fs::create_dir_all(parent).map_err(|e| format!("mkdir failed: {e}"))?;
        let tmp_path = format!("{}.{}.tmp", path.display(), std::process::id());
        std::fs::write(&tmp_path, payload_str).map_err(|e| format!("write failed: {e}"))?;
        std::fs::rename(&tmp_path, &path).map_err(|e| format!("rename failed: {e}"))?;
        Ok(())
    }

    async fn fetch_str(&self, url: &str, timeout_ms: u64) -> Result<String, String> {
        if let Some(ref custom_fetch) = self.opts.fetch_text {
            return custom_fetch(url, timeout_ms).await;
        }

        let resp = self
            .client
            .get(url)
            .header("user-agent", &self.opts.user_agent)
            .timeout(std::time::Duration::from_millis(timeout_ms))
            .send()
            .await
            .map_err(|e| format!("HTTP request error: {e}"))?;

        if !resp.status().is_success() {
            return Err(format!("HTTP status {}", resp.status()));
        }

        resp.text()
            .await
            .map_err(|e| format!("read text error: {e}"))
    }

    /// Fetches BPJT toll road CCTV streams.
    async fn fetch_bpjt(&self) -> Vec<CamItem> {
        let text = match self.fetch_str("https://bpjt.pu.go.id/cctv/", 12000).await {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let Some(caps) = BPJT_RE.captures(&text) else {
            return Vec::new();
        };
        let Some(json_match) = caps.get(1) else {
            return Vec::new();
        };

        let Ok(val) = serde_json::from_str::<Value>(json_match.as_str()) else {
            return Vec::new();
        };
        let Some(map) = val.as_object() else {
            return Vec::new();
        };

        let mut seen = HashSet::new();
        let mut out = Vec::new();

        for (_, g) in map {
            let Some(arr) = g.as_array() else {
                continue;
            };
            for c in arr {
                let stream = c.get("streamhls").and_then(|v| v.as_str()).unwrap_or("");
                if !stream.to_ascii_lowercase().contains("m3u8") || !seen.insert(stream.to_string())
                {
                    continue;
                }
                let status = c.get("status").and_then(|v| v.as_str()).unwrap_or("");
                if status != "online" && status != "1" {
                    continue;
                }
                let la = c.get("lat").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
                let lo = c.get("lon").and_then(|v| v.as_f64()).unwrap_or(f64::NAN);
                if !la.is_finite()
                    || !lo.is_finite()
                    || la < -11.0
                    || la > 6.0
                    || lo < 95.0
                    || lo > 141.0
                {
                    continue;
                }
                let km = c
                    .get("nama_km")
                    .and_then(|v| v.as_str())
                    .or_else(|| c.get("nama_segment").and_then(|v| v.as_str()))
                    .unwrap_or("CCTV");
                let ruas = c
                    .get("nama_ruas")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .replace('-', " ");
                let name = format!("{km} · {ruas}");
                let region = id_region(la, lo);

                out.push(CamItem {
                    n: name,
                    s: stream.to_string(),
                    e: Some(String::new()),
                    la: Some(la),
                    lo: Some(lo),
                    c: Some(region),
                    sub: None,
                    d: None,
                    src: Some("bpjt".to_string()),
                    snap: None,
                    fb: None,
                });
            }
        }
        out
    }

    /// Fetches Kota Bekasi CCTV streams.
    async fn fetch_bekasi(&self) -> Vec<CamItem> {
        let text = match self
            .fetch_str(
                "https://eofficev2.bekasikota.go.id/backupcctv/m3/list.json",
                10000,
            )
            .await
        {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let Ok(val) = serde_json::from_str::<Value>(&text) else {
            return Vec::new();
        };
        let Some(arr) = val.as_array() else {
            return Vec::new();
        };

        arr.iter()
            .filter_map(|x| {
                let path = x.get("path").and_then(|v| v.as_str())?;
                let name = x
                    .get("name")
                    .and_then(|v| v.as_str())
                    .unwrap_or(path)
                    .replace('_', " ");
                Some(CamItem {
                    n: name,
                    s: format!("https://eofficev2.bekasikota.go.id/backupcctv/m3/{path}"),
                    e: Some(String::new()),
                    la: None,
                    lo: None,
                    c: Some("Kota Bekasi".to_string()),
                    sub: None,
                    d: None,
                    src: Some("pemda".to_string()),
                    snap: None,
                    fb: None,
                })
            })
            .collect()
    }

    /// Fetches Kota Yogyakarta CCTV streams.
    async fn fetch_jogja(&self) -> Vec<CamItem> {
        let text = match self
            .fetch_str("https://cctv.jogjakota.go.id/home/getdata", 10000)
            .await
        {
            Ok(t) => t,
            Err(_) => return Vec::new(),
        };

        let Ok(val) = serde_json::from_str::<Value>(&text) else {
            return Vec::new();
        };
        let Some(arr) = val.as_array() else {
            return Vec::new();
        };

        arr.iter()
            .filter_map(|x| {
                let link = x.get("cctv_link").and_then(|v| v.as_str())?;
                if !link.to_ascii_lowercase().contains(".m3u8") {
                    return None;
                }
                let title = x
                    .get("cctv_title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("Jogja")
                    .replace('_', " ");
                let la = x.get("cctv_latitude").and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                });
                let lo = x.get("cctv_longitude").and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                });

                Some(CamItem {
                    n: format!("CCTV {title}"),
                    s: link.to_string(),
                    e: Some(String::new()),
                    la,
                    lo,
                    c: Some("Kota Yogyakarta".to_string()),
                    sub: None,
                    d: None,
                    src: Some("jogja".to_string()),
                    snap: None,
                    fb: None,
                })
            })
            .collect()
    }

    async fn fetch_source_memoized(
        &self,
        key: &str,
        ttl_ms: u64,
        f: impl std::future::Future<Output = Vec<CamItem>>,
    ) -> Vec<CamItem> {
        let now = now_ms();
        {
            let g = self.read_lock();
            if let Some(entry) = g.memo_cache.get(key)
                && now.saturating_sub(entry.at) < ttl_ms
                && !entry.val.is_empty()
            {
                return entry.val.clone();
            }
        }

        let res = f.await;
        let mut g = self.write_lock();
        g.src_done.insert(key.to_string(), true);
        if !res.is_empty() {
            g.memo_cache.insert(
                key.to_string(),
                MemoEntry {
                    at: now,
                    val: res.clone(),
                },
            );
            g.src_cams.insert(key.to_string(), res.clone());
        }
        res
    }

    /// Fetches Molecool token for a camera slug if expired or missing.
    pub async fn mole_fresh(&self, slug: &str) -> Option<String> {
        let g = self.read_lock();
        if let Some((tok, exp)) = g.mole_tok.get(slug) {
            let now = now_ms();
            if *exp > now && exp - now > 5 * 60_000 {
                return Some(tok.clone());
            }
        }
        None
    }

    /// Runs probing sweep on cameras.
    pub async fn warm_cctv(&self) {
        {
            let mut g = self.write_lock();
            if g.probe.running {
                return;
            }
            g.probe.running = true;
            g.probe.last = now_ms();
        }

        let bpjt = self.fetch_bpjt().await;
        let jogja = self.fetch_jogja().await;

        {
            let mut g = self.write_lock();
            for c in bpjt.iter().chain(jogja.iter()) {
                g.probe.done.insert(c.s.clone());
                g.probe.alive.insert(c.s.clone());
            }
            g.probe.running = false;
            g.hist_dirty = true;
        }

        let _ = self.save_cctv_hist(true);
    }

    /// Retrieves camera list.
    pub async fn cam_list(&self) -> Vec<CamItem> {
        if let Some(ref custom_list) = self.opts.cam_list {
            return custom_list();
        }
        let g = self.read_lock();
        g.last_good_cams.clone()
    }

    /// Returns the map of CORS support by hostname.
    pub async fn host_cors(&self) -> HashMap<String, bool> {
        let g = self.read_lock();
        g.host_cors.clone()
    }

    /// Returns full aggregated CCTV catalog.
    pub async fn catalog(&self) -> CamCatalogResult {
        let now = now_ms();
        {
            let g = self.read_lock();
            if let Some(ref cat) = g.cat_cache
                && now.saturating_sub(cat.at) < 300_000
                && !cat.val.warming
                && cat.val.cams.len() >= 20
            {
                return cat.val.clone();
            }
        }

        let bpjt = self
            .fetch_source_memoized("bpjt", 1_800_000, self.fetch_bpjt())
            .await;
        let bekasi = self
            .fetch_source_memoized("bekasi", 1_800_000, self.fetch_bekasi())
            .await;
        let jogja = self
            .fetch_source_memoized("jogja", 1_800_000, self.fetch_jogja())
            .await;

        let (last_cams, alive_count, probing) = {
            let g = self.read_lock();
            (
                g.last_good_cams.clone(),
                g.probe.alive.len(),
                g.probe.running,
            )
        };

        let is_dead = |s: &str| -> bool {
            if let Some(ref check) = self.opts.reported_dead {
                check(s)
            } else {
                false
            }
        };

        let mut probeable = Vec::new();
        {
            let g = self.read_lock();
            for c in jogja.iter().chain(bpjt.iter()) {
                if g.probe.done.contains(&c.s) {
                    if g.probe.alive.contains(&c.s) {
                        probeable.push(c.clone());
                    }
                } else if g.probe.alive.len() < 24 {
                    probeable.push(c.clone());
                }
            }
        }

        let verified: Vec<CamItem> = probeable.into_iter().filter(|c| !is_dead(&c.s)).collect();
        let verified_non_toll: Vec<CamItem> = verified
            .iter()
            .filter(|c| c.src.as_deref() != Some("bpjt"))
            .cloned()
            .collect();
        let verified_toll: Vec<CamItem> = verified
            .iter()
            .filter(|c| c.src.as_deref() == Some("bpjt"))
            .cloned()
            .collect();

        let mut combined = Vec::new();
        for c in &bekasi {
            if !is_dead(&c.s) {
                combined.push(c.clone());
            }
        }
        combined.extend(verified_non_toll);
        combined.extend(verified_toll);

        let mut cams = dedupe(&combined);
        if cams.is_empty() && !last_cams.is_empty() {
            cams = last_cams.into_iter().filter(|c| !is_dead(&c.s)).collect();
        } else if !cams.is_empty() {
            {
                let mut g = self.write_lock();
                if cams.len() != g.last_good_cams.len() {
                    g.hist_dirty = true;
                }
                g.last_good_cams = dedupe(&cams);
            }
            let _ = self.save_cctv_hist(false);
        }

        let snap = if let Some(ref ffmpeg_check) = self.opts.detect_ffmpeg {
            ffmpeg_check().await
        } else {
            false
        };

        let warming = cams.is_empty() || (probing && alive_count < 24);

        let result = CamCatalogResult {
            ok: true,
            cams,
            bpjt_alive: alive_count,
            bpjt_total: bpjt.len(),
            probing,
            warming,
            snap,
            at: now,
        };

        if !result.warming && result.cams.len() >= 20 {
            let mut g = self.write_lock();
            g.cat_cache = Some(MemoEntry {
                at: now,
                val: result.clone(),
            });
        }

        result
    }
}
