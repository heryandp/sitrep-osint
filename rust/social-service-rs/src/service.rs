//! Async `SocialService` for X GraphQL search, Threads scraping, feed caching, deduplication, and diagnostics.

use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, RwLock};
use tokio::process::Command;

pub const DEFAULT_BEARER: &str = "AAAAAAAAAAAAAAAAAAAAANRILgAAAAAAnNwIzUejRCOuH5E6I8xnZz4puTs%3D1Zv7ttfk8LF81IUq16cHjhLTvJu4FA33AGWWjCpTnA";
pub const CLASSIFIER_VERSION: &str = "2026-11-23.1";
pub const HIST_AGE_MS: i64 = 48 * 3600 * 1000;
pub const MAX_HIST_X: usize = 1500;
pub const MAX_HIST_TH: usize = 1200;

static ASTROTURF_URL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"https?://\S+").expect("valid regex"));
static ASTROTURF_MENTION_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"@[\w]+").expect("valid regex"));
static ASTROTURF_HASHTAG_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"#[\w]+").expect("valid regex"));
static ASTROTURF_PUNCT_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\w\s]").expect("valid regex"));
static MULTI_SPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("valid regex"));

static AUTH_ACCOUNTS: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    let mut s = HashSet::new();
    s.insert("ditjenpajakri");
    s.insert("kring_pajak");
    s.insert("kemenkeuri");
    s.insert("infobmkg");
    s.insert("bnpb_indonesia");
    s.insert("basarnas");
    s.insert("damkardki");
    s.insert("tmcpoldametro");
    s.insert("halo_polisi");
    s.insert("divhumas_polri");
    s.insert("puspentni");
    s.insert("kemkesri");
    s
});

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XAccount {
    pub id: usize,
    pub auth_token: String,
    pub ct0: String,
    pub cooldown_until: i64,
    pub disabled_until: i64,
}

impl XAccount {
    pub fn is_healthy(&self, now_ms: i64) -> bool {
        now_ms >= self.cooldown_until && now_ms >= self.disabled_until
    }
}

#[derive(Debug, Clone, Default)]
pub struct XPool {
    accounts: Vec<XAccount>,
    cursor: usize,
}

impl XPool {
    pub fn new(accounts: Vec<XAccount>) -> Self {
        Self {
            accounts,
            cursor: 0,
        }
    }

    pub fn size(&self) -> usize {
        self.accounts.len()
    }

    pub fn has(&self) -> bool {
        !self.accounts.is_empty()
    }

    pub fn all(&self) -> &[XAccount] {
        &self.accounts
    }

    pub fn pick(&mut self, now_ms: i64, exclude_ids: &HashSet<usize>) -> Option<XAccount> {
        let len = self.accounts.len();
        if len == 0 {
            return None;
        }
        for i in 0..len {
            let idx = (self.cursor + i) % len;
            let acc = &self.accounts[idx];
            if exclude_ids.contains(&acc.id) || !acc.is_healthy(now_ms) {
                continue;
            }
            self.cursor = (idx + 1) % len;
            return Some(acc.clone());
        }
        None
    }

    pub fn mark_rate_limit(&mut self, id: usize, reset_ms: i64) {
        if let Some(acc) = self.accounts.iter_mut().find(|a| a.id == id) {
            acc.cooldown_until = reset_ms;
        }
    }

    pub fn mark_auth_error(&mut self, id: usize, now_ms: i64) {
        if let Some(acc) = self.accounts.iter_mut().find(|a| a.id == id) {
            acc.disabled_until = now_ms + 30 * 60 * 1000;
        }
    }

    pub fn all_cooling(&self, now_ms: i64) -> bool {
        !self.accounts.is_empty() && self.accounts.iter().all(|a| !a.is_healthy(now_ms))
    }

    pub fn earliest_reset(&self) -> i64 {
        self.accounts
            .iter()
            .map(|a| a.cooldown_until.max(a.disabled_until))
            .min()
            .unwrap_or(0)
    }

