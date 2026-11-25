//! Async `MarketService` for TradingView, Yahoo Finance, CNBC, Binance, and FX history.

#![forbid(unsafe_code)]

use crate::port::{cnbc_symbol_for, downsample_closes};
use futures_util::future::join_all;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::path::{Path, PathBuf};
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicI64, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::Mutex;

pub const STOCK_BLUE: &[&str] = &["BBCA", "TLKM", "GOTO", "BMRI", "ASII", "UNVR", "KLBF"];
pub const FOREX_PAIRS: &[(&str, &str)] = &[("USDIDR", "USD/IDR")];

pub type Commodity = (
    &'static str,
    &'static str,
    &'static str,
    &'static str,
    &'static str,
);
pub const COMMODITIES: &[Commodity] = &[
    ("COMEX:GC1!", "Gold", "futures", "GC=F", "@GC.1"),
    ("COMEX:SI1!", "Silver", "futures", "SI=F", "@SI.1"),
    ("NYMEX:CL1!", "WTI Oil", "futures", "CL=F", "@CL.1"),
    ("NYMEX:NG1!", "Nat Gas", "futures", "NG=F", "@NG.1"),
    ("COMEX:HG1!", "Copper", "futures", "HG=F", "@HG.1"),
    ("COMEX:ALI1!", "Aluminum", "futures", "ALI=F", "@AL.1"),
    ("NYMEX:HO1!", "Heat Oil", "futures", "HO=F", "@HO.1"),
    ("CBOT:ZR1!", "Rice", "futures", "ZR=F", "@RR.1"),
    ("CBOT:ZW1!", "Wheat", "futures", "ZW=F", "@W.1"),
    ("CBOT:ZC1!", "Corn", "futures", "ZC=F", "@C.1"),
    ("CBOT:ZS1!", "Soybean", "futures", "ZS=F", "@S.1"),
    ("CME:DC1!", "Milk", "futures", "DC=F", "@DA.1"),
    ("ICEUS:KC1!", "Coffee", "futures", "KC=F", "@KC.1"),
    ("ICEUS:SB1!", "Sugar", "futures", "SB=F", "@SB.1"),
    ("ICEUS:CC1!", "Cocoa", "futures", "CC=F", "@CC.1"),
    ("BINANCE:BTCUSDT", "Bitcoin", "crypto", "BTC-USD", "BTC.CM="),
];

pub type WorldIndex = (&'static str, &'static str, &'static str, &'static str);
pub const WORLD_IDX: &[WorldIndex] = &[
    ("^GSPC", "S&P 500", ".SPX", "SP:SPX"),
    ("^NDX", "NASDAQ 100", ".NDX", "NASDAQ:NDX"),
    ("^DJI", "DOW", ".DJI", "DJ:DJI"),
    ("^RUT", "RUSSELL", ".RUT", "TVC:RUT"),
    ("^N225", "NIKKEI", ".N225", "TVC:NI225"),
    ("^KS11", "KOSPI", ".KS11", "KRX:KOSPI"),
    ("^HSI", "HANG SENG", ".HSI", "HSI:HSI"),
    ("^TWII", "TAIEX", ".TWII", "TWSE:TAIEX"),
    ("^GDAXI", "DAX", ".GDAXI", "XETR:DAX"),
    ("^FTSE", "FTSE 100", ".FTSE", "FTSE:UKX"),
    ("^FCHI", "CAC 40", ".FCHI", "EURONEXT:PX1"),
    ("^SSMI", "SMI", ".SSMI", "SIX:SMI"),
    ("^AXJO", "ASX 200", ".AXJO", "ASX:XJO"),
    ("^BSESN", "SENSEX", ".BSESN", "BSE:SENSEX"),
    ("^NSEI", "NIFTY", ".NSEI", "NSE:NIFTY"),
    ("^STI", "STI", ".STI", "TVC:STI"),
    ("000001.SS", "SSE", ".SSEC", "SSE:000001"),
    ("^BVSP", "IBOVESPA", ".BVSP", "BMFBOVESPA:IBOV"),
];

fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'a'..=b'z' | b'A'..=b'Z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => {
                use std::fmt::Write;
                let _ = write!(out, "%{:02X}", b);
            }
        }
    }
    out
}

fn json_num(v: Option<&Value>) -> Option<f64> {
    let v = v?;
    if let Some(f) = v.as_f64() {
        Some(f)
    } else {
        v.as_str()?.trim().parse::<f64>().ok()
    }
}

fn downsample_raw(raw: &[f64], max_pts: usize) -> Vec<f64> {
    let val_arr = Value::Array(raw.iter().copied().map(Value::from).collect());
    downsample_closes(val_arr, Some(max_pts as f64))
        .ok()
        .and_then(|v| {
            v.as_array()
                .map(|arr| arr.iter().filter_map(|x| x.as_f64()).collect())
        })
        .unwrap_or_default()
}

#[derive(Debug, Clone)]
pub struct HttpResponse {
    pub ok: bool,
    pub status: u16,
    pub body: String,
}

