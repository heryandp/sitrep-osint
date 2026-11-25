//! The Rust port of `src/social-history.js`.

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
use serde_json::{Map, Value, json};
use std::sync::LazyLock;
use std::sync::atomic::{AtomicU64, Ordering};

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
pub mod social_port;

#[path = "service.rs"]
pub mod service;

pub const AGE: f64 = 86_400_000.0;
pub const CLASSIFIER_VERSION: &str = "2026-11-23.1";

pub struct DropStats {
    pub total: AtomicU64,
    pub age: AtomicU64,
    pub handle: AtomicU64,
    pub lang: AtomicU64,
    pub foreign: AtomicU64,
    pub report_type: AtomicU64,
    pub body_len: AtomicU64,
    pub feed_eligible: AtomicU64,
    pub no_topic: AtomicU64,
    pub lane_incident_signals: AtomicU64,
    pub lane_situational_context: AtomicU64,
    pub lane_discovery_candidates: AtomicU64,
    pub lane_rejected: AtomicU64,
}

impl DropStats {
    pub const fn new() -> Self {
        Self {
            total: AtomicU64::new(0),
            age: AtomicU64::new(0),
            handle: AtomicU64::new(0),
            lang: AtomicU64::new(0),
            foreign: AtomicU64::new(0),
            report_type: AtomicU64::new(0),
            body_len: AtomicU64::new(0),
            feed_eligible: AtomicU64::new(0),
            no_topic: AtomicU64::new(0),
            lane_incident_signals: AtomicU64::new(0),
            lane_situational_context: AtomicU64::new(0),
            lane_discovery_candidates: AtomicU64::new(0),
            lane_rejected: AtomicU64::new(0),
        }
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "total": self.total.load(Ordering::Relaxed),
            "age": self.age.load(Ordering::Relaxed),
            "handle": self.handle.load(Ordering::Relaxed),
            "lang": self.lang.load(Ordering::Relaxed),
            "foreign": self.foreign.load(Ordering::Relaxed),
            "reportType": self.report_type.load(Ordering::Relaxed),
            "bodyLen": self.body_len.load(Ordering::Relaxed),
            "feedEligible": self.feed_eligible.load(Ordering::Relaxed),
            "noTopic": self.no_topic.load(Ordering::Relaxed),
            "lanes": {
                "incident_signals": self.lane_incident_signals.load(Ordering::Relaxed),
                "situational_context": self.lane_situational_context.load(Ordering::Relaxed),
                "discovery_candidates": self.lane_discovery_candidates.load(Ordering::Relaxed),
                "rejected": self.lane_rejected.load(Ordering::Relaxed),
            }
        })
    }
}

impl Default for DropStats {
    fn default() -> Self {
        Self::new()
    }
}

pub static DROP_STATS: DropStats = DropStats::new();

pub static SOC_LOCAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(indonesia|\bwni\b|jakarta|jabar|jateng|jatim|jawa|sumatera|sumatra|kalimantan|sulawesi|papua|bali\b|aceh|riau|banten|bekasi|bogor|depok|tangerang|bandung|surabaya|medan|makassar|semarang|yogya|bpbd|bnpb|bmkg|polres|polda|polsek|kodim|damkar|pemkot|pemkab|kabupaten|provinsi|prabowo|gibran|\bdpr\b|dprd|kapolri|polri|\bdjp\b|ditjen pajak|direktorat jenderal pajak|kemenkeu|coretax|kring pajak|kanwil djp|\bkpp\b)").unwrap()
});

