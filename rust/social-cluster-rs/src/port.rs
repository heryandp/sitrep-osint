//! The Rust port of `src/social-cluster.js`.

#![forbid(unsafe_code)]

#[path = "helpers.rs"]
pub mod helpers;
mod pinned;

pub use helpers::jaccard_similarity;

use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use regex::Regex;
use serde_json::{Map, Value, json};
use unicode_normalization::UnicodeNormalization;

use crate::js;

macro_rules! re {
    ($pat:literal) => {
        LazyLock::new(|| {
            // INFALLIBLE: compile-time constant regex literal
            Regex::new($pat).unwrap()
        })
    };
}

macro_rules! word_set {
    ($s:expr) => {
        LazyLock::new(|| $s.split_whitespace().collect())
    };
}

static OFFICIAL_HANDLES: LazyLock<HashSet<&'static str>> = word_set!(
    "ditjenpajakri kring_pajak kemenkeuri infobmkg bnpb_indonesia basarnas damkardki \
     tmcpoldametro halo_polisi divhumas_polri puspentni kemkesri bssn_ri pvmbg_ pvmbg \
     bpbd_dki bpbdjabar bpbdjatim bpbdjateng pln_123 infobps kemenhub151 komdigi_id kemenkominfo"
);
static NEWS_OUTLET_HANDLES: LazyLock<HashSet<&'static str>> = word_set!(
    "detikcom kompascom kompastv cnnindonesia cnbcindonesia antaranews liputan6 tempodotco \
     tribunnews tribunjogja republikaonline merdekadotcom sindonews okezonecom jawapos \
     inewsdotid tvonenews metrotv elshinta elshinta90fm suarasurabayamedia kumparancom \
     vivanews bisniscom kontannews rri_co_id beritasatu idntimes tirtoid vice_id ddtcnews ortaxid"
);
static STOP_WORDS: LazyLock<HashSet<&'static str>> = word_set!(
    "yang dan di ke dari untuk pada dengan adalah ini itu ada tidak akan juga telah sudah bisa \
     lebih sangat para dia kami kita mereka sebuah seorang suatu saat ketika setelah sebelum \
     hingga oleh karena maka jika hari minggu senin selasa rabu kamis jumat sabtu kini tadi \
     barusan sekarang baru saja info update berita kabar terkini viral breaking news via oleh rt"
);
static GENERIC_EVENT_TOKENS: LazyLock<HashSet<&'static str>> = word_set!(
    "banjir gempa kebakaran kecelakaan longsor demo unjuk rasa warga rumah jalan kota kabupaten \
     provinsi indonesia jakarta bandung surabaya bekasi depok bogor tangerang semarang terjadi \
     melanda merendam terbakar ambruk korban evakuasi luka tewas meninggal polisi bpbd bmkg damkar bnpb"
);
static RARE_TYPES: LazyLock<HashSet<&'static str>> =
    word_set!("earthquake tsunami eruption ransomware data_breach ddos maritime_accident wildfire");

static OFFICIAL_HANDLE_RE: LazyLock<Regex> = re!(
    r"(?i)^(?:info)?(?:bmkg|bnpb|bpbd|basarnas|pvmbg|bssn|damkar|polres|polda|polsek|tmcpolda|divhumas|puspen|kemkes|kemenkes|kemenhub|komdigi|pemprov|pemkot|pemkab|ditjenpajak|kring_?pajak|kemenkeu|kanwildjp|kpp)(?:[._-]|$)"
);
static NEWS_HANDLE_RE: LazyLock<Regex> = re!(
    r"(?i)(?:detik|kompas|cnnind|cnbcind|antara|liputan6|tempo|tribun|republika|merdeka|sindo|okezone|jawapos|inews|tvone|metrotv|elshinta|suarasurabaya|kumparan|beritasatu|rri)(?:com|tv|id|news|dotco|fm|media)?$"
);
static RE_RT_CHECK: LazyLock<Regex> = re!(r"(?i)^[\s\u{FEFF}]*rt[\s\u{FEFF}]+@");
static RE_BOIL_RT: LazyLock<Regex> =
    re!(r"(?i)^[\s\u{FEFF}]*(?:rt|repost)[\s\u{FEFF}]+@[A-Za-z0-9_.]+:?[\s\u{FEFF}]*");
static RE_BOIL_VIA: LazyLock<Regex> = re!(
    r"(?i)(?-u:\b)(?:via|sumber|source|credit|cc)[\s\u{FEFF}]*[:@]?[\s\u{FEFF}]*@?[A-Za-z0-9_.]+(?-u:\b)"
);
static RE_BOIL_URL: LazyLock<Regex> = re!(r"(?i)https?://[^\s\u{FEFF}]+|www\.[^\s\u{FEFF}]+");
static RE_BOIL_MENTION: LazyLock<Regex> = re!(r"@[A-Za-z0-9_.]+");
static RE_BOIL_HASHTAG: LazyLock<Regex> = re!(r"#[\p{L}\p{N}_]+");
static RE_BOIL_NON_ALNUM: LazyLock<Regex> = re!(r"[^\p{L}\p{N}]+");
static RE_BOIL_WS: LazyLock<Regex> = re!(r"[\s\u{FEFF}]+");
static MICRO_QUAKE_RE: LazyLock<Regex> = re!(
    r"(?i)(?-u:\b)(?:info[\s\u{FEFF}]+gempa(?:[\s\u{FEFF}]+[^.\n]{0,80}(?-u:\b)mag(?:nitudo)?)?|gempa[\s\u{FEFF}]+mag(?:nitudo)?)[\s\u{FEFF}]*[:=]?[\s\u{FEFF}]*([0-9]+(?:[.,][0-9]+)?)"
);
static QUAKE_MAG_RE: LazyLock<Regex> = re!(
    r"(?i)(?:gempa[^.\n]{0,60}(?-u:\b)(?:mag(?:nitudo)?|sr|(?-u:\b)m)[\s\u{FEFF}]*[:=]?[\s\u{FEFF}]*([0-9]+(?:[.,][0-9]+)?)|([0-9]+(?:[.,][0-9]+)?)[\s\u{FEFF}]*(?:sr|mag|magnitudo)(?-u:\b))"
);
static QUAKE_IMPACT_RE: LazyLock<Regex> = re!(
    r"(?i)(?-u:\b)(?:dirasakan|terasa|kerasa|(?-u:\b)mmi(?-u:\b)|tsunami|rusak|roboh|retak|korban|evakuasi|panik|susulan|mengguncang|guncang|bergetar|bergoyang|berhamburan|luka|tewas|meninggal|ambruk|hancur)(?-u:\b)"
);

fn provenance_weight(source_type: &str) -> f64 {
    match source_type {
        "official_agency" => 1.0,
        "news_outlet" => 0.85,
        "eyewitness" => 0.72,
        _ => 0.35,
    }
}

fn is_official_handle(h: &str) -> bool {
    !h.is_empty() && (OFFICIAL_HANDLES.contains(h) || OFFICIAL_HANDLE_RE.is_match(h))
}

fn is_news_handle(h: &str) -> bool {
    !h.is_empty() && (NEWS_OUTLET_HANDLES.contains(h) || NEWS_HANDLE_RE.is_match(h))
}

fn is_marker(v: &Value, key: &str) -> bool {
    v.as_object()
        .is_some_and(|o| o.len() == 1 && o.contains_key(key))
}

fn is_js_nullish(v: &Value) -> bool {
    v.is_null() || js::is_undefined(v)
}

fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) if js::is_undefined(v) => false,
        Value::Object(_) if is_marker(v, "$num") => {
            js::as_number(v).is_some_and(|x| x != 0.0 && !x.is_nan())
        }
        Value::Object(_) => js::string_units(v).is_none_or(|u| !u.is_empty()),
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(items) => items
            .iter()
            .map(|x| {
                if is_js_nullish(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) if js::is_undefined(v) => "undefined".into(),
        Value::Object(_) if is_marker(v, "$num") => {
            js::number_to_string(js::as_number(v).unwrap_or(f64::NAN))
        }
        Value::Object(_) if is_marker(v, "$set") => "[object Set]".into(),
        Value::Object(_) if is_marker(v, "$map") => "[object Map]".into(),
        Value::Object(_) => js::string_units(v)
            .map(|u| String::from_utf16_lossy(&u))
            .unwrap_or_else(|| "[object Object]".into()),
    }
}

fn js_str(v: Option<&Value>) -> String {
    v.filter(|x| is_truthy(x))
        .map(js_to_string)
        .unwrap_or_default()
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn js_str_to_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    match t {
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (p1, p2, radix) in [("0x", "0X", 16), ("0b", "0B", 2), ("0o", "0O", 8)] {
        if let Some(rest) = t.strip_prefix(p1).or_else(|| t.strip_prefix(p2)) {
            return u64::from_str_radix(rest, radix)
                .map(|n| n as f64)
                .unwrap_or(f64::NAN);
        }
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_str_to_number(s),
        Value::Array(_) => js_str_to_number(&js_to_string(v)),
        Value::Object(_) if js::is_undefined(v) => f64::NAN,
        Value::Object(_) if is_marker(v, "$num") => js::as_number(v).unwrap_or(f64::NAN),
        Value::Object(_) => js::string_units(v)
            .map(|u| js_str_to_number(&String::from_utf16_lossy(&u)))
            .unwrap_or(f64::NAN),
    }
}

fn js_num_or(v: Option<&Value>, default: f64) -> f64 {
    let n = v.map(js_to_number).unwrap_or(f64::NAN);
    if n.is_nan() || n == 0.0 { default } else { n }
}

fn prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    v.as_object()
        .filter(|o| {
            !(o.len() == 1
                && ["$undefined", "$num", "$utf16", "$set", "$map"]
                    .iter()
                    .any(|m| o.contains_key(*m)))
        })
        .and_then(|o| o.get(key))
}

fn truthy_prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    prop(v, key).filter(|x| is_truthy(x))
}

fn js_strict_eq(a: &Value, b: &Value) -> bool {
    if js::is_undefined(a) || js::is_undefined(b) {
        return js::is_undefined(a) && js::is_undefined(b);
    }
    if let (Some(na), Some(nb)) = (js::as_number(a), js::as_number(b)) {
        return na == nb;
    }
    a == b
}

fn js_date_parse(v: Option<&Value>) -> f64 {
    let raw = js_str(v);
    let s = js_trim(&raw);
    if s.is_empty() {
        return f64::NAN;
    }
    if let Ok(dt) = DateTime::parse_from_rfc3339(s)
        .or_else(|_| DateTime::parse_from_rfc2822(s))
        .or_else(|_| DateTime::parse_from_str(s, "%a %b %d %H:%M:%S %z %Y"))
    {
        return dt.timestamp_millis() as f64;
    }
    if let Ok(d) = NaiveDate::parse_from_str(s, "%Y-%m-%d")
        && let Some(ndt) = d.and_hms_opt(0, 0, 0)
    {
        return DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc).timestamp_millis() as f64;
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(ndt) = NaiveDateTime::parse_from_str(s, fmt) {
            return DateTime::<Utc>::from_naive_utc_and_offset(ndt, Utc).timestamp_millis() as f64;
        }
    }
    f64::NAN
}

fn post_ts(post: &Value, fallback: f64) -> f64 {
    let n = prop(post, "ts").map(js_to_number).unwrap_or(f64::NAN);
    if !n.is_nan() && n != 0.0 {
        return n;
    }
    let d = js_date_parse(prop(post, "created_at"));
    if !d.is_nan() && d != 0.0 { d } else { fallback }
}

fn js_to_fixed_num(x: f64, digits: i32) -> f64 {
    if !x.is_finite() {
        return x;
    }
    let f = 10f64.powi(digits);
    let r = (x * f).round() / f;
    if r == 0.0 { 0.0 } else { r }
}

fn js_cmp_or(a: f64, b: f64) -> f64 {
    if a.is_nan() || a == 0.0 { b } else { a }
}

fn check_opts(opts: Value, field: &str) -> Result<Value, String> {
    if opts.is_null() {
        Err(format!(
            "TypeError: Cannot read properties of null (reading '{field}')"
        ))
    } else if js::is_undefined(&opts) {
        Ok(Value::Object(Map::new()))
    } else {
        Ok(opts)
    }
}

fn push_unique(vec: &mut Vec<String>, set: &mut HashSet<String>, val: String) {
    if set.insert(val.clone()) {
        vec.push(val);
    }
}

fn is_signal_eligible(post: &Value) -> bool {
    truthy_prop(post, "assessment").and_then(|a| prop(a, "signalEligible"))
        == Some(&Value::Bool(true))
}

fn clean_handle(post: &Value) -> String {
    if !is_truthy(post) {
        return String::new();
    }
    let raw = ["handle", "username", "user", "author"]
        .iter()
        .find_map(|k| truthy_prop(post, k));
    let s = js_str(raw).to_lowercase();
    js_trim(s.strip_prefix('@').unwrap_or(&s)).to_string()
}

fn classify_provenance_val(post: &Value, assessment: Option<&Value>) -> Value {
    let handle = clean_handle(post);
    let a = assessment.filter(|v| is_truthy(v)).or_else(|| {
        is_truthy(post)
            .then(|| truthy_prop(post, "assessment"))
            .flatten()
    });
    let text = if is_truthy(post) {
        js_str(prop(post, "text"))
    } else {
        String::new()
    };

    let (source_type, reason) = if is_official_handle(&handle) {
        ("official_agency", "verified_official_handle")
    } else if is_news_handle(&handle) {
        ("news_outlet", "recognized_news_handle")
    } else if a.and_then(|av| prop(av, "intent")).and_then(Value::as_str)
        == Some("eyewitness_report")
        && !RE_RT_CHECK.is_match(&text)
    {
        ("eyewitness", "firsthand_eyewitness_markers")
    } else if a
        .and_then(|av| truthy_prop(av, "event"))
        .and_then(|ev| prop(ev, "sourceType"))
        .and_then(Value::as_str)
        == Some("eyewitness")
        && !RE_RT_CHECK.is_match(&text)
    {
        ("eyewitness", "eyewitness_event_framing")
    } else {
        ("anonymous", "unverified_social_account")
    };

    json!({
        "sourceType": source_type,
        "authorityWeight": js::number(provenance_weight(source_type)),
        "handle": if handle.is_empty() { String::new() } else { format!("@{handle}") },
        "reason": reason,
        "engagementExcludedFromCredibility": true,
    })
}

/// Port of `classifySourceProvenance` (`src/social-cluster.js:46`).
pub fn classify_source_provenance(post: Value, assessment: Value) -> Result<Value, String> {
    Ok(classify_provenance_val(&post, Some(&assessment)))
}

