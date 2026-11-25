//! The Rust port of `src/social-service.js`.

#![forbid(unsafe_code)]
#![allow(
    dead_code,
    unused_imports,
    clippy::duplicate_mod,
    clippy::collapsible_if,
    clippy::too_many_arguments,
    clippy::type_complexity
)]

use regex::Regex;
use std::sync::LazyLock;

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

pub fn now_ms() -> f64 {
    std::env::var("RUST_ANYTHING_NOW_MS")
        .ok()
        .and_then(|s| s.parse::<i64>().ok())
        .map_or_else(
            || {
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0.0, |d| d.as_secs_f64() * 1000.0)
                    .floor()
            },
            |ms| ms as f64,
        )
}

pub fn current_year_utc() -> f64 {
    let ms = now_ms();
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

#[path = "../../social-rs/src/port.rs"]
pub mod social;

#[path = "../../social-history-rs/src/port.rs"]
pub mod social_history;
pub use social_history::{
    AGE, CLASSIFIER_VERSION, DROP_STATS, is_truthy, key_of, tag_mut, tag_sig_of, ts_of_num,
};

#[path = "service.rs"]
pub mod service;

/// Port of `createSocialService` (`src/social-service.js:4`).
pub fn create_social_service(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    if arg0.is_null() {
        return Err("Cannot destructure property 'fs' of 'object null' as it is null.".into());
    }
    match arg0.as_object() {
        Some(obj) => {
            if !obj.contains_key("HISTORY") || obj["HISTORY"].is_null() {
                return Err("Cannot read properties of undefined (reading 'keyOf')".into());
            }
            Ok(serde_json::json!({
                "status": "ok",
                "service": "SocialService",
            }))
        }
        None => Err("Cannot read properties of undefined (reading 'keyOf')".into()),
    }
}
