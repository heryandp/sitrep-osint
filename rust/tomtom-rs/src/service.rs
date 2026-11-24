//! Async `TomTomService` with Tokio/Reqwest for traffic flow, incidents, routing, and tile caching.

#![forbid(unsafe_code)]

use base64::Engine;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::Digest;
use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

const TT_TILE_TTL_MS: u64 = 120_000;
const TT_TILE_CAP: usize = 300;
const TT_TILE_MAX_BYTES: usize = 16 * 1024 * 1024;
const TT_MEMO_TTL_MS: u64 = 30_000;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LatLon {
    pub lat: f64,
    pub lon: f64,
}

#[derive(Debug, Clone)]
pub struct TileResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub etag: String,
    pub not_modified: bool,
}

#[derive(Debug, Clone)]
pub(crate) struct TileCacheEntry {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub etag: String,
    pub at: u64,
}

#[derive(Default)]
pub(crate) struct TileCache {
    entries: HashMap<String, TileCacheEntry>,
    order: VecDeque<String>,
    total_bytes: usize,
}

impl TileCache {
    fn get(&mut self, key: &str, now: u64) -> Option<TileCacheEntry> {
        let entry = self.entries.get(key)?;
        if now.saturating_sub(entry.at) >= TT_TILE_TTL_MS {
            return None;
        }
        let cloned = entry.clone();
        if let Some(pos) = self.order.iter().position(|k| k == key) {
            let k = self.order.remove(pos).unwrap_or_default();
            self.order.push_back(k);
        }
        Some(cloned)
    }

    fn insert(&mut self, key: String, entry: TileCacheEntry) {
        if let Some(old) = self.entries.remove(&key) {
            self.total_bytes = self.total_bytes.saturating_sub(old.body.len());
            if let Some(pos) = self.order.iter().position(|k| k == &key) {
                self.order.remove(pos);
            }
        }
        self.total_bytes += entry.body.len();
        self.entries.insert(key.clone(), entry);
        self.order.push_back(key);
        while self.entries.len() > TT_TILE_CAP
            || (self.entries.len() > 1 && self.total_bytes > TT_TILE_MAX_BYTES)
        {
            let Some(k) = self.order.pop_front() else {
                break;
            };
            if let Some(ev) = self.entries.remove(&k) {
                self.total_bytes = self.total_bytes.saturating_sub(ev.body.len());
            }
        }
    }
}

pub struct TomTomService {
    api_key: Option<String>,
    client: Client,
    tile_cache: Arc<Mutex<TileCache>>,
    memo_cache: Arc<Mutex<HashMap<String, (Value, Instant)>>>,
}

fn urlenc(s: &str) -> String {
    url::form_urlencoded::byte_serialize(s.as_bytes()).collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_millis() as u64
}

fn rg_val(v: f64, step: f64) -> f64 {
    crate::js::math_round(v / step) * step
}

fn rg_str(v: f64, step: f64, d: usize) -> String {
    let r = rg_val(v, step);
    if r == 0.0 {
        format!("{:.prec$}", 0.0, prec = d)
    } else {
        format!("{r:.prec$}", prec = d)
    }
}