fn strip_boilerplate_str(text: &str) -> String {
    let lower = text.nfkc().collect::<String>().to_lowercase();
    let s = RE_BOIL_RT.replace(&lower, "");
    let s = RE_BOIL_VIA.replace_all(&s, " ");
    let s = RE_BOIL_URL.replace_all(&s, " ");
    let s = RE_BOIL_MENTION.replace_all(&s, " ");
    let s = RE_BOIL_HASHTAG.replace_all(&s, " ");
    let s = RE_BOIL_NON_ALNUM.replace_all(&s, " ");
    let s = RE_BOIL_WS.replace_all(&s, " ");
    js_trim(&s).to_string()
}

/// Port of `stripBoilerplateForSimilarity` (`src/social-cluster.js:77`).
pub fn strip_boilerplate_for_similarity(text: Value) -> Result<Value, String> {
    Ok(Value::String(strip_boilerplate_str(&js_str(Some(&text)))))
}

fn similarity_tokens_vec(text: &str) -> Vec<String> {
    let clean = strip_boilerplate_str(text);
    let mut seen = HashSet::new();
    clean
        .split(' ')
        .filter(|w| w.encode_utf16().count() >= 3 && !STOP_WORDS.contains(w) && seen.insert(*w))
        .map(str::to_string)
        .collect()
}

/// Port of `similarityTokens` (`src/social-cluster.js:91`).
pub fn similarity_tokens(text: Value) -> Result<Value, String> {
    Ok(Value::Array(
        similarity_tokens_vec(&js_str(Some(&text)))
            .into_iter()
            .map(Value::String)
            .collect(),
    ))
}

fn jaccard_sets(a: &HashSet<String>, b: &HashSet<String>) -> f64 {
    if a.is_empty() || b.is_empty() {
        return 0.0;
    }
    let inter = a.iter().filter(|t| b.contains(*t)).count();
    let union = a.len() + b.len() - inter;
    if union > 0 {
        inter as f64 / union as f64
    } else {
        0.0
    }
}

/// Port of `isNearDuplicate` (`src/social-cluster.js:106`).
pub fn is_near_duplicate(text_a: Value, text_b: Value, threshold: Value) -> Result<Value, String> {
    let thresh = if js::is_undefined(&threshold) {
        0.78
    } else {
        js_to_number(&threshold)
    };
    let norm_a = strip_boilerplate_str(&js_str(Some(&text_a)));
    let norm_b = strip_boilerplate_str(&js_str(Some(&text_b)));
    if norm_a.is_empty() || norm_b.is_empty() {
        return Ok(Value::Bool(false));
    }
    if norm_a == norm_b {
        return Ok(Value::Bool(true));
    }
    let tok_a = similarity_tokens_vec(&norm_a);
    let tok_b = similarity_tokens_vec(&norm_b);
    if tok_a.len() < 3 || tok_b.len() < 3 {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(jaccard_similarity(&tok_a, &tok_b) >= thresh))
}

fn canonical_url_key(post: &Value) -> String {
    if !is_truthy(post) {
        return String::new();
    }
    let trimmed = js_trim(&js_str(prop(post, "url"))).to_lowercase();
    let cleaned = trimmed
        .split_once('?')
        .map_or(trimmed.as_str(), |(a, _)| a)
        .trim_end_matches('/');
    if !cleaned.is_empty() {
        cleaned.to_string()
    } else {
        js_trim(&js_str(prop(post, "code"))).to_string()
    }
}

fn has_media_flag(post: &Value) -> bool {
    prop(post, "media")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty())
        || prop(post, "images")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
        || prop(post, "has_media").is_some_and(is_truthy)
}

fn representative_priority(post: &Value) -> f64 {
    if !is_truthy(post) {
        return 0.0;
    }
    let assess = prop(post, "assessment");
    let prov = assess
        .filter(|a| is_truthy(a))
        .and_then(|a| truthy_prop(a, "provenance"))
        .cloned()
        .unwrap_or_else(|| classify_provenance_val(post, assess));
    let auth = if is_truthy(&prov) {
        js_num_or(prop(&prov, "authorityWeight"), 0.35)
    } else {
        0.35
    };
    let sig = if is_signal_eligible(post) { 1.0 } else { 0.0 };
    let media = if has_media_flag(post) { 0.1 } else { 0.0 };
    auth * 10.0 + sig * 2.0 + media
}

#[derive(Clone)]
struct QuakeSig {
    mag: Option<f64>,
    impact: bool,
    ts: f64,
}

fn extract_quake_signature(post: &Value) -> Option<QuakeSig> {
    let txt = if is_truthy(post) {
        js_str(prop(post, "text"))
    } else {
        String::new()
    };
    let caps = MICRO_QUAKE_RE
        .captures(&txt)
        .or_else(|| QUAKE_MAG_RE.captures(&txt));
    let impact = QUAKE_IMPACT_RE.is_match(&txt);
    if caps.is_none() && !impact {
        return None;
    }
    let raw_mag = caps
        .as_ref()
        .and_then(|c| c.get(1).or_else(|| c.get(2)))
        .map_or("", |m| m.as_str());
    let mag_val = if raw_mag.is_empty() {
        f64::NAN
    } else {
        raw_mag.replace(',', ".").parse::<f64>().unwrap_or(f64::NAN)
    };
    let ts = if is_truthy(post) {
        post_ts(post, 0.0)
    } else {
        0.0
    };
    Some(QuakeSig {
        mag: mag_val.is_finite().then_some(mag_val),
        impact,
        ts,
    })
}

fn can_fold_quake_pair(q_a: Option<&QuakeSig>, q_b: Option<&QuakeSig>) -> bool {
    match (q_a, q_b) {
        (None, None) => true,
        (None, Some(_)) | (Some(_), None) => false,
        (Some(a), Some(b)) => {
            if a.impact != b.impact {
                return false;
            }
            let major =
                a.mag.is_some_and(|m| m >= 4.0) || b.mag.is_some_and(|m| m >= 4.0) || a.impact;
            if let (Some(ma), Some(mb)) = (a.mag, b.mag)
                && major
                && (ma - mb).abs() > 0.2
            {
                return false;
            }
            !(major
                && a.ts != 0.0
                && !a.ts.is_nan()
                && b.ts != 0.0
                && !b.ts.is_nan()
                && (a.ts - b.ts).abs() > 600_000.0)
        }
    }
}

fn micro_quake_bucket_key(post: &Value) -> String {
    let txt = if is_truthy(post) {
        js_str(prop(post, "text"))
    } else {
        String::new()
    };
    let Some(m1) = MICRO_QUAKE_RE.captures(&txt).and_then(|c| c.get(1)) else {
        return String::new();
    };
    let mag = m1
        .as_str()
        .replace(',', ".")
        .parse::<f64>()
        .unwrap_or(f64::NAN);
    if !mag.is_finite() || mag >= 4.0 || QUAKE_IMPACT_RE.is_match(&txt) {
        return String::new();
    }
    let bucket = (post_ts(post, 0.0) / 7_200_000.0).floor();
    format!("microquake:{}", js::number_to_string(bucket))
}

#[derive(Clone)]
struct RepostVariant {
    platform: String,
    handle: String,
    url: Value,
}

struct DedupEntry {
    post: Value,
    norm: String,
    toks: HashSet<String>,
    quake: Option<QuakeSig>,
    norms: Vec<String>,
    norms_set: HashSet<String>,
    urls: Vec<String>,
    urls_set: HashSet<String>,
    duplicate_count: usize,
    repost_variants: Vec<RepostVariant>,
}

