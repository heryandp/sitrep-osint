//! The Rust port of `src/news-service.js`.

#![forbid(unsafe_code)]
#![allow(
    dead_code,
    unused_imports,
    clippy::duplicate_mod,
    clippy::inconsistent_digit_grouping,
    clippy::collapsible_if,
    clippy::unnecessary_map_or
)]

pub mod pinned;

#[path = "../../indonesian-nlp-rs/src/analysis.rs"]
#[allow(dead_code)]
pub mod analysis;

#[path = "../../indonesian-nlp-rs/src/frames.rs"]
#[allow(dead_code)]
pub mod frames;

#[path = "../../geocode-rs/src/service.rs"]
#[allow(dead_code)]
pub mod geo_service;

#[path = "../../indonesian-nlp-rs/src/tables.rs"]
#[allow(dead_code)]
pub mod tables;

pub use tables::{
    ABBR_DOT_RE, CEREMONY_RE, FOREIGN_MARKER_RE, GEO_AUTHOR_CURRENT, GEO_AUTHOR_HOMETOWN,
    GEO_EVENT_LOCATION, GEO_HISTORICAL, GEO_MENTIONED_IN_PASSING, GEO_READER_LOCATION,
    HAS_CURRENT_TIME_RE, HIST_CLEAN_1_RE, HIST_CLEAN_2_RE, HIST_NOSTALGIA_RE, HISTORICAL_YEAR_RE,
    LOCAL_MACRO_RE, LOCAL_ORG_RE, URL_RE, WS_RE, fre, metaphor_blocks_topic_str, re,
};

pub static ASSESS_HISTORICAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(dulu|jaman dulu|zaman dulu|tempo dulu|sejarah|mengenang|kilas balik|tahun lalu|bulan lalu|pekan lalu|kemarin|last (?:year|month|week)|historical|anniversary|orde baru|orba|soeharto|suharto|perang dunia|world war|penjajahan|kolonial|(?:jaman|zaman|waktu|pas|saat|era|awal|masa|suasana|situasi|musim|periode|ketika|selama)\s+(?:covid(?:-?19)?|pandemi|lockdown|korona|corona)|(?:2019|2020|2021|2022)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:covid|pandemi|lockdown))(?-u:\b)",
    )
});

pub fn current_year_utc() -> f64 {
    let ms = pinned::now_ms();
    let z = (ms / 86_400_000.0).floor() as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2)) as f64
}

pub fn is_historical_str(span_raw: &str, span_or_empty: &str, year: f64) -> bool {
    let has_current_time = HAS_CURRENT_TIME_RE.is_match(span_raw);
    let c1 = HIST_CLEAN_1_RE.replace_all(span_or_empty, " ");
    let cleaned = HIST_CLEAN_2_RE.replace_all(&c1, " ");
    if has_current_time && !HIST_NOSTALGIA_RE.is_match(span_raw) {
        return false;
    }
    if ASSESS_HISTORICAL_RE.is_match(&cleaned) {
        return true;
    }
    if let Some(caps) = HISTORICAL_YEAR_RE.captures(&cleaned)
        && let Some(m) = caps.get(1)
        && let Ok(y) = m.as_str().parse::<f64>()
    {
        return y < year;
    }
    false
}

#[path = "service.rs"]
pub mod service;
pub use service::NewsService;

use crate::js;
use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;
use url::Url;

static BANNED_SRC: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)infopublik\.id|infopublik|kompasiana|koran\.tempo\.co|majalah\.tempo\.co|plus\.tempo\.co|tempo\.co/(?:plus|langganan|subscribe)\b|mozaik\.inilah\.com|mediacenter\.palangkaraya\.go\.id|viva\.co\.id/foto|nokenlive|polreskedirikota|korankaltim|pelakita|cilegon\.go\.id|twaslnews|kilangbara\.com|klikmu\.co|aboutcirebon\.id|about\s*cirebon").expect("valid regex")
});

static BANNED_SOCIAL_HOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|\.)(?:instagram\.com|instagr\.am|tiktok\.com|vt\.tiktok\.com|vm\.tiktok\.com|facebook\.com|fb\.com|fb\.watch|threads\.net|threads\.com|x\.com|twitter\.com)$").expect("valid regex")
});

static BANNED_SOCIAL_PUB_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(?:instagram|tiktok|facebook|threads|x|twitter)$").expect("valid regex")
});

static CLEAN_PUB_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(https?:)?//|^www\.").expect("valid regex"));

static WWW_PREFIX_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^www\.").expect("valid regex"));

static BREAK_NORM_DASH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\s+-\s+[^-]{2,45}$").expect("valid regex"));

static NON_ALPHANUM_SPACE_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^a-z0-9 ]").expect("valid regex"));

static MULTI_SPACE_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").expect("valid regex"));