impl HttpResponse {
    pub fn new(status: u16, body: impl Into<String>) -> Self {
        let body = body.into();
        Self {
            ok: (200..300).contains(&status),
            status,
            body,
        }
    }
}

pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;
pub type FetchFn = Arc<
    dyn Fn(
            String,
            Option<String>,
            Option<HashMap<String, String>>,
            u64,
        ) -> BoxFuture<Result<HttpResponse, String>>
        + Send
        + Sync,
>;
pub type NowFn = Arc<dyn Fn() -> i64 + Send + Sync>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloseMemoEntry {
    pub at: i64,
    pub closes: Vec<f64>,
    #[serde(default, rename = "failAt")]
    pub fail_at: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FxHistEntry {
    pub d: String,
    pub v: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsdIdrResult {
    pub price: f64,
    pub chg: f64,
    pub pct: f64,
    pub closes: Vec<f64>,
}

#[derive(Debug, Clone)]
struct MemoItem {
    at: i64,
    data: Value,
}

pub struct MarketService {
    hist_dir: PathBuf,
    user_agent: String,
    client: reqwest::Client,
    fetch_fn: Option<FetchFn>,
    now_fn: Option<NowFn>,
    pub close_memo: Arc<Mutex<HashMap<String, CloseMemoEntry>>>,
    pub fxhist: Arc<Mutex<Vec<FxHistEntry>>>,
    yahoo_blocked_until: Arc<AtomicI64>,
    mkt_closes_dirty: Arc<AtomicBool>,
    mkt_closes_save_at: Arc<AtomicI64>,
    mkt_memo: Arc<Mutex<Option<MemoItem>>>,
    world_memo: Arc<Mutex<Option<MemoItem>>>,
    gold_memo: Arc<Mutex<Option<MemoItem>>>,
}

const GOLD_URL: &str = "https://logam-mulia-api.iamutaki.workers.dev/api/prices/anekalogam";

fn antam_2026(item: &Value) -> bool {
    item["materialType"]
        .as_str()
        .is_some_and(|s| s.contains("2026"))
}

fn gold_number(item: &Value, key: &str) -> Option<f64> {
    json_num(item.get(key)).filter(|n| n.is_finite())
}

static GOOGLE_FIN_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static literal regex pattern
    Regex::new(r"\[\[\d{4},\d{1,2},\d{1,2},[^\]]+\],\s*\[([0-9]+(?:\.[0-9]+)?),").unwrap()
});

impl MarketService {
    pub fn new(
        hist_dir: impl AsRef<Path>,
        user_agent: impl Into<String>,
        fetch_fn: Option<FetchFn>,
        now_fn: Option<NowFn>,
    ) -> Self {
        let hist_dir = hist_dir.as_ref().to_path_buf();
        let user_agent = user_agent.into();
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(15))
            .build()
            .unwrap_or_default();
        let fxhist = std::fs::read_to_string(hist_dir.join("fxhist.json"))
            .ok()
            .and_then(|s| serde_json::from_str::<Vec<FxHistEntry>>(&s).ok())
            .unwrap_or_default();