impl DedupEntry {
    fn member_keys(&self) -> Vec<String> {
        self.urls.iter().chain(&self.norms).cloned().collect()
    }

    fn to_value(&self) -> Value {
        let variants: Vec<Value> = self
            .repost_variants
            .iter()
            .map(|v| json!({ "platform": v.platform, "handle": v.handle, "url": v.url }))
            .collect();
        json!({
            "post": self.post,
            "norm": self.norm,
            "duplicateCount": self.duplicate_count as u64,
            "repostVariants": variants,
            "memberKeys": self.member_keys(),
        })
    }
}

fn deduplicate_posts_internal(
    posts: &Value,
    threshold: f64,
    window_size: f64,
    fold_micro_quakes: bool,
) -> Result<Vec<DedupEntry>, String> {
    let mut sorted = if !is_truthy(posts) {
        Vec::new()
    } else if let Some(arr) = posts.as_array() {
        arr.clone()
    } else if posts.is_string() || js::string_units(posts).is_some() {
        return Err("TypeError: (posts || []).slice(...).sort is not a function".into());
    } else {
        return Err("TypeError: (posts || []).slice is not a function".into());
    };

    if sorted.len() >= 2 {
        let mut sort_err: Option<String> = None;
        js::sort(&mut sorted, |a, b| {
            if sort_err.is_some() {
                return 0.0;
            }
            for x in [b, a] {
                if is_js_nullish(x) {
                    let w = if x.is_null() { "null" } else { "undefined" };
                    sort_err = Some(format!(
                        "TypeError: Cannot read properties of {w} (reading 'ts')"
                    ));
                    return 0.0;
                }
            }
            js_num_or(prop(b, "ts"), 0.0) - js_num_or(prop(a, "ts"), 0.0)
        });
        if let Some(err) = sort_err {
            return Err(err);
        }
    }

    let mut unique: Vec<DedupEntry> = Vec::new();
    let mut seen_urls = HashSet::new();
    let mut seen_exact: HashMap<String, usize> = HashMap::new();
    let mut seen_micro_quakes: HashMap<String, usize> = HashMap::new();

    let fold_into = |entry: &mut DedupEntry, cand: &Value, plat: &str, cand_norm: &str| {
        entry.duplicate_count += 1;
        entry.repost_variants.push(RepostVariant {
            platform: plat.to_string(),
            handle: clean_handle(cand),
            url: truthy_prop(cand, "url")
                .cloned()
                .unwrap_or_else(|| Value::String(String::new())),
        });
        if !cand_norm.is_empty() {
            push_unique(&mut entry.norms, &mut entry.norms_set, cand_norm.into());
        }
        let cu = canonical_url_key(cand);
        if !cu.is_empty() {
            push_unique(&mut entry.urls, &mut entry.urls_set, cu);
        }
        if representative_priority(cand) > representative_priority(&entry.post) {
            entry.post = cand.clone();
        }
    };

    for p in &sorted {
        if !is_truthy(p) || truthy_prop(p, "text").is_none() {
            continue;
        }
        let u_key = canonical_url_key(p);
        let plat = truthy_prop(p, "from")
            .or_else(|| truthy_prop(p, "platform"))
            .map(|v| js_to_string(v).to_lowercase())
            .unwrap_or_else(|| {
                if truthy_prop(p, "code").is_some() {
                    "threads".into()
                } else {
                    "x".into()
                }
            });
        if !u_key.is_empty() && !seen_urls.insert(format!("{plat}|{u_key}")) {
            continue;
        }

        let norm = strip_boilerplate_str(&js_str(prop(p, "text")));
        if fold_micro_quakes {
            let mq_key = micro_quake_bucket_key(p);
            if !mq_key.is_empty()
                && let Some(&idx) = seen_micro_quakes.get(&mq_key)
            {
                fold_into(&mut unique[idx], p, &plat, &norm);
                continue;
            }
        }
        if norm.is_empty() {
            continue;
        }
        if let Some(&idx) = seen_exact.get(&norm) {
            fold_into(&mut unique[idx], p, &plat, &norm);
            continue;
        }

        let p_toks: HashSet<String> = similarity_tokens_vec(&norm).into_iter().collect();
        let p_quake = extract_quake_signature(p);
        let start = if window_size.is_infinite() {
            0
        } else {
            (unique.len() as f64 - window_size).max(0.0) as usize
        };
        if let Some(idx) = unique.iter().enumerate().skip(start).find_map(|(i, c)| {
            (can_fold_quake_pair(p_quake.as_ref(), c.quake.as_ref())
                && p_toks.len() >= 4
                && c.toks.len() >= 4
                && jaccard_sets(&p_toks, &c.toks) >= threshold)
                .then_some(i)
        }) {
            fold_into(&mut unique[idx], p, &plat, &norm);
            continue;
        }

        let mut norms = Vec::new();
        let mut norms_set = HashSet::new();
        push_unique(&mut norms, &mut norms_set, norm.clone());
        let mut urls = Vec::new();
        let mut urls_set = HashSet::new();
        if !u_key.is_empty() {
            push_unique(&mut urls, &mut urls_set, u_key);
        }
        let idx = unique.len();
        seen_exact.insert(norm.clone(), idx);
        if fold_micro_quakes {
            let mq_key = micro_quake_bucket_key(p);
            if !mq_key.is_empty() {
                seen_micro_quakes.entry(mq_key).or_insert(idx);
            }
        }
        unique.push(DedupEntry {
            post: p.clone(),
            norm,
            toks: p_toks,
            quake: p_quake,
            norms,
            norms_set,
            urls,
            urls_set,
            duplicate_count: 1,
            repost_variants: vec![RepostVariant {
                platform: plat,
                handle: clean_handle(p),
                url: truthy_prop(p, "url")
                    .cloned()
                    .unwrap_or_else(|| Value::String(String::new())),
            }],
        });
    }
    Ok(unique)
}

/// Port of `deduplicatePosts` (`src/social-cluster.js:169`).
pub fn deduplicate_posts(posts: Value, opts: Value) -> Result<Value, String> {
    let opts = check_opts(opts, "threshold")?;
    let t = prop(&opts, "threshold")
        .map(js_to_number)
        .unwrap_or(f64::NAN);
    let w = prop(&opts, "windowSize")
        .map(js_to_number)
        .unwrap_or(f64::NAN);
    let entries = deduplicate_posts_internal(
        &posts,
        if t > 0.0 { t } else { 0.72 },
        if w > 0.0 { w } else { 320.0 },
        prop(&opts, "foldMicroQuakes").is_some_and(is_truthy),
    )?;
    Ok(Value::Array(
        entries.iter().map(DedupEntry::to_value).collect(),
    ))
}

fn event_specific_tokens_vec(ev_text: &str, place_name: &str) -> Vec<String> {
    let place_lower = place_name.to_lowercase();
    let place_set: HashSet<&str> = place_lower
        .split(|c: char| !(c.is_ascii_lowercase() || c.is_ascii_digit()))
        .filter(|s| !s.is_empty())
        .collect();
    similarity_tokens_vec(ev_text)
        .into_iter()
        .filter(|t| {
            t.encode_utf16().count() >= 4
                && !GENERIC_EVENT_TOKENS.contains(t.as_str())
                && !place_set.contains(t.as_str())
        })
        .collect()
}

/// Port of `eventSpecificTokens` (`src/social-cluster.js:264`).
pub fn event_specific_tokens(ev_text: Value, place_name: Value) -> Result<Value, String> {
    let toks = event_specific_tokens_vec(&js_str(Some(&ev_text)), &js_str(Some(&place_name)));
    Ok(json!({ "$set": toks }))
}

