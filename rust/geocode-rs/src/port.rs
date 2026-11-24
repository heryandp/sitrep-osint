//! The Rust port of `src/geocode.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;

use serde_json::Value;

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
        Value::Array(arr) => arr
            .iter()
            .map(|x| {
                if x.is_null() || crate::js::is_undefined(x) {
                    String::new()
                } else {
                    js_to_string(x)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => "[object Object]".into(),
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
    const BAD: &[&str] = &[
        "nan",
        "inf",
        "-inf",
        "+inf",
        "infinity",
        "-infinity",
        "+infinity",
    ];
    if BAD.iter().any(|x| t.eq_ignore_ascii_case(x)) {
        return f64::NAN;
    }
    for (p, r) in [
        ("0x", 16),
        ("0X", 16),
        ("0o", 8),
        ("0O", 8),
        ("0b", 2),
        ("0B", 2),
    ] {
        if let Some(rest) = t.strip_prefix(p) {
            return u64::from_str_radix(rest, r)
                .map(|v| v as f64)
                .unwrap_or(f64::NAN);
        }
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_to_number(v: Option<&Value>) -> f64 {
    let Some(v) = v.filter(|v| !crate::js::is_undefined(v)) else {
        return f64::NAN;
    };
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => f64::from(u8::from(*b)),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => parse_js_number(s),
        Value::Array(arr) => match arr.as_slice() {
            [] => 0.0,
            [x] if x.is_null() || crate::js::is_undefined(x) => 0.0,
            [x] => js_to_number(Some(x)),
            _ => f64::NAN,
        },
        Value::Object(_) => f64::NAN,
    }
}

/// Port of `locate` (`src/geocode.js:455`).
pub fn locate(title: Value) -> Result<Value, String> {
    let g = service::gazetteer()?;
    let s = js_to_string(&title);
    Ok(g.locate(&s).unwrap_or(Value::Null))
}

/// Port of `norm` (`src/geocode.js:17`).
pub fn norm(s: Value) -> Result<Value, String> {
    Ok(Value::String(service::norm_str(&js_to_string(&s))))
}

/// Port of `queryMapLabels` (`src/geocode.js:658`).
pub fn query_map_labels(opts: Value) -> Result<Value, String> {
    if opts.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 's')".into());
    }
    let g = service::gazetteer()?;
    let obj = if crate::js::is_undefined(&opts) {
        None
    } else {
        opts.as_object()
    };
    let get = |k: &str| obj.and_then(|m| m.get(k));
    let s = js_to_number(get("s"));
    let w = js_to_number(get("w"));
    let n = js_to_number(get("n"));
    let e = js_to_number(get("e"));
    let z = js_to_number(get("z"));
    let limit = js_to_number(get("limit"));
    let items = g.query_map_labels_raw(s, w, n, e, z, limit);
    serde_json::to_value(items).map_err(|err| err.to_string())
}
