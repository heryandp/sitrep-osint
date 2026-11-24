//! The Rust port of `src/exa-summary.js`.

#![forbid(unsafe_code)]
#![allow(clippy::collapsible_if)]

#[path = "service.rs"]
pub mod service;
pub use service::{ExaSummaryService, ServiceConfig, StatusResponse, wib_date_key};

use regex::Regex;
use serde_json::{Value, json};
use std::sync::LazyLock;

static PRIVATE_HOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex pattern is verified at compile time
    Regex::new(
        r"(?i)^(?:localhost|127\.|10\.|192\.168\.|169\.254\.|0\.0\.0\.0|\[::1\]|172\.(?:1[6-9]|2\d|3[01])\.)",
    )
    .expect("valid regex")
});

static GNEWS_HOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex pattern is verified at compile time
    Regex::new(r"(?i)(?:^|\.)news\.google\.com$").expect("valid regex")
});

static ENTITY_NUM_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex pattern is verified at compile time
    Regex::new(r"(?i)\b(?:Rp\s?[\d.,]+\s*(?:triliun|miliar|juta|T|M)\b|\d+(?:[.,]\d+)?%)")
        .expect("valid regex")
});

static ENTITY_PATTERNS: LazyLock<Vec<Regex>> = LazyLock::new(|| {
    vec![
        Regex::new(r"(?i)\bCoretax\b").expect("valid regex"),
        Regex::new(r"\bDJP\b").expect("valid regex"),
        Regex::new(r"(?i)\bKemenkeu\b").expect("valid regex"),
        Regex::new(r"(?i)\bMenkeu\b").expect("valid regex"),
        Regex::new(r"\bAPBN\b").expect("valid regex"),
        Regex::new(r"\bPNBP\b").expect("valid regex"),
        Regex::new(r"\bDJBC\b").expect("valid regex"),
        Regex::new(r"(?i)\bBea Cukai\b").expect("valid regex"),
        Regex::new(r"\bPPATK\b").expect("valid regex"),
        Regex::new(r"(?i)\bKPP\s+(?:Pratama|Madya|Khusus)\b").expect("valid regex"),
        Regex::new(r"(?i)\bPengadilan Pajak\b").expect("valid regex"),
        Regex::new(r"(?i)\bKomisi XI\b").expect("valid regex"),
        Regex::new(r"\bPPN\b").expect("valid regex"),
        Regex::new(r"(?i)\bPPh(?:\s*(?:21|22|23|25|29|Final|Badan))?\b").expect("valid regex"),
        Regex::new(r"(?i)\bSPT(?:\s+Tahunan)?\b").expect("valid regex"),
        Regex::new(r"\bNPWP\b").expect("valid regex"),
        Regex::new(r"(?i)\bPMK\s*(?:No\.?|Nomor)?\s*\d+[^\s,.;]*").expect("valid regex"),
        Regex::new(r"(?i)\bUU\s+HPP\b").expect("valid regex"),
        Regex::new(r"(?i)\bTax\s+Ratio\b").expect("valid regex"),
        Regex::new(r"(?i)\bBapenda\b").expect("valid regex"),
        Regex::new(r"(?i)\bSamsat\b").expect("valid regex"),
    ]
});

static TRAILING_WORD_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex pattern is verified at compile time
    Regex::new(r"\s+\S*$").expect("valid regex")
});

static ATENSI_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex pattern is verified at compile time
    Regex::new(r"(?i)coretax|error|gangguan|down|pidana|sita|perampasan|boikot|ppatk")
        .expect("valid regex")
});

static WHITESPACE_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex pattern is verified at compile time
    Regex::new(r"\s+").expect("valid regex")
});

fn js_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                f != 0.0 && !f.is_nan()
            } else {
                true
            }
        }
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if crate::js::is_undefined(v) {
                false
            } else if let Some(n) = crate::js::as_number(v) {
                n != 0.0 && !n.is_nan()
            } else {
                true
            }
        }
    }
}

fn js_to_string_in_array(v: &Value) -> String {
    match v {
        Value::Null => String::new(),
        Value::Object(_) if crate::js::is_undefined(v) => String::new(),
        _ => js_to_string(v),
    }
}