struct IncidentRecord {
    post: Value,
    duplicate_count: usize,
    repost_variants: Vec<RepostVariant>,
    member_keys: Vec<String>,
    source_type: String,
    authority_weight: f64,
    handle: String,
    ts: f64,
    category: Value,
    event_type: Value,
    location_raw: Value,
    location: String,
    prov: String,
    target_raw: Value,
    target: String,
    spec_tokens: HashSet<String>,
}

fn can_cluster_events(a: &IncidentRecord, b: &IncidentRecord, time_window_ms: f64) -> bool {
    if (a.ts - b.ts).abs() > time_window_ms
        || !js_strict_eq(&a.category, &b.category)
        || !js_strict_eq(&a.event_type, &b.event_type)
    {
        return false;
    }
    let (loc_a, loc_b) = (
        js_trim(&a.location.to_lowercase()).to_string(),
        js_trim(&b.location.to_lowercase()).to_string(),
    );
    if !loc_a.is_empty() && !loc_b.is_empty() && loc_a != loc_b {
        return false;
    }
    if loc_a.is_empty() || loc_b.is_empty() {
        let (t_a, t_b) = (
            js_trim(&a.target.to_lowercase()).to_string(),
            js_trim(&b.target.to_lowercase()).to_string(),
        );
        if t_a.is_empty() || t_b.is_empty() || t_a != t_b {
            return false;
        }
    }
    let (p_a, p_b) = (
        js_trim(&a.prov.to_lowercase()).to_string(),
        js_trim(&b.prov.to_lowercase()).to_string(),
    );
    if !p_a.is_empty() && !p_b.is_empty() && p_a != p_b {
        return false;
    }
    let shared = a
        .spec_tokens
        .iter()
        .filter(|t| b.spec_tokens.contains(*t))
        .count();
    let union = a.spec_tokens.len() + b.spec_tokens.len() - shared;
    let jac = if union > 0 {
        shared as f64 / union as f64
    } else {
        0.0
    };
    if a.event_type
        .as_str()
        .is_some_and(|et| RARE_TYPES.contains(et))
    {
        shared >= 1 && jac >= 0.12
    } else {
        shared >= 2 && jac >= 0.18
    }
}

struct ClusterSummary {
    corroboration_level: String,
    independent_claims: usize,
    platforms: Vec<String>,
    member_keys: Vec<String>,
    posts: Vec<Value>,
    value: Value,
}

fn cluster_incidents_internal(
    posts: &Value,
    time_window_ms: f64,
) -> Result<Vec<ClusterSummary>, String> {
    let eligible: Vec<Value> = if !is_truthy(posts) {
        Vec::new()
    } else if let Some(arr) = posts.as_array() {
        arr.iter()
            .filter(|p| {
                is_truthy(p)
                    && is_signal_eligible(p)
                    && truthy_prop(p, "assessment")
                        .and_then(|a| prop(a, "events"))
                        .and_then(Value::as_array)
                        .is_some_and(|e| !e.is_empty())
            })
            .cloned()
            .collect()
    } else {
        return Err("TypeError: (posts || []).filter is not a function".into());
    };

    let deduped = deduplicate_posts_internal(&Value::Array(eligible), 0.72, 320.0, false)?;
    let mut clusters: Vec<Vec<IncidentRecord>> = Vec::new();

    for item in deduped {
        let p = &item.post;
        let assess = prop(p, "assessment").unwrap_or(&Value::Null);
        let prov = truthy_prop(assess, "provenance")
            .cloned()
            .unwrap_or_else(|| classify_provenance_val(p, Some(assess)));
        let ev = prop(assess, "events")
            .and_then(Value::as_array)
            .and_then(|a| a.first())
            .ok_or_else(|| "TypeError: Cannot read properties of undefined".to_string())?;
        if is_js_nullish(ev) {
            let w = if ev.is_null() { "null" } else { "undefined" };
            return Err(format!(
                "TypeError: Cannot read properties of {w} (reading 'location')"
            ));
        }

        let loc_obj = truthy_prop(ev, "location");
        let location_raw = loc_obj
            .and_then(|l| truthy_prop(l, "name"))
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));
        let location = js_str(Some(&location_raw));
        let target_raw = truthy_prop(ev, "target")
            .cloned()
            .unwrap_or_else(|| Value::String(String::new()));
        let target = js_str(Some(&target_raw));
        let ev_text = truthy_prop(ev, "evidence")
            .map(js_to_string)
            .unwrap_or_else(|| js_str(prop(p, "text")));

        let rec = IncidentRecord {
            post: p.clone(),
            duplicate_count: item.duplicate_count,
            repost_variants: item.repost_variants.clone(),
            member_keys: item.member_keys(),
            source_type: js_str(prop(&prov, "sourceType")),
            authority_weight: prop(&prov, "authorityWeight")
                .map(js_to_number)
                .unwrap_or(f64::NAN),
            handle: js_str(prop(&prov, "handle")),
            ts: post_ts(p, 0.0),
            category: prop(ev, "category").cloned().unwrap_or_else(js::undefined),
            event_type: prop(ev, "type").cloned().unwrap_or_else(js::undefined),
            location_raw,
            location: location.clone(),
            prov: js_str(loc_obj.and_then(|l| prop(l, "prov"))),
            target_raw,
            target,
            spec_tokens: event_specific_tokens_vec(&ev_text, &location)
                .into_iter()
                .collect(),
        };

        if let Some(cl) = clusters.iter_mut().find(|cl| {
            cl.iter()
                .all(|r| can_cluster_events(r, &rec, time_window_ms))
        }) {
            cl.push(rec);
        } else {
            clusters.push(vec![rec]);
        }
    }

    let mut summaries = Vec::with_capacity(clusters.len());
    for cl in clusters {
        let (mut accounts, mut source_types) = (HashSet::new(), HashSet::new());
        let (mut platforms, mut platforms_set) = (Vec::new(), HashSet::new());
        let (mut member_keys, mut mk_set) = (Vec::new(), HashSet::new());
        let (mut total_copies, mut max_auth, mut latest_ts) = (0usize, 0.0f64, 0.0f64);

        for r in &cl {
            total_copies += r.duplicate_count;
            for mk in &r.member_keys {
                push_unique(&mut member_keys, &mut mk_set, mk.clone());
            }
            for v in &r.repost_variants {
                push_unique(&mut platforms, &mut platforms_set, v.platform.clone());
            }
            if !r.handle.is_empty() {
                accounts.insert(format!("{}:{}", r.source_type, r.handle));
            }
            source_types.insert(r.source_type.clone());
            if r.authority_weight > max_auth {
                max_auth = r.authority_weight;
            }
            if r.ts > latest_ts {
                latest_ts = r.ts;
            }
        }

        let has_official = source_types.contains("official_agency");
        let has_news = source_types.contains("news_outlet");
        let has_eyewitness = source_types.contains("eyewitness");
        let independent_claims = cl.len();
        let corroboration_level = if has_official {
            "official_confirmed"
        } else if independent_claims >= 2 && (has_news || has_eyewitness || accounts.len() >= 2) {
            "multi_source_corroborated"
        } else if has_eyewitness {
            "single_source_eyewitness"
        } else {
            "uncorroborated"
        };

        let mut idx: Vec<usize> = (0..cl.len()).collect();
        js::sort(&mut idx, |&a, &b| {
            js_cmp_or(
                cl[b].authority_weight - cl[a].authority_weight,
                cl[b].ts - cl[a].ts,
            )
        });
        let rep = &cl[idx[0]];
        let location = if is_truthy(&rep.location_raw) {
            rep.location_raw.clone()
        } else if is_truthy(&rep.target_raw) {
            rep.target_raw.clone()
        } else {
            Value::String("unknown".into())
        };
        let auth_score = js_to_fixed_num(max_auth, 2);
        let posts_sorted: Vec<Value> = idx.iter().map(|&i| cl[i].post.clone()).collect();

        let value = json!({
            "category": rep.category,
            "eventType": rep.event_type,
            "location": location,
            "latestTs": js::number(latest_ts),
            "independentClaims": independent_claims as u64,
            "independentAccounts": accounts.len() as u64,
            "totalCopiesWithDuplicates": total_copies as u64,
            "platforms": platforms,
            "corroborationLevel": corroboration_level,
            "hasOfficialSource": has_official,
            "hasNewsOutlet": has_news,
            "hasEyewitness": has_eyewitness,
            "authorityScore": js::number(auth_score),
            "representative": rep.post,
            "posts": posts_sorted,
            "memberKeys": member_keys,
        });

        summaries.push((
            auth_score,
            latest_ts,
            ClusterSummary {
                corroboration_level: corroboration_level.into(),
                independent_claims,
                platforms,
                member_keys,
                posts: posts_sorted,
                value,
            },
        ));
    }

    js::sort(&mut summaries, |a, b| js_cmp_or(b.0 - a.0, b.1 - a.1));
    Ok(summaries.into_iter().map(|(_, _, s)| s).collect())
}