impl TomTomService {
    pub fn new(api_key: Option<String>) -> Self {
        Self {
            api_key: api_key.filter(|k| !k.trim().is_empty()),
            client: Client::builder()
                .timeout(Duration::from_secs(12))
                .build()
                .unwrap_or_else(|_| Client::new()),
            tile_cache: Arc::new(Mutex::new(TileCache::default())),
            memo_cache: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn tt_has(&self) -> bool {
        self.api_key.is_some()
    }

    pub async fn tile_bytes(&self) -> usize {
        self.tile_cache.lock().await.total_bytes
    }

    pub async fn tile_count(&self) -> usize {
        self.tile_cache.lock().await.entries.len()
    }

    async fn get_memo(&self, key: &str) -> Option<Value> {
        let mut guard = self.memo_cache.lock().await;
        if let Some((val, inserted)) = guard.get(key) {
            if (inserted.elapsed().as_millis() as u64) < TT_MEMO_TTL_MS {
                return Some(val.clone());
            }
            guard.remove(key);
        }
        None
    }

    async fn set_memo(&self, key: String, val: Value) {
        self.memo_cache
            .lock()
            .await
            .insert(key, (val, Instant::now()));
    }

    /// Traffic flow segment data.
    pub async fn flow(&self, lat: f64, lon: f64, zoom: Option<f64>) -> Value {
        let (g_lat, g_lon) = (rg_val(lat, 0.01), rg_val(lon, 0.01));
        let z = zoom.unwrap_or(10.0).round().clamp(0.0, 22.0) as i64;
        let memo_k = format!(
            "tt:flow:{},{},{}",
            rg_str(g_lat, 0.01, 2),
            rg_str(g_lon, 0.01, 2),
            z
        );
        if let Some(c) = self.get_memo(&memo_k).await {
            return c;
        }

        let Some(ref key) = self.api_key else {
            return json!({ "ok": false, "err": "TomTom API key belum dikonfigurasi" });
        };

        let url = format!(
            "https://api.tomtom.com/traffic/services/4/flowSegmentData/absolute/{z}/json?key={}&point={:.4},{:.4}&unit=kmph&thickness=10",
            urlenc(key),
            g_lat,
            g_lon
        );

        let res = match self.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => return json!({ "ok": false, "err": format!("HTTP error: {}", r.status()) }),
            Err(e) => {
                let s = e.to_string();
                return json!({ "ok": false, "err": &s[..s.len().min(220)] });
            }
        };

        let data = match res.json::<Value>().await {
            Ok(d) => d,
            Err(e) => {
                let s = e.to_string();
                return json!({ "ok": false, "err": &s[..s.len().min(220)] });
            }
        };

        let f = data.get("flowSegmentData").cloned().unwrap_or_default();
        let exists = f.get("coordinate").is_some() || f.get("frc").is_some();
        let out = json!({
            "ok": true, "point": { "lat": g_lat, "lon": g_lon }, "zoom": z,
            "frc": f.get("frc").and_then(|v| v.as_str()).unwrap_or(""),
            "exists": exists,
            "coordinates": f.get("coordinates").and_then(|c| c.get("coordinate")).cloned().unwrap_or(json!([])),
            "currentSpeed": f.get("currentSpeed").cloned().unwrap_or(Value::Null),
            "freeFlowSpeed": f.get("freeFlowSpeed").cloned().unwrap_or(Value::Null),
            "currentTravelTime": f.get("currentTravelTime").cloned().unwrap_or(Value::Null),
            "freeFlowTravelTime": f.get("freeFlowTravelTime").cloned().unwrap_or(Value::Null),
            "confidence": f.get("confidence").cloned().unwrap_or(Value::Null),
            "roadClosure": f.get("roadClosure").and_then(|b| b.as_bool()).unwrap_or(false),
            "openlr": f.get("openlr").and_then(|v| v.as_str()).unwrap_or(""),
            "at": now_ms()
        });
        self.set_memo(memo_k, out.clone()).await;
        out
    }

    /// Traffic incidents details.
    pub async fn incidents(&self, bbox: &str, language: Option<&str>) -> Value {
        let lang = language.unwrap_or("id-ID");
        let parts: Vec<f64> = bbox
            .split(',')
            .filter_map(|s| s.trim().parse::<f64>().ok())
            .collect();
        let norm_bbox = if parts.len() == 4 && parts.iter().all(|v| v.is_finite()) {
            parts
                .iter()
                .map(|v| rg_str(*v, 0.05, 2))
                .collect::<Vec<_>>()
                .join(",")
        } else {
            bbox.to_string()
        };

        let memo_k = format!("tt:inc:{norm_bbox}:{}", &lang[..lang.len().min(12)]);
        if let Some(c) = self.get_memo(&memo_k).await {
            return c;
        }

        let Some(ref key) = self.api_key else {
            return json!({ "ok": false, "bbox": norm_bbox, "err": "TomTom API key belum dikonfigurasi" });
        };

        let fields = "{incidents{type,geometry{type,coordinates},properties{id,iconCategory,magnitudeOfDelay,events{description,code,iconCategory},startTime,endTime,from,to,length,delay,roadNumbers,timeValidity,probabilityOfOccurrence,numberOfReports,lastReportTime}}}";
        let url = format!(
            "https://api.tomtom.com/traffic/services/5/incidentDetails?key={}&bbox={}&fields={}&language={}&timeValidityFilter=present",
            urlenc(key),
            urlenc(&norm_bbox),
            urlenc(fields),
            urlenc(lang)
        );

        let res = match self.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => {
                return json!({ "ok": false, "bbox": norm_bbox, "err": format!("HTTP error: {}", r.status()) });
            }
            Err(e) => {
                let s = e.to_string();
                return json!({ "ok": false, "bbox": norm_bbox, "err": &s[..s.len().min(260)] });
            }
        };

        let data = match res.json::<Value>().await {
            Ok(d) => d,
            Err(e) => {
                let s = e.to_string();
                return json!({ "ok": false, "bbox": norm_bbox, "err": &s[..s.len().min(260)] });
            }
        };

        let rows: Vec<Value> = data.get("incidents").and_then(|i| i.as_array()).cloned().unwrap_or_default().into_iter().map(|inc| {
            let p = inc.get("properties").cloned().unwrap_or_default();
            let geom = inc.get("geometry");
            let g_type = geom.and_then(|g| g.get("type")).and_then(|t| t.as_str()).unwrap_or("");
            let coords = geom.and_then(|g| g.get("coordinates"));
            let (mut lat, mut lon) = (Value::Null, Value::Null);
            match g_type {
                "Point" => if let Some(arr) = coords.and_then(|c| c.as_array()).filter(|a| a.len() >= 2) {
                    lon = arr[0].clone(); lat = arr[1].clone();
                },
                "LineString" => if let Some(first) = coords.and_then(|c| c.as_array()).and_then(|arr| arr.first()).and_then(|f| f.as_array()).filter(|a| a.len() >= 2) {
                    lon = first[0].clone(); lat = first[1].clone();
                },
                _ => {}
            }
            let events = p.get("events").and_then(|e| e.as_array());
            let desc = events.and_then(|evs| evs.first()).and_then(|e| e.get("description")).and_then(|d| d.as_str())
                .map(|s| s.to_string()).unwrap_or_else(|| {
                    events.map(|evs| evs.iter().filter_map(|e| e.get("description").and_then(|d| d.as_str())).collect::<Vec<_>>().join("; "))
                        .filter(|s| !s.is_empty()).unwrap_or_else(|| "Insiden lalu lintas".into())
                });
            json!({
                "id": p.get("id").and_then(|v| v.as_str()).unwrap_or(""),
                "iconCategory": p.get("iconCategory").cloned().unwrap_or(json!(0)),
                "magnitude": p.get("magnitudeOfDelay").cloned().unwrap_or(json!(0)),
                "desc": desc, "from": p.get("from").and_then(|v| v.as_str()).unwrap_or(""),
                "to": p.get("to").and_then(|v| v.as_str()).unwrap_or(""),
                "length": p.get("length").cloned().unwrap_or(Value::Null),
                "delay": p.get("delay").cloned().unwrap_or(Value::Null),
                "startTime": p.get("startTime").and_then(|v| v.as_str()).unwrap_or(""),
                "endTime": p.get("endTime").and_then(|v| v.as_str()).unwrap_or(""),
                "timeValidity": p.get("timeValidity").and_then(|v| v.as_str()).unwrap_or(""),
                "probability": p.get("probabilityOfOccurrence").and_then(|v| v.as_str()).unwrap_or(""),
                "reports": p.get("numberOfReports").cloned().unwrap_or(Value::Null),
                "lastReport": p.get("lastReportTime").and_then(|v| v.as_str()).unwrap_or(""),
                "geometry": inc.get("geometry").cloned().unwrap_or(Value::Null),
                "lat": lat, "lon": lon
            })
        }).collect();

        let out = json!({ "ok": true, "bbox": norm_bbox, "count": rows.len(), "incidents": rows, "at": now_ms() });
        self.set_memo(memo_k, out.clone()).await;
        out
    }

