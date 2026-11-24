//! Complete async `ExaSummaryService` with Tokio / Reqwest.

#![forbid(unsafe_code)]
#![allow(clippy::collapsible_if, clippy::too_many_arguments)]

use crate::port::{
    build_fallback_summary, extract_key_entities_inner, is_google_news_host,
    is_valid_public_http_url, parse_exa_summary_object,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, broadcast};

pub const EXA_CONTENTS_URL: &str = "https://api.exa.ai/contents";
pub const EXA_ANSWER_URL: &str = "https://api.exa.ai/answer";

pub const SUMMARY_QUERY: &str = "Ringkas kejadian perpajakan atau keuangan negara Indonesia ini dalam Bahasa Indonesia baku untuk pimpinan Direktorat Jenderal Pajak (DJP): (1) ringkasan 2-3 kalimat padat atas fakta kejadian; (2) dampak_djp terhadap penerimaan pajak, APBN, Coretax, kepatuhan wajib pajak, atau reputasi DJP; (3) entitas_kunci maksimal 6 item berupa instansi, regulasi/PMK, atau angka penting (Rp/persen); (4) atensi singkat 1 kalimat rekomendasi pemantauan atau respons komunikasi.";

pub fn summary_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "ringkasan": { "type": "string" },
            "dampak_djp": { "type": "string" },
            "entitas_kunci": { "type": "array", "items": { "type": "string" } },
            "atensi": { "type": "string" },
        },
        "required": ["ringkasan", "dampak_djp", "entitas_kunci", "atensi"],
    })
}

pub fn now_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

pub fn wib_date_key(now: i64) -> String {
    let wib_ms = now + 7 * 3600 * 1000;
    let secs = wib_ms.div_euclid(1000);
    let nsecs = (wib_ms.rem_euclid(1000) * 1_000_000) as u32;
    DateTime::from_timestamp(secs, nsecs)
        .unwrap_or_else(|| DateTime::<Utc>::from(UNIX_EPOCH))
        .format("%Y-%m-%d")
        .to_string()
}

