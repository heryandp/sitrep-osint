//! Async `GeoSensors` managing real-time sensor polling:
//! flights, disasters (earthquakes, volcanoes, floods), weather, radar,
//! fires (NASA FIRMS, Sipongi), BMKG alerts, maritime AIS, and TransJakarta transit.

#![forbid(unsafe_code)]
#![allow(dead_code, clippy::collapsible_if)]

use crate::port::{parse_cap_field, parse_csv, parse_magma, parse_polygon};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::RwLock;

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn geo_centroid(geom: &Value) -> (f64, f64) {
    let Some(coords) = geom.get("coordinates") else {
        return (f64::NAN, f64::NAN);
    };

    let mut pts: Vec<[f64; 2]> = Vec::new();
    fn walk(val: &Value, pts: &mut Vec<[f64; 2]>) {
        if let Some(arr) = val.as_array() {
            if arr.len() >= 2
                && arr[0].is_number()
                && arr[1].is_number()
                && let (Some(x), Some(y)) = (arr[0].as_f64(), arr[1].as_f64())
            {
                pts.push([x, y]);
                return;
            }
            for item in arr {
                walk(item, pts);
            }
        }
    }
    walk(coords, &mut pts);

    if pts.is_empty() {
        return (f64::NAN, f64::NAN);
    }

    let mut min_la = 90.0_f64;
    let mut max_la = -90.0_f64;
    let mut min_lo = 180.0_f64;
    let mut max_lo = -180.0_f64;

    for pt in pts {
        let lo = pt[0];
        let la = pt[1];
        if !la.is_finite() || !lo.is_finite() {
            continue;
        }
        if la < min_la {
            min_la = la;
        }
        if la > max_la {
            max_la = la;
        }
        if lo < min_lo {
            min_lo = lo;
        }
        if lo > max_lo {
            max_lo = lo;
        }
    }

    if min_la.is_finite() && min_la <= max_la {
        ((min_la + max_la) / 2.0, (min_lo + max_lo) / 2.0)
    } else {
        (f64::NAN, f64::NAN)
    }
}

pub const FL_PTS: [[f64; 3]; 6] = [
    [3.6, 98.7, 250.0],
    [-6.2, 106.8, 250.0],
    [-3.0, 114.6, 250.0],
    [-2.0, 120.5, 250.0],
    [-8.6, 115.3, 250.0],
    [-2.5, 140.7, 250.0],
];

pub const WX_PTS: [[f64; 2]; 20] = [
    [95.3, 5.5],
    [99.5, 0.5],
    [105.8, -6.2],
    [110.4, -7.5],
    [114.5, -3.0],
    [119.4, -0.5],
    [117.0, -5.1],
    [113.6, -7.3],
    [120.9, 0.5],
    [119.9, -4.5],
    [123.0, -1.5],
    [124.8, 0.9],
    [127.4, 0.9],
    [130.8, -1.5],
    [131.3, -3.0],
    [136.1, -4.5],
    [137.8, -3.0],
    [140.5, -3.7],
    [113.9, 0.5],
    [95.3, 3.6],
];