    pub fn status(&self, now_ms: i64) -> Vec<Value> {
        self.accounts
            .iter()
            .map(|a| {
                let wait = ((a.cooldown_until.max(a.disabled_until) - now_ms) / 1000).max(0);
                json!({
                    "id": a.id,
                    "healthy": a.is_healthy(now_ms),
                    "waitSec": wait,
                })
            })
            .collect()
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DiagnosticMetrics {
    pub retrieved_x: usize,
    pub retrieved_th: usize,
    pub classified: usize,
    pub incident_signals: usize,
    pub contextual_posts: usize,
    pub rejected: usize,
}

pub struct SocialService {
    client: reqwest::Client,
    x_pool: RwLock<XPool>,
    x_hist: RwLock<HashMap<String, Value>>,
    t_hist: RwLock<HashMap<String, Value>>,
    cache: RwLock<HashMap<String, (i64, Value)>>,
    adhoc: RwLock<Vec<Value>>,
    x_query_id: RwLock<Option<String>>,
    x_bearer: RwLock<String>,
    python_path: RwLock<String>,
    root_dir: PathBuf,
    metrics: RwLock<DiagnosticMetrics>,
}

impl Default for SocialService {
    fn default() -> Self {
        Self::new()
    }
}

impl SocialService {
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(20))
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
            .build()
            .unwrap_or_default();

        let root_dir = find_project_root();
        let py_bin = find_python_bin(&root_dir);

        let mut svc = Self {
            client,
            x_pool: RwLock::new(XPool::default()),
            x_hist: RwLock::new(HashMap::new()),
            t_hist: RwLock::new(HashMap::new()),
            cache: RwLock::new(HashMap::new()),
            adhoc: RwLock::new(Vec::new()),
            x_query_id: RwLock::new(None),
            x_bearer: RwLock::new(DEFAULT_BEARER.to_string()),
            python_path: RwLock::new(py_bin),
            root_dir,
            metrics: RwLock::new(DiagnosticMetrics::default()),
        };
        svc.init_from_env();
        svc
    }

    fn init_from_env(&mut self) {
        if let Ok(qid) = std::env::var("X_QUERY_ID") {
            if !qid.trim().is_empty() {
                *self.x_query_id.write().unwrap() = Some(qid.trim().to_string());
            }
        }
        if let Ok(b) = std::env::var("X_BEARER") {
            if !b.trim().is_empty() {
                *self.x_bearer.write().unwrap() = b.trim().to_string();
            }
        }

        let mut accs = Vec::new();
        let auth_token = std::env::var("X_AUTH_TOKEN").unwrap_or_default();
        let ct0 = std::env::var("X_CT0").unwrap_or_default();
        if !auth_token.is_empty() && !ct0.is_empty() {
            accs.push(XAccount {
                id: 1,
                auth_token,
                ct0,
                cooldown_until: 0,
                disabled_until: 0,
            });
        }
        if !accs.is_empty() {
            *self.x_pool.write().unwrap() = XPool::new(accs);
        }
    }

    pub fn set_accounts(&self, accs: Vec<XAccount>) {
        *self.x_pool.write().unwrap() = XPool::new(accs);
    }

    pub fn set_query_id(&self, qid: String) {
        *self.x_query_id.write().unwrap() = Some(qid);
    }

    pub fn x_account_status(&self) -> Vec<Value> {
        let now = now_ms() as i64;
        self.x_pool.read().unwrap().status(now)
    }

    // --- Astroturf & Balance Filtering ---

    pub fn astroturf_full(text: &str) -> String {
        let s = text.to_lowercase();
        let s = ASTROTURF_URL_RE.replace_all(&s, " ");
        let s = ASTROTURF_MENTION_RE.replace_all(&s, " ");
        let s = ASTROTURF_HASHTAG_RE.replace_all(&s, " ");
        let s = ASTROTURF_PUNCT_RE.replace_all(&s, " ");
        let s = MULTI_SPACE_RE.replace_all(&s, " ");
        s.trim().to_string()
    }

