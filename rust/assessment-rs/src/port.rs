//! The Rust port of `src/assessment.js`.

#![forbid(unsafe_code)]

#[path = "../../indonesian-nlp-rs/src/analysis.rs"]
#[allow(dead_code)]
pub mod analysis;
#[path = "../../indonesian-nlp-rs/src/frames.rs"]
#[allow(dead_code)]
pub mod frames;
#[path = "../../geocode-rs/src/service.rs"]
#[allow(dead_code)]
pub mod geo_service;
mod pinned;
#[path = "service.rs"]
pub mod service;
#[path = "../../indonesian-nlp-rs/src/tables.rs"]
#[allow(dead_code)]
pub mod tables;

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

use crate::port::tables::{
    ABBR_DOT_RE, FOREIGN_MARKER_RE, HAS_CURRENT_TIME_RE, HIST_CLEAN_1_RE, HIST_CLEAN_2_RE,
    HIST_NOSTALGIA_RE, HISTORICAL_YEAR_RE, LOCAL_MACRO_RE, LOCAL_ORG_RE, URL_RE, WS_RE, fre, re,
};

pub static ASSESS_HISTORICAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(dulu|jaman dulu|zaman dulu|tempo dulu|sejarah|mengenang|kilas balik|tahun lalu|bulan lalu|pekan lalu|kemarin|last (?:year|month|week)|historical|anniversary|orde baru|orba|soeharto|suharto|perang dunia|world war|penjajahan|kolonial|(?:jaman|zaman|waktu|pas|saat|era|awal|masa|suasana|situasi|musim|periode|ketika|selama)\s+(?:covid(?:-?19)?|pandemi|lockdown|korona|corona)|(?:2019|2020|2021|2022)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:covid|pandemi|lockdown))(?-u:\b)",
    )
});

static ASSESS_CLAUSE_SPLIT_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| fre(r"\n+|[!?;]+|\.(?=\s+[A-Za-z\x{00C0}-\x{024F}])"));

static WEAK_PREC_RE: LazyLock<Regex> = LazyLock::new(|| re(r"^(desa|kelurahan|poi)$"));

fn js_is_truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    if let Some(units) = crate::js::string_units(v) {
        return !units.is_empty();
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_to_string(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    if let Some(units) = crate::js::string_units(v) {
        return String::from_utf16_lossy(&units);
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let mut out = String::new();
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if !item.is_null() && !crate::js::is_undefined(item) {
                    out.push_str(&js_to_string(item));
                }
            }
            out
        }
        Value::Object(_) => "[object Object]".into(),
    }
}

fn js_or_empty_string(v: &Value) -> String {
    if !js_is_truthy(v) {
        String::new()
    } else {
        js_to_string(v)
    }
}

fn parse_js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_to_number(v: &Value) -> f64 {
    if crate::js::is_undefined(v) {
        return f64::NAN;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    if let Some(units) = crate::js::string_units(v) {
        return parse_js_number(&String::from_utf16_lossy(&units));
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
        Value::String(s) => parse_js_number(s),
        Value::Array(_) => parse_js_number(&js_to_string(v)),
        Value::Object(_) => f64::NAN,
    }
}

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

/// Port of `isHistorical` (`src/assessment.js:59`).
pub fn is_historical(
    span: serde_json::Value,
    year: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let span_raw = js_to_string(&span);
    let span_or_empty = js_or_empty_string(&span);
    let y = match year {
        None => current_year_utc(),
        Some(v) if crate::js::is_undefined(&v) => current_year_utc(),
        Some(v) => js_to_number(&v),
    };
    Ok(Value::Bool(is_historical_str(&span_raw, &span_or_empty, y)))
}

pub fn norm_str(s: &str) -> String {
    let lower: String = s.to_lowercase().nfkc().collect();
    let no_url = URL_RE.replace_all(&lower, " ");
    let collapsed = WS_RE.replace_all(&no_url, " ");
    collapsed.trim().to_string()
}

/// Port of `norm` (`src/assessment.js:77`).
pub fn norm(s: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(Value::String(norm_str(&js_or_empty_string(&s))))
}

pub fn clauses_str(text: &str) -> Vec<String> {
    let decoded = analysis::decode_basic_entities_str(text);
    let nfkc: String = decoded.nfkc().collect();
    let no_url = URL_RE.replace_all(&nfkc, " ");
    let clean = ABBR_DOT_RE.replace_all(&no_url, "$1\u{0000}");
    let mut parts = Vec::new();
    let mut last = 0;
    let mut pos = 0;
    while pos <= clean.len() {
        match ASSESS_CLAUSE_SPLIT_RE.find_from_pos(&clean, pos) {
            Ok(Some(m)) => {
                parts.push(&clean[last..m.start()]);
                last = m.end();
                pos = if m.end() > pos { m.end() } else { pos + 1 };
            }
            _ => break,
        }
    }
    parts.push(&clean[last..]);
    parts
        .into_iter()
        .map(|s| s.replace('\u{0000}', ".").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Port of `clauses` (`src/assessment.js:84`).
pub fn clauses(text: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_or_empty_string(&text);
    Ok(Value::Array(
        clauses_str(&s).into_iter().map(Value::String).collect(),
    ))
}

fn strip_combining_marks(s: &str) -> String {
    s.nfd()
        .filter(|&c| !('\u{0300}'..='\u{036f}').contains(&c))
        .collect()
}

fn copy_location_fields(loc: &Value) -> Value {
    let mut m = Map::new();
    for k in ["name", "prov", "prec", "la", "lo"] {
        if let Some(v) = loc.get(k) {
            m.insert(k.to_string(), v.clone());
        }
    }
    for k in ["provCode", "kabCode", "kecCode", "code"] {
        if let Some(v) = loc.get(k)
            && js_is_truthy(v)
        {
            m.insert(k.to_string(), v.clone());
        }
    }
    Value::Object(m)
}

pub fn classify_scope_str(span: &str) -> Value {
    let location = geo_service::locate_text(span);
    let foreign = FOREIGN_MARKER_RE.is_match(&strip_combining_marks(span));
    if foreign {
        let weak = location.as_ref().is_some_and(|loc| {
            let prec = loc.get("prec").and_then(Value::as_str).unwrap_or("");
            WEAK_PREC_RE.is_match(prec)
        });
        let scope = if (location.is_some() && !weak) || LOCAL_ORG_RE.is_match(span) {
            "unknown"
        } else {
            "foreign"
        };
        return json!({ "scope": scope, "location": Value::Null });
    }
    let local = location.is_some() || LOCAL_ORG_RE.is_match(span) || LOCAL_MACRO_RE.is_match(span);
    let loc_val = location
        .as_ref()
        .map(copy_location_fields)
        .unwrap_or(Value::Null);
    json!({
        "scope": if local { "Indonesia" } else { "unknown" },
        "location": loc_val,
    })
}

/// Port of `classifyScope` (`src/assessment.js:89`).
pub fn classify_scope(
    span: serde_json::Value,
    _target: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if crate::js::is_undefined(&span) {
        return Err("Cannot read properties of undefined (reading 'normalize')".into());
    }
    if span.is_null() {
        return Err("Cannot read properties of null (reading 'normalize')".into());
    }
    let s = if let Some(units) = crate::js::string_units(&span) {
        String::from_utf16_lossy(&units)
    } else if let Some(s) = span.as_str() {
        s.to_string()
    } else {
        return Err("span.normalize is not a function".into());
    };
    Ok(classify_scope_str(&s))
}
