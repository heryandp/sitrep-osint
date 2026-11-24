//! The Rust port of `src/tomtom.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;

use serde_json::Value;

/// Convert JS value to f64 following JS semantics for `Number(v)`
pub(crate) fn to_js_number(v: &Value) -> f64 {
    if v.is_null() {
        return 0.0;
    }
    if crate::js::is_undefined(v) {
        return f64::NAN;
    }
    if let Some(b) = v.as_bool() {
        return if b { 1.0 } else { 0.0 };
    }
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
    if let Some(s) = v.as_str() {
        let s = s.trim();
        return if s.is_empty() {
            0.0
        } else {
            s.parse::<f64>().unwrap_or(f64::NAN)
        };
    }
    if let Some(o) = v.as_object() {
        if let Some(s) = o.get("$bigint").and_then(|b| b.as_str()) {
            return s.parse::<f64>().unwrap_or(f64::NAN);
        }
        return f64::NAN;
    }
    if let Some(arr) = v.as_array() {
        return if arr.is_empty() {
            0.0
        } else if arr.len() == 1 {
            to_js_number(&arr[0])
        } else {
            f64::NAN
        };
    }
    f64::NAN
}

/// Convert property value to f64 for object properties like `x.lat`
fn to_prop_f64(v: Option<&Value>) -> f64 {
    match v {
        None => f64::NAN,
        Some(val) => to_js_number(val),
    }
}

/// Port of `haversine` (`src/tomtom.js:3`).
pub fn haversine(a: Value, b: Value) -> Result<Value, String> {
    if b.is_null() {
        return Err("Cannot read properties of null (reading 'lat')".into());
    }
    if crate::js::is_undefined(&b) {
        return Err("Cannot read properties of undefined (reading 'lat')".into());
    }
    if a.is_null() {
        return Err("Cannot read properties of null (reading 'lat')".into());
    }
    if crate::js::is_undefined(&a) {
        return Err("Cannot read properties of undefined (reading 'lat')".into());
    }

    let b_lat = to_prop_f64(b.as_object().and_then(|o| o.get("lat")));
    let a_lat = to_prop_f64(a.as_object().and_then(|o| o.get("lat")));
    let b_lon = to_prop_f64(b.as_object().and_then(|o| o.get("lon")));
    let a_lon = to_prop_f64(a.as_object().and_then(|o| o.get("lon")));

    let rad = |x: f64| x * std::f64::consts::PI / 180.0;
    let d_lat = rad(b_lat - a_lat);
    let d_lon = rad(b_lon - a_lon);
    let s = (d_lat / 2.0).sin().powi(2)
        + rad(a_lat).cos() * rad(b_lat).cos() * (d_lon / 2.0).sin().powi(2);
    let dist = 2.0 * 6371.0 * s.sqrt().asin();

    Ok(crate::js::number(dist))
}

/// Port of `roundGrid` (`src/tomtom.js:10`).
pub fn round_grid(v: Value, step: Value) -> Result<Value, String> {
    if step.as_object().is_some_and(|o| o.contains_key("$bigint")) {
        return Err("Cannot mix BigInt and other types, use explicit conversions".into());
    }
    let v_num = to_js_number(&v);
    let step_num = to_js_number(&step);
    let quotient = v_num / step_num;
    let rounded = crate::js::math_round(quotient);
    let result = rounded * step_num;
    Ok(crate::js::number(result))
}

/// Port of `roundGridStr` (`src/tomtom.js:11`).
pub fn round_grid_str(v: Value, step: Value, d: Option<f64>) -> Result<Value, String> {
    let d_val = d.unwrap_or(2.0);
    if d_val < 0.0 || d_val > 100.0 || d_val.is_infinite() {
        return Err("toFixed() digits argument must be between 0 and 100".into());
    }
    let digits = if d_val.is_nan() {
        0usize
    } else {
        d_val.trunc() as usize
    };

    let v_num = to_js_number(&v);
    let step_num = to_js_number(&step);
    let quotient = v_num / step_num;
    let rounded = crate::js::math_round(quotient);
    let grid_num = rounded * step_num;

    if grid_num.is_nan() {
        return Ok(Value::String("NaN".into()));
    }
    if grid_num.is_infinite() {
        return Ok(Value::String(
            if grid_num > 0.0 {
                "Infinity"
            } else {
                "-Infinity"
            }
            .into(),
        ));
    }

    let formatted = if grid_num == 0.0 {
        format!("{:.prec$}", 0.0, prec = digits)
    } else {
        format!("{grid_num:.prec$}", prec = digits)
    };

    Ok(Value::String(formatted))
}

/// Port of `ttHostOK` (`src/tomtom.js:9`).
pub fn tt_host_ok(u: Value) -> Result<Value, String> {
    let Some(s) = u.as_str() else {
        return Ok(Value::Bool(false));
    };
    let Ok(url) = url::Url::parse(s) else {
        return Ok(Value::Bool(false));
    };
    let Some(host) = url.host_str() else {
        return Ok(Value::Bool(false));
    };
    let ok = host == "api.tomtom.com" || host.ends_with(".api.tomtom.com");
    Ok(Value::Bool(ok))
}