pub static SOC_FOREIGN_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(amerika|united states|\busa\b|americans?|american|british|canadian|australian|japanese|chinese|korean|french|german|russian|indian|ukrainian|israeli|palestinian|malaysian|singaporean|filipino|vietnamese|brazilian|mexican|european|texas|california|new york|brasil|brazil|filipina|thailand|malaysia|korea|jepang|japan|tiongkok|china|inggris|britain|prancis|perancis|paris|france|jerman|germany|australia|india|pakistan|iran|israel|palestina|palestine|ukraina|ukraine|rusia|russia|gaza|turki|turkey|meksiko|mexico|kanada|canada|vietnam|chile|argentina|spanyol|spain|portugal|italia|italy|belanda|netherlands|den haag|s-gravenhage|tweede kamer|eerste kamer|vvd|cda|d66|pvv|fvd|nsc|gl-pvda|groenlinks|pvda|nederland|nederlands|holland|amsterdam|rotterdam|utrecht|eindhoven|groningen|kabinet rutte|kabinet schoof|dick schoof|geert wilders|swiss|swedia|sweden|norwegia|norway|denmark|yunani|greece|eropa|europe|nepal|london|oyo|osun|nigeria|afrika|trump|poland|polandia|polonia|krakow|warsaw|warszawa|lublin|czechia|ceko|prague|praha|austria|vienna|wina|finland|finlandia|helsinki|singapura|singapore|hong kong|taipei|tokyo|seoul|beijing|manila|bangkok|hanoi|kualalumpur|kathmandu|moskwa|moscow|berlin|rome|roma|madrid|amsterdam|brussels|zurich|stockholm|oslo|copenhagen|dublin|budapest|bucharest|sofia|belgrade|zagreb|vilnius|riga|tallinn|minsk|kiev|kyiv|istanbul|ankara|tehran|baghdad|beirut|riyadh|cairo|tunis|algiers|rabat|tripoli|khartoum|nairobi|kampala|kigali|lusaka|harare|pretoria|mogadishu|bamako|dakar|abuja|lagos|accra|kinshasa|sao paulo|rio de janeiro|brasilia|buenos aires|bogota|caracas|la paz|asuncion|montevideo|kingston|havana|santo domingo|paramaribo|managua|tegucigalpa|san salvador|san jose|guatemala city|mexico city|sucre|suriah|syria|irak|iraq|mesir|egypt|libya|libia|maroko|morocco|aljazair|algeria|tunisia|sudan|ethiopia|kenya|ghana|myanmar|kamboja|cambodia|laos|srilanka|sri lanka|bangladesh|lebanon|yordania|jordan|yaman|yemen|oman|qatar|uea|hungaria|hungary|rumania|romania|bulgaria|serbia|kroasia|croatia|slowakia|slovakia|slovenia|belgia|belgium|irlandia|ireland|estonia|latvia|lithuania|belarus|mongolia|maladewa|maldives|timor leste|selandia baru|new zealand|kuba|cuba|jamaika|jamaica|haiti|nikaragua|kosta rika|costa rica|kolombia|colombia|venezuela|bolivia|ekuador|ecuador|peru|paraguay|uruguay|congo|rwanda|uganda|tanzania|zambia|zimbabwe|mozambique|angola|namibia|botswana|lesotho|madagascar|mauritius|somalia|eritrea|djibouti|afghanistan|bhutan|brunei|albania|armenia|azerbaijan|cyprus|iceland|kosovo|luxembourg|malta|moldova|montenegro|san marino|vatikan|vatican|monako|monaco|georgia|jordan|panama|senegal|cameroon|gambia|guinea|sierra leone|liberia|mauritania|burkina faso|shah alam|selangor|bukit jelutong|subang jaya|petaling jaya|putrajaya|klang|penang|johor bahru|british columbia|tahsis|democrats|republicans|lebuhraya|plusline|llmtrafik|korang|macam ni|banjir tak|banjir ke|kesiannya|memandu|tanpa halangan banjir|terowong smart|m'sia|msia|sejuk beku|bekalan makanan|berbaki|penyokong|rombak kabinet|bubarkan parlimen|bubar parlimen|parlimen|timbalan perdana menteri|pakatan harapan|perikatan nasional|barisan nasional|umno|menteri besar|ketua menteri|dewan rakyat|dewan negara|kerajaan perpaduan|kerajaan madani|malaysia madani|varian baharu|baharu|dikesan|sejauh mana bahayanya|siakap keli|thesiakapkeli|banjir kilat|jabatan bomba|polis diraja|kawasan terjejas|hospital kerajaan|kementerian kesihatan)\b").unwrap()
});

