//! The Rust port of `src/copernicus.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;

use serde_json::Value;

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

fn js_to_number(v: &Value) -> f64 {
    if crate::js::is_undefined(v) {
        return f64::NAN;
    }
    if let Some(n) = crate::js::as_number(v) {
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
        Value::String(s) => parse_js_number(s),
        Value::Array(arr) => match arr.as_slice() {
            [] => 0.0,
            [x] if x.is_null() || crate::js::is_undefined(x) => 0.0,
            [x] => js_to_number(x),
            _ => f64::NAN,
        },
        Value::Object(_) => f64::NAN,
    }
}

/// Port of `tileToBbox3857` (`src/copernicus.js:27`).
pub fn tile_to_bbox3857(z: Value, x: Value, y: Value) -> Result<Value, String> {
    let z_n = js_to_number(&z);
    let x_n = js_to_number(&x);
    let y_n = js_to_number(&y);

    let [min_x, min_y, max_x, max_y] = service::tile_to_bbox3857_coords(z_n, x_n, y_n);

    Ok(Value::Array(vec![
        crate::js::number(min_x),
        crate::js::number(min_y),
        crate::js::number(max_x),
        crate::js::number(max_y),
    ]))
}

/// Port of `tileToBbox4326` (`src/copernicus.js:37`).
pub fn tile_to_bbox4326(z: Value, x: Value, y: Value) -> Result<Value, String> {
    let z_n = js_to_number(&z);
    let x_n = js_to_number(&x);
    let y_n = js_to_number(&y);

    let [lon_min, lat_min, lon_max, lat_max] = service::tile_to_bbox4326_coords(z_n, x_n, y_n);

    Ok(Value::Array(vec![
        crate::js::number(lon_min),
        crate::js::number(lat_min),
        crate::js::number(lon_max),
        crate::js::number(lat_max),
    ]))
}

/// Port of `tileIntersectsBounds` (`src/copernicus.js:44`).
pub fn tile_intersects_bounds(
    z: Value,
    x: Value,
    y: Value,
    bounds: Value,
) -> Result<Value, String> {
    let z_n = js_to_number(&z);
    let x_n = js_to_number(&x);
    let y_n = js_to_number(&y);

    let [min_lon, min_lat, max_lon, max_lat] = service::tile_to_bbox4326_coords(z_n, x_n, y_n);

    if bounds.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'minLon')".into());
    }

    let (b_min_lon, b_min_lat, b_max_lon, b_max_lat) = if crate::js::is_undefined(&bounds) {
        (
            service::ID_BOUNDS.min_lon,
            service::ID_BOUNDS.min_lat,
            service::ID_BOUNDS.max_lon,
            service::ID_BOUNDS.max_lat,
        )
    } else if let Some(obj) = bounds.as_object() {
        let get = |k: &str| match obj.get(k) {
            Some(v) if !crate::js::is_undefined(v) => js_to_number(v),
            _ => f64::NAN,
        };
        (get("minLon"), get("minLat"), get("maxLon"), get("maxLat"))
    } else {
        (f64::NAN, f64::NAN, f64::NAN, f64::NAN)
    };

    let intersects = max_lon >= b_min_lon
        && min_lon <= b_max_lon
        && max_lat >= b_min_lat
        && min_lat <= b_max_lat;

    Ok(Value::Bool(intersects))
}
