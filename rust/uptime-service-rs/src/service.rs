//! Async `UptimeService` for DJP & Kemenkeu portal availability checks (`isdownapi.com`).

#![forbid(unsafe_code)]

use crate::port::normalize_check_result;
use futures_util::future::join_all;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{Mutex, watch};

pub const DEFAULT_TTL_MS: i64 = 90_000;
pub const HISTORY_LIMIT: usize = 12;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Portal {
    pub id: Cow<'static, str>,
    pub name: Cow<'static, str>,
    pub domain: Cow<'static, str>,
    pub url: Cow<'static, str>,
}

macro_rules! portals {
    ($(($a:literal, $b:literal, $c:literal, $d:literal)),* $(,)?) => {
        &[$(Portal { id: Cow::Borrowed($a), name: Cow::Borrowed($b), domain: Cow::Borrowed($c), url: Cow::Borrowed($d) }),*]
    };
}

pub const DJP_PORTALS: &[Portal] = portals! {
    ("pajak", "Pajak.go.id", "pajak.go.id", "https://www.pajak.go.id"),
    ("coretax", "Coretax DJP", "coretaxdjp.pajak.go.id", "https://coretaxdjp.pajak.go.id"),
    ("djponline", "DJP Online", "djponline.pajak.go.id", "https://djponline.pajak.go.id"),
    ("kemenkeu", "Kemenkeu", "kemenkeu.go.id", "https://www.kemenkeu.go.id"),
};

#[derive(Debug, Clone)]
pub struct ProbeResponse {
    pub ok: bool,
    pub status: u16,
    pub body: Value,
}

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type FetchFn = Arc<dyn Fn(String) -> BoxFuture<Result<ProbeResponse, String>> + Send + Sync>;
pub type NowFn = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DomainStatus {
    pub id: String,
    pub name: String,
    pub domain: String,
    pub url: String,
    pub status: String,
    pub state: String,
    pub http_code: Option<i64>,
    pub response_ms: Option<i64>,
    pub ip: Option<String>,
    pub error: Option<String>,
    pub checked_at: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub probe_error: Option<bool>,
    #[serde(default)]
    pub history: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct UptimePayload {
    pub ok: bool,
    pub provider: String,
    pub total: usize,
    pub up_count: usize,
    pub slow_count: usize,
    pub down_count: usize,
    pub items: Vec<DomainStatus>,
    pub at: i64,
    pub cached: bool,
}

#[derive(Default)]
struct State {
    cache: Option<UptimePayload>,
    cache_at: i64,
    in_flight: Option<watch::Receiver<Option<UptimePayload>>>,
    history: HashMap<String, VecDeque<String>>,
}

pub struct UptimeService {
    portals: Vec<Portal>,
    ttl_ms: i64,
    fetch_fn: FetchFn,
    now_fn: NowFn,
    state: Mutex<State>,
}

impl Default for UptimeService {
    fn default() -> Self {
        Self::new()
    }
}

impl UptimeService {
    pub fn new() -> Self {
        Self::with_client(reqwest::Client::new())
    }

    pub fn with_client(client: reqwest::Client) -> Self {
        let fetch: FetchFn = Arc::new(move |url| {
            let c = client.clone();
            Box::pin(async move {
                let req = c
                    .get(&url)
                    .header("accept", "application/json")
                    .timeout(Duration::from_secs(12));
                let r = req.send().await.map_err(|e| e.to_string())?;
                let (ok, status) = (r.status().is_success(), r.status().as_u16());
                let body = if ok {
                    r.json().await.map_err(|e| e.to_string())?
                } else {
                    Value::Null
                };
                Ok(ProbeResponse { ok, status, body })
            })
        });
        let now: NowFn = Arc::new(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map_or(0, |d| d.as_millis() as i64)
        });
        Self::with_options(DJP_PORTALS.to_vec(), DEFAULT_TTL_MS, fetch, now)
    }

    pub fn with_options(
        portals: Vec<Portal>,
        ttl_ms: i64,
        fetch_fn: FetchFn,
        now_fn: NowFn,
    ) -> Self {
        Self {
            portals,
            ttl_ms,
            fetch_fn,
            now_fn,
            state: Mutex::new(State::default()),
        }
    }

    pub fn portals(&self) -> &[Portal] {
        &self.portals
    }
    pub async fn get_cached(&self) -> Option<UptimePayload> {
        self.state.lock().await.cache.clone()
    }

    async fn check_one(&self, portal: &Portal) -> DomainStatus {
        let base = "https://isdownapi.com/api/check";
        let url = reqwest::Url::parse_with_params(base, &[("domain", portal.domain.as_ref())])
            .map_or_else(|_| format!("{base}?domain={}", portal.domain), String::from);
        let (raw, err) = match (self.fetch_fn)(url).await {
            Ok(r) if r.ok => (r.body, false),
            Ok(r) => (
                json!({"status": "down", "error": format!("isdownapi HTTP {}", r.status)}),
                true,
            ),
            Err(e) => (
                json!({"status": "down", "error": String::from_utf16_lossy(&e.encode_utf16().take(120).collect::<Vec<_>>())}),
                true,
            ),
        };
        let pval = serde_json::to_value(portal).unwrap_or(Value::Null);
        let mut it: DomainStatus = normalize_check_result(pval, raw, Value::from((self.now_fn)()))
            .ok()
            .and_then(|v| serde_json::from_value(v).ok())
            .unwrap_or_default();
        it.probe_error = err.then_some(true);
        it
    }

    pub async fn check_all(&self, force: bool) -> UptimePayload {
        loop {
            let tx = {
                let mut g = self.state.lock().await;
                if !force
                    && let Some(c) = g.cache.as_ref()
                    && (self.now_fn)() - g.cache_at < self.ttl_ms
                {
                    return UptimePayload {
                        cached: true,
                        ..c.clone()
                    };
                }
                if let Some(mut rx) = g.in_flight.clone() {
                    drop(g);
                    if rx.wait_for(|v| v.is_some()).await.is_ok()
                        && let Some(p) = rx.borrow().clone()
                    {
                        return p;
                    }
                    continue;
                }
                let (tx, rx) = watch::channel(None);
                g.in_flight = Some(rx);
                tx
            };
            let mut items = join_all(self.portals.iter().map(|p| self.check_one(p))).await;
            let mut g = self.state.lock().await;
            for it in &mut items {
                let q = g.history.entry(it.id.clone()).or_default();
                q.push_back(it.state.clone());
                if q.len() > HISTORY_LIMIT {
                    q.pop_front();
                }
                it.history = q.iter().cloned().collect();
            }
            let up = items.iter().filter(|x| x.status == "up").count();
            let slow = items.iter().filter(|x| x.state == "slow").count();
            let at = (self.now_fn)();
            let payload = UptimePayload {
                ok: true,
                provider: "isdownapi.com".into(),
                total: items.len(),
                up_count: up,
                slow_count: slow,
                down_count: items.len() - up,
                items,
                at,
                cached: false,
            };
            (g.cache, g.cache_at, g.in_flight) = (Some(payload.clone()), at, None);
            drop(g);
            let _ = tx.send(Some(payload.clone()));
            return payload;
        }
    }
}