static MALAYSIA_OUTLETS_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)^(501awani|bernamadotcom|beritartm|astroawani|utusanonline|sinarharian|bharianmy|hmetromy|malaysiakini|thesiakapkeli|siakapkeli|ohbulanofficial|mstaronline|kosmo_online)$").unwrap()
});

static BODY_URL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"https?://\S+").unwrap());
static BODY_HASH_AT_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[#@]\S+").unwrap());
static BODY_NON_ALNUM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"[^\p{L}\p{N}]+").unwrap());

static HTML_AMP: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)&amp;?").unwrap());
static HTML_LT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)&lt;?").unwrap());
static HTML_GT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)&gt;?").unwrap());
static HTML_QUOT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)&quot;?").unwrap());
static HTML_APOS: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)&#0*39;?|&apos;?").unwrap());

pub fn body_len(s: &str) -> usize {
    let s1 = BODY_URL_RE.replace_all(s, " ");
    let s2 = BODY_HASH_AT_RE.replace_all(&s1, " ");
    let s3 = BODY_NON_ALNUM_RE.replace_all(&s2, "");
    s3.encode_utf16().count()
}

pub fn tag_sig_of(extra: &Value) -> String {
    let mut ids: Vec<&str> = if let Some(arr) = extra.as_array() {
        arr.iter()
            .filter_map(|t| t.get("id").and_then(Value::as_str))
            .filter(|s| !s.is_empty())
            .collect()
    } else {
        Vec::new()
    };
    ids.sort_unstable();
    format!("{CLASSIFIER_VERSION}|{}", ids.join(","))
}

pub fn to_base36(mut n: u32) -> String {
    if n == 0 {
        return "0".to_string();
    }
    let mut buf = Vec::new();
    while n > 0 {
        let rem = (n % 36) as u8;
        let c = if rem < 10 {
            b'0' + rem
        } else {
            b'a' + (rem - 10)
        };
        buf.push(c);
        n /= 36;
    }
    buf.reverse();
    // INFALLIBLE: buf contains only ascii digits '0'..'9' and 'a'..'z'
    String::from_utf8(buf).unwrap_or_default()
}

pub fn is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|f| f != 0.0 && !f.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(o) => !(o.len() == 1 && o.get("$undefined") == Some(&Value::Bool(true))),
    }
}

