//! The Rust port of `src/uptime-service.js`.

#![forbid(unsafe_code)]

mod pinned;
#[path = "service.rs"]
pub mod service;
pub use service::{
    DJP_PORTALS, DomainStatus, FetchFn, NowFn, Portal, ProbeResponse, UptimePayload, UptimeService,
};

use crate::js;
use chrono::DateTime;
use serde_json::{Value, json};

pub const SLOW_MS: f64 = 2500.0;

fn get_prop<'a>(v: &'a Value, key: &str) -> Option<&'a Value> {
    let ok = v.is_object()
        && !js::is_undefined(v)
        && js::as_number(v).is_none()
        && js::string_units(v).is_none();
    ok.then(|| v.as_object()?.get(key)).flatten()
}

fn is_truthy(v: &Value) -> bool {
    if let Some(f) = js::as_number(v) {
        return f != 0.0 && !f.is_nan();
    }
    match v {
        Value::Null | Value::Number(_) => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            !js::is_undefined(v) && js::string_units(v).is_none_or(|u| !u.is_empty())
        }
    }
}

fn js_to_utf16(v: &Value) -> Vec<u16> {
    if let Some(u) = js::string_units(v) {
        return u;
    }
    if let Some(f) = js::as_number(v) {
        return js::number_to_string(f).encode_utf16().collect();
    }
    match v {
        Value::Array(xs) => xs
            .iter()
            .map(|x| {
                if x.is_null() || js::is_undefined(x) {
                    Vec::new()
                } else {
                    js_to_utf16(x)
                }
            })
            .collect::<Vec<_>>()
            .join(&[b',' as u16][..]),
        Value::Null => "null".encode_utf16().collect(),
        Value::Bool(b) => (if *b { "true" } else { "false" }).encode_utf16().collect(),
        _ if js::is_undefined(v) => "undefined".encode_utf16().collect(),
        _ => "[object Object]".encode_utf16().collect(),
    }
}

fn js_to_number(v: &Value) -> f64 {
    if let Some(f) = js::as_number(v) {
        return f;
    }
    let owned = match v {
        Value::Null => return 0.0,
        Value::Bool(b) => return f64::from(u8::from(*b)),
        Value::Array(a) if a.is_empty() => return 0.0,
        Value::Array(a) if a.len() == 1 && (a[0].is_null() || js::is_undefined(&a[0])) => {
            return 0.0;
        }
        Value::Array(a) if a.len() == 1 => String::from_utf16_lossy(&js_to_utf16(&a[0])),
        _ => match js::string_units(v).and_then(|u| String::from_utf16(&u).ok()) {
            Some(s) => s,
            None => return f64::NAN,
        },
    };
    let t =
        owned.trim_matches(|c: char| c.is_whitespace() || "\u{000B}\u{000C}\u{FEFF}".contains(c));
    match t {
        "" => 0.0,
        "Infinity" | "+Infinity" => f64::INFINITY,
        "-Infinity" => f64::NEG_INFINITY,
        _ if t.len() > 2 && t.starts_with('0') && b"xXbBoO".contains(&t.as_bytes()[1]) => {
            let r = match t.as_bytes()[1] {
                b'x' | b'X' => 16,
                b'b' | b'B' => 2,
                _ => 8,
            };
            u128::from_str_radix(&t[2..], r).map_or(f64::NAN, |n| n as f64)
        }
        _ if t
            .bytes()
            .all(|b| b.is_ascii_digit() || b"+-.eE".contains(&b)) =>
        {
            t.parse().unwrap_or(f64::NAN)
        }
        _ => f64::NAN,
    }
}

fn js_date_parse(s: &str) -> f64 {
    let t = s.trim();
    let norm = match (t.len(), t.as_bytes()) {
        (4, b) if b.iter().all(u8::is_ascii_digit) => format!("{t}-01-01T00:00:00Z"),
        (7, b) if b[4] == b'-' => format!("{t}-01T00:00:00Z"),
        (10, b) if b[4] == b'-' && b[7] == b'-' => format!("{t}T00:00:00Z"),
        _ => {
            let mut n = t.replacen(' ', "T", 1);
            if let Some((_, time)) = n.split_once('T') {
                let tz = time.ends_with(['Z', 'z']) || time.contains(['+', '-']);
                if time.len() == 5 && time.as_bytes()[2] == b':' {
                    n.push_str(":00");
                }
                if !tz {
                    n.push('Z');
                }
            }
            n
        }
    };
    DateTime::parse_from_rfc3339(&norm)
        .or_else(|_| DateTime::parse_from_rfc2822(t))
        .ok()
        .map(|dt| dt.timestamp_millis())
        .filter(|m| m.abs() <= 8_640_000_000_000_000)
        .map_or(f64::NAN, |m| m as f64)
}

/// Port of `normalizeCheckResult` (`src/uptime-service.js:14`).
pub fn normalize_check_result(portal: Value, raw: Value, now: Value) -> Result<Value, String> {
    let now_val = if js::is_undefined(&now) {
        js::number(pinned::now_ms())
    } else {
        now
    };
    if portal.is_null() || js::is_undefined(&portal) {
        let kind = if portal.is_null() {
            "null"
        } else {
            "undefined"
        };
        return Err(format!("Cannot read properties of {kind} (reading 'id')"));
    }
    let prop = |k| is_truthy(&raw).then(|| get_prop(&raw, k)).flatten();
    let p = |k| get_prop(&portal, k).cloned().unwrap_or_else(js::undefined);
    let num = |k| {
        prop(k)
            .filter(|v| !v.is_null() && !js::is_undefined(v))
            .map(js_to_number)
            .filter(|n| n.is_finite())
    };
    let str_prop = |k, max: usize| {
        prop(k).filter(|v| is_truthy(v)).map_or(Value::Null, |v| {
            let u = js_to_utf16(v);
            js::utf16_string(&u[..u.len().min(max)])
        })
    };
    let is_up = prop("status")
        .filter(|v| is_truthy(v))
        .is_some_and(|v| String::from_utf16_lossy(&js_to_utf16(v)).eq_ignore_ascii_case("up"));
    let ms = num("response_time_ms").map(js::math_round);
    let state = match (is_up, ms) {
        (false, _) => "down",
        (true, Some(m)) if m >= SLOW_MS => "slow",
        _ => "up",
    };
    let checked_at = prop("checked_at")
        .filter(|v| is_truthy(v))
        .map(|v| js_date_parse(&String::from_utf16_lossy(&js_to_utf16(v))))
        .filter(|n| n.is_finite() && *n != 0.0)
        .map_or(now_val, js::number);
    Ok(json!({
        "id": p("id"), "name": p("name"), "domain": p("domain"), "url": p("url"),
        "status": if is_up { "up" } else { "down" }, "state": state,
        "httpCode": num("http_code").map_or(Value::Null, js::number),
        "responseMs": ms.map_or(Value::Null, js::number),
        "ip": str_prop("ip", usize::MAX), "error": str_prop("error", 140), "checkedAt": checked_at,
    }))
}