    pub fn filter_astroturf(posts: &mut Vec<Value>) {
        if posts.is_empty() {
            return;
        }
        let eligible = |p: &Value| -> bool {
            let t = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
            if t.len() < 50 || t.trim_start().to_lowercase().starts_with("rt @") {
                return false;
            }
            let u = p
                .get("handle")
                .or_else(|| p.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start_matches('@')
                .to_lowercase();
            !AUTH_ACCOUNTS.contains(u.as_str())
        };

        let mut cand: HashMap<String, HashMap<String, HashSet<String>>> = HashMap::new();
        for p in posts.iter() {
            if !eligible(p) {
                continue;
            }
            let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let sig_val = crate::port::social::astroturf_sig(Value::String(text.to_string()))
                .unwrap_or(Value::String(String::new()));
            let sig = sig_val.as_str().unwrap_or("").to_string();
            if sig.len() < 40 {
                continue;
            }
            let full = Self::astroturf_full(text);
            let user = p
                .get("handle")
                .or_else(|| p.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start_matches('@')
                .to_lowercase();

            let by_full = cand.entry(sig).or_default();
            by_full.entry(full).or_default().insert(user);
        }

        posts.retain(|p| {
            let u = p
                .get("handle")
                .or_else(|| p.get("username"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim_start_matches('@')
                .to_lowercase();
            if AUTH_ACCOUNTS.contains(u.as_str()) {
                return true;
            }
            if let Some(assess) = p.get("assessment") {
                if assess
                    .get("signalEligible")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false)
                {
                    return true;
                }
            }
            if !eligible(p) {
                return true;
            }
            let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
            let sig_val = crate::port::social::astroturf_sig(Value::String(text.to_string()))
                .unwrap_or(Value::String(String::new()));
            let sig = sig_val.as_str().unwrap_or("");
            let full = Self::astroturf_full(text);
            if let Some(by_full) = cand.get(sig) {
                if let Some(users) = by_full.get(&full) {
                    if users.len() >= 3 {
                        return false;
                    }
                }
            }
            true
        });
    }

    // --- Deduplication & History Management ---

    pub fn hist_key(p: &Value) -> String {
        if let Some(url) = p.get("url").and_then(|v| v.as_str()) {
            if !url.is_empty() {
                return url.to_string();
            }
        }
        if let Some(code) = p.get("code").and_then(|v| v.as_str()) {
            if !code.is_empty() {
                return code.to_string();
            }
        }
        if let Some(id) = p.get("id").and_then(|v| v.as_str()) {
            if !id.is_empty() {
                return id.to_string();
            }
        }
        let text = p.get("text").and_then(|v| v.as_str()).unwrap_or("");
        if text.is_empty() {
            return String::new();
        }
        let author = p
            .get("handle")
            .or_else(|| p.get("username"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        format!("{text}#{author}")
    }

    pub fn hist_ts(p: &Value) -> i64 {
        if let Some(ts) = p.get("ts").and_then(|v| v.as_i64()) {
            return ts;
        }
        if let Some(ts_f) = p.get("ts").and_then(|v| v.as_f64()) {
            return ts_f as i64;
        }
        if let Some(created) = p.get("created_at").and_then(|v| v.as_str()) {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(created) {
                return dt.timestamp_millis();
            }
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(created) {
                return dt.timestamp_millis();
            }
        }
        0
    }

    pub fn hist_merge(&self, platform: &str, new_posts: &[Value]) {
        let now = now_ms() as i64;
        let is_x = platform == "x";
        let max_cap = if is_x { MAX_HIST_X } else { MAX_HIST_TH };

        let mut hist_lock = if is_x {
            self.x_hist.write().unwrap()
        } else {
            self.t_hist.write().unwrap()
        };

        for p in new_posts {
            let k = Self::hist_key(p);
            if !k.is_empty() {
                hist_lock.insert(k, p.clone());
            }
        }

        let mut arr: Vec<Value> = hist_lock
            .values()
            .filter(|p| {
                let ts = Self::hist_ts(p);
                ts > 0 && ts <= now && now - ts <= HIST_AGE_MS
            })
            .cloned()
            .collect();

        Self::filter_astroturf(&mut arr);
        arr.sort_by(|a, b| Self::hist_ts(b).cmp(&Self::hist_ts(a)));
        if arr.len() > max_cap {
            arr.truncate(max_cap);
        }

        hist_lock.clear();
        for p in arr {
            let k = Self::hist_key(&p);
            if !k.is_empty() {
                hist_lock.insert(k, p);
            }
        }
    }

    // --- X GraphQL Search Queries ---

    pub async fn x_search(
        &self,
        q: &str,
        cursor: Option<&str>,
        win_h: Option<f64>,
    ) -> Result<Value, String> {
        let now = now_ms() as i64;
        let acc_opt = {
            let mut pool = self.x_pool.write().unwrap();
            if !pool.has() {
                return Err("auth_token / ct0 belum diisi".to_string());
            }
            if pool.all_cooling(now) {
                return Err("X rate-limit cooldown".to_string());
            }
            let exclude = HashSet::new();
            pool.pick(now, &exclude)
        };

        let acc = match acc_opt {
            Some(a) => a,
            None => return Err("Semua akun X sedang cooldown/nonaktif".to_string()),
        };

        let win = win_h.unwrap_or(46.0);
        let since_clause = if win > 0.0 {
            format!(" since_time:{}", (now / 1000) - (win * 3600.0) as i64)
        } else {
            String::new()
        };
        let raw_query = format!("{q}{since_clause}");

        let qid = self
            .x_query_id
            .read()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "SearchByRawQuery".to_string());

        let mut vars = Map::new();
        vars.insert("rawQuery".to_string(), json!(raw_query));
        vars.insert("count".to_string(), json!(20));
        vars.insert("querySource".to_string(), json!("typed_query"));
        vars.insert("product".to_string(), json!("Latest"));
        vars.insert("withMediaGridTweet".to_string(), json!(false));
        if let Some(c) = cursor {
            vars.insert("cursor".to_string(), json!(c));
        }

        let vars_str = serde_json::to_string(&vars).unwrap_or_default();
        let url = format!(
            "https://api.x.com/graphql/{}/SearchByRawQuery?variables={}",
            qid,
            urlencoding(&vars_str)
        );

        let bearer = self.x_bearer.read().unwrap().clone();
        let cookie_hdr = format!("auth_token={}; ct0={}", acc.auth_token, acc.ct0);

        let res = self
            .client
            .get(&url)
            .header("authorization", format!("Bearer {bearer}"))
            .header("cookie", cookie_hdr)
            .header("x-csrf-token", &acc.ct0)
            .header("x-twitter-auth-type", "OAuth2Session")
            .header("x-twitter-active-user", "yes")
            .header("x-twitter-client-language", "id")
            .header("origin", "https://x.com")
            .header(
                "referer",
                format!("https://x.com/search?q={}&f=live", urlencoding(q)),
            )
            .send()
            .await;

        match res {
            Ok(resp) => {
                let status = resp.status().as_u16();
                if status == 429 {
                    self.x_pool
                        .write()
                        .unwrap()
                        .mark_rate_limit(acc.id, now + 15 * 60 * 1000);
                    return Err("X rate-limit 429".to_string());
                }
                if status == 401 || status == 403 {
                    self.x_pool.write().unwrap().mark_auth_error(acc.id, now);
                    return Err(format!("X auth error {status}"));
                }
                if !resp.status().is_success() {
                    return Err(format!("X HTTP error {status}"));
                }

                let text = resp.text().await.map_err(|e| e.to_string())?;
                let j: Value = serde_json::from_str(&text).unwrap_or(Value::Null);

                let mut out_posts = HashMap::new();
                collect_tweet_nodes(&j, &mut out_posts);

                let bottom_cur = find_bottom_cursor(&j).unwrap_or_default();
                let posts_vec: Vec<Value> = out_posts.into_values().collect();

                let mut m = self.metrics.write().unwrap();
                m.retrieved_x += posts_vec.len();

                Ok(json!({
                    "posts": posts_vec,
                    "cursor": bottom_cur,
                }))
            }
            Err(e) => Err(format!("X request network error: {e}")),
        }
    }

    // --- Threads Feed Scraping (ADR-004) ---

    pub async fn threads_scrape(
        &self,
        searches: &[String],
        profiles: &[String],
        auth: Option<&Value>,
    ) -> Result<Value, String> {
        let py_bin = self.python_path.read().unwrap().clone();
        let script = self.root_dir.join("vendor").join("threads_worker.py");

        let mut spec = Map::new();
        spec.insert("searches".to_string(), json!(searches));
        spec.insert("profiles".to_string(), json!(profiles));
        if let Some(a) = auth {
            spec.insert("auth".to_string(), a.clone());
        }

        let spec_json = serde_json::to_string(&spec).map_err(|e| e.to_string())?;

        let output = Command::new(&py_bin)
            .arg(&script)
            .arg(&spec_json)
            .current_dir(&self.root_dir)
            .output()
            .await
            .map_err(|e| format!("Failed to spawn threads_worker.py: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim()).map_err(|e| {
            format!("threads_worker output was not valid JSON: {e} | stdout={stdout}")
        })?;

        if let Some(posts) = parsed.get("posts").and_then(|v| v.as_array()) {
            let mut m = self.metrics.write().unwrap();
            m.retrieved_th += posts.len();
        }

        Ok(parsed)
    }

    pub async fn thread_fetch(&self, url: &str) -> Result<Value, String> {
        if url.contains("twitter.com") || url.contains("x.com") {
            return Ok(json!({
                "ok": true,
                "kind": "x",
                "post": null,
                "replies": [],
                "count": 0,
            }));
        }

        let py_bin = self.python_path.read().unwrap().clone();
        let script = self.root_dir.join("vendor").join("threads_thread.py");

        let mut spec = Map::new();
        spec.insert("url".to_string(), json!(url));

        let spec_json = serde_json::to_string(&spec).map_err(|e| e.to_string())?;

        let output = Command::new(&py_bin)
            .arg(&script)
            .arg(&spec_json)
            .current_dir(&self.root_dir)
            .output()
            .await
            .map_err(|e| format!("Failed to run threads_thread.py: {e}"))?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let parsed: Value = serde_json::from_str(stdout.trim())
            .map_err(|e| format!("threads_thread output not JSON: {e}"))?;

        Ok(parsed)
    }

    // --- Endpoints & Diagnostics ---

    pub fn api_x(&self) -> Result<Value, String> {
        let now = now_ms() as i64;
        let posts: Vec<Value> = {
            let mut arr: Vec<Value> = self.x_hist.read().unwrap().values().cloned().collect();
            arr.sort_by(|a, b| Self::hist_ts(b).cmp(&Self::hist_ts(a)));
            arr
        };
        let m = self.metrics.read().unwrap().clone();

        Ok(json!({
            "ok": true,
            "snapshot": true,
            "classifierVersion": CLASSIFIER_VERSION,
            "snapshotAt": now,
            "posts": posts,
            "at": now,
            "coverage": "sampled",
            "diagnostics": {
                "retrieved": { "x": m.retrieved_x, "threads": m.retrieved_th },
                "classified": m.classified,
                "incidentSignals": m.incident_signals,
                "contextualPosts": m.contextual_posts,
                "rejected": m.rejected,
            },
            "queries": [],
            "trending": [],
            "both": [],
            "hot": [],
            "stats": {},
            "trends": [],
            "newsTrending": [],
            "emerging": [],
            "newsKeywords": [],
        }))
    }

    pub fn api_threads(&self) -> Result<Value, String> {
        let now = now_ms() as i64;
        let posts: Vec<Value> = {
            let mut arr: Vec<Value> = self.t_hist.read().unwrap().values().cloned().collect();
            arr.sort_by(|a, b| Self::hist_ts(b).cmp(&Self::hist_ts(a)));
            arr
        };
        let m = self.metrics.read().unwrap().clone();

        Ok(json!({
            "ok": true,
            "snapshot": true,
            "classifierVersion": CLASSIFIER_VERSION,
            "snapshotAt": now,
            "posts": posts,
            "at": now,
            "coverage": "sampled",
            "diagnostics": {
                "retrieved": { "x": m.retrieved_x, "threads": m.retrieved_th },
                "classified": m.classified,
                "incidentSignals": m.incident_signals,
                "contextualPosts": m.contextual_posts,
                "rejected": m.rejected,
            },
            "queries": [],
            "trending": [],
            "both": [],
            "hot": [],
            "stats": {},
            "trends": [],
            "newsTrending": [],
            "emerging": [],
            "newsKeywords": [],
        }))
    }

    pub fn api_social_diagnostics(&self) -> Result<Value, String> {
        let m = self.metrics.read().unwrap().clone();
        let adhoc_topics: Vec<Value> = self.adhoc.read().unwrap().clone();

        Ok(json!({
            "ok": true,
            "classifierVersion": CLASSIFIER_VERSION,
            "diagnostics": {
                "retrieved": { "x": m.retrieved_x, "threads": m.retrieved_th },
                "classified": m.classified,
                "incidentSignals": m.incident_signals,
                "contextualPosts": m.contextual_posts,
                "rejected": m.rejected,
            },
            "dropStats": {
                "total": 0,
                "age": 0,
                "handle": 0,
                "lang": 0,
                "foreign": 0,
                "reportType": 0,
                "bodyLen": 0,
                "feedEligible": 0,
                "noTopic": 0,
                "lanes": {
                    "incident_signals": m.incident_signals,
                    "situational_context": m.contextual_posts,
                    "discovery_candidates": 0,
                    "rejected": m.rejected,
                }
            },
            "nlpAdapter": { "available": false },
            "clustersCount": 0,
            "adhocTopics": adhoc_topics,
        }))
    }

    pub fn api_social_explain(&self, text: &str) -> Result<Value, String> {
        let adhoc = self.adhoc.read().unwrap().clone();
        let explanation = crate::port::social::explain_post(
            Value::String(text.to_string()),
            Value::Array(adhoc),
            Value::Null,
        )?;
        Ok(json!({
            "ok": true,
            "explanation": explanation,
        }))
    }

    pub fn api_xdiag(&self) -> Result<Value, String> {
        let qid = self
            .x_query_id
            .read()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "auto".to_string());
        let x_accounts = self.x_account_status();

        Ok(json!({
            "host": "local",
            "qid": qid,
            "xAccounts": x_accounts,
            "out": [],
        }))
    }
}

// --- Utility Functions ---

fn urlencoding(s: &str) -> String {
    let mut encoded = String::new();
    for b in s.bytes() {
        if b.is_ascii_alphanumeric() || b == b'-' || b == b'_' || b == b'.' || b == b'~' {
            encoded.push(b as char);
        } else {
            encoded.push_str(&format!("%{:02X}", b));
        }
    }
    encoded
}

fn now_ms() -> f64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs_f64() * 1000.0)
        .unwrap_or(0.0)
}

fn find_project_root() -> PathBuf {
    let mut cur = std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    for _ in 0..5 {
        if cur.join("vendor").join("threads_worker.py").exists() {
            return cur;
        }
        if !cur.pop() {
            break;
        }
    }
    PathBuf::from("/home/ditip/Documents/djp-sitrep")
}

fn find_python_bin(root: &Path) -> String {
    let venv_py = root.join(".venv").join("bin").join("python");
    if venv_py.exists() {
        return venv_py.to_string_lossy().to_string();
    }
    let venv_win = root.join(".venv").join("Scripts").join("python.exe");
    if venv_win.exists() {
        return venv_win.to_string_lossy().to_string();
    }
    "python3".to_string()
}

fn collect_tweet_nodes(val: &Value, out: &mut HashMap<String, Value>) {
    if let Value::Object(map) = val {
        let lg_t = map.get("legacy").and_then(|v| v.as_object());
        let dt = map.get("details").and_then(|v| v.as_object());

        let raw_full_text = lg_t
            .and_then(|l| l.get("full_text").and_then(|t| t.as_str()))
            .or_else(|| dt.and_then(|d| d.get("full_text").and_then(|t| t.as_str())))
            .unwrap_or("");

        let rest_id = map.get("rest_id").and_then(|r| r.as_str()).unwrap_or("");

        let is_tweet_node = !raw_full_text.is_empty() && !rest_id.is_empty();

        if is_tweet_node {
            let u = map
                .get("core")
                .and_then(|c| c.get("user_results"))
                .and_then(|ur| ur.get("result"));
            let lg = u.and_then(|u| u.get("legacy")).and_then(|v| v.as_object());
            let uc = u.and_then(|u| u.get("core")).and_then(|v| v.as_object());

            let screen_name = uc
                .and_then(|c| c.get("screen_name").and_then(|s| s.as_str()))
                .or_else(|| lg.and_then(|l| l.get("screen_name").and_then(|s| s.as_str())))
                .unwrap_or("");

            let name = uc
                .and_then(|c| c.get("name").and_then(|s| s.as_str()))
                .or_else(|| lg.and_then(|l| l.get("name").and_then(|s| s.as_str())))
                .unwrap_or(screen_name);

            let avatar = u
                .and_then(|u| u.get("avatar"))
                .and_then(|a| a.get("image_url").and_then(|i| i.as_str()))
                .or_else(|| {
                    lg.and_then(|l| l.get("profile_image_url_https").and_then(|i| i.as_str()))
                })
                .unwrap_or("");

            let created_at = lg_t
                .and_then(|l| l.get("created_at").and_then(|c| c.as_str()))
                .unwrap_or("");

            let ts_ms = dt
                .and_then(|d| d.get("created_at_ms").and_then(|t| t.as_str()))
                .and_then(|s| s.parse::<i64>().ok())
                .unwrap_or(0);

            let post = json!({
                "id": rest_id,
                "handle": if screen_name.is_empty() { String::new() } else { format!("@{screen_name}") },
                "name": name,
                "avatar": avatar,
                "text": raw_full_text,
                "ts": ts_ms,
                "created_at": created_at,
                "url": if screen_name.is_empty() { String::new() } else { format!("https://x.com/{screen_name}/status/{rest_id}") },
                "replies": lg_t.and_then(|l| l.get("reply_count")).unwrap_or(&json!(0)),
                "reposts": lg_t.and_then(|l| l.get("retweet_count")).unwrap_or(&json!(0)),
                "likes": lg_t.and_then(|l| l.get("favorite_count")).unwrap_or(&json!(0)),
                "views": map.get("views").and_then(|v| v.get("count")).unwrap_or(&json!(0)),
            });

            out.insert(rest_id.to_string(), post);
        }

        for (k, v) in map {
            if is_tweet_node && (k == "quoted_status_result" || k == "retweeted_status_result") {
                continue;
            }
            collect_tweet_nodes(v, out);
        }
    } else if let Value::Array(arr) = val {
        for v in arr {
            collect_tweet_nodes(v, out);
        }
    }
}

fn find_bottom_cursor(val: &Value) -> Option<String> {
    if let Value::Object(map) = val {
        let cursor_type = map
            .get("cursorType")
            .or_else(|| map.get("cursor_type"))
            .and_then(|c| c.as_str());
        if cursor_type == Some("Bottom") {
            if let Some(v) = map.get("value").and_then(|v| v.as_str()) {
                return Some(v.to_string());
            }
        }
        for v in map.values() {
            if let Some(c) = find_bottom_cursor(v) {
                return Some(c);
            }
        }
    } else if let Value::Array(arr) = val {
        for v in arr {
            if let Some(c) = find_bottom_cursor(v) {
                return Some(c);
            }
        }
    }
    None
}