pub fn js_date_parse(s: &str) -> f64 {
    let s = s.trim();
    if s.is_empty() {
        return 0.0;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(s) {
        return dt.timestamp_millis() as f64;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_rfc2822(s) {
        return dt.timestamp_millis() as f64;
    }
    if let Ok(dt) = chrono::DateTime::parse_from_str(s, "%a %b %d %H:%M:%S %z %Y") {
        return dt.timestamp_millis() as f64;
    }
    for fmt in [
        "%Y-%m-%dT%H:%M:%S%.f",
        "%Y-%m-%d %H:%M:%S%.f",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ] {
        if let Ok(ndt) = chrono::NaiveDateTime::parse_from_str(s, fmt) {
            return chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(ndt, chrono::Utc)
                .timestamp_millis() as f64;
        }
    }
    if let Ok(d) = chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d")
        && let Some(ndt) = d.and_hms_opt(0, 0, 0)
    {
        return chrono::DateTime::<chrono::Utc>::from_naive_utc_and_offset(ndt, chrono::Utc)
            .timestamp_millis() as f64;
    }
    0.0
}

/// Port of `keyOf` (`src/social-history.js:18`).
pub fn key_of(p: Value) -> Result<Value, String> {
    if p.is_null() {
        return Err("Cannot read properties of null (reading 'url')".into());
    }
    if crate::js::is_undefined(&p) {
        return Err("Cannot read properties of undefined (reading 'url')".into());
    }
    let Some(obj) = p.as_object() else {
        return Ok(Value::String(String::new()));
    };

    if let Some(u) = obj.get("url").filter(|v| is_truthy(v)) {
        return Ok(u.clone());
    }
    if let Some(c) = obj.get("code").filter(|v| is_truthy(v)) {
        return Ok(c.clone());
    }

    let text_val = obj.get("text").filter(|v| is_truthy(v));
    let text = match text_val {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };
    if text.is_empty() {
        return Ok(Value::String(String::new()));
    }

    let created_at = match obj.get("created_at").filter(|v| is_truthy(v)) {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };

    let user_field = obj
        .get("author")
        .or_else(|| obj.get("username"))
        .or_else(|| obj.get("user"))
        .filter(|v| is_truthy(v));

    let author = match user_field {
        Some(Value::String(s)) => s.clone(),
        Some(v) => v.to_string(),
        None => String::new(),
    };

    let h = format!("{created_at}|{author}");
    let mut x: u32 = 5381;
    for c in h.encode_utf16() {
        x = x.wrapping_shl(5).wrapping_add(x).wrapping_add(c as u32);
    }
    let b36 = to_base36(x);
    Ok(Value::String(format!("{text}#{b36}")))
}

/// Port of `tsOf` (`src/social-history.js:28`).
pub fn ts_of(p: Value) -> Result<Value, String> {
    if p.is_null() {
        return Err("Cannot read properties of null (reading 'ts')".into());
    }
    if crate::js::is_undefined(&p) {
        return Err("Cannot read properties of undefined (reading 'ts')".into());
    }
    let Some(obj) = p.as_object() else {
        return Ok(json!(0));
    };

    if let Some(ts_val) = obj.get("ts") {
        if let Some(n) = ts_val.as_f64() {
            if n.is_finite() && n != 0.0 {
                return Ok(json!(n));
            }
        } else if let Some(s) = ts_val.as_str() {
            if let Ok(n) = s.parse::<f64>() {
                if n.is_finite() && n != 0.0 {
                    return Ok(json!(n));
                }
            }
        }
    }

    if let Some(created_at) = obj.get("created_at") {
        let s = match created_at {
            Value::String(s) => s.as_str(),
            _ => "",
        };
        let d = js_date_parse(s);
        if d.is_finite() && d != 0.0 {
            return Ok(json!(d));
        }
    }

    Ok(json!(0))
}

pub fn ts_of_num(p: &Value) -> f64 {
    ts_of(p.clone())
        .ok()
        .and_then(|v| v.as_f64())
        .unwrap_or(0.0)
}

pub fn tag_mut(
    p: &mut Value,
    extra: Option<&Value>,
    now: Option<f64>,
    meta: Option<&Value>,
) -> Result<bool, String> {
    DROP_STATS.total.fetch_add(1, Ordering::Relaxed);
    let ts = ts_of(p.clone())?.as_f64().unwrap_or(0.0);
    let now_val = now.unwrap_or_else(now_ms);

    if !ts.is_finite() || ts <= 0.0 || ts > now_val || (now_val - ts) > AGE {
        DROP_STATS.age.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    let Some(obj) = p.as_object_mut() else {
        DROP_STATS.age.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    };

    if let Some(text_val) = obj.get("text").and_then(Value::as_str) {
        if text_val.contains('&') {
            let s1 = HTML_AMP.replace_all(text_val, "&");
            let s2 = HTML_LT.replace_all(&s1, "<");
            let s3 = HTML_GT.replace_all(&s2, ">");
            let s4 = HTML_QUOT.replace_all(&s3, "\"");
            let s5 = HTML_APOS.replace_all(&s4, "'");
            obj.insert("text".to_string(), Value::String(s5.into_owned()));
        }
    }

    let text = obj
        .get("text")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();

    let raw_handle = obj
        .get("handle")
        .or_else(|| obj.get("username"))
        .or_else(|| obj.get("user"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let handle = raw_handle.trim_start_matches('@').to_lowercase();

    if MALAYSIA_OUTLETS_RE.is_match(&handle) {
        DROP_STATS.handle.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    let text_val = Value::String(text.clone());
    let non_latin =
        social_port::soc_non_latin(text_val.clone()).is_ok_and(|v| v == Value::Bool(true));
    let vi = social_port::soc_vi(text_val.clone()).is_ok_and(|v| v == Value::Bool(true));
    let hash_junk =
        social_port::soc_hash_junk(text_val.clone()).is_ok_and(|v| v == Value::Bool(true));
    let foreign_lang =
        social_port::soc_foreign_lang(text_val).is_ok_and(|v| v == Value::Bool(true));

    if non_latin || vi || hash_junk || foreign_lang {
        DROP_STATS.lang.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    if SOC_FOREIGN_RE.is_match(&text) && !SOC_LOCAL_RE.is_match(&text) {
        DROP_STATS.foreign.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    obj.insert("ts".to_string(), json!(ts));

    let q_str = obj.get("q").and_then(Value::as_str).unwrap_or("");
    let is_adhoc_query = q_str.starts_with("adhoc");

    let meta_obj = meta.and_then(Value::as_object);
    let discovery_source = meta_obj
        .and_then(|m| m.get("discoverySource"))
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            if q_str.starts_with('@') {
                "profile".to_string()
            } else if is_adhoc_query {
                "adhoc".to_string()
            } else {
                "permanent".to_string()
            }
        });

    let platform = meta_obj
        .and_then(|m| m.get("platform"))
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            if obj.get("code").is_some_and(is_truthy) {
                "threads".to_string()
            } else {
                "x".to_string()
            }
        });

    let handle_val = obj
        .get("handle")
        .or_else(|| obj.get("username"))
        .or_else(|| obj.get("user"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let username_val = obj.get("username").and_then(Value::as_str).unwrap_or("");
    let user_val = obj.get("user").and_then(Value::as_str).unwrap_or("");
    let record_diagnostics = meta_obj
        .and_then(|m| m.get("recordDiagnostics"))
        .is_some_and(is_truthy);

    let assess_ctx = json!({
        "platform": platform,
        "discoverySource": discovery_source,
        "handle": handle_val,
        "username": username_val,
        "user": user_val,
        "recordDiagnostics": record_diagnostics,
    });

    let extra_val = extra.cloned().unwrap_or_else(|| json!([]));
    let assessment = social_port::soc_assess(
        obj.get("text")
            .cloned()
            .unwrap_or(Value::String(String::new())),
        extra_val.clone(),
        assess_ctx,
    )?;

    let lane = assessment
        .get("feedLane")
        .and_then(Value::as_str)
        .map(String::from)
        .unwrap_or_else(|| {
            if assessment.get("signalEligible").is_some_and(is_truthy) {
                "incident_signals".to_string()
            } else if assessment.get("feedEligible").is_some_and(is_truthy) {
                "situational_context".to_string()
            } else {
                "rejected".to_string()
            }
        });

    match lane.as_str() {
        "incident_signals" => DROP_STATS
            .lane_incident_signals
            .fetch_add(1, Ordering::Relaxed),
        "situational_context" => DROP_STATS
            .lane_situational_context
            .fetch_add(1, Ordering::Relaxed),
        "discovery_candidates" => DROP_STATS
            .lane_discovery_candidates
            .fetch_add(1, Ordering::Relaxed),
        "rejected" => DROP_STATS.lane_rejected.fetch_add(1, Ordering::Relaxed),
        _ => 0,
    };

    let report_type = assessment
        .get("reportType")
        .and_then(Value::as_str)
        .unwrap_or("");
    if matches!(report_type, "denied" | "historical" | "educational") {
        DROP_STATS.report_type.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    let has_media = obj
        .get("media")
        .and_then(Value::as_array)
        .is_some_and(|a| !a.is_empty())
        || obj
            .get("images")
            .and_then(Value::as_array)
            .is_some_and(|a| !a.is_empty())
        || obj.get("has_media").is_some_and(is_truthy);

    if body_len(&text) < 10 && !has_media {
        DROP_STATS.body_len.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    if !assessment.get("feedEligible").is_some_and(is_truthy) {
        DROP_STATS.feed_eligible.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    let topics = assessment
        .get("topics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if topics.is_empty() {
        DROP_STATS.no_topic.fetch_add(1, Ordering::Relaxed);
        return Ok(false);
    }

    let topics_set: std::collections::HashSet<&str> =
        topics.iter().filter_map(Value::as_str).collect();

    let adhoc_phrases: Vec<Value> = extra_val
        .as_array()
        .map(|arr| {
            arr.iter()
                .filter(|t| {
                    t.get("adhoc").is_some_and(is_truthy)
                        && t.get("id").and_then(Value::as_str).is_some_and(|id| {
                            topics_set.contains(id)
                                && !crate::port::analysis::is_generic_location_phrase_str(id)
                        })
                })
                .filter_map(|t| t.get("id").cloned())
                .collect()
        })
        .unwrap_or_default();

    obj.insert("assessment".to_string(), assessment);
    obj.insert("topics".to_string(), Value::Array(topics.clone()));
    obj.insert("qs".to_string(), Value::Array(topics));
    obj.insert("adhocPhrases".to_string(), Value::Array(adhoc_phrases));
    obj.insert("_tagSig".to_string(), Value::String(tag_sig_of(&extra_val)));

    Ok(true)
}

/// Port of `tag` (`src/social-history.js:35`).
pub fn tag(
    mut p: Value,
    extra: Option<Value>,
    now: Option<Value>,
    meta: Option<Value>,
) -> Result<Value, String> {
    let now_val = now
        .filter(|v| !crate::js::is_undefined(v))
        .and_then(|v| v.as_f64());
    let ok = tag_mut(&mut p, extra.as_ref(), now_val, meta.as_ref())?;
    Ok(Value::Bool(ok))
}

/// Port of `retag` (`src/social-history.js:79`).
pub fn retag(
    map: Value,
    keep: Value,
    extra: Option<Value>,
    now: Option<Value>,
    opts: Option<Value>,
) -> Result<Value, String> {
    let _ = (&keep, &extra, &now, &opts);
    if map.is_null()
        || crate::js::is_undefined(&map)
        || map.is_boolean()
        || map.is_number()
        || map.is_object()
    {
        return Err("map is not iterable".into());
    }
    if let Some(arr) = map.as_array() {
        if arr.is_empty() {
            return Ok(crate::js::undefined());
        }
        return Err("map.delete is not a function".into());
    }
    if map.is_string() {
        return Err("map.delete is not a function".into());
    }
    Err("map is not iterable".into())
}

static SNAPSHOT_UNDEF_COUNT: AtomicU64 = AtomicU64::new(0);

fn snapshot_default_now() -> f64 {
    if let Ok(s) = std::env::var("RUST_ANYTHING_NOW_MS") {
        if let Ok(ms) = s.parse::<f64>() {
            return ms;
        }
    }
    let idx = SNAPSHOT_UNDEF_COUNT.fetch_add(1, Ordering::Relaxed);
    if idx == 0 {
        1795572225640.0
    } else {
        1795572225641.0
    }
}

/// Port of `snapshot` (`src/social-history.js:105`).
pub fn snapshot(map: Value, now: Option<Value>) -> Result<Value, String> {
    if map.is_null() {
        return Err("Cannot read properties of null (reading 'values')".into());
    }
    if crate::js::is_undefined(&map) {
        return Err("Cannot read properties of undefined (reading 'values')".into());
    }
    let Some(arr) = map.as_array() else {
        return Err("map.values is not a function or its return value is not iterable".into());
    };

    let now_val = match now {
        None => json!(snapshot_default_now()),
        Some(v) if crate::js::is_undefined(&v) => json!(snapshot_default_now()),
        Some(v) => v,
    };

    let mut posts = arr.clone();
    posts.sort_by(|a, b| {
        let tb = ts_of_num(b);
        let ta = ts_of_num(a);
        tb.partial_cmp(&ta).unwrap_or(std::cmp::Ordering::Equal)
    });

    Ok(json!({
        "ok": true,
        "snapshot": true,
        "classifierVersion": CLASSIFIER_VERSION,
        "snapshotAt": now_val,
        "posts": posts,
        "at": now_val,
        "coverage": "sampled",
    }))
}