pub const WIND_LATS_DESC: [f64; 9] = [6.0, 4.0, 2.0, 0.0, -2.0, -4.0, -6.0, -8.0, -10.0];
pub const WIND_LONS: [f64; 16] = [
    95.0, 98.0, 101.0, 104.0, 107.0, 110.0, 113.0, 116.0, 119.0, 122.0, 125.0, 128.0, 131.0, 134.0,
    137.0, 140.0,
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Flight {
    pub cs: String,
    pub lat: f64,
    pub lon: f64,
    pub alt: i64,
    pub gs: Option<i64>,
    pub hdg: Option<i64>,
    #[serde(rename = "type")]
    pub flight_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ship {
    pub mmsi: Value,
    pub name: String,
    pub dest: String,
    #[serde(rename = "type")]
    pub ship_type: Value,
    pub lat: f64,
    pub lon: f64,
    pub sog: Option<f64>,
    pub hdg: Option<i64>,
    pub at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheEntry<T> {
    pub at: i64,
    pub data: Option<T>,
}

impl<T> Default for CacheEntry<T> {
    fn default() -> Self {
        Self { at: 0, data: None }
    }
}

pub struct GeoSensors {
    client: Client,
    user_agent: String,
    firms_key: String,
    fl_cache: Arc<RwLock<CacheEntry<Vec<Flight>>>>,
    tma_cache: Arc<RwLock<CacheEntry<Value>>>,
    pb_cache: Arc<RwLock<CacheEntry<Value>>>,
    dis_cache: Arc<RwLock<CacheEntry<Value>>>,
    wx_cache: Arc<RwLock<CacheEntry<Value>>>,
    radar_cache: Arc<RwLock<CacheEntry<Value>>>,
    fire_cache: Arc<RwLock<CacheEntry<Value>>>,
    sipongi_cache: Arc<RwLock<CacheEntry<Value>>>,
    model_run_cache: Arc<RwLock<CacheEntry<Value>>>,
    sat_cache: Arc<RwLock<CacheEntry<Value>>>,
    wind_cache: Arc<RwLock<CacheEntry<Value>>>,
    alert_cache: Arc<RwLock<CacheEntry<Value>>>,
    bus_cache: Arc<RwLock<CacheEntry<Value>>>,
    pub ship_map: Arc<RwLock<HashMap<String, Ship>>>,
    tj_token: Arc<RwLock<Option<(String, i64)>>>,
    ais_live: Arc<RwLock<bool>>,
}

impl Default for GeoSensors {
    fn default() -> Self {
        Self::new()
    }
}

impl GeoSensors {
    pub fn new() -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .unwrap_or_else(|_| Client::new());
        let firms_key = std::env::var("NASA_FIRMS_API_KEY").unwrap_or_default();

        Self {
            client,
            user_agent: "sitrep-console/1.0".to_string(),
            firms_key,
            fl_cache: Arc::new(RwLock::new(CacheEntry::default())),
            tma_cache: Arc::new(RwLock::new(CacheEntry::default())),
            pb_cache: Arc::new(RwLock::new(CacheEntry::default())),
            dis_cache: Arc::new(RwLock::new(CacheEntry::default())),
            wx_cache: Arc::new(RwLock::new(CacheEntry::default())),
            radar_cache: Arc::new(RwLock::new(CacheEntry::default())),
            fire_cache: Arc::new(RwLock::new(CacheEntry::default())),
            sipongi_cache: Arc::new(RwLock::new(CacheEntry::default())),
            model_run_cache: Arc::new(RwLock::new(CacheEntry::default())),
            sat_cache: Arc::new(RwLock::new(CacheEntry::default())),
            wind_cache: Arc::new(RwLock::new(CacheEntry::default())),
            alert_cache: Arc::new(RwLock::new(CacheEntry::default())),
            bus_cache: Arc::new(RwLock::new(CacheEntry::default())),
            ship_map: Arc::new(RwLock::new(HashMap::new())),
            tj_token: Arc::new(RwLock::new(None)),
            ais_live: Arc::new(RwLock::new(false)),
        }
    }

    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    pub async fn is_ais_live(&self) -> bool {
        *self.ais_live.read().await
    }

    pub async fn stop_ais(&self) {
        *self.ais_live.write().await = false;
    }

    pub async fn ships(&self) -> Vec<Ship> {
        let cutoff = now_ms() - 10 * 60 * 1000;
        let map = self.ship_map.read().await;
        let mut list: Vec<Ship> = map.values().filter(|s| s.at > cutoff).cloned().collect();
        list.sort_by(|a, b| {
            let sog_a = a.sog.unwrap_or(0.0);
            let sog_b = b.sog.unwrap_or(0.0);
            sog_b
                .partial_cmp(&sog_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        if list.len() > 800 {
            list.truncate(800);
        }
        list
    }

    pub async fn tma_pintu_air(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.tma_cache.read().await;
            if now - cache.at < 60_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "gates": [] }));
            }
        }

        let resp = self
            .client
            .get("https://sisteminformasibanjir.jakarta.go.id/map_master/pintu_air")
            .header("accept", "application/json, text/plain, */*")
            .header("User-Agent", &self.user_agent)
            .send()
            .await;

        let parsed: Value = match resp {
            Ok(r) => r.json().await.unwrap_or(json!({})),
            Err(e) => {
                let err_msg = e.to_string();
                let fallback = json!({
                    "ok": false,
                    "err": if err_msg.len() > 150 { &err_msg[..150] } else { &err_msg },
                    "gates": []
                });
                return fallback;
            }
        };

        let mut gates = Vec::new();
        let empty_vec = Vec::new();
        let features = parsed
            .get("features")
            .and_then(|f| f.as_array())
            .unwrap_or(&empty_vec);

        for f in features {
            let p = f.get("properties").unwrap_or(&Value::Null);
            let pa = p.get("pintu_air").unwrap_or(&Value::Null);
            let coords = f
                .get("geometry")
                .and_then(|g| g.get("coordinates"))
                .and_then(|c| c.as_array());

            let lon = coords.and_then(|c| c.first()).and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            });
            let lat = coords.and_then(|c| c.get(1)).and_then(|v| {
                v.as_f64()
                    .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
            });

            let siaga = p
                .get("status_siaga")
                .and_then(|v| {
                    v.as_i64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(4);

            let siaga_label = match siaga {
                1 => "Siaga 1 (Bahaya)",
                2 => "Siaga 2 (Kritis)",
                3 => "Siaga 3 (Waspada)",
                _ => "Siaga 4 (Normal)",
            };

            let cuaca = p
                .get("cuaca")
                .and_then(|c| c.get("nama").and_then(|n| n.as_str()))
                .or_else(|| p.get("cuaca").and_then(|c| c.as_str()))
                .unwrap_or("—");

            let id_val = pa
                .get("id")
                .unwrap_or_else(|| p.get("id").unwrap_or(&Value::Null));
            let name_val = pa
                .get("name")
                .and_then(|v| v.as_str())
                .or_else(|| p.get("name").and_then(|v| v.as_str()))
                .unwrap_or("Pintu Air");

            let sistem = pa
                .get("sistem_aliran")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ketinggian = p
                .get("ketinggian")
                .and_then(|v| {
                    v.as_f64()
                        .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                })
                .unwrap_or(0.0);

            gates.push(json!({
                "id": id_val,
                "name": name_val,
                "sistem": sistem,
                "ketinggian": ketinggian,
                "siaga": siaga,
                "siaga_label": siaga_label,
                "cuaca": cuaca,
                "tanggal": p.get("tanggal").and_then(|v| v.as_str()).unwrap_or(""),
                "jam": p.get("jam").and_then(|v| v.as_str()).unwrap_or(""),
                "lat": lat.filter(|l| l.is_finite()),
                "lon": lon.filter(|l| l.is_finite()),
            }));
        }

        let result = json!({
            "ok": true,
            "gates": gates,
            "at": now,
        });

        let mut cache = self.tma_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn petabencana_floods(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.pb_cache.read().await;
            if now - cache.at < 120_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "areas": [], "reports": [] }));
            }
        }

        let max_age = 48 * 3600 * 1000;
        let floods_url = "https://data.petabencana.id/floods?minimum_state=1&geoformat=geojson";
        let reports_url =
            "https://data.petabencana.id/reports?disaster=flood&timeperiod=86400&geoformat=geojson";

        let (f_res, r_res) = tokio::join!(
            self.client
                .get(floods_url)
                .header("User-Agent", &self.user_agent)
                .send(),
            self.client
                .get(reports_url)
                .header("User-Agent", &self.user_agent)
                .send()
        );

        let f_val: Option<Value> = match f_res {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };
        let r_val: Option<Value> = match r_res {
            Ok(r) => r.json().await.ok(),
            Err(_) => None,
        };

        if f_val.is_none() && r_val.is_none() {
            let cache = self.pb_cache.read().await;
            if let Some(prev) = &cache.data {
                return prev.clone();
            }
            return json!({ "ok": false, "areas": [], "reports": [] });
        }

        let mut areas = Vec::new();
        if let Some(val) = f_val {
            let empty_vec = Vec::new();
            let feats = val
                .get("result")
                .and_then(|r| r.get("features"))
                .and_then(|f| f.as_array())
                .or_else(|| val.get("features").and_then(|f| f.as_array()))
                .unwrap_or(&empty_vec);

            for f in feats {
                let p = f.get("properties").unwrap_or(&Value::Null);
                let lu = p.get("last_updated").and_then(|v| v.as_str()).unwrap_or("");
                let ts = chrono::DateTime::parse_from_rfc3339(lu)
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0);
                if ts == 0 || (now - ts) > max_age {
                    continue;
                }
                let geom = f.get("geometry").unwrap_or(&Value::Null);
                let (lat, lon) = geo_centroid(geom);
                if !lat.is_finite() || !lon.is_finite() {
                    continue;
                }
                let area_id = p
                    .get("area_id")
                    .and_then(|v| v.as_str())
                    .or_else(|| p.get("area_name").and_then(|v| v.as_str()))
                    .unwrap_or("");
                let area_name = p.get("area_name").and_then(|v| v.as_str()).unwrap_or("");
                let parent = p.get("parent_name").and_then(|v| v.as_str()).unwrap_or("");
                let city = p.get("city_name").and_then(|v| v.as_str()).unwrap_or("");
                let state = p.get("state").and_then(|v| v.as_i64()).unwrap_or(1);

                areas.push(json!({
                    "id": area_id,
                    "name": area_name,
                    "parent": parent,
                    "city": city,
                    "state": state,
                    "ts": ts,
                    "lat": lat,
                    "lon": lon,
                    "geometry": geom,
                }));
            }
        }

        let mut reports = Vec::new();
        if let Some(val) = r_val {
            let empty_vec = Vec::new();
            let feats = val
                .get("result")
                .and_then(|r| r.get("features"))
                .and_then(|f| f.as_array())
                .or_else(|| val.get("features").and_then(|f| f.as_array()))
                .unwrap_or(&empty_vec);

            for f in feats {
                let p = f.get("properties").unwrap_or(&Value::Null);
                let coords = f
                    .get("geometry")
                    .and_then(|g| g.get("coordinates"))
                    .and_then(|c| c.as_array());
                let lon = coords.and_then(|c| c.first()).and_then(|v| v.as_f64());
                let lat = coords.and_then(|c| c.get(1)).and_then(|v| v.as_f64());
                let (lat, lon) = match (lat, lon) {
                    (Some(lat), Some(lon)) if lat.is_finite() && lon.is_finite() => (lat, lon),
                    _ => continue,
                };

                let ca = p.get("created_at").and_then(|v| v.as_str()).unwrap_or("");
                let ts = chrono::DateTime::parse_from_rfc3339(ca)
                    .map(|dt| dt.timestamp_millis())
                    .unwrap_or(0);
                if ts == 0 || (now - ts) > max_age {
                    continue;
                }

                let rd = p.get("report_data").unwrap_or(&Value::Null);
                let depth = rd
                    .get("flood_depth")
                    .and_then(|v| v.as_f64().or_else(|| v.as_i64().map(|n| n as f64)))
                    .unwrap_or(0.0);
                let state = if depth >= 70.0 {
                    4
                } else if depth >= 30.0 {
                    3
                } else if depth >= 10.0 {
                    2
                } else {
                    1
                };

                let pkey = p.get("pkey").and_then(|v| v.as_str()).unwrap_or("");
                let title = p
                    .get("title")
                    .and_then(|v| v.as_str())
                    .or_else(|| p.get("text").and_then(|v| v.as_str()))
                    .unwrap_or("Laporan Banjir");
                let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
                let img = p.get("image_url").and_then(|v| v.as_str()).unwrap_or("");
                let src = p
                    .get("source")
                    .and_then(|v| v.as_str())
                    .unwrap_or("PetaBencana");

                reports.push(json!({
                    "id": pkey,
                    "title": title,
                    "text": text,
                    "depth": depth,
                    "state": state,
                    "ts": ts,
                    "lat": lat,
                    "lon": lon,
                    "img": img,
                    "src": src,
                    "url": format!("petabencana://report/{pkey}"),
                }));
            }
        }

        let result = json!({
            "ok": true,
            "areas": areas,
            "reports": reports,
            "at": now,
        });

        let mut cache = self.pb_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn disaster(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.dis_cache.read().await;
            if now - cache.at < 120_000 && cache.data.is_some() {
                return cache.data.clone().unwrap_or(json!({ "ok": false }));
            }
        }

        let auto_url = "https://data.bmkg.go.id/DataMKG/TEWS/autogempa.json";
        let hist_url = "https://data.bmkg.go.id/DataMKG/TEWS/gempaterkini.json";
        let magma_url = "https://magma.esdm.go.id/v1/gunung-api/tingkat-aktivitas";

        let (auto_res, hist_res, magma_res) = tokio::join!(
            self.client
                .get(auto_url)
                .header("User-Agent", &self.user_agent)
                .send(),
            self.client
                .get(hist_url)
                .header("User-Agent", &self.user_agent)
                .send(),
            self.client
                .get(magma_url)
                .header("User-Agent", &self.user_agent)
                .send(),
        );

        let mut gempa = Vec::new();
        let mut push_gempa = |g: &Value| {
            let coords_str = g.get("Coordinates").and_then(|v| v.as_str()).unwrap_or("");
            let parts: Vec<&str> = coords_str.split(',').collect();
            if parts.len() == 2
                && let (Ok(lat), Ok(lon)) = (
                    parts[0].trim().parse::<f64>(),
                    parts[1].trim().parse::<f64>(),
                )
                && lat.is_finite()
                && lon.is_finite()
            {
                let mag = g
                    .get("Magnitude")
                    .and_then(|v| {
                        v.as_f64()
                            .or_else(|| v.as_str().and_then(|s| s.parse().ok()))
                    })
                    .unwrap_or(0.0);
                gempa.push(json!({
                    "mag": mag,
                    "lat": lat,
                    "lon": lon,
                    "depth": g.get("Kedalaman").and_then(|v| v.as_str()).unwrap_or(""),
                    "wilayah": g.get("Wilayah").and_then(|v| v.as_str()).unwrap_or(""),
                    "mmi": g.get("Dirasakan").and_then(|v| v.as_str()).unwrap_or(""),
                    "potensi": g.get("Potensi").and_then(|v| v.as_str()).unwrap_or(""),
                    "time": g.get("DateTime").and_then(|v| v.as_str())
                        .or_else(|| g.get("Tanggal").and_then(|v| v.as_str())).unwrap_or(""),
                }));
            }
        };

        let auto_ok = if let Ok(resp) = auto_res {
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(g) = val.get("Infogempa").and_then(|i| i.get("gempa")) {
                    push_gempa(g);
                }
                true
            } else {
                false
            }
        } else {
            false
        };

        let hist_ok = if let Ok(resp) = hist_res {
            if let Ok(val) = resp.json::<Value>().await {
                if let Some(arr) = val
                    .get("Infogempa")
                    .and_then(|i| i.get("gempa"))
                    .and_then(|g| g.as_array())
                {
                    for g in arr.iter().take(15) {
                        push_gempa(g);
                    }
                }
                true
            } else {
                false
            }
        } else {
            false
        };

        let (magma_ok, gunung) = if let Ok(resp) = magma_res {
            if let Ok(text) = resp.text().await {
                let parsed = parse_magma(Value::String(text)).unwrap_or(json!([]));
                (true, parsed)
            } else {
                (false, json!([]))
            }
        } else {
            (false, json!([]))
        };

        // Deduplicate gempa
        let mut seen = std::collections::HashSet::new();
        let mut dedup_gempa = Vec::new();
        for g in gempa {
            let lat = g.get("lat").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let lon = g.get("lon").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let mag = g.get("mag").and_then(|v| v.as_f64()).unwrap_or(0.0);
            let key = format!("{lat:.3}{lon:.3}{mag}");
            if seen.insert(key) {
                dedup_gempa.push(g);
            }
        }

        let (tma_val, pb_val) = tokio::join!(self.tma_pintu_air(), self.petabencana_floods());

        let gempa_ok = auto_ok || hist_ok;
        let pb_ok = pb_val.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);

        let prev = {
            let cache = self.dis_cache.read().await;
            cache.data.clone()
        };

        let gempa_out = if gempa_ok {
            Value::Array(dedup_gempa)
        } else {
            prev.as_ref()
                .and_then(|p| p.get("gempa"))
                .cloned()
                .unwrap_or(json!([]))
        };

        let gunung_out = if magma_ok {
            gunung
        } else {
            prev.as_ref()
                .and_then(|p| p.get("gunung"))
                .cloned()
                .unwrap_or(json!([]))
        };

        let tma_out = tma_val.get("gates").cloned().unwrap_or_else(|| json!([]));

        let pb_out = if pb_ok {
            json!({
                "areas": pb_val.get("areas").unwrap_or(&json!([])),
                "reports": pb_val.get("reports").unwrap_or(&json!([])),
            })
        } else {
            prev.as_ref()
                .and_then(|p| p.get("floods"))
                .cloned()
                .unwrap_or(json!({ "areas": [], "reports": [] }))
        };

        let stale = prev.is_some() && (!gempa_ok || !magma_ok);
        let result = json!({
            "ok": true,
            "gempa": gempa_out,
            "gunung": gunung_out,
            "tma": tma_out,
            "floods": pb_out,
            "at": now,
            "stale": stale,
            "srcErr": {
                "gempa": !gempa_ok,
                "gunung": !magma_ok,
                "floods": !pb_ok
            }
        });

        let mut cache = self.dis_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn flights(&self) -> Vec<Flight> {
        let now = now_ms();
        {
            let cache = self.fl_cache.read().await;
            if now - cache.at < 20_000 && cache.data.is_some() {
                return cache.data.clone().unwrap_or_default();
            }
        }

        let mut seen: HashMap<String, Flight> = HashMap::new();
        let mut any_ok = false;

        for [lat, lon, dist] in FL_PTS {
            let url1 = format!("https://opendata.adsb.fi/api/v3/lat/{lat}/lon/{lon}/dist/{dist}");
            let resp1 = self.client.get(&url1).send().await;
            let mut got_data = false;
            if let Ok(r) = resp1
                && r.status().is_success()
                && let Ok(val) = r.json::<Value>().await
            {
                any_ok = true;
                got_data = true;
                if let Some(ac) = val.get("ac").and_then(|v| v.as_array()) {
                    for a in ac {
                        self.norm_flight(a, &mut seen);
                    }
                }
            }
            if !got_data {
                let url2 = format!("https://api.adsb.lol/v2/lat/{lat}/lon/{lon}/dist/{dist}");
                if let Ok(r) = self.client.get(&url2).send().await
                    && r.status().is_success()
                    && let Ok(val) = r.json::<Value>().await
                {
                    any_ok = true;
                    if let Some(ac) = val.get("ac").and_then(|v| v.as_array()) {
                        for a in ac {
                            self.norm_flight(a, &mut seen);
                        }
                    }
                }
            }
        }

        let mut list: Vec<Flight> = seen.into_values().collect();
        if list.len() > 1500 {
            list.truncate(1500);
        }

        let mut cache = self.fl_cache.write().await;
        if any_ok || cache.data.as_ref().is_none_or(|v| v.is_empty()) {
            cache.data = Some(list.clone());
            cache.at = if any_ok { now } else { now - 20000 + 8000 };
        } else {
            cache.at = now - 20000 + 8000;
        }
        cache.data.clone().unwrap_or_default()
    }

    fn norm_flight(&self, a: &Value, seen: &mut HashMap<String, Flight>) {
        let lat = a.get("lat").and_then(|v| v.as_f64());
        let lon = a.get("lon").and_then(|v| v.as_f64());
        let alt_baro = a.get("alt_baro").and_then(|v| v.as_f64());
        let (lat, lon, alt_baro) = match (lat, lon, alt_baro) {
            (Some(la), Some(lo), Some(alt)) if alt >= 0.0 => (la, lo, alt),
            _ => return,
        };

        let hex = a.get("hex").and_then(|v| v.as_str()).unwrap_or("");
        let flight = a
            .get("flight")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .trim();
        let k = if !hex.is_empty() {
            hex.to_string()
        } else {
            format!("{flight}{lat}{lon}")
        };

        seen.entry(k).or_insert_with(|| {
            let cs = if !flight.is_empty() {
                flight.to_string()
            } else if !hex.is_empty() {
                hex.to_string()
            } else {
                "?".to_string()
            };
            let alt = (alt_baro * 0.3048).round() as i64;
            let gs = a
                .get("gs")
                .and_then(|v| v.as_f64())
                .map(|s| (s * 1.852).round() as i64);
            let hdg = a
                .get("track")
                .and_then(|v| v.as_f64())
                .map(|t| t.round() as i64);
            let ftype = a
                .get("t")
                .and_then(|v| v.as_str())
                .or_else(|| a.get("desc").and_then(|v| v.as_str()))
                .unwrap_or("")
                .to_string();

            Flight {
                cs,
                lat,
                lon,
                alt,
                gs,
                hdg,
                flight_type: ftype,
            }
        });
    }

    pub async fn weather(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.wx_cache.read().await;
            if now - cache.at < 600_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "items": [] }));
            }
        }

        let lats: Vec<String> = WX_PTS.iter().map(|p| p[1].to_string()).collect();
        let lons: Vec<String> = WX_PTS.iter().map(|p| p[0].to_string()).collect();
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=temperature_2m,weather_code,wind_speed_10m,wind_direction_10m,wind_gusts_10m,cloud_cover,precipitation,precipitation_probability,pressure_msl,relative_humidity_2m,visibility&timezone=Asia%2FJakarta",
            lats.join(","),
            lons.join(",")
        );

        let resp = self.client.get(&url).send().await;
        let d: Value = match resp {
            Ok(r) => r.json().await.unwrap_or(json!([])),
            Err(e) => {
                let err_msg = e.to_string();
                return json!({
                    "ok": false,
                    "err": if err_msg.len() > 150 { &err_msg[..150] } else { &err_msg },
                    "items": []
                });
            }
        };

        let list: Vec<&Value> = if let Some(arr) = d.as_array() {
            arr.iter().collect()
        } else {
            vec![&d]
        };

        let mut items = Vec::new();
        for (i, w) in list.iter().enumerate() {
            if i >= WX_PTS.len() {
                break;
            }
            let cur = w.get("current").unwrap_or(&Value::Null);
            items.push(json!({
                "lat": WX_PTS[i][1],
                "lon": WX_PTS[i][0],
                "temp": cur.get("temperature_2m"),
                "code": cur.get("weather_code"),
                "wind": cur.get("wind_speed_10m"),
                "wdir": cur.get("wind_direction_10m"),
                "gust": cur.get("wind_gusts_10m"),
                "cloud": cur.get("cloud_cover"),
                "precip": cur.get("precipitation"),
                "precipProb": cur.get("precipitation_probability"),
                "pressure": cur.get("pressure_msl"),
                "humidity": cur.get("relative_humidity_2m"),
                "visibility": cur.get("visibility")
            }));
        }

        let result = json!({
            "ok": true,
            "items": items,
            "at": now,
        });

        let mut cache = self.wx_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn radar(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.radar_cache.read().await;
            if now - cache.at < 300_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "frames": [] }));
            }
        }

        let resp = self
            .client
            .get("https://api.rainviewer.com/public/weather-maps.json")
            .send()
            .await;
        let d: Value = match resp {
            Ok(r) => r.json().await.unwrap_or(json!({})),
            Err(e) => {
                let err_msg = e.to_string();
                return json!({
                    "ok": false,
                    "err": if err_msg.len() > 150 { &err_msg[..150] } else { &err_msg },
                    "frames": []
                });
            }
        };

        let generated = d
            .get("generated")
            .and_then(|v| v.as_f64())
            .unwrap_or((now as f64) / 1000.0);
        let host = d
            .get("host")
            .and_then(|v| v.as_str())
            .unwrap_or("https://tilecache.rainviewer.com");
        let empty_vec = Vec::new();
        let past = d
            .get("radar")
            .and_then(|r| r.get("past"))
            .and_then(|p| p.as_array())
            .unwrap_or(&empty_vec);

        let frames: Vec<Value> = past
            .iter()
            .map(|f| {
                json!({
                    "time": f.get("time").and_then(|v| v.as_i64()).unwrap_or(0),
                    "path": f.get("path").and_then(|v| v.as_str()).unwrap_or(""),
                })
            })
            .collect();

        let result = json!({
            "ok": true,
            "generated": generated,
            "host": host,
            "frames": frames,
            "at": now,
        });

        let mut cache = self.radar_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn fires(&self) -> Value {
        let now = now_ms();
        if self.firms_key.is_empty() {
            return json!({
                "ok": false,
                "err": "NASA_FIRMS_API_KEY belum diset",
                "items": [],
                "at": now,
                "sources": []
            });
        }

        {
            let cache = self.fire_cache.read().await;
            if now - cache.at < 600_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "items": [] }));
            }
        }

        let sources = [
            ("VIIRS_SNPP_NRT", "VIIRS SNPP"),
            ("VIIRS_NOAA20_NRT", "VIIRS NOAA-20"),
            ("VIIRS_NOAA21_NRT", "VIIRS NOAA-21"),
            ("MODIS_NRT", "MODIS"),
        ];

        let mut out = Vec::new();
        let mut source_status = Vec::new();
        let mut preferred_host = "https://firms.modaps.eosdis.nasa.gov";

        for (id, label) in sources {
            let url = format!(
                "{preferred_host}/api/area/csv/{}/{id}/95,-11,141,6/1",
                self.firms_key
            );
            let resp = self.client.get(&url).send().await;
            let mut text = match resp {
                Ok(r) => r.text().await.unwrap_or_default(),
                Err(_) => String::new(),
            };

            if (text.is_empty() || text.contains("Invalid MAP_KEY"))
                && preferred_host == "https://firms.modaps.eosdis.nasa.gov"
            {
                let alt_host = "https://firms2.modaps.eosdis.nasa.gov";
                let alt_url = format!(
                    "{alt_host}/api/area/csv/{}/{id}/95,-11,141,6/1",
                    self.firms_key
                );
                if let Ok(r) = self.client.get(&alt_url).send().await
                    && let Ok(alt_text) = r.text().await
                    && !alt_text.contains("Invalid MAP_KEY")
                {
                    preferred_host = alt_host;
                    text = alt_text;
                }
            }

            if text.is_empty() || text.contains("Invalid MAP_KEY") {
                source_status
                    .push(json!({ "id": id, "ok": false, "err": "invalid key or request" }));
                continue;
            }

            let parsed = parse_csv(Value::String(text)).unwrap_or(json!([]));
            let items = parsed.as_array().cloned().unwrap_or_default();
            let count = items.len();
            source_status.push(json!({ "id": id, "ok": true, "count": count }));

            for mut it in items {
                if let Some(obj) = it.as_object_mut() {
                    obj.insert("source".to_string(), json!(id));
                    obj.insert("sensor".to_string(), json!(label));
                }
                out.push(it);
            }
        }

        out.sort_by(|a, b| {
            let ta = a.get("time").and_then(|v| v.as_i64()).unwrap_or(0);
            let tb = b.get("time").and_then(|v| v.as_i64()).unwrap_or(0);
            tb.cmp(&ta)
        });

        if out.len() > 3000 {
            out.truncate(3000);
        }

        let ok = source_status
            .iter()
            .any(|s| s.get("ok").and_then(|v| v.as_bool()).unwrap_or(false));
        let result = json!({
            "ok": ok,
            "items": out,
            "sources": source_status,
            "at": now,
        });

        let mut cache = self.fire_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn sipongi(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.sipongi_cache.read().await;
            if now - cache.at < 600_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "hotspots": [], "air": [] }));
            }
        }

        let sipongi = "https://opsroom.sipongidata.my.id";
        let hs_url = format!(
            "{sipongi}/api/opsroom/indoHotspot?wilayah=IN&filterperiode=false&from=&to=&late=24&satelit%5B%5D=NASA-MODIS&satelit%5B%5D=NASA-SNPP&satelit%5B%5D=NASA-NOAA20&satelit%5B%5D=NASA-NOAA21&satelit%5B%5D=LPN-MODIS&satelit%5B%5D=LPN-NPP&satelit%5B%5D=LPN-NOAA20&satelit%5B%5D=LPN-LANDSAT8&confidence%5B%5D=low&confidence%5B%5D=medium&confidence%5B%5D=high&provinsi=&kabkota="
        );
        let aq_url = format!("{sipongi}/api/aqms");

        let (hs_res, aq_res) = tokio::join!(
            self.client.get(&hs_url).send(),
            self.client.get(&aq_url).send()
        );

        let hs_val: Value = match hs_res {
            Ok(r) => r.json().await.unwrap_or(json!({ "features": [] })),
            Err(_) => json!({ "features": [] }),
        };
        let aq_val: Value = match aq_res {
            Ok(r) => r.json().await.unwrap_or(json!({ "features": [] })),
            Err(_) => json!({ "features": [] }),
        };

        let empty_vec = Vec::new();
        let hs_feats = hs_val
            .get("features")
            .and_then(|f| f.as_array())
            .unwrap_or(&empty_vec);
        let mut spots = Vec::new();
        for f in hs_feats {
            let coords = f
                .get("geometry")
                .and_then(|g| g.get("coordinates"))
                .and_then(|c| c.as_array());
            let lon = coords.and_then(|c| c.first()).and_then(|v| v.as_f64());
            let lat = coords.and_then(|c| c.get(1)).and_then(|v| v.as_f64());
            let (lat, lon) = match (lat, lon) {
                (Some(lat), Some(lon)) if lat.is_finite() && lon.is_finite() => (lat, lon),
                _ => continue,
            };
            let p = f.get("properties").unwrap_or(&Value::Null);
            let time_str = p
                .get("date_hotspot_ori")
                .and_then(|v| v.as_str())
                .unwrap_or("");
            let ts = chrono::DateTime::parse_from_rfc3339(time_str)
                .map(|dt| dt.timestamp_millis())
                .unwrap_or(0);

            spots.push(json!({
                "lat": lat,
                "lon": lon,
                "time": ts,
                "sumber": p.get("sumber").and_then(|v| v.as_str()).unwrap_or(""),
                "confidence": p.get("confidence_level").and_then(|v| v.as_str()).unwrap_or(""),
                "desa": p.get("desa").and_then(|v| v.as_str()).unwrap_or(""),
                "kecamatan": p.get("kecamatan").and_then(|v| v.as_str()).unwrap_or(""),
                "kabkota": p.get("kabkota").and_then(|v| v.as_str()).unwrap_or(""),
                "provinsi": p.get("nama_provinsi").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }

        let aq_feats = aq_val
            .get("features")
            .and_then(|f| f.as_array())
            .unwrap_or(&empty_vec);
        let mut air = Vec::new();
        for f in aq_feats {
            let coords = f
                .get("geometry")
                .and_then(|g| g.get("coordinates"))
                .and_then(|c| c.as_array());
            let lon = coords.and_then(|c| c.first()).and_then(|v| v.as_f64());
            let lat = coords.and_then(|c| c.get(1)).and_then(|v| v.as_f64());
            let (lat, lon) = match (lat, lon) {
                (Some(lat), Some(lon)) if lat.is_finite() && lon.is_finite() => (lat, lon),
                _ => continue,
            };
            let p = f.get("properties").unwrap_or(&Value::Null);
            let val_str = p
                .get("nilai")
                .map(|v| v.to_string())
                .or_else(|| p.get("val").map(|v| v.to_string()))
                .unwrap_or_default();

            air.push(json!({
                "lat": lat,
                "lon": lon,
                "kota": p.get("kota").and_then(|v| v.as_str()).unwrap_or(""),
                "provinsi": p.get("provinsi").and_then(|v| v.as_str()).unwrap_or(""),
                "alamat": p.get("alamat").and_then(|v| v.as_str()).unwrap_or(""),
                "nilai": val_str,
                "cat": p.get("cat").and_then(|v| v.as_str()).unwrap_or(""),
                "waktu": p.get("waktu").and_then(|v| v.as_str()).unwrap_or(""),
            }));
        }

        let ok = !spots.is_empty() || !air.is_empty();
        let result = json!({
            "ok": ok,
            "hotspots": spots,
            "air": air,
            "at": now,
        });

        let mut cache = self.sipongi_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn bmkg_model_run(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.model_run_cache.read().await;
            if now - cache.at < 300_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "himawari9": [] }));
            }
        }

        let resp = self
            .client
            .get("https://satellite.bmkg.go.id/api22/modelrun")
            .send()
            .await;
        let d: Value = match resp {
            Ok(r) => r.json().await.unwrap_or(json!({})),
            Err(e) => {
                let err_msg = e.to_string();
                return json!({
                    "ok": false,
                    "err": if err_msg.len() > 150 { &err_msg[..150] } else { &err_msg },
                    "himawari9": []
                });
            }
        };

        let result = json!({
            "ok": true,
            "himawari9": d.get("himawari9").unwrap_or(&json!([])),
            "at": now,
        });

        let mut cache = self.model_run_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn satellite_products(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.sat_cache.read().await;
            if now - cache.at < 600_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "products": [] }));
            }
        }

        let sat_products = [
            (
                "ir",
                "IR Enhanced",
                "tile",
                "EH",
                10,
                "observation",
                "https://inderaja.bmkg.go.id/IMAGE/HIMA/H08_EH_Indonesia.png",
            ),
            (
                "natural",
                "Natural Color (siang)",
                "tile",
                "NC",
                10,
                "observation",
                "https://inderaja.bmkg.go.id/IMAGE/HIMA/H08_NC_Indonesia.png",
            ),
            (
                "water",
                "Water Vapor Enhanced",
                "tile",
                "WV",
                10,
                "observation",
                "https://inderaja.bmkg.go.id/IMAGE/HIMA/H08_WE_Indonesia.png",
            ),
            (
                "rain",
                "Rainfall Potential",
                "tile",
                "RP",
                10,
                "derived",
                "https://inderaja.bmkg.go.id/IMAGE/HIMA/H08_RP_Indonesia.png",
            ),
        ];

        let mut products = Vec::new();
        for (id, name, layer_type, param, mins, data_type, url) in sat_products {
            let resp = self.client.head(url).send().await;
            let (status, last_mod, content_len, err) = match resp {
                Ok(r) => {
                    let s = r.status().as_u16();
                    let lm = r
                        .headers()
                        .get("last-modified")
                        .and_then(|v| v.to_str().ok())
                        .map(|s| s.to_string());
                    let cl = r
                        .headers()
                        .get("content-length")
                        .and_then(|v| v.to_str().ok())
                        .and_then(|s| s.parse::<i64>().ok());
                    (s, lm, cl, None)
                }
                Err(e) => (0, None, None, Some(e.to_string())),
            };

            products.push(json!({
                "id": id,
                "name": name,
                "layerType": layer_type,
                "param": param,
                "updateMinutes": mins,
                "dataType": data_type,
                "source": "BMKG Himawari-9",
                "url": url,
                "status": status,
                "lastModified": last_mod,
                "contentLength": content_len,
                "error": err,
            }));
        }

        let ok = products
            .iter()
            .any(|p| p.get("status").and_then(|v| v.as_i64()).unwrap_or(0) == 200);
        let result = json!({
            "ok": ok,
            "products": products,
            "attribution": "BMKG · Himawari-9",
            "at": now,
        });

        let mut cache = self.sat_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn wind_grid(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.wind_cache.read().await;
            if now - cache.at < 900_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "data": null }));
            }
        }

        let mut grid = Vec::new();
        for la in WIND_LATS_DESC {
            for lo in WIND_LONS {
                grid.push([la, lo]);
            }
        }

        let lats: Vec<String> = grid.iter().map(|p| p[0].to_string()).collect();
        let lons: Vec<String> = grid.iter().map(|p| p[1].to_string()).collect();
        let url = format!(
            "https://api.open-meteo.com/v1/forecast?latitude={}&longitude={}&current=wind_speed_10m,wind_direction_10m&timezone=Asia%2FJakarta",
            lats.join(","),
            lons.join(",")
        );

        let resp = self.client.get(&url).send().await;
        let d: Value = match resp {
            Ok(r) => r.json().await.unwrap_or(json!([])),
            Err(e) => {
                let err_msg = e.to_string();
                return json!({
                    "ok": false,
                    "err": if err_msg.len() > 150 { &err_msg[..150] } else { &err_msg },
                    "data": null
                });
            }
        };

        let list: Vec<&Value> = if let Some(arr) = d.as_array() {
            arr.iter().collect()
        } else {
            vec![&d]
        };

        let mut cur_map = HashMap::new();
        for (i, item) in list.iter().enumerate() {
            if i < grid.len() {
                let key = format!("{},{}", grid[i][0], grid[i][1]);
                let cur = item.get("current").unwrap_or(&Value::Null);
                cur_map.insert(key, cur);
            }
        }

        let mut u_data = Vec::new();
        let mut v_data = Vec::new();
        for la in WIND_LATS_DESC {
            for lo in WIND_LONS {
                let key = format!("{la},{lo}");
                let cur = cur_map.get(&key).unwrap_or(&&Value::Null);
                let speed = cur
                    .get("wind_speed_10m")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    / 3.6;
                let dir = cur
                    .get("wind_direction_10m")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    * std::f64::consts::PI
                    / 180.0;
                u_data.push(-speed * dir.sin());
                v_data.push(-speed * dir.cos());
            }
        }

        let mk_header = |n: i64| {
            json!({
                "lo1": WIND_LONS[0],
                "la1": WIND_LATS_DESC[0],
                "dx": 3,
                "dy": 2,
                "nx": WIND_LONS.len(),
                "ny": WIND_LATS_DESC.len(),
                "refTime": now,
                "forecastTime": 0,
                "parameterCategory": 2,
                "parameterNumber": n,
                "parameterName": if n == 2 { "u-component_of_wind" } else { "v-component_of_wind" }
            })
        };

        let result = json!({
            "ok": true,
            "data": [
                { "header": mk_header(2), "data": u_data },
                { "header": mk_header(3), "data": v_data },
            ],
            "at": now,
        });

        let mut cache = self.wind_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn alerts(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.alert_cache.read().await;
            if now - cache.at < 300_000 && cache.data.is_some() {
                return cache
                    .data
                    .clone()
                    .unwrap_or(json!({ "ok": false, "items": [] }));
            }
        }

        let resp = self
            .client
            .get("https://www.bmkg.go.id/alerts/nowcast/id")
            .send()
            .await;
        let xml = match resp {
            Ok(r) => r.text().await.unwrap_or_default(),
            Err(e) => {
                let err_msg = e.to_string();
                return json!({
                    "ok": false,
                    "err": if err_msg.len() > 150 { &err_msg[..150] } else { &err_msg },
                    "items": []
                });
            }
        };

        let item_re = regex::Regex::new(r"(?is)<item>([\s\S]*?)</item>").expect("valid regex");
        let poly_re =
            regex::Regex::new(r"(?is)<polygon[^>]*>([\s\S]*?)</polygon>").expect("valid regex");
        let mut out = Vec::new();

        for cap in item_re.captures_iter(&xml) {
            let it = &cap[1];
            let title = parse_cap_field(
                Value::String(it.to_string()),
                Value::String("title".to_string()),
            )
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
            let link = parse_cap_field(
                Value::String(it.to_string()),
                Value::String("link".to_string()),
            )
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
            let desc = parse_cap_field(
                Value::String(it.to_string()),
                Value::String("description".to_string()),
            )
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();

            if link.is_empty() {
                continue;
            }

            if let Ok(detail_resp) = self.client.get(&link).send().await
                && let Ok(cx) = detail_resp.text().await
            {
                let cx_val = Value::String(cx.clone());
                let event = parse_cap_field(cx_val.clone(), Value::String("event".to_string()))
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .filter(|s| !s.is_empty())
                    .unwrap_or_else(|| title.clone());
                let headline =
                    parse_cap_field(cx_val.clone(), Value::String("headline".to_string()))
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| title.clone());
                let description =
                    parse_cap_field(cx_val.clone(), Value::String("description".to_string()))
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| desc.clone());
                let effective =
                    parse_cap_field(cx_val.clone(), Value::String("effective".to_string()))
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default();
                let expires = parse_cap_field(cx_val.clone(), Value::String("expires".to_string()))
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                let severity =
                    parse_cap_field(cx_val.clone(), Value::String("severity".to_string()))
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default();
                let urgency = parse_cap_field(cx_val.clone(), Value::String("urgency".to_string()))
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();
                let certainty =
                    parse_cap_field(cx_val.clone(), Value::String("certainty".to_string()))
                        .ok()
                        .and_then(|v| v.as_str().map(|s| s.to_string()))
                        .unwrap_or_default();
                let web = parse_cap_field(cx_val.clone(), Value::String("web".to_string()))
                    .ok()
                    .and_then(|v| v.as_str().map(|s| s.to_string()))
                    .unwrap_or_default();

                let poly_text = poly_re
                    .captures(&cx)
                    .map(|m| m[1].to_string())
                    .unwrap_or_default();
                let polygon = parse_polygon(Value::String(poly_text)).unwrap_or(json!([]));

                out.push(json!({
                    "event": event,
                    "headline": headline,
                    "description": description,
                    "effective": effective,
                    "expires": expires,
                    "severity": severity,
                    "urgency": urgency,
                    "certainty": certainty,
                    "web": web,
                    "polygon": polygon,
                }));
            }
        }

        let result = json!({
            "ok": true,
            "items": out,
            "at": now,
        });

        let mut cache = self.alert_cache.write().await;
        cache.at = now;
        cache.data = Some(result.clone());
        result
    }

    pub async fn tj_auth(&self) -> Result<String, String> {
        let now = now_ms();
        {
            let guard = self.tj_token.read().await;
            if let Some((tok, at)) = &*guard
                && now - *at < 20 * 3600 * 1000
            {
                return Ok(tok.clone());
            }
        }

        let tj_api = "https://tijeapi.transjakarta.co.id";
        let resp = self
            .client
            .post(format!("{tj_api}/v1/auth/login/guest"))
            .header("Content-Type", "application/json")
            .header("User-Agent", "okhttp/4.12.0")
            .header("X-App-OS", "android")
            .header("X-App-Version", "2.10.2")
            .header("X-Device-ID", "sitrep-console")
            .body("{}")
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let d: Value = resp.json().await.map_err(|e| e.to_string())?;
        let token = d
            .get("token")
            .and_then(|v| v.as_str())
            .ok_or_else(|| "tj guest login gagal".to_string())?
            .to_string();

        let mut guard = self.tj_token.write().await;
        *guard = Some((token.clone(), now));
        Ok(token)
    }

    pub async fn transit(&self) -> Value {
        let now = now_ms();
        {
            let cache = self.bus_cache.read().await;
            if now - cache.at < 25_000 && cache.data.is_some() {
                return cache.data.clone().unwrap_or(json!([]));
            }
        }

        let tok = match self.tj_auth().await {
            Ok(t) => t,
            Err(_) => {
                let mut cache = self.bus_cache.write().await;
                cache.at = now;
                return cache.data.clone().unwrap_or(json!([]));
            }
        };

        let tj_api = "https://tijeapi.transjakarta.co.id";
        let url = format!("{tj_api}/v1/bus?latitude=-6.1754&longitude=106.8272&radius=20");
        let resp = self
            .client
            .get(&url)
            .header("Content-Type", "application/json")
            .header("User-Agent", "okhttp/4.12.0")
            .header("X-App-OS", "android")
            .header("X-App-Version", "2.10.2")
            .header("X-Device-ID", "sitrep-console")
            .header("Authorization", format!("Bearer {tok}"))
            .send()
            .await;

        let d: Value = match resp {
            Ok(r) => r.json().await.unwrap_or(json!({})),
            Err(_) => {
                let mut cache = self.bus_cache.write().await;
                cache.at = now;
                return cache.data.clone().unwrap_or(json!([]));
            }
        };

        let empty_vec = Vec::new();
        let arr = d
            .get("data")
            .and_then(|v| v.as_array())
            .or_else(|| {
                d.get("data")
                    .and_then(|v| v.get("buses"))
                    .and_then(|b| b.as_array())
            })
            .unwrap_or(&empty_vec);

        let mut items = Vec::new();
        for b in arr {
            let lat = b.get("latitude").and_then(|v| v.as_f64());
            let lon = b.get("longitude").and_then(|v| v.as_f64());
            let (lat, lon) = match (lat, lon) {
                (Some(lat), Some(lon)) if lat.is_finite() && lon.is_finite() => (lat, lon),
                _ => continue,
            };

            let id = b
                .get("bus_body_no")
                .and_then(|v| v.as_str())
                .or_else(|| b.get("trip_id").and_then(|v| v.as_str()))
                .unwrap_or("?");
            let route = b.get("route_code").and_then(|v| v.as_str()).unwrap_or("");
            let name = b
                .get("route_name")
                .and_then(|v| v.as_str())
                .or_else(|| b.get("route_long_name").and_then(|v| v.as_str()))
                .unwrap_or("");
            let dest = b
                .get("trip_headsign")
                .and_then(|v| v.as_str())
                .or_else(|| b.get("direction").and_then(|v| v.as_str()))
                .unwrap_or("");
            let speed = b.get("speed").and_then(|v| v.as_f64());
            let bearing = b
                .get("bearing")
                .and_then(|v| v.as_f64())
                .map(|b| b.round() as i64);

            items.push(json!({
                "id": id,
                "route": route,
                "name": name,
                "dest": dest,
                "speed": speed,
                "bearing": bearing,
                "lat": lat,
                "lon": lon,
            }));
        }

        if items.len() > 2500 {
            items.truncate(2500);
        }

        let res = Value::Array(items);
        let mut cache = self.bus_cache.write().await;
        cache.at = now;
        if let Value::Array(a) = &res
            && !a.is_empty()
        {
            cache.data = Some(res.clone());
        }
        res
    }
}