pub fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => {
            if let Some(f) = n.as_f64() {
                crate::js::number_to_string(f)
            } else {
                n.to_string()
            }
        }
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let parts: Vec<String> = arr.iter().map(js_to_string_in_array).collect();
            parts.join(",")
        }
        Value::Object(_) => {
            if crate::js::is_undefined(v) {
                "undefined".to_string()
            } else if let Some(n) = crate::js::as_number(v) {
                crate::js::number_to_string(n)
            } else if let Some(u) = crate::js::string_units(v) {
                String::from_utf16_lossy(&u)
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

pub fn js_str_or_empty(v: &Value) -> String {
    if !js_truthy(v) {
        String::new()
    } else {
        js_to_string(v)
    }
}

fn utf16_slice(s: &str, start: usize, end: usize) -> String {
    let units: Vec<u16> = s.encode_utf16().collect();
    let len = units.len();
    let start = start.min(len);
    let end = end.min(len);
    if start >= end {
        String::new()
    } else {
        String::from_utf16_lossy(&units[start..end])
    }
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn is_google_news_host(host: &str) -> bool {
    GNEWS_HOST_RE.is_match(host)
}

/// Port of `isValidPublicHttpUrl` (`src/exa-summary.js:26`).
pub fn is_valid_public_http_url(raw: Value) -> Result<Value, String> {
    let s = js_str_or_empty(&raw);
    let s = s.trim();
    if s.is_empty() {
        return Ok(Value::Bool(false));
    }
    let Ok(u) = url::Url::parse(s) else {
        return Ok(Value::Bool(false));
    };
    if u.scheme() != "http" && u.scheme() != "https" {
        return Ok(Value::Bool(false));
    }
    if !u.username().is_empty() || u.password().is_some() {
        return Ok(Value::Bool(false));
    }
    let Some(host) = u.host_str() else {
        return Ok(Value::Bool(false));
    };
    if !host.contains('.') || PRIVATE_HOST_RE.is_match(host) {
        return Ok(Value::Bool(false));
    }
    Ok(Value::Bool(true))
}

pub fn extract_key_entities_inner(s: &str) -> Vec<String> {
    let mut found = Vec::new();
    let add = |found: &mut Vec<String>, v: &str| {
        let clean = v.trim();
        if !clean.is_empty()
            && !found
                .iter()
                .any(|x| x.to_lowercase() == clean.to_lowercase())
        {
            found.push(clean.to_string());
        }
    };

    for m in ENTITY_NUM_RE.find_iter(s) {
        add(&mut found, m.as_str());
    }

    for re in ENTITY_PATTERNS.iter() {
        if let Some(m) = re.find(s) {
            add(&mut found, m.as_str());
        }
        if found.len() >= 6 {
            break;
        }
    }
    found.truncate(6);
    found
}

/// Port of `extractKeyEntities` (`src/exa-summary.js:47`).
pub fn extract_key_entities(text: Value) -> Result<Value, String> {
    let s = js_str_or_empty(&text);
    let found = extract_key_entities_inner(&s);
    Ok(Value::Array(found.into_iter().map(Value::String).collect()))
}

fn build_fallback_summary_from_values(
    title_val: &Value,
    desc_val: &Value,
    theme_val: &Value,
    urls_val: &Value,
    reason_val: &Value,
) -> Value {
    let clean_title = js_str_or_empty(title_val).trim().to_string();
    let raw_desc = js_str_or_empty(desc_val);
    let clean_desc = WHITESPACE_RE.replace_all(&raw_desc, " ").trim().to_string();
    let combined = format!("{clean_title} {clean_desc}").trim().to_string();
    let entities = extract_key_entities_inner(&combined);

    let desc_len = utf16_len(&clean_desc);
    let ringkasan = if !clean_desc.is_empty() && desc_len > 36 {
        if desc_len > 340 {
            let sliced = utf16_slice(&clean_desc, 0, 340);
            let trimmed = TRAILING_WORD_RE.replace(&sliced, "");
            format!("{trimmed}…")
        } else {
            clean_desc
        }
    } else if !clean_title.is_empty() {
        clean_title.clone()
    } else {
        "Ringkasan berita perpajakan tidak tersedia.".to_string()
    };

    let theme_label = js_str_or_empty(theme_val);
    let dampak = if !theme_label.is_empty() {
        let suffix = if !entities.is_empty() {
            let top3: Vec<String> = entities.iter().take(3).cloned().collect();
            format!(" ({})", top3.join(", "))
        } else {
            String::new()
        };
        format!(
            "Terkait dimensi {theme_label} pada pemantauan perpajakan & keuangan negara{suffix}."
        )
    } else {
        "Memerlukan pemantauan lanjutan terkait implikasi kebijakan fiskal, penerimaan negara, atau layanan wajib pajak DJP.".to_string()
    };

    let atensi = if ATENSI_RE.is_match(&combined) {
        "Perlu verifikasi teknis/humas segera dan pemantauan sentimen lintas kanal."
    } else {
        "Pantau perkembangan pemberitaan lanjutan serta respons pemangku kepentingan."
    };

    let sources = match urls_val {
        Value::Array(arr) => arr
            .iter()
            .take(3)
            .map(|u| {
                json!({
                    "url": u,
                    "title": clean_title,
                })
            })
            .collect(),
        _ => Vec::new(),
    };

    let reason_str = js_str_or_empty(reason_val);
    let fallback_reason = if reason_str.is_empty() {
        "local_extractive".to_string()
    } else {
        reason_str
    };

    json!({
        "ok": true,
        "mode": "fallback",
        "fallbackReason": fallback_reason,
        "ringkasan": ringkasan,
        "dampak_djp": dampak,
        "entitas_kunci": entities,
        "atensi": atensi,
        "sources": sources,
    })
}

/// Port of `buildFallbackSummary` (`src/exa-summary.js:70`).
pub fn build_fallback_summary(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'title')".to_string());
    }
    if crate::js::is_undefined(&arg0) {
        return Err("TypeError: Cannot read properties of undefined (reading 'title')".to_string());
    }

    let (title, desc, theme_label, urls, reason) = if let Some(obj) = arg0.as_object() {
        (
            obj.get("title").unwrap_or(&Value::Null),
            obj.get("desc").unwrap_or(&Value::Null),
            obj.get("themeLabel").unwrap_or(&Value::Null),
            obj.get("urls").unwrap_or(&Value::Null),
            obj.get("reason").unwrap_or(&Value::Null),
        )
    } else {
        (
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &Value::Null,
            &Value::Null,
        )
    };

    Ok(build_fallback_summary_from_values(
        title,
        desc,
        theme_label,
        urls,
        reason,
    ))
}

/// Port of `parseExaSummaryObject` (`src/exa-summary.js:96`).
pub fn parse_exa_summary_object(
    raw_summary: Value,
    fallback_title: Value,
    fallback_desc: Value,
) -> Result<Value, String> {
    let is_js_object_or_array = (raw_summary.is_object()
        && !crate::js::is_undefined(&raw_summary)
        && crate::js::as_number(&raw_summary).is_none()
        && crate::js::string_units(&raw_summary).is_none())
        || raw_summary.is_array();

    if is_js_object_or_array {
        let ringkasan = raw_summary
            .as_object()
            .and_then(|o| o.get("ringkasan"))
            .map(js_str_or_empty)
            .unwrap_or_default()
            .trim()
            .to_string();
        let dampak_djp = raw_summary
            .as_object()
            .and_then(|o| o.get("dampak_djp"))
            .map(js_str_or_empty)
            .unwrap_or_default()
            .trim()
            .to_string();
        let entitas_kunci = raw_summary
            .as_object()
            .and_then(|o| o.get("entitas_kunci"))
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .map(|x| js_str_or_empty(x).trim().to_string())
                    .filter(|s| !s.is_empty())
                    .take(6)
                    .collect::<Vec<String>>()
            })
            .unwrap_or_default();
        let atensi = raw_summary
            .as_object()
            .and_then(|o| o.get("atensi"))
            .map(js_str_or_empty)
            .unwrap_or_default()
            .trim()
            .to_string();
        return Ok(json!({
            "ringkasan": ringkasan,
            "dampak_djp": dampak_djp,
            "entitas_kunci": entitas_kunci,
            "atensi": atensi,
        }));
    }

    let str = js_str_or_empty(&raw_summary).trim().to_string();
    if str.starts_with('{') {
        if let Ok(obj) = serde_json::from_str::<Value>(&str) {
            if obj.is_object() || obj.is_array() {
                return parse_exa_summary_object(obj, fallback_title, fallback_desc);
            }
        }
    }

    if !str.is_empty() {
        let sliced_ringkasan = utf16_slice(&str, 0, 500);
        let fb_title_str = js_to_string(&fallback_title);
        let text_for_ents = format!("{str} {fb_title_str}");
        let entitas_kunci = extract_key_entities_inner(&text_for_ents);
        return Ok(json!({
            "ringkasan": sliced_ringkasan,
            "dampak_djp": "Perlu pemantauan dampak terhadap penerimaan pajak, kebijakan fiskal, dan layanan DJP.",
            "entitas_kunci": entitas_kunci,
            "atensi": "Tinjau sumber asli untuk tindak lanjut unit kerja terkait.",
        }));
    }

    let fb = build_fallback_summary_from_values(
        &fallback_title,
        &fallback_desc,
        &Value::Null,
        &Value::Null,
        &Value::Null,
    );
    Ok(json!({
        "ringkasan": fb["ringkasan"],
        "dampak_djp": fb["dampak_djp"],
        "entitas_kunci": fb["entitas_kunci"],
        "atensi": fb["atensi"],
    }))
}