/// Port of `clusterIncidents` (`src/social-cluster.js:303`).
pub fn cluster_incidents(posts: Value, opts: Value) -> Result<Value, String> {
    let opts = check_opts(opts, "timeWindowMs")?;
    let tw = truthy_prop(&opts, "timeWindowMs")
        .map(js_to_number)
        .unwrap_or(5_400_000.0);
    let clusters = cluster_incidents_internal(&posts, tw)?;
    Ok(Value::Array(
        clusters.into_iter().map(|c| c.value).collect(),
    ))
}

fn post_specific_tokens(post: &Value, loc_name: &str) -> HashSet<String> {
    let ev_text = if is_truthy(post) {
        truthy_prop(post, "assessment")
            .and_then(|a| prop(a, "events"))
            .and_then(Value::as_array)
            .and_then(|arr| arr.first())
            .filter(|ev| is_truthy(ev))
            .and_then(|ev| truthy_prop(ev, "evidence"))
            .map(js_to_string)
            .unwrap_or_else(|| js_str(prop(post, "text")))
    } else {
        String::new()
    };
    event_specific_tokens_vec(&ev_text, loc_name)
        .into_iter()
        .collect()
}

fn extract_iterable_strings(v: &Value) -> Vec<String> {
    if let Some(arr) = v.as_array() {
        arr.iter().map(js_to_string).collect()
    } else if let Some(arr) = v
        .as_object()
        .filter(|o| o.len() == 1)
        .and_then(|o| o.get("$set"))
        .and_then(Value::as_array)
    {
        arr.iter().map(js_to_string).collect()
    } else if let Some(s) = v.as_str() {
        s.chars().map(|c| c.to_string()).collect()
    } else {
        Vec::new()
    }
}

fn match_post_to_news_internal(post: &Value, news_events: &Value, now: f64) -> Value {
    let Some(events) = news_events.as_array().filter(|a| !a.is_empty()) else {
        return Value::Null;
    };
    if !is_truthy(post) {
        return Value::Null;
    }
    let empty = Value::Object(Map::new());
    let a = truthy_prop(post, "assessment").unwrap_or(&empty);
    let ts = post_ts(post, now);
    let ev = prop(a, "events")
        .and_then(Value::as_array)
        .and_then(|arr| arr.first())
        .filter(|v| is_truthy(v));
    let ev_loc = ev.and_then(|e| truthy_prop(e, "location"));
    let geo = truthy_prop(a, "geographicContext");

    let loc = js_trim(
        &js_str(
            ev_loc
                .and_then(|l| truthy_prop(l, "name"))
                .or_else(|| geo.and_then(|g| truthy_prop(g, "name"))),
        )
        .to_lowercase(),
    )
    .to_string();
    let kab_code = js_trim(&js_str(
        ev_loc
            .and_then(|l| truthy_prop(l, "kabCode"))
            .or_else(|| geo.and_then(|g| truthy_prop(g, "kabCode"))),
    ))
    .to_string();
    let cat = js_trim(&js_str(ev.and_then(|e| prop(e, "category"))).to_lowercase()).to_string();

    let tps: HashSet<String> = truthy_prop(post, "topics")
        .or_else(|| truthy_prop(a, "topics"))
        .and_then(Value::as_array)
        .map(|arr| arr.iter().map(|x| js_to_string(x).to_lowercase()).collect())
        .unwrap_or_default();
    let p_toks = post_specific_tokens(post, &loc);

    let (mut best, mut best_score) = (None, 0.0f64);
    for ne in events {
        if !is_truthy(ne) {
            continue;
        }
        let ne_ts = js_num_or(prop(ne, "ts"), 0.0);
        if ne_ts != 0.0 && (ts - ne_ts).abs() > 43_200_000.0 {
            continue;
        }
        let ne_loc = js_trim(&js_str(prop(ne, "loc")).to_lowercase()).to_string();
        let ne_kab = js_trim(&js_str(prop(ne, "kabCode"))).to_string();
        let ne_cat = js_trim(&js_str(prop(ne, "cat")).to_lowercase()).to_string();

        let same_cat = (!cat.is_empty() && !ne_cat.is_empty() && cat == ne_cat)
            || (!ne_cat.is_empty() && tps.contains(&ne_cat))
            || truthy_prop(ne, "topic").is_some_and(|t| tps.contains(&js_to_string(t)));
        let same_loc = (!kab_code.is_empty() && !ne_kab.is_empty() && kab_code == ne_kab)
            || (!loc.is_empty()
                && !ne_loc.is_empty()
                && (loc == ne_loc || loc.contains(&ne_loc) || ne_loc.contains(&loc)));
        if (!loc.is_empty() && !ne_loc.is_empty() && !same_loc)
            || (!cat.is_empty() && !ne_cat.is_empty() && !same_cat)
        {
            continue;
        }

        let shared = if !p_toks.is_empty() {
            truthy_prop(ne, "tokens")
                .map(|t| {
                    extract_iterable_strings(t)
                        .into_iter()
                        .filter(|tok| p_toks.contains(tok))
                        .count()
                })
                .unwrap_or(0)
        } else {
            0
        };
        let s = if same_loc && same_cat {
            3.0 + shared as f64 + js_num_or(prop(ne, "count"), 1.0).min(2.0) * 0.4
        } else if same_loc && shared >= 1 {
            2.5 + shared as f64
        } else if same_cat && shared >= 2 {
            2.0 + shared as f64 * 0.8
        } else if shared >= 3 {
            shared as f64
        } else {
            0.0
        };
        if s > best_score {
            best_score = s;
            best = Some(ne);
        }
    }

    match best {
        None => Value::Null,
        Some(b) => json!({
            "status": "confirmed",
            "count": js::number(js_num_or(prop(b, "count"), 1.0).max(1.0)),
            "title": truthy_prop(b, "title").cloned().unwrap_or_else(|| Value::String(String::new())),
            "url": truthy_prop(b, "url").cloned().unwrap_or_else(|| Value::String(String::new())),
        }),
    }
}