pub(crate) fn js_to_string(v: &Value) -> String {
    if js::is_undefined(v) {
        return "undefined".to_string();
    }
    if let Some(s) = js::string_units(v) {
        return String::from_utf16_lossy(&s);
    }
    if let Some(n) = js::as_number(v) {
        return js::number_to_string(n);
    }
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        Value::Number(n) => n
            .as_f64()
            .map_or_else(|| n.to_string(), js::number_to_string),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let parts: Vec<String> = arr
                .iter()
                .map(|item| {
                    if item.is_null() || js::is_undefined(item) {
                        String::new()
                    } else {
                        js_to_string(item)
                    }
                })
                .collect();
            parts.join(",")
        }
        Value::Object(o) => {
            if let Some(b) = o.get("$bigint").and_then(|v| v.as_str()) {
                b.to_string()
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

pub(crate) fn is_truthy(v: &Value) -> bool {
    if v.is_null() || js::is_undefined(v) {
        return false;
    }
    if let Some(n) = js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().map_or(false, |f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => true,
        Value::Null => false,
    }
}

fn js_to_number(v: &Value) -> f64 {
    if let Some(n) = js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(_) => {
            let s = js_to_string(v);
            let trimmed = s.trim();
            if trimmed.is_empty() {
                0.0
            } else {
                trimmed.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Object(_) => f64::NAN,
    }
}

/// Port of `cleanPub` (`src/news-service.js:9`).
pub fn clean_pub(v: Value) -> Result<Value, String> {
    let v_str = if is_truthy(&v) {
        js_to_string(&v)
    } else {
        String::new()
    };
    let v_str = v_str.trim();
    if CLEAN_PUB_PREFIX_RE.is_match(v_str) {
        let to_parse = if WWW_PREFIX_RE.is_match(v_str) {
            format!("https://{v_str}")
        } else {
            v_str.to_string()
        };
        if let Ok(url) = Url::parse(&to_parse) {
            if let Some(host) = url.host_str() {
                let host_clean = WWW_PREFIX_RE.replace(host, "");
                return Ok(Value::String(host_clean.into_owned()));
            }
        }
        return Ok(Value::String(String::new()));
    }
    Ok(Value::String(v_str.to_string()))
}

pub(crate) fn is_banned_host_url_internal(u: &Value) -> bool {
    if !is_truthy(u) {
        return false;
    }
    let s = js_to_string(u);
    if BANNED_SRC.is_match(&s) {
        return true;
    }
    if let Ok(url) = Url::parse(&s) {
        if let Some(host) = url.host_str() {
            let h = WWW_PREFIX_RE.replace(host, "");
            if BANNED_SOCIAL_HOST_RE.is_match(&h) {
                return true;
            }
        }
    }
    false
}

/// Port of `isBannedHostUrl` (`src/news-service.js:15`).
pub fn is_banned_host_url(u: Value) -> Result<Value, String> {
    Ok(Value::Bool(is_banned_host_url_internal(&u)))
}

/// Port of `isBannedNewsItem` (`src/news-service.js:26`).
pub fn is_banned_news_item(it: Value) -> Result<Value, String> {
    if !is_truthy(&it) {
        return Ok(Value::Bool(false));
    }
    let pub_val = it.get("pub").unwrap_or(&Value::Null);
    let pub_str = if is_truthy(pub_val) {
        js_to_string(pub_val).trim().to_string()
    } else {
        String::new()
    };
    if !pub_str.is_empty() && BANNED_SOCIAL_PUB_RE.is_match(&pub_str) {
        return Ok(Value::Bool(true));
    }
    let url_val = it.get("url").unwrap_or(&Value::Null);
    let src_url_val = it.get("srcUrl").unwrap_or(&Value::Null);
    let final_val = it.get("final").unwrap_or(&Value::Null);
    if is_banned_host_url_internal(url_val)
        || is_banned_host_url_internal(src_url_val)
        || is_banned_host_url_internal(final_val)
    {
        return Ok(Value::Bool(true));
    }
    let title_val = it.get("title").unwrap_or(&Value::Null);
    let title_str = if is_truthy(title_val) {
        js_to_string(title_val)
    } else {
        String::new()
    };
    let url_str = if is_truthy(url_val) {
        js_to_string(url_val)
    } else {
        String::new()
    };
    let src_url_str = if is_truthy(src_url_val) {
        js_to_string(src_url_val)
    } else {
        String::new()
    };
    let bid = format!("{title_str} {url_str} {pub_str} {src_url_str}");
    Ok(Value::Bool(BANNED_SRC.is_match(&bid)))
}

/// Port of `breakNorm` (`src/news-service.js:35`).
pub fn break_norm(t: Value) -> Result<Value, String> {
    let s = js_to_string(&t).to_lowercase();
    let s = BREAK_NORM_DASH_RE.replace(&s, "");
    let s = NON_ALPHANUM_SPACE_RE.replace_all(&s, "");
    let s = MULTI_SPACE_RE.replace_all(&s, " ");
    let s = s.trim();
    Ok(Value::String(s.to_string()))
}

/// Port of `wibDayStartMs` (`src/news-service.js:37`).
pub fn wib_day_start_ms(now: Value) -> Result<Value, String> {
    if js::is_undefined(&now) {
        let t = pinned::now_ms();
        let res = ((t + 7.0 * 3600_000.0) / 86400_000.0).floor() * 86400_000.0 - 7.0 * 3600_000.0;
        return Ok(js::number(res));
    }
    if let Value::Object(o) = &now {
        if o.get("$bigint").is_some() {
            return Err("Cannot mix BigInt and other types, use explicit conversions".to_string());
        }
    }
    let num = js_to_number(&now);
    let res = ((num + 7.0 * 3600_000.0) / 86400_000.0).floor() * 86400_000.0 - 7.0 * 3600_000.0;
    Ok(js::number(res))
}