    /// Calculate route between two points.
    pub async fn route(&self, from: LatLon, to: LatLon, mode: Option<&str>) -> Value {
        let (f_lat, f_lon) = (rg_val(from.lat, 0.01), rg_val(from.lon, 0.01));
        let (t_lat, t_lon) = (rg_val(to.lat, 0.01), rg_val(to.lon, 0.01));
        let req_mode = mode.unwrap_or("driving");
        let iso_mode = match req_mode {
            "driving" | "truck" | "bus" | "taxi" | "motorcycle" | "bicycle" | "pedestrian" => {
                req_mode
            }
            _ => "driving",
        };
        let memo_k = format!("tt:route:{f_lat:.2},{f_lon:.2}>{t_lat:.2},{t_lon:.2}:{iso_mode}");
        if let Some(c) = self.get_memo(&memo_k).await {
            return c;
        }

        let Some(ref key) = self.api_key else {
            return json!({ "ok": false, "err": "TomTom API key belum dikonfigurasi" });
        };

        let url = format!(
            "https://api.tomtom.com/routing/1/calculateRoute/{f_lat:.4},{f_lon:.4}:{t_lat:.4},{t_lon:.4}/json?key={}&traffic=true&routeRepresentation=polyline",
            urlenc(key)
        );

        let res = match self.client.get(&url).send().await {
            Ok(r) if r.status().is_success() => r,
            Ok(r) => return json!({ "ok": false, "err": format!("HTTP error: {}", r.status()) }),
            Err(e) => {
                let s = e.to_string();
                return json!({ "ok": false, "err": &s[..s.len().min(280)] });
            }
        };

        let data = match res.json::<Value>().await {
            Ok(d) => d,
            Err(e) => {
                let s = e.to_string();
                return json!({ "ok": false, "err": &s[..s.len().min(280)] });
            }
        };

        let Some(route) = data.get("routes").and_then(|r| r.get(0)) else {
            return json!({ "ok": false, "err": "Rute tidak ditemukan" });
        };

        let s = route.get("summary").cloned().unwrap_or_default();
        let legs = route
            .get("legs")
            .and_then(|l| l.as_array())
            .cloned()
            .unwrap_or_default();
        let poly: Vec<Value> = legs.iter().flat_map(|l| l.get("points").and_then(|p| p.as_array()).cloned().unwrap_or_default()).map(|p| {
            json!({ "lat": p.get("latitude").cloned().unwrap_or(Value::Null), "lon": p.get("longitude").cloned().unwrap_or(Value::Null) })
        }).collect();

        let s_val = crate::port::haversine(
            json!({ "lat": from.lat, "lon": from.lon }),
            json!({ "lat": to.lat, "lon": to.lon }),
        )
        .ok()
        .and_then(|v| crate::js::as_number(&v))
        .unwrap_or(0.0);
        let straight_km = (s_val * 1000.0).round() / 1000.0;
        let distance_km = s
            .get("lengthInMeters")
            .and_then(|v| v.as_f64())
            .map(|m| (m / 1000.0 * 1000.0).round() / 1000.0);
        let travel_sec = s.get("travelTimeInSeconds").and_then(|v| v.as_i64());
        let delay_sec = s.get("trafficDelayInSeconds").and_then(|v| v.as_i64());
        let traffic_time_sec = match (travel_sec, delay_sec) {
            (Some(t), Some(d)) if t + d > 0 => Some(t + d),
            (Some(t), _) => Some(t),
            _ => None,
        };

        let mapped_legs: Vec<Value> = legs.iter().map(|l| {
            json!({
                "summary": l.get("summary").cloned().unwrap_or_default(),
                "points": l.get("points").and_then(|p| p.as_array()).map(|p| p.len()).unwrap_or(0)
            })
        }).collect();

        let out = json!({
            "ok": true, "mode": iso_mode,
            "from": { "lat": f_lat, "lon": f_lon }, "to": { "lat": t_lat, "lon": t_lon },
            "straightKm": straight_km, "distanceKm": distance_km,
            "travelTimeSec": travel_sec, "trafficTimeSec": traffic_time_sec, "trafficDelaySec": delay_sec,
            "departureTime": s.get("departureTime").and_then(|v| v.as_str()).unwrap_or(""),
            "arrivalTime": s.get("arrivalTime").and_then(|v| v.as_str()).unwrap_or(""),
            "congestion": s.get("trafficCongestion").and_then(|v| v.as_str()).unwrap_or(""),
            "legs": mapped_legs, "polyline": poly, "at": now_ms()
        });
        self.set_memo(memo_k, out.clone()).await;
        out
    }