/// Port of `matchPostToNews` (`src/social-cluster.js:399`).
pub fn match_post_to_news(post: Value, news_events: Value, now: Value) -> Result<Value, String> {
    Ok(match_post_to_news_internal(
        &post,
        &news_events,
        js_to_number(&now),
    ))
}

/// Port of `collapseSocialFeed` (`src/social-cluster.js:447`).
pub fn collapse_social_feed(posts: Value, opts: Value) -> Result<Value, String> {
    let opts = check_opts(opts, "now")?;
    let now = js_num_or(prop(&opts, "now"), pinned::now_ms());
    let all_combined = prop(&opts, "corroborationPool")
        .filter(|v| v.as_array().is_some_and(|a| !a.is_empty()))
        .unwrap_or(&posts);
    let tw = truthy_prop(&opts, "timeWindowMs")
        .map(js_to_number)
        .unwrap_or(5_400_000.0);

    let clusters = cluster_incidents_internal(all_combined, tw)?;
    let mut post_to_cluster: HashMap<String, usize> = HashMap::new();
    for (ci, cl) in clusters.iter().enumerate() {
        for k in &cl.member_keys {
            if !k.is_empty() {
                post_to_cluster.entry(k.clone()).or_insert(ci);
            }
        }
        for member in &cl.posts {
            let u = canonical_url_key(member);
            let n = strip_boilerplate_str(&js_str(prop(member, "text")));
            if !u.is_empty() {
                post_to_cluster.entry(u).or_insert(ci);
            }
            if !n.is_empty() {
                post_to_cluster.entry(n).or_insert(ci);
            }
        }
    }

    let raw_t = js_num_or(prop(&opts, "threshold"), 0.72);
    let raw_w = js_num_or(prop(&opts, "windowSize"), 320.0);
    let mut deduped = deduplicate_posts_internal(
        &posts,
        if raw_t > 0.0 { raw_t } else { 0.72 },
        if raw_w > 0.0 { raw_w } else { 320.0 },
        prop(&opts, "foldMicroQuakes") != Some(&Value::Bool(false)),
    )?;

    js::sort(&mut deduped, |a, b| {
        let sig_diff = f64::from(u8::from(is_signal_eligible(&b.post)))
            - f64::from(u8::from(is_signal_eligible(&a.post)));
        if sig_diff != 0.0 {
            return sig_diff;
        }
        let pri_diff = representative_priority(&b.post) - representative_priority(&a.post);
        if pri_diff != 0.0 {
            return pri_diff;
        }
        js_num_or(prop(&b.post, "ts"), 0.0) - js_num_or(prop(&a.post, "ts"), 0.0)
    });

    let empty_news = Value::Array(Vec::new());
    let news_events = prop(&opts, "newsEvents")
        .filter(|v| v.is_array())
        .unwrap_or(&empty_news);
    let (mut author_hourly, mut author_total) = (
        HashMap::<String, usize>::new(),
        HashMap::<String, usize>::new(),
    );
    let mut out: Vec<(f64, f64, Value)> = Vec::new();

    for item in &deduped {
        let p = &item.post;
        if !is_truthy(p) {
            continue;
        }
        let u = clean_handle(p);
        let assess = truthy_prop(p, "assessment");
        let is_sig = is_signal_eligible(p);
        let is_agency = is_official_handle(&u);
        let ts = post_ts(p, now);

        if !u.is_empty() && !(is_sig && is_agency) {
            let h_key = format!("{}|{}", u, js::number_to_string((ts / 3_600_000.0).floor()));
            let (max_h, max_t) = if is_agency {
                (4, 10)
            } else if is_sig {
                (3, 6)
            } else {
                (2, 4)
            };
            if author_hourly.get(&h_key).copied().unwrap_or(0) >= max_h
                || author_total.get(&u).copied().unwrap_or(0) >= max_t
            {
                continue;
            }
            *author_hourly.entry(h_key).or_insert(0) += 1;
            *author_total.entry(u.clone()).or_insert(0) += 1;
        }

        let prov = assess
            .and_then(|a| truthy_prop(a, "provenance"))
            .cloned()
            .unwrap_or_else(|| classify_provenance_val(p, assess));
        let source_type = js_str(prop(&prov, "sourceType"));
        let auth_weight = js_num_or(prop(&prov, "authorityWeight"), 0.35);

        let u_key = canonical_url_key(p);
        let n_key = strip_boilerplate_str(&js_str(prop(p, "text")));
        let cl = (!u_key.is_empty())
            .then(|| post_to_cluster.get(&u_key))
            .flatten()
            .or_else(|| {
                (!n_key.is_empty())
                    .then(|| post_to_cluster.get(&n_key))
                    .flatten()
            })
            .map(|&i| &clusters[i]);

        let corroboration = match cl {
            Some(c) => c.corroboration_level.as_str(),
            None => match source_type.as_str() {
                "official_agency" => "official_confirmed",
                "eyewitness" => "single_source_eyewitness",
                _ => "uncorroborated",
            },
        };
        let independent_claims = cl.map_or(1, |c| c.independent_claims);

        let (mut cross_platforms, mut cp_seen) = (Vec::new(), HashSet::new());
        for pl in item
            .repost_variants
            .iter()
            .map(|v| &v.platform)
            .chain(cl.into_iter().flat_map(|c| c.platforms.iter()))
        {
            if !pl.is_empty() {
                push_unique(&mut cross_platforms, &mut cp_seen, pl.clone());
            }
        }
        let (mut folded_handles, mut fh_seen) = (Vec::new(), HashSet::new());
        for v in &item.repost_variants {
            if !v.handle.is_empty() && v.handle != u && folded_handles.len() < 6 {
                push_unique(&mut folded_handles, &mut fh_seen, v.handle.clone());
            }
        }

        let mut news_match = match_post_to_news_internal(p, news_events, now);
        if news_match.is_null()
            && is_sig
            && (corroboration == "multi_source_corroborated"
                || source_type == "official_agency"
                || source_type == "eyewitness")
        {
            news_match = json!({ "status": "early_warning", "count": 0 });
        }

        let age_h = ((now - ts) / 3_600_000.0).max(0.0);
        let has_event_loc = assess
            .and_then(|a| truthy_prop(a, "geographicContext"))
            .and_then(|g| prop(g, "role"))
            .and_then(Value::as_str)
            == Some("event_location");
        let base_score = auth_weight
            + if is_sig { 0.55 } else { 0.0 }
            + match corroboration {
                "official_confirmed" => 0.35,
                "multi_source_corroborated" => 0.28,
                "single_source_eyewitness" => 0.14,
                _ => 0.0,
            }
            + match prop(&news_match, "status").and_then(Value::as_str) {
                Some("confirmed") => 0.35,
                Some("early_warning") => 0.22,
                _ => 0.0,
            }
            + if has_media_flag(p) { 0.08 } else { 0.0 }
            + if has_event_loc { 0.12 } else { 0.0 };
        let score = js_to_fixed_num(base_score / (1.0 + age_h * 0.42), 3);

        let mut intel = Map::new();
        intel.insert("dupCount".into(), Value::from(item.duplicate_count as u64));
        intel.insert("foldedHandles".into(), json!(folded_handles));
        intel.insert("crossPlatforms".into(), json!(cross_platforms));
        intel.insert("sourceType".into(), Value::String(source_type));
        intel.insert("corroboration".into(), Value::String(corroboration.into()));
        intel.insert(
            "independentClaims".into(),
            Value::from(independent_claims as u64),
        );
        if !news_match.is_null() {
            intel.insert("newsMatch".into(), news_match);
        }
        intel.insert("score".into(), js::number(score));

        let mut card = p.as_object().cloned().unwrap_or_default();
        card.insert("intel".into(), Value::Object(intel));
        out.push((score, js_num_or(prop(p, "ts"), 0.0), Value::Object(card)));
    }

    js::sort(&mut out, |a, b| js_cmp_or(b.0 - a.0, b.1 - a.1));
    Ok(Value::Array(out.into_iter().map(|(_, _, v)| v).collect()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn provenance_djp_and_news_handles() {
        for h in [
            "@DitjenPajakRI",
            "kring_pajak",
            "@KemenkeuRI",
            "@kanwildjp_jaksel",
        ] {
            let res =
                classify_source_provenance(json!({ "handle": h, "text": "Info" }), Value::Null)
                    .unwrap();
            assert_eq!(res["sourceType"], "official_agency", "handle={h}");
            assert_eq!(res["authorityWeight"], 1.0);
            assert_eq!(res["engagementExcludedFromCredibility"], true);
        }
        for h in ["@ddtcnews", "@ortaxid", "@detikcom", "@berita_kompastv"] {
            let res =
                classify_source_provenance(json!({ "handle": h, "text": "Berita" }), Value::Null)
                    .unwrap();
            assert_eq!(res["sourceType"], "news_outlet", "handle={h}");
            assert_eq!(res["authorityWeight"], 0.85);
        }
        let eye = classify_source_provenance(
            json!({ "username": "warga_kemang", "text": "Air sudah masuk rumah" }),
            json!({ "intent": "eyewitness_report" }),
        )
        .unwrap();
        assert_eq!(eye["sourceType"], "eyewitness");
        assert_eq!(eye["authorityWeight"], 0.72);
        let rt = classify_source_provenance(
            json!({ "username": "warga_kemang", "text": "RT @other: Air masuk rumah" }),
            json!({ "intent": "eyewitness_report" }),
        )
        .unwrap();
        assert_eq!(rt["sourceType"], "anonymous");
        assert_eq!(rt["authorityWeight"], 0.35);
    }

    #[test]
    fn dedupe_and_microquake_folding() {
        let posts = json!([
            { "from": "X", "handle": "@reposter", "ts": 1700000000000_i64, "text": "RT @detikcom: Kebakaran hebat melalap 20 kios di Pasar Baru Bekasi pagi ini https://t.co/abc" },
            { "from": "X", "handle": "@detikcom", "ts": 1699999900000_i64, "text": "Kebakaran hebat melalap 20 kios di Pasar Baru Bekasi pagi ini #viral via @detikcom" },
            { "from": "X", "handle": "@infoBMKG", "ts": 1699999800000_i64, "text": "Info Gempa Mag:3.1, Lok:7.12 LS,106.45 BT Sukabumi #BMKG" },
            { "from": "X", "handle": "@infoBMKG", "ts": 1699999700000_i64, "text": "Info Gempa Mag:3.4, Lok:7.20 LS,106.50 BT Garut #BMKG" },
            { "from": "X", "handle": "@infoBMKG", "ts": 1699999600000_i64, "text": "Info Gempa Mag:5.8, Lok:7.15 LS,106.48 BT Sukabumi Dirasakan III MMI #BMKG" }
        ]);
        let res = deduplicate_posts(posts, json!({ "foldMicroQuakes": true })).unwrap();
        let arr = res.as_array().unwrap();
        assert_eq!(arr.len(), 3);
        assert_eq!(arr[0]["post"]["handle"], "@detikcom");
        assert_eq!(arr[0]["duplicateCount"], 2);
        assert_eq!(arr[1]["duplicateCount"], 2);
        assert_eq!(arr[2]["duplicateCount"], 1);
    }

    #[test]
    fn clustering_corroboration_and_separation() {
        let posts = json!([
            {
                "from": "X", "handle": "@warga_pondok", "ts": 1700000000000_i64,
                "text": "Air masuk rumah di Pondok Gede Bekasi tanggul jebol meluap",
                "assessment": { "signalEligible": true, "intent": "eyewitness_report",
                    "events": [{ "category": "flood", "type": "flood", "location": { "name": "Bekasi", "prov": "Jawa Barat" }, "evidence": "Air masuk rumah Pondok Gede tanggul jebol meluap" }] }
            },
            {
                "from": "Threads", "handle": "@detikcom", "ts": 1699999900000_i64,
                "text": "Tanggul Kali Bekasi jebol di Pondok Gede puluhan rumah terendam meluap",
                "assessment": { "signalEligible": true,
                    "events": [{ "category": "flood", "type": "flood", "location": { "name": "Bekasi", "prov": "Jawa Barat" }, "evidence": "Tanggul Kali Bekasi jebol Pondok Gede meluap" }] }
            },
            {
                "from": "X", "handle": "@ditjenpajakri", "ts": 1699999800000_i64,
                "text": "Pemeliharaan server Coretax e-Faktur sedang berlangsung",
                "assessment": { "signalEligible": true,
                    "events": [{ "category": "pajak", "type": "coretax_outage", "target": "coretax", "evidence": "Pemeliharaan server Coretax e-Faktur berlangsung" }] }
            }
        ]);
        let arr = cluster_incidents(posts, js::undefined())
            .unwrap()
            .as_array()
            .unwrap()
            .clone();
        assert_eq!(arr.len(), 2);
        assert_eq!(arr[0]["category"], "pajak");
        assert_eq!(arr[0]["corroborationLevel"], "official_confirmed");
        assert_eq!(arr[1]["category"], "flood");
        assert_eq!(arr[1]["independentClaims"], 2);
        assert_eq!(arr[1]["corroborationLevel"], "multi_source_corroborated");
    }

    #[test]
    fn news_matching_and_jaccard_helper() {
        assert!(
            (jaccard_similarity(
                &["kemang", "raya", "tanggul"],
                &["kemang", "raya", "meluap"]
            ) - 0.5)
                .abs()
                < 1e-9
        );
        assert_eq!(jaccard_similarity::<&str>(&[], &["a"]), 0.0);
        let now = 1700000000000_i64;
        let post = json!({
            "ts": now, "text": "Banjir merendam Jalan Kemang Raya Jakarta Selatan", "topics": ["banjir"],
            "assessment": { "events": [{ "category": "flood", "location": { "name": "Jakarta Selatan", "kabCode": "31.74" }, "evidence": "Banjir merendam Jalan Kemang Raya Jakarta Selatan" }] }
        });
        let news = json!([
            { "ts": now - 600_000, "loc": "surabaya", "cat": "flood", "count": 4, "title": "Banjir Surabaya", "url": "https://news.example/sby", "tokens": ["kemang", "raya"] },
            { "ts": now - 600_000, "loc": "jakarta selatan", "kabCode": "31.74", "cat": "flood", "count": 3, "title": "Banjir Kemang Raya", "url": "https://news.example/kemang", "tokens": { "$set": ["kemang", "raya"] } }
        ]);
        let matched = match_post_to_news(post, news, json!(now)).unwrap();
        assert_eq!(matched["status"], "confirmed");
        assert_eq!(matched["title"], "Banjir Kemang Raya");
        assert_eq!(matched["count"], 3);
    }
}