pub fn hash_key(s: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(s.as_bytes());
    let out = hasher.finalize();
    let mut hex = String::with_capacity(40);
    for b in out {
        use std::fmt::Write;
        let _ = write!(hex, "{b:02x}");
    }
    hex.chars().take(20).collect()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheItem {
    pub at: i64,
    pub val: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatusResponse {
    pub ok: bool,
    pub configured: bool,
    #[serde(rename = "dailyUsed")]
    pub daily_used: u32,
    #[serde(rename = "dailyCap")]
    pub daily_cap: u32,
    #[serde(rename = "dailyRemaining")]
    pub daily_remaining: u32,
    #[serde(rename = "dailyDate")]
    pub daily_date: String,
    #[serde(rename = "cacheEntries")]
    pub cache_entries: usize,
    #[serde(rename = "cooldownUntil")]
    pub cooldown_until: i64,
}

#[derive(Debug, Clone)]
pub struct ServiceConfig {
    pub hist_dir: PathBuf,
    pub api_key: String,
    pub daily_cap: u32,
    pub ttl_ms: i64,
    pub brief_ttl_ms: i64,
    pub persist: bool,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        let api_key = std::env::var("EXA_API_KEY")
            .unwrap_or_default()
            .trim()
            .to_string();
        let daily_cap = std::env::var("EXA_DAILY_CAP")
            .ok()
            .and_then(|v| v.parse::<u32>().ok())
            .unwrap_or(150)
            .max(1);
        let ttl_ms = std::env::var("EXA_CACHE_TTL_MS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(12 * 3600 * 1000)
            .max(60_000);
        let brief_ttl_ms = std::env::var("EXA_BRIEF_TTL_MS")
            .ok()
            .and_then(|v| v.parse::<i64>().ok())
            .unwrap_or(30 * 60 * 1000)
            .max(60_000);

        Self {
            hist_dir: PathBuf::new(),
            api_key,
            daily_cap,
            ttl_ms,
            brief_ttl_ms,
            persist: true,
        }
    }
}

struct State {
    mem_cache: HashMap<String, CacheItem>,
    cache_order: Vec<String>,
    daily_date: String,
    daily_used: u32,
    cooldown_until: i64,
}

#[derive(Clone)]
pub struct ExaSummaryService {
    config: ServiceConfig,
    state: Arc<Mutex<State>>,
    client: reqwest::Client,
    in_flight: Arc<Mutex<HashMap<String, broadcast::Sender<Value>>>>,
}

impl ExaSummaryService {
    pub fn new(config: ServiceConfig) -> Self {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(25))
            .build()
            .unwrap_or_default();

        let initial_date = wib_date_key(now_millis());
        let mut state = State {
            mem_cache: HashMap::new(),
            cache_order: Vec::new(),
            daily_date: initial_date,
            daily_used: 0,
            cooldown_until: 0,
        };

        if config.persist && !config.hist_dir.as_os_str().is_empty() {
            Self::load_disk_cache(&config.hist_dir, config.ttl_ms, &mut state);
        }

        Self {
            config,
            state: Arc::new(Mutex::new(state)),
            client,
            in_flight: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    fn cache_file_path(hist_dir: &Path) -> PathBuf {
        hist_dir.join("exa-summary-cache.json")
    }

    fn load_disk_cache(hist_dir: &Path, ttl_ms: i64, state: &mut State) {
        let path = Self::cache_file_path(hist_dir);
        if !path.exists() {
            return;
        }
        let Ok(data) = std::fs::read_to_string(&path) else {
            return;
        };
        let Ok(parsed) = serde_json::from_str::<Value>(&data) else {
            return;
        };

        let now = now_millis();
        let today = wib_date_key(now);
        if let Some(date_str) = parsed.get("dailyDate").and_then(|v| v.as_str()) {
            if date_str == today {
                state.daily_date = date_str.to_string();
                if let Some(used) = parsed.get("dailyUsed").and_then(|v| v.as_u64()) {
                    state.daily_used = used as u32;
                }
            }
        }

        if let Some(entries) = parsed.get("entries").and_then(|v| v.as_array()) {
            for item in entries {
                let Some(arr) = item.as_array() else { continue };
                if arr.len() < 2 {
                    continue;
                }
                let Some(k) = arr[0].as_str() else { continue };
                let val_obj = &arr[1];
                let at = val_obj.get("at").and_then(|v| v.as_i64()).unwrap_or(0);
                if at > 0 && (now - at) < ttl_ms {
                    if let Some(val) = val_obj.get("val") {
                        state.mem_cache.insert(
                            k.to_string(),
                            CacheItem {
                                at,
                                val: val.clone(),
                            },
                        );
                        state.cache_order.push(k.to_string());
                    }
                }
            }
        }
    }

    async fn schedule_save(&self) {
        if !self.config.persist || self.config.hist_dir.as_os_str().is_empty() {
            return;
        }
        let (daily_date, daily_used, entries) = {
            let state = self.state.lock().await;
            let mut entries_json = Vec::new();
            let start = if state.cache_order.len() > 400 {
                state.cache_order.len() - 400
            } else {
                0
            };
            for k in &state.cache_order[start..] {
                if let Some(item) = state.mem_cache.get(k) {
                    entries_json.push(json!([k, item]));
                }
            }
            (state.daily_date.clone(), state.daily_used, entries_json)
        };

        let cache_path = Self::cache_file_path(&self.config.hist_dir);
        let tmp_path = format!("{}.tmp", cache_path.to_string_lossy());
        let payload = json!({
            "dailyDate": daily_date,
            "dailyUsed": daily_used,
            "entries": entries,
        });

        let _ = std::fs::create_dir_all(&self.config.hist_dir);
        if let Ok(content) = serde_json::to_string(&payload) {
            if std::fs::write(&tmp_path, content).is_ok() {
                let _ = std::fs::rename(&tmp_path, cache_path);
            }
        }
    }

    async fn cache_get(&self, key: &str, custom_ttl: i64, now: i64) -> Option<Value> {
        let mut state = self.state.lock().await;
        let item = state.mem_cache.get(key)?.clone();
        if now - item.at > custom_ttl {
            state.mem_cache.remove(key);
            state.cache_order.retain(|k| k != key);
            return None;
        }
        state.cache_order.retain(|k| k != key);
        state.cache_order.push(key.to_string());
        Some(item.val)
    }

    async fn cache_set(&self, key: &str, val: Value, now: i64) {
        {
            let mut state = self.state.lock().await;
            state
                .mem_cache
                .insert(key.to_string(), CacheItem { at: now, val });
            state.cache_order.retain(|k| k != key);
            state.cache_order.push(key.to_string());
            while state.cache_order.len() > 400 {
                let oldest = state.cache_order.remove(0);
                state.mem_cache.remove(&oldest);
            }
        }
        self.schedule_save().await;
    }

    fn reset_daily_if_needed(state: &mut State, now: i64) {
        let today = wib_date_key(now);
        if today != state.daily_date {
            state.daily_date = today;
            state.daily_used = 0;
        }
    }

    pub async fn status(&self) -> StatusResponse {
        let now = now_millis();
        let mut state = self.state.lock().await;
        Self::reset_daily_if_needed(&mut state, now);

        let daily_remaining = self.config.daily_cap.saturating_sub(state.daily_used);
        let cooldown_until = if state.cooldown_until > now {
            state.cooldown_until
        } else {
            0
        };

        StatusResponse {
            ok: true,
            configured: !self.config.api_key.is_empty(),
            daily_used: state.daily_used,
            daily_cap: self.config.daily_cap,
            daily_remaining,
            daily_date: state.daily_date.clone(),
            cache_entries: state.mem_cache.len(),
            cooldown_until,
        }
    }

    fn pick_direct_urls(&self, urls: &[String]) -> Vec<String> {
        let mut valid: Vec<String> = Vec::new();
        for raw in urls {
            let s = raw.trim();
            if let Ok(v) = is_valid_public_http_url(Value::String(s.to_string())) {
                if v.as_bool().unwrap_or(false) && !valid.contains(&s.to_string()) {
                    valid.push(s.to_string());
                }
            }
        }
        valid.sort_by(|a, b| {
            let ga = url::Url::parse(a)
                .ok()
                .and_then(|u| u.host_str().map(is_google_news_host))
                .unwrap_or(false);
            let gb = url::Url::parse(b)
                .ok()
                .and_then(|u| u.host_str().map(is_google_news_host))
                .unwrap_or(false);
            ga.cmp(&gb)
        });
        valid.truncate(2);
        valid
    }

    pub async fn summarize(
        &self,
        url: &str,
        title: &str,
        desc: &str,
        force: bool,
        theme_label: &str,
        now_ms: Option<i64>,
    ) -> Result<Value, String> {
        let urls = if !url.is_empty() {
            vec![url.to_string()]
        } else {
            Vec::new()
        };
        self.summarize_advanced(&urls, title, desc, force, theme_label, "", now_ms)
            .await
    }

    pub async fn summarize_advanced(
        &self,
        urls: &[String],
        title: &str,
        desc: &str,
        force: bool,
        theme_label: &str,
        cluster_id: &str,
        now_ms: Option<i64>,
    ) -> Result<Value, String> {
        let now = now_ms.unwrap_or_else(now_millis);
        let clean_urls = self.pick_direct_urls(urls);
        let key_target = if !clean_urls.is_empty() {
            clean_urls.join("|")
        } else {
            title.to_string()
        };
        let cluster_prefix = if !cluster_id.is_empty() {
            format!("{cluster_id}|")
        } else {
            String::new()
        };
        let cache_key = format!("sum:{}", hash_key(&format!("{cluster_prefix}{key_target}")));

        let api_configured = !self.config.api_key.is_empty();

        if !force {
            if let Some(mut cached) = self.cache_get(&cache_key, self.config.ttl_ms, now).await {
                if let Some(obj) = cached.as_object_mut() {
                    obj.insert("cached".to_string(), Value::Bool(true));
                    obj.insert("exaConfigured".to_string(), Value::Bool(api_configured));
                }
                return Ok(cached);
            }
        }

        // In-flight deduplication
        let mut rx = {
            let mut inflight = self.in_flight.lock().await;
            if let Some(tx) = inflight.get(&cache_key) {
                Some(tx.subscribe())
            } else {
                let (tx, _rx) = broadcast::channel(1);
                inflight.insert(cache_key.clone(), tx);
                None
            }
        };

        if let Some(ref mut subscriber) = rx {
            if let Ok(mut shared) = subscriber.recv().await {
                if let Some(obj) = shared.as_object_mut() {
                    obj.insert("cached".to_string(), Value::Bool(true));
                    obj.insert("exaConfigured".to_string(), Value::Bool(api_configured));
                }
                return Ok(shared);
            }
        }

        let run_res = self
            .run_summarize(&clean_urls, title, desc, theme_label, now)
            .await;

        let res = match run_res {
            Ok(val) => {
                if val.get("mode").and_then(|v| v.as_str()) == Some("exa") {
                    self.cache_set(&cache_key, val.clone(), now).await;
                }
                val
            }
            Err(err_msg) => {
                let fb = build_fallback_summary(json!({
                    "title": title,
                    "desc": desc,
                    "themeLabel": theme_label,
                    "urls": clean_urls,
                    "reason": err_msg,
                }))?;
                let mut out = fb;
                if let Some(obj) = out.as_object_mut() {
                    obj.insert("cached".to_string(), Value::Bool(false));
                    obj.insert("exaConfigured".to_string(), Value::Bool(api_configured));
                    obj.insert("at".to_string(), json!(now));
                }
                out
            }
        };

        let mut inflight = self.in_flight.lock().await;
        if let Some(tx) = inflight.remove(&cache_key) {
            let _ = tx.send(res.clone());
        }

        Ok(res)
    }

    async fn run_summarize(
        &self,
        clean_urls: &[String],
        title: &str,
        desc: &str,
        theme_label: &str,
        now: i64,
    ) -> Result<Value, String> {
        let direct_only: Vec<String> = clean_urls
            .iter()
            .filter(|u| {
                !url::Url::parse(u)
                    .ok()
                    .and_then(|url| url.host_str().map(is_google_news_host))
                    .unwrap_or(false)
            })
            .cloned()
            .collect();

        let target_urls: Vec<String> = if !direct_only.is_empty() {
            direct_only.into_iter().take(2).collect()
        } else {
            clean_urls.iter().take(1).cloned().collect()
        };

        if target_urls.is_empty() || self.config.api_key.is_empty() {
            let reason = if self.config.api_key.is_empty() {
                "no_exa_key"
            } else {
                "no_public_url"
            };
            let mut out = build_fallback_summary(json!({
                "title": title,
                "desc": desc,
                "themeLabel": theme_label,
                "urls": clean_urls,
                "reason": reason,
            }))?;
            if let Some(obj) = out.as_object_mut() {
                obj.insert("cached".to_string(), Value::Bool(false));
                obj.insert(
                    "exaConfigured".to_string(),
                    Value::Bool(!self.config.api_key.is_empty()),
                );
                obj.insert("at".to_string(), json!(now));
            }
            return Ok(out);
        }

        let resp_data = self
            .post_exa(
                EXA_CONTENTS_URL,
                json!({
                    "urls": target_urls,
                    "text": false,
                    "summary": {
                        "query": SUMMARY_QUERY,
                        "schema": summary_schema(),
                    },
                    "maxAgeHours": 24,
                    "livecrawlTimeout": 9000,
                }),
                now,
            )
            .await?;

        let results: Vec<&Value> = resp_data
            .get("results")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter(|r| r.get("summary").is_some()).collect())
            .unwrap_or_default();

        if results.is_empty() {
            let mut out = build_fallback_summary(json!({
                "title": title,
                "desc": desc,
                "themeLabel": theme_label,
                "urls": target_urls,
                "reason": "empty_exa_result",
            }))?;
            if let Some(obj) = out.as_object_mut() {
                obj.insert("cached".to_string(), Value::Bool(false));
                obj.insert("exaConfigured".to_string(), Value::Bool(true));
                obj.insert("at".to_string(), json!(now));
            }
            return Ok(out);
        }

        let first = results[0];
        let first_title = first.get("title").and_then(|v| v.as_str()).unwrap_or(title);
        let primary = parse_exa_summary_object(
            first.get("summary").cloned().unwrap_or(Value::Null),
            Value::String(first_title.to_string()),
            Value::String(desc.to_string()),
        )?;

        let mut entities: Vec<String> = primary
            .get("entitas_kunci")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|x| x.as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();

        let mut merged_summary = primary
            .get("ringkasan")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();

        if results.len() > 1 {
            let second = results[1];
            let second_title = second.get("title").and_then(|v| v.as_str()).unwrap_or("");
            if let Ok(sec_obj) = parse_exa_summary_object(
                second.get("summary").cloned().unwrap_or(Value::Null),
                Value::String(second_title.to_string()),
                Value::Null,
            ) {
                if let Some(sec_ents) = sec_obj.get("entitas_kunci").and_then(|v| v.as_array()) {
                    for e in sec_ents {
                        if let Some(s) = e.as_str() {
                            if !entities
                                .iter()
                                .any(|x| x.to_lowercase() == s.to_lowercase())
                                && entities.len() < 6
                            {
                                entities.push(s.to_string());
                            }
                        }
                    }
                }
                if let Some(sec_sum) = sec_obj.get("ringkasan").and_then(|v| v.as_str()) {
                    if !sec_sum.is_empty() && merged_summary.chars().count() < 280 {
                        merged_summary = format!("{merged_summary} {sec_sum}");
                    }
                }
            }
        }

        let sources: Vec<Value> = results
            .iter()
            .map(|r| {
                let u = r
                    .get("url")
                    .and_then(|v| v.as_str())
                    .unwrap_or_else(|| target_urls.first().map(|s| s.as_str()).unwrap_or_default());
                let t = r.get("title").and_then(|v| v.as_str()).unwrap_or(title);
                json!({ "url": u, "title": t })
            })
            .collect();

        Ok(json!({
            "ok": true,
            "mode": "exa",
            "ringkasan": merged_summary,
            "dampak_djp": primary.get("dampak_djp").unwrap_or(&Value::Null),
            "entitas_kunci": entities,
            "atensi": primary.get("atensi").unwrap_or(&Value::Null),
            "sources": sources,
            "cached": false,
            "exaConfigured": true,
            "at": now,
        }))
    }

    async fn post_exa(&self, endpoint: &str, body: Value, now: i64) -> Result<Value, String> {
        if self.config.api_key.is_empty() {
            return Err("NO_KEY".to_string());
        }

        {
            let mut state = self.state.lock().await;
            Self::reset_daily_if_needed(&mut state, now);
            if now < state.cooldown_until {
                return Err("COOLDOWN".to_string());
            }
            if state.daily_used >= self.config.daily_cap {
                return Err("DAILY_CAP".to_string());
            }
        }

        let resp = self
            .client
            .post(endpoint)
            .header("x-api-key", &self.config.api_key)
            .header("content-type", "application/json")
            .header("accept", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| e.to_string())?;

        let status = resp.status();
        if !status.is_success() {
            let status_code = status.as_u16();
            if status_code == 402 || status_code == 429 {
                let mut state = self.state.lock().await;
                state.cooldown_until = now + 15 * 60 * 1000;
            }
            let txt = resp.text().await.unwrap_or_default();
            let snippet = if txt.len() > 140 { &txt[..140] } else { &txt };
            return Err(format!("Exa HTTP {status_code}: {snippet}"));
        }

        {
            let mut state = self.state.lock().await;
            state.daily_used += 1;
        }
        self.schedule_save().await;

        resp.json::<Value>().await.map_err(|e| e.to_string())
    }

    pub async fn brief(&self, topic: &str, now_ms: Option<i64>) -> Result<Value, String> {
        let clusters = vec![json!({
            "title": topic,
            "desc": topic,
            "themeLabel": "Perpajakan",
        })];
        self.executive_brief(&clusters, "24h", "nasional", "", now_ms)
            .await
    }

    pub async fn executive_brief(
        &self,
        clusters: &[Value],
        win: &str,
        scope: &str,
        theme: &str,
        now_ms: Option<i64>,
    ) -> Result<Value, String> {
        let now = now_ms.unwrap_or_else(now_millis);
        let top_clusters: Vec<&Value> = clusters.iter().take(5).collect();
        let sig = top_clusters
            .iter()
            .map(|c| {
                c.get("id")
                    .and_then(|v| v.as_str())
                    .or_else(|| c.get("title").and_then(|v| v.as_str()))
                    .unwrap_or_default()
            })
            .collect::<Vec<_>>()
            .join("|");

        let cache_key = format!(
            "brief:{}",
            hash_key(&format!("{win}|{scope}|{theme}|{sig}"))
        );
        let api_configured = !self.config.api_key.is_empty();

        if let Some(mut cached) = self
            .cache_get(&cache_key, self.config.brief_ttl_ms, now)
            .await
        {
            if let Some(obj) = cached.as_object_mut() {
                obj.insert("cached".to_string(), Value::Bool(true));
                obj.insert("exaConfigured".to_string(), Value::Bool(api_configured));
            }
            return Ok(cached);
        }

        // In-flight
        let mut rx = {
            let mut inflight = self.in_flight.lock().await;
            if let Some(tx) = inflight.get(&cache_key) {
                Some(tx.subscribe())
            } else {
                let (tx, _rx) = broadcast::channel(1);
                inflight.insert(cache_key.clone(), tx);
                None
            }
        };

        if let Some(ref mut subscriber) = rx {
            if let Ok(mut shared) = subscriber.recv().await {
                if let Some(obj) = shared.as_object_mut() {
                    obj.insert("cached".to_string(), Value::Bool(true));
                    obj.insert("exaConfigured".to_string(), Value::Bool(api_configured));
                }
                return Ok(shared);
            }
        }

        let build_local_brief = |reason: &str| -> Value {
            let headline = format!(
                "Briefing Lanskap {} ({})",
                if scope == "daerah" {
                    "Pajak Daerah"
                } else {
                    "Pajak Nasional (DJP) & Keuangan Negara"
                },
                win.to_uppercase()
            );

            let highlights: Vec<Value> = top_clusters
                .iter()
                .take(4)
                .map(|c| {
                    let title = c.get("title").and_then(|v| v.as_str()).unwrap_or("");
                    let desc = c.get("desc").and_then(|v| v.as_str()).unwrap_or(title);
                    let theme = c
                        .get("themeLabel")
                        .and_then(|v| v.as_str())
                        .unwrap_or("Perpajakan");
                    let items = c.get("totalItems").and_then(|v| v.as_i64()).unwrap_or(1);
                    let sources = c.get("sourceCount").and_then(|v| v.as_i64()).unwrap_or(1);
                    let url = c.get("url").and_then(|v| v.as_str()).unwrap_or("");
                    let urgency = c
                        .get("urgency")
                        .and_then(|v| v.as_str())
                        .unwrap_or("normal");

                    json!({
                        "title": title,
                        "ringkasan": desc,
                        "dampak_djp": format!("{theme} · {items} item ({sources} sumber)"),
                        "url": url,
                        "urgency": urgency,
                    })
                })
                .collect();

            let all_text = top_clusters
                .iter()
                .map(|c| {
                    format!(
                        "{} {}",
                        c.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                        c.get("desc").and_then(|v| v.as_str()).unwrap_or("")
                    )
                })
                .collect::<Vec<_>>()
                .join(" ");
            let entities = extract_key_entities_inner(&all_text);

            json!({
                "ok": true,
                "mode": "fallback",
                "fallbackReason": reason,
                "headline": headline,
                "highlights": highlights,
                "entitas_kunci": entities,
                "cached": false,
                "exaConfigured": api_configured,
                "at": now,
            })
        };

        if top_clusters.is_empty() {
            let res = build_local_brief("no_clusters");
            let mut inflight = self.in_flight.lock().await;
            if let Some(tx) = inflight.remove(&cache_key) {
                let _ = tx.send(res.clone());
            }
            return Ok(res);
        }

        if !api_configured {
            let res = build_local_brief("no_exa_key");
            let mut inflight = self.in_flight.lock().await;
            if let Some(tx) = inflight.remove(&cache_key) {
                let _ = tx.send(res.clone());
            }
            return Ok(res);
        }

        let mut direct_urls: Vec<String> = Vec::new();
        for c in &top_clusters {
            let mut cand_urls = Vec::new();
            if let Some(u) = c.get("url").and_then(|v| v.as_str()) {
                cand_urls.push(u.to_string());
            }
            if let Some(items) = c.get("newsItems").and_then(|v| v.as_array()) {
                for item in items {
                    if let Some(u) = item.get("url").and_then(|v| v.as_str()) {
                        cand_urls.push(u.to_string());
                    }
                }
            }
            let picked = self.pick_direct_urls(&cand_urls);
            if let Some(u) = picked.into_iter().find(|u| {
                !url::Url::parse(u)
                    .ok()
                    .and_then(|url| url.host_str().map(is_google_news_host))
                    .unwrap_or(false)
                    && !direct_urls.contains(u)
            }) {
                direct_urls.push(u);
                if direct_urls.len() >= 3 {
                    break;
                }
            }
        }

        let run_brief = async {
            if !direct_urls.is_empty() {
                if let Ok(data) = self
                    .post_exa(
                        EXA_CONTENTS_URL,
                        json!({
                            "urls": direct_urls,
                            "text": false,
                            "summary": {
                                "query": SUMMARY_QUERY,
                                "schema": summary_schema(),
                            },
                            "maxAgeHours": 24,
                            "livecrawlTimeout": 9000,
                        }),
                        now,
                    )
                    .await
                {
                    let results: Vec<&Value> = data
                        .get("results")
                        .and_then(|v| v.as_array())
                        .map(|arr| arr.iter().filter(|r| r.get("summary").is_some()).collect())
                        .unwrap_or_default();

                    if !results.is_empty() {
                        let mut highlights = Vec::new();
                        let mut all_ents: Vec<String> = Vec::new();
                        for r in results {
                            let parsed = parse_exa_summary_object(
                                r.get("summary").cloned().unwrap_or(Value::Null),
                                Value::String(
                                    r.get("title")
                                        .and_then(|v| v.as_str())
                                        .unwrap_or("")
                                        .to_string(),
                                ),
                                Value::Null,
                            )
                            .unwrap_or_default();
                            highlights.push(json!({
                                "title": r.get("title").and_then(|v| v.as_str()).unwrap_or(""),
                                "ringkasan": parsed.get("ringkasan").unwrap_or(&Value::Null),
                                "dampak_djp": parsed.get("dampak_djp").unwrap_or(&Value::Null),
                                "atensi": parsed.get("atensi").unwrap_or(&Value::Null),
                                "url": r.get("url").and_then(|v| v.as_str()).unwrap_or(""),
                            }));
                            if let Some(ents) =
                                parsed.get("entitas_kunci").and_then(|v| v.as_array())
                            {
                                for e in ents {
                                    if let Some(s) = e.as_str() {
                                        if !all_ents
                                            .iter()
                                            .any(|x| x.to_lowercase() == s.to_lowercase())
                                            && all_ents.len() < 8
                                        {
                                            all_ents.push(s.to_string());
                                        }
                                    }
                                }
                            }
                        }

                        let headline = format!(
                            "Ringkasan Eksekutif Exa AI · {} ({})",
                            if scope == "daerah" {
                                "Pajak Daerah"
                            } else {
                                "Pajak Nasional (DJP) & Keuangan Negara"
                            },
                            win.to_uppercase()
                        );

                        let out = json!({
                            "ok": true,
                            "mode": "exa",
                            "headline": headline,
                            "highlights": highlights,
                            "entitas_kunci": all_ents,
                            "cached": false,
                            "exaConfigured": true,
                            "at": now,
                        });
                        self.cache_set(&cache_key, out.clone(), now).await;
                        return out;
                    }
                }
            }

            // Exa Answer fallback
            let q_topics = top_clusters
                .iter()
                .take(3)
                .filter_map(|c| c.get("title").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("; ");

            let ans = self
                .post_exa(
                    EXA_ANSWER_URL,
                    json!({
                        "query": format!("Buat ringkasan eksekutif singkat (Bahasa Indonesia baku) untuk Direktorat Jenderal Pajak atas perkembangan berita perpajakan & keuangan negara Indonesia terkait: {q_topics}"),
                        "text": false,
                    }),
                    now,
                )
                .await;

            match ans {
                Ok(ans_data) => {
                    let answer_text = ans_data
                        .get("answer")
                        .and_then(|v| v.as_str())
                        .unwrap_or("")
                        .trim();
                    if !answer_text.is_empty() {
                        let citation_url = ans_data
                            .get("citations")
                            .and_then(|v| v.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|c| c.get("url"))
                            .and_then(|v| v.as_str())
                            .unwrap_or("");

                        let headline = format!(
                            "Ringkasan Eksekutif Exa AI · {} ({})",
                            if scope == "daerah" {
                                "Pajak Daerah"
                            } else {
                                "Pajak Nasional (DJP) & Keuangan Negara"
                            },
                            win.to_uppercase()
                        );

                        let out = json!({
                            "ok": true,
                            "mode": "exa",
                            "headline": headline,
                            "highlights": [{
                                "title": "Sintesis Lanskap Fiskal & Perpajakan",
                                "ringkasan": answer_text,
                                "dampak_djp": "Sintesis lintas sumber melalui Exa AI Answer.",
                                "url": citation_url,
                            }],
                            "entitas_kunci": extract_key_entities_inner(answer_text),
                            "cached": false,
                            "exaConfigured": true,
                            "at": now,
                        });
                        self.cache_set(&cache_key, out.clone(), now).await;
                        out
                    } else {
                        build_local_brief("empty_exa_brief")
                    }
                }
                Err(e) => build_local_brief(&e),
            }
        };

        let res = run_brief.await;

        let mut inflight = self.in_flight.lock().await;
        if let Some(tx) = inflight.remove(&cache_key) {
            let _ = tx.send(res.clone());
        }

        Ok(res)
    }
}