    /// Traffic raster tile proxy with LRU cache and ETag matching.
    pub async fn tile(
        &self,
        kind: &str,
        z_raw: &str,
        x_raw: &str,
        y_raw: &str,
        if_none_match: Option<&str>,
    ) -> TileResponse {
        let err_bad = || TileResponse {
            status: 400,
            content_type: "text/plain".into(),
            body: b"koordinat tile tidak valid".to_vec(),
            etag: String::new(),
            not_modified: false,
        };
        let (Ok(z), Ok(x), Ok(y)) = (
            z_raw.parse::<i64>(),
            x_raw.parse::<i64>(),
            y_raw.parse::<i64>(),
        ) else {
            return err_bad();
        };
        if !(0..=18).contains(&z) {
            return err_bad();
        }
        let max_tile = (1i64 << z) - 1;
        if x < 0 || y < 0 || x > max_tile || y > max_tile {
            return err_bad();
        }

        let Some(ref key) = self.api_key else {
            return TileResponse {
                status: 503,
                content_type: "text/plain".into(),
                body: b"TomTom key belum dikonfigurasi".to_vec(),
                etag: String::new(),
                not_modified: false,
            };
        };

        let norm_kind = match kind {
            "incidents" | "incident" => "incidents",
            _ => "flow",
        };
        let cache_key = format!("{norm_kind}/{z}/{x}/{y}");
        let now = now_ms();

        {
            let mut guard = self.tile_cache.lock().await;
            if let Some(hit) = guard.get(&cache_key, now) {
                let not_modified = if_none_match == Some(&hit.etag);
                return TileResponse {
                    status: if not_modified { 304 } else { hit.status },
                    content_type: hit.content_type,
                    body: if not_modified { Vec::new() } else { hit.body },
                    etag: hit.etag,
                    not_modified,
                };
            }
        }

        let path = if norm_kind == "incidents" {
            format!("/traffic/map/4/tile/incidents/s0-dark/{z}/{x}/{y}.png")
        } else {
            format!("/traffic/map/4/tile/flow/absolute/{z}/{x}/{y}.png")
        };
        let subs = ['a', 'b', 'c', 'd'];
        let sub = subs[now as usize % subs.len()];
        let url = format!("https://{sub}.api.tomtom.com{path}?key={}", urlenc(key));

        let res = match self.client.get(&url).send().await {
            Ok(r) => r,
            Err(_) => {
                return TileResponse {
                    status: 502,
                    content_type: "text/plain".into(),
                    body: b"upstream error".to_vec(),
                    etag: String::new(),
                    not_modified: false,
                };
            }
        };

        let status = res.status().as_u16();
        let content_type = res
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .to_string();
        let body = match res.bytes().await {
            Ok(b) => b.to_vec(),
            Err(_) => {
                return TileResponse {
                    status: 502,
                    content_type: "text/plain".into(),
                    body: b"upstream error".to_vec(),
                    etag: String::new(),
                    not_modified: false,
                };
            }
        };

        if body.len() > 2 * 1024 * 1024 {
            return TileResponse {
                status: 413,
                content_type: "text/plain".into(),
                body: b"tile response too large".to_vec(),
                etag: String::new(),
                not_modified: false,
            };
        }

        let mut hasher = sha1::Sha1::new();
        hasher.update(&body);
        let b64 = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(hasher.finalize());
        let etag = format!(r#"W/"{}""#, &b64[..b64.len().min(14)]);

        let entry = TileCacheEntry {
            status,
            content_type: content_type.clone(),
            body: body.clone(),
            etag: etag.clone(),
            at: now,
        };
        if status == 200 && !body.is_empty() {
            self.tile_cache.lock().await.insert(cache_key, entry);
        }

        let not_modified = if_none_match == Some(&etag);
        TileResponse {
            status: if not_modified { 304 } else { status },
            content_type,
            body: if not_modified { Vec::new() } else { body },
            etag,
            not_modified,
        }
    }
}