        let mut close_memo = HashMap::new();
        let now_load = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);
        if let Ok(s) = std::fs::read_to_string(hist_dir.join("market-closes.json"))
            && let Ok(v) = serde_json::from_str::<Value>(&s)
            && let Some(items) = v.get("items").and_then(|i| i.as_object())
        {
            for (k, e) in items {
                if let Ok(entry) = serde_json::from_value::<CloseMemoEntry>(e.clone())
                    && entry.closes.len() >= 2
                    && now_load - entry.at < 24 * 3_600_000
                {
                    close_memo.insert(k.clone(), entry);
                }
            }
        }

        Self {
            hist_dir,
            user_agent,
            client,
            fetch_fn,
            now_fn,
            close_memo: Arc::new(Mutex::new(close_memo)),
            fxhist: Arc::new(Mutex::new(fxhist)),
            yahoo_blocked_until: Arc::new(AtomicI64::new(0)),
            mkt_closes_dirty: Arc::new(AtomicBool::new(false)),
            mkt_closes_save_at: Arc::new(AtomicI64::new(0)),
            mkt_memo: Arc::new(Mutex::new(None)),
            world_memo: Arc::new(Mutex::new(None)),
            gold_memo: Arc::new(Mutex::new(None)),
        }
    }

    pub fn now_ms(&self) -> i64 {
        self.now_fn.as_ref().map(|f| f()).unwrap_or_else(|| {
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_millis() as i64)
                .unwrap_or(0)
        })
    }

    async fn do_fetch(
        &self,
        url: &str,
        body: Option<&str>,
        headers: Option<&HashMap<String, String>>,
        timeout_ms: u64,
    ) -> Result<HttpResponse, String> {
        if let Some(f) = &self.fetch_fn {
            return f(
                url.to_string(),
                body.map(|b| b.to_string()),
                headers.cloned(),
                timeout_ms,
            )
            .await;
        }
        let mut req = if let Some(b) = body {
            self.client.post(url).body(b.to_string())
        } else {
            self.client.get(url)
        };
        req = req.timeout(Duration::from_millis(timeout_ms));
        if let Some(hdrs) = headers {
            for (k, v) in hdrs {
                req = req.header(k, v);
            }
        }
        let res = req.send().await.map_err(|e| e.to_string())?;
        let status = res.status().as_u16();
        let text = res.text().await.map_err(|e| e.to_string())?;
        Ok(HttpResponse::new(status, text))
    }

    pub async fn tv_scan(&self, market_name: &str, body: Value) -> Result<Vec<Value>, String> {
        let url = format!("https://scanner.tradingview.com/{market_name}/scan");
        let mut headers = HashMap::new();
        headers.insert("content-type".to_string(), "application/json".to_string());
        let body_str = serde_json::to_string(&body).unwrap_or_default();
        let res = self
            .do_fetch(&url, Some(&body_str), Some(&headers), 15_000)
            .await?;
        if !res.ok {
            return Err(format!("tv scan {}", res.status));
        }
        let parsed: Value = serde_json::from_str(&res.body).map_err(|e| e.to_string())?;
        Ok(parsed
            .get("data")
            .and_then(|d| d.as_array())
            .cloned()
            .unwrap_or_default())
    }

    pub async fn save_fx(&self) {
        let hist = self.fxhist.lock().await;
        let start = hist.len().saturating_sub(400);
        let file = self.hist_dir.join("fxhist.json");
        if let Ok(json) = serde_json::to_string(&hist[start..]) {
            let _ = tokio::fs::create_dir_all(&self.hist_dir).await;
            let _ = tokio::fs::write(file, json).await;
        }
    }

    pub async fn backfill_fx(&self) {
        let now = self.now_ms();
        let mut days = Vec::with_capacity(30);
        for i in (1..=29).rev() {
            let (y, m, d) = days_to_ymd((now - i * 86_400_000) / 86_400_000);
            days.push(format!("{y:04}-{m:02}-{d:02}"));
        }
        for chunk in days.chunks(4) {
            let tasks: Vec<_> = chunk.iter().map(|day| {
                let day = day.clone();
                async move {
                    let url = format!("https://cdn.jsdelivr.net/npm/@fawazahmed0/currency-api@{day}/v1/currencies/usd.json");
                    let res = self.do_fetch(&url, None, None, 10_000).await.ok()?;
                    if !res.ok { return None; }
                    let parsed: Value = serde_json::from_str(&res.body).ok()?;
                    let v = parsed.pointer("/usd/idr").and_then(|v| v.as_f64())?;
                    Some((day, v))
                }
            }).collect();
            let results = join_all(tasks).await;
            let mut hist = self.fxhist.lock().await;
            for (d, v) in results.into_iter().flatten() {
                if !hist.iter().any(|h| h.d == d) {
                    hist.push(FxHistEntry { d, v });
                }
            }
        }
        {
            let mut hist = self.fxhist.lock().await;
            let mut map = HashMap::new();
            for item in hist.drain(..) {
                map.insert(item.d.clone(), item);
            }
            let mut sorted: Vec<FxHistEntry> = map.into_values().collect();
            sorted.sort_by(|a, b| a.d.cmp(&b.d));
            *hist = sorted;
        }
        self.save_fx().await;
    }

    pub async fn usdidr(&self) -> Option<UsdIdrResult> {
        let res = self
            .do_fetch(
                "https://www.floatrates.com/daily/usd.json",
                None,
                None,
                10_000,
            )
            .await
            .ok()?;
        if !res.ok {
            return None;
        }
        let parsed: Value = serde_json::from_str(&res.body).ok()?;
        let idr = parsed.get("idr")?;
        let price = json_num(idr.get("rate"))?;
        let day = idr
            .get("date")
            .and_then(|v| v.as_str())
            .and_then(|s| s.get(..10))
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                let (y, m, d) = days_to_ymd(self.now_ms() / 86_400_000);
                format!("{y:04}-{m:02}-{d:02}")
            });

        let mut should_backfill = false;
        let (prev, closes) = {
            let mut hist = self.fxhist.lock().await;
            if !hist.last().map(|h| h.d == day).unwrap_or(false) {
                hist.push(FxHistEntry { d: day, v: price });
            }
            let prev = if hist.len() > 1 {
                Some(hist[hist.len() - 2].v)
            } else {
                None
            };
            if hist.len() < 25 {
                should_backfill = true;
            }
            let start = hist.len().saturating_sub(60);
            let closes: Vec<f64> = hist[start..].iter().map(|h| h.v).collect();
            (prev, closes)
        };

        self.save_fx().await;
        if should_backfill {
            let this = self.clone_ref();
            tokio::spawn(async move {
                this.backfill_fx().await;
            });
        }
        let chg = prev.map(|p| price - p).unwrap_or(0.0);
        let pct = prev.map(|p| (chg / p) * 100.0).unwrap_or(0.0);
        Some(UsdIdrResult {
            price,
            chg,
            pct,
            closes,
        })
    }

    pub async fn save_mkt_closes(&self, force: bool) {
        let now = self.now_ms();
        if !self.mkt_closes_dirty.load(Ordering::Relaxed) && !force {
            return;
        }
        let last_save = self.mkt_closes_save_at.load(Ordering::Relaxed);
        if !force && (now - last_save < 60_000) {
            return;
        }
        self.mkt_closes_save_at.store(now, Ordering::Relaxed);
        self.mkt_closes_dirty.store(false, Ordering::Relaxed);

        let items: HashMap<String, Value> = {
            let memo = self.close_memo.lock().await;
            memo.iter()
                .filter(|(_, v)| v.closes.len() >= 2)
                .map(|(k, v)| {
                    let start = v.closes.len().saturating_sub(90);
                    (
                        k.clone(),
                        json!({ "at": v.at, "closes": &v.closes[start..] }),
                    )
                })
                .collect()
        };
        let tmp = self
            .hist_dir
            .join(format!("market-closes.json.{}.tmp", std::process::id()));
        let dest = self.hist_dir.join("market-closes.json");
        if let Ok(bytes) = serde_json::to_vec(&json!({ "at": now, "items": items })) {
            let _ = tokio::fs::create_dir_all(&self.hist_dir).await;
            if tokio::fs::write(&tmp, bytes).await.is_ok() {
                let _ = tokio::fs::rename(&tmp, &dest).await;
            }
        }
    }

    pub async fn cnbc_closes(&self, cnbc_sym: &str, tf: &str, max_pts: usize) -> Vec<f64> {
        if cnbc_sym.is_empty() {
            return Vec::new();
        }
        let url = format!(
            "https://ts-api.cnbc.com/harmony/app/charts/{tf}.json?symbol={}",
            urlencode(cnbc_sym)
        );
        let mut headers = HashMap::new();
        headers.insert("user-agent".to_string(), self.user_agent.clone());
        headers.insert("accept".to_string(), "application/json".to_string());
        let Ok(res) = self.do_fetch(&url, None, Some(&headers), 8500).await else {
            return Vec::new();
        };
        if !res.ok {
            return Vec::new();
        }
        let Ok(j) = serde_json::from_str::<Value>(&res.body) else {
            return Vec::new();
        };
        let bars = j.pointer("/barData/priceBars").and_then(|v| v.as_array());
        let raw: Vec<f64> = bars
            .unwrap_or(&Vec::new())
            .iter()
            .filter_map(|b| {
                let n = json_num(b.get("close"))?;
                (n.is_finite() && n > 0.0).then_some(n)
            })
            .collect();
        if raw.len() < 2 {
            return Vec::new();
        }
        if tf == "1M" {
            let start = raw.len().saturating_sub(40);
            raw[start..].to_vec()
        } else {
            downsample_raw(&raw, max_pts)
        }
    }

    pub async fn binance_closes(&self, symbol: &str) -> Vec<f64> {
        let sym = if symbol.is_empty() { "BTCUSDT" } else { symbol };
        let url = format!(
            "https://api.binance.com/api/v3/klines?symbol={}&interval=1h&limit=48",
            urlencode(sym)
        );
        let Ok(res) = self.do_fetch(&url, None, None, 7000).await else {
            return Vec::new();
        };
        if !res.ok {
            return Vec::new();
        }
        let Ok(rows) = serde_json::from_str::<Vec<Value>>(&res.body) else {
            return Vec::new();
        };
        rows.into_iter()
            .filter_map(|r| {
                let n = json_num(r.as_array()?.get(4))?;
                (n.is_finite() && n > 0.0).then_some(n)
            })
            .collect()
    }

    pub async fn google_finance_composite_closes(&self) -> Vec<f64> {
        let url = "https://www.google.com/finance/quote/COMPOSITE:IDX?hl=en";
        let mut headers = HashMap::new();
        headers.insert("user-agent".to_string(), self.user_agent.clone());
        let Ok(res) = self.do_fetch(url, None, Some(&headers), 8500).await else {
            return Vec::new();
        };
        if !res.ok {
            return Vec::new();
        }
        let mut vals = Vec::new();
        for cap in GOOGLE_FIN_RE.captures_iter(&res.body) {
            if let Some(m) = cap.get(1)
                && let Ok(n) = m.as_str().parse::<f64>()
                && n.is_finite()
                && n > 500.0
            {
                vals.push(n);
            }
        }
        downsample_raw(&vals, 60)
    }

    pub async fn yahoo_closes(&self, sym: &str, query: &str) -> Vec<f64> {
        if sym.is_empty() {
            return Vec::new();
        }
        let now = self.now_ms();
        if now < self.yahoo_blocked_until.load(Ordering::Relaxed) {
            return Vec::new();
        }
        let mut closes_arr = Vec::new();
        for host in ["query1.finance.yahoo.com", "query2.finance.yahoo.com"] {
            let url = format!("https://{host}/v8/finance/chart/{}?{query}", urlencode(sym));
            let mut headers = HashMap::new();
            headers.insert("user-agent".to_string(), self.user_agent.clone());
            let Ok(res) = self.do_fetch(&url, None, Some(&headers), 9000).await else {
                continue;
            };
            if res.status == 429 || res.status == 403 {
                self.yahoo_blocked_until
                    .store(now + 15 * 60_000, Ordering::Relaxed);
                break;
            }
            if !res.ok {
                continue;
            }
            let Ok(d) = serde_json::from_str::<Value>(&res.body) else {
                continue;
            };
            let close_list = d
                .pointer("/chart/result/0/indicators/quote/0/close")
                .and_then(|v| v.as_array());
            closes_arr = close_list
                .unwrap_or(&Vec::new())
                .iter()
                .filter_map(|v| {
                    let n = v.as_f64()?;
                    (n.is_finite() && n > 0.0).then_some(n)
                })
                .collect();
            if closes_arr.len() >= 2 {
                return downsample_raw(&closes_arr, 90);
            }
        }
        closes_arr
    }

    pub async fn fetch_symbol_closes(&self, k: &str) -> Vec<f64> {
        if k.is_empty() {
            return Vec::new();
        }
        let now = self.now_ms();
        let mem = { self.close_memo.lock().await.get(k).cloned() };
        if let Some(entry) = &mem {
            if entry.closes.len() >= 2 && now - entry.at < 10 * 60_000 {
                return entry.closes.clone();
            }
            if entry.closes.len() < 2 && entry.fail_at > 0 && now - entry.fail_at < 30_000 {
                return Vec::new();
            }
        }

        let mut series = Vec::new();
        let cnbc_sym = cnbc_symbol_for(Value::String(k.to_string()))
            .ok()
            .and_then(|v| v.as_str().map(|s| s.to_string()))
            .unwrap_or_default();
        if k == "FX_IDC:USDIDR" {
            let hist = self.fxhist.lock().await;
            if hist.len() >= 2 {
                let start = hist.len().saturating_sub(60);
                series = hist[start..].iter().map(|h| h.v).collect();
            }
            drop(hist);
            if series.len() < 2 {
                series = self.cnbc_closes("IDR=", "5D", 60).await;
            }
            if series.len() < 2 {
                series = self.yahoo_closes("IDR=X", "interval=15m&range=5d").await;
            }
        } else if k == "IDX:COMPOSITE" {
            series = self.cnbc_closes(".JKSE", "1D", 90).await;
            if series.len() < 12 {
                series = self.cnbc_closes(".JKSE", "5D", 90).await;
            }
            if series.len() < 2 {
                series = self.cnbc_closes(".JKSE", "1M", 45).await;
            }
            if series.len() < 2 {
                series = self.yahoo_closes("^JKSE", "interval=1m&range=1d").await;
            }
            if series.len() < 2 {
                series = self.yahoo_closes("^JKSE", "interval=1d&range=1mo").await;
            }
            if series.len() < 2 {
                series = self.google_finance_composite_closes().await;
            }
        } else if let Some(ticker) = k.strip_prefix("IDX:") {
            let yh = format!("{ticker}.JK");
            series = self.cnbc_closes(&cnbc_sym, "1M", 40).await;
            if series.len() < 2 {
                series = self.yahoo_closes(&yh, "interval=1d&range=1mo").await;
            }
        } else if k == "BINANCE:BTCUSDT" {
            series = self.cnbc_closes("BTC.CM=", "5D", 90).await;
            if series.len() < 2 {
                series = self.binance_closes("BTCUSDT").await;
            }
            if series.len() < 2 {
                series = self.yahoo_closes("BTC-USD", "interval=1m&range=1d").await;
            }
        } else {
            series = self.cnbc_closes(&cnbc_sym, "5D", 90).await;
            if series.len() < 2 {
                series = self.cnbc_closes(&cnbc_sym, "1M", 45).await;
            }
            if series.len() < 2 {
                let yh = COMMODITIES
                    .iter()
                    .find(|c| c.0 == k)
                    .map(|c| c.3)
                    .unwrap_or(k);
                series = self.yahoo_closes(yh, "interval=1m&range=1d").await;
                if series.len() < 2 {
                    series = self.yahoo_closes(yh, "interval=1d&range=1mo").await;
                }
            }
        }

        if series.len() >= 2 {
            let mut memo = self.close_memo.lock().await;
            memo.insert(
                k.to_string(),
                CloseMemoEntry {
                    at: now,
                    closes: series.clone(),
                    fail_at: 0,
                },
            );
            if memo.len() > 500
                && let Some(first) = memo.keys().next().cloned()
            {
                memo.remove(&first);
            }
            self.mkt_closes_dirty.store(true, Ordering::Relaxed);
            let this = self.clone_ref();
            tokio::spawn(async move {
                this.save_mkt_closes(false).await;
            });
            return series;
        }

        if let Some(mut prev_entry) = mem
            && prev_entry.closes.len() >= 2
        {
            prev_entry.fail_at = now;
            self.close_memo
                .lock()
                .await
                .insert(k.to_string(), prev_entry.clone());
            return prev_entry.closes;
        }

        self.close_memo.lock().await.insert(
            k.to_string(),
            CloseMemoEntry {
                at: 0,
                closes: Vec::new(),
                fail_at: now,
            },
        );
        Vec::new()
    }

    pub async fn closes(&self, list: &[String]) -> Value {
        let mut seen = std::collections::HashSet::new();
        let mut keys = Vec::new();
        for item in list {
            if !item.is_empty() && seen.insert(item.clone()) {
                keys.push(item.clone());
                if keys.len() >= 60 {
                    break;
                }
            }
        }
        let mut out = serde_json::Map::new();
        for chunk in keys.chunks(6) {
            let tasks: Vec<_> = chunk
                .iter()
                .map(|k| async {
                    let s = self.fetch_symbol_closes(k).await;
                    (k.clone(), s)
                })
                .collect();
            for (k, series) in join_all(tasks).await {
                out.insert(k, json!(series));
            }
        }
        json!({ "ok": true, "closes": out })
    }

    pub async fn world_index(&self) -> Result<Value, String> {
        let now = self.now_ms();
        {
            let memo = self.world_memo.lock().await;
            if let Some(m) = &*memo
                && now - m.at < 60_000
            {
                return Ok(m.data.clone());
            }
        }

        let mut tv_map = HashMap::new();
        let cols = json!(["name", "close", "change", "change_abs"]);
        let tickers: Vec<Value> = WORLD_IDX.iter().map(|w| json!(w.3)).collect();
        if let Ok(tv_rows) = self.tv_scan("global", json!({ "columns": cols, "symbols": { "query": { "types": [] }, "tickers": tickers } })).await {
            for s in tv_rows {
                if let Some(sym) = s.get("s").and_then(|v| v.as_str()) {
                    tv_map.insert(sym.to_string(), parse_row("WORLD", &s));
                }
            }
        }

        let mut out = Vec::new();
        for chunk in WORLD_IDX.chunks(6) {
            let tasks: Vec<_> = chunk.iter().map(|w| async {
                let tv = tv_map.get(w.3);
                let cl = self.fetch_symbol_closes(w.0).await;
                let price = tv.and_then(|t| t.get("price")?.as_f64()).or_else(|| cl.last().copied())?;
                if !price.is_finite() { return None; }
                let prev = if cl.len() > 1 { Some(cl[cl.len() - 2]) } else { None };
                let chg = tv.and_then(|t| t.get("chg")?.as_f64()).or_else(|| prev.map(|p| price - p)).unwrap_or(0.0);
                let pct = tv.and_then(|t| t.get("pct")?.as_f64()).or_else(|| prev.map(|p| (chg / p) * 100.0)).unwrap_or(0.0);
                let start = cl.len().saturating_sub(45);
                Some(json!({ "sym": w.0, "ticker": w.1, "name": w.1, "price": price, "prev": prev, "chg": chg, "pct": pct, "closes": &cl[start..] }))
            }).collect();
            for item in join_all(tasks).await.into_iter().flatten() {
                out.push(item);
            }
        }

        let result = Value::Array(out);
        *self.world_memo.lock().await = Some(MemoItem {
            at: now,
            data: result.clone(),
        });
        Ok(result)
    }

    async fn gold_items(&self, url: &str) -> Result<Vec<Value>, String> {
        let headers = HashMap::from([
            ("user-agent".into(), self.user_agent.clone()),
            ("accept".into(), "application/json".into()),
        ]);
        let response = self.do_fetch(url, None, Some(&headers), 10_000).await?;
        if !response.ok {
            return Err(format!("gold fetch {}", response.status));
        }
        if let Ok(mut payload) = serde_json::from_str::<Value>(&response.body)
            && payload["success"] == true
            && let Value::Array(items) = payload["data"].take()
            && !items.is_empty()
        {
            Ok(items)
        } else {
            Err("gold invalid data".into())
        }
    }

    pub async fn gold(&self) -> Value {
        let mut memo = self.gold_memo.lock().await;
        let now = self.now_ms();
        if let Some(cached) = memo.as_ref()
            && now - cached.at < 15 * 60_000
        {
            return cached.data.clone();
        }
        let items = match self.gold_items(GOLD_URL).await {
            Ok(items) => items,
            Err(err) => {
                return memo.as_ref().map_or_else(
                    || json!({ "ok": false, "err": err }),
                    |cached| cached.data.clone(),
                );
            }
        };
        let one_gram = |item: &&Value| gold_number(item, "weight") == Some(1.0);
        let selected = items
            .iter()
            .filter(one_gram)
            .find(|it| antam_2026(it))
            .or_else(|| items.iter().find(one_gram))
            .unwrap_or(&items[0]);
        let price = gold_number(selected, "sellPrice").unwrap_or_default();
        let buyback = gold_number(selected, "buybackPrice").unwrap_or_default();
        let date = selected["recordedDate"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map_or_else(
                || {
                    chrono::DateTime::from_timestamp_millis(now)
                        .unwrap_or_default()
                        .date_naive()
                        .to_string()
                },
                str::to_owned,
            );

        let history = self
            .gold_items(&format!("{GOLD_URL}/history?weight=1&length=10"))
            .await
            .unwrap_or_default();
        let prefer_2026 = history.iter().filter(|it| antam_2026(it)).take(2).count() == 2;
        let mut seen = HashSet::new();
        let mut closes: Vec<_> = history
            .iter()
            .filter(|it| !prefer_2026 || antam_2026(it))
            .filter(|it| seen.insert(it.get("recordedDate").and_then(Value::as_str)))
            .map(|it| gold_number(it, "sellPrice").unwrap_or_default())
            .take(14)
            .collect();
        let prev_price = closes.get(1).copied().unwrap_or(price);
        let chg = price - prev_price;
        let pct = if prev_price > 0.0 {
            chg / prev_price * 100.0
        } else {
            0.0
        };
        if closes.len() >= 2 {
            closes.reverse();
        } else {
            closes = vec![price, price];
        }

        let mut tiers: Vec<_> = items.iter().filter(|it| antam_2026(it)).collect();
        tiers.sort_by(|a, b| {
            gold_number(a, "weight")
                .unwrap_or(f64::INFINITY)
                .total_cmp(&gold_number(b, "weight").unwrap_or(f64::INFINITY))
        });
        let tier_prices = |item: &Value| {
            json!({
                "weight": gold_number(item, "weight"),
                "sell": gold_number(item, "sellPrice").unwrap_or_default(),
            })
        };
        let tiers: Vec<_> = if tiers.is_empty() {
            items.iter().map(tier_prices).collect()
        } else {
            tiers
                .into_iter()
                .map(|item| {
                    let mut tier = tier_prices(item);
                    tier["unit"] = json!(
                        item["weightUnit"]
                            .as_str()
                            .filter(|s| !s.is_empty())
                            .unwrap_or("gr")
                    );
                    tier["buyback"] = json!(gold_number(item, "buybackPrice").unwrap_or_default());
                    tier
                })
                .collect()
        };
        let data = json!({
            "ok": true, "source": "anekalogam", "displayName": "Aneka Logam (Antam)",
            "date": date, "price": price, "buyback": buyback, "prevPrice": prev_price,
            "chg": chg, "pct": format!("{pct:.2}").parse::<f64>().ok(),
            "closes": closes, "tiers": tiers, "at": now,
        });
        *memo = Some(MemoItem {
            at: now,
            data: data.clone(),
        });
        data
    }

    pub async fn market(&self) -> Result<Value, String> {
        let now = self.now_ms();
        {
            let memo = self.mkt_memo.lock().await;
            if let Some(m) = &*memo
                && now - m.at < 45_000
            {
                return Ok(m.data.clone());
            }
        }

        let cols5 = json!(["name", "close", "change", "change_abs", "volume"]);
        let cols4 = json!(["name", "close", "change", "change_abs"]);
        let cmd_fut: Vec<Value> = COMMODITIES
            .iter()
            .filter(|c| c.2 == "futures")
            .map(|c| json!(c.0))
            .collect();
        let cmd_crypt: Vec<Value> = COMMODITIES
            .iter()
            .filter(|c| c.2 == "crypto")
            .map(|c| json!(c.0))
            .collect();

        let (idx, fx, cmd_f, cmd_c, ihsg_r, idr_q, world, ihsg_closes, gold) = tokio::join!(
            self.tv_scan("indonesia", json!({ "columns": cols5, "range": [0, 900], "symbols": { "query": { "types": ["stock"] }, "tickers": [] } })),
            self.tv_scan("forex", json!({ "columns": cols4, "symbols": { "query": { "types": [] }, "tickers": ["FX_IDC:USDIDR"] } })),
            self.tv_scan("futures", json!({ "columns": cols4, "symbols": { "query": { "types": [] }, "tickers": cmd_fut } })),
            self.tv_scan("crypto", json!({ "columns": cols4, "symbols": { "query": { "types": [] }, "tickers": cmd_crypt } })),
            self.tv_scan("indonesia", json!({ "columns": cols5, "symbols": { "query": { "types": [] }, "tickers": ["IDX:COMPOSITE"] } })),
            self.usdidr(),
            self.world_index(),
            self.fetch_symbol_closes("IDX:COMPOSITE"),
            self.gold()
        );

        let idx = idx.unwrap_or_default();
        let fx = fx.unwrap_or_default();
        let cmd_f = cmd_f.unwrap_or_default();
        let cmd_c = cmd_c.unwrap_or_default();
        let ihsg_r = ihsg_r.unwrap_or_default();
        let world = world.unwrap_or_else(|_| json!([]));

        let memo_guard = self.close_memo.lock().await;
        let mut stocks = Vec::new();
        for s in idx {
            let mut it = parse_row("IDX", &s);
            let ticker = it
                .get("ticker")
                .and_then(|v| v.as_str())
                .unwrap_or_default();
            it["blue"] = json!(STOCK_BLUE.contains(&ticker));
            let sym = it.get("sym").and_then(|v| v.as_str()).unwrap_or_default();
            if let Some(cached) = memo_guard.get(sym)
                && cached.closes.len() >= 2
            {
                it["closes"] = json!(cached.closes);
            }
            let price = it.get("price").and_then(|v| v.as_f64()).unwrap_or(0.0);
            if price.is_finite() && price > 0.0 {
                stocks.push(it);
            }
        }
        if stocks.is_empty() {
            return Err("market kosong".to_string());
        }

        let mut ihsg = if let Some(first) = ihsg_r.first() {
            let mut it = parse_row("IDX", first);
            it["name"] = json!("IHSG");
            it["ticker"] = json!("IHSG");
            if ihsg_closes.len() >= 2 {
                it["closes"] = json!(ihsg_closes);
            }
            Some(it)
        } else {
            None
        };
        if ihsg.is_none() && ihsg_closes.len() >= 2 {
            ihsg = Some(
                json!({ "sym": "IDX:COMPOSITE", "ticker": "IHSG", "name": "IHSG", "price": ihsg_closes.last().copied().unwrap_or(0.0), "closes": ihsg_closes }),
            );
        }

        let mut forex = Vec::new();
        for s in fx {
            let mut it = parse_row("FX", &s);
            let ticker = it
                .get("ticker")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string();
            for (p_ticker, p_name) in FOREX_PAIRS {
                if *p_ticker == ticker {
                    it["name"] = json!(p_name);
                }
            }
            if it
                .get("price")
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0)
                .is_finite()
            {
                forex.push(it);
            }
        }

        let mut cmd_rows = cmd_f;
        cmd_rows.extend(cmd_c);
        for cmd in COMMODITIES {
            if let Some(s) = cmd_rows
                .iter()
                .find(|x| x.get("s").and_then(|v| v.as_str()) == Some(cmd.0))
            {
                let mut it = parse_row("CMD", s);
                it["name"] = json!(cmd.1);
                if let Some(cached) = memo_guard.get(cmd.0)
                    && cached.closes.len() >= 2
                {
                    it["closes"] = json!(cached.closes);
                }
                if it
                    .get("price")
                    .and_then(|v| v.as_f64())
                    .unwrap_or(0.0)
                    .is_finite()
                {
                    forex.push(it);
                }
            }
        }
        drop(memo_guard);

        if let Some(idr_q) = idr_q {
            for f in &mut forex {
                if f.get("ticker").and_then(|v| v.as_str()) == Some("USDIDR") {
                    f["price"] = json!(idr_q.price);
                    f["chg"] = json!(idr_q.chg);
                    f["pct"] = json!(idr_q.pct);
                    f["closes"] = json!(idr_q.closes);
                }
            }
        }

        if gold["ok"] == true && gold["price"].as_f64().is_some_and(|p| p != 0.0) {
            forex.push(json!({
                "sym": "EMAS:ANTAM", "ticker": "ANTAM", "name": "Emas 1g",
                "price": gold["price"], "pct": gold["pct"], "chg": gold["chg"],
                "closes": gold["closes"], "vol": 0, "mkt": "LOGAMMULIA"
            }));
        }
        let result = json!({
            "ok": true,
            "ihsg": ihsg,
            "stocks": { "ok": true, "items": stocks },
            "forex": { "ok": true, "items": forex },
            "world": world,
            "gold": gold,
            "at": now
        });

        *self.mkt_memo.lock().await = Some(MemoItem {
            at: now,
            data: result.clone(),
        });
        Ok(result)
    }

    fn clone_ref(&self) -> Self {
        Self {
            hist_dir: self.hist_dir.clone(),
            user_agent: self.user_agent.clone(),
            client: self.client.clone(),
            fetch_fn: self.fetch_fn.clone(),
            now_fn: self.now_fn.clone(),
            close_memo: self.close_memo.clone(),
            fxhist: self.fxhist.clone(),
            yahoo_blocked_until: self.yahoo_blocked_until.clone(),
            mkt_closes_dirty: self.mkt_closes_dirty.clone(),
            mkt_closes_save_at: self.mkt_closes_save_at.clone(),
            mkt_memo: self.mkt_memo.clone(),
            world_memo: self.world_memo.clone(),
            gold_memo: self.gold_memo.clone(),
        }
    }
}

fn parse_row(market_name: &str, s: &Value) -> Value {
    let sym = s.get("s").and_then(|v| v.as_str()).unwrap_or_default();
    let d = s.get("d").and_then(|v| v.as_array());
    let ticker = d
        .and_then(|arr| arr.first())
        .and_then(|v| v.as_str())
        .unwrap_or_default();
    let price = d
        .and_then(|arr| arr.get(1))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let pct = d
        .and_then(|arr| arr.get(2))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let chg = d
        .and_then(|arr| arr.get(3))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    let vol = d
        .and_then(|arr| arr.get(4))
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0);
    json!({ "sym": sym, "ticker": ticker, "name": ticker, "price": price, "pct": pct, "chg": chg, "vol": vol, "mkt": market_name })
}

fn days_to_ymd(days: i64) -> (i64, i64, i64) {
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let y = y + if m <= 2 { 1 } else { 0 };
    (y, m, d)
}
