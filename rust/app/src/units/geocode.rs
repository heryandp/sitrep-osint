//! Glue from the Axum front door to `geocode-rs` (`GET /api/nodes`, `GET /api/geo/labels`).

use std::collections::HashMap;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use geocode_rs::port::service::{nodes, query_map_labels_typed};
use serde_json::json;

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as u64)
}

fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    let q = query?;
    for pair in q.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            return Some(it.next().unwrap_or("").to_string());
        }
    }
    None
}

fn json_resp(status: StatusCode, cache_control: &'static str, body: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, cache_control),
            (header::REFERRER_POLICY, "same-origin"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

pub async fn handle(route: &str, _params: HashMap<String, String>, req: Request) -> Response {
    match route {
        "GET /api/nodes" => {
            let ts = now_ms();
            let list = nodes();
            let items: Vec<serde_json::Value> = list
                .iter()
                .map(|p| {
                    json!({
                        "name": p.name,
                        "la": p.la,
                        "lo": p.lo,
                        "prov": p.prov,
                        "prec": "kab",
                        "kind": "node",
                        "label": "Node",
                        "title": format!("Node pemantauan · {}", p.name),
                        "src": "NODE",
                        "url": "",
                        "ts": ts,
                        "weight": 0.16,
                        "fixed": true
                    })
                })
                .collect();
            json_resp(
                StatusCode::OK,
                "public, max-age=86400",
                json!({ "ok": true, "items": items, "places": list.len() }).to_string(),
            )
        }
        "GET /api/geo/labels" => {
            let q = req.uri().query();
            let bbox_raw = query_param(q, "bbox").unwrap_or_default();
            let parts: Vec<&str> = bbox_raw.split(',').map(str::trim).collect();
            let nums: Vec<f64> = parts
                .iter()
                .map(|s| s.parse::<f64>().unwrap_or(f64::NAN))
                .collect();
            let z_num = query_param(q, "z")
                .and_then(|s| s.parse::<f64>().ok())
                .filter(|v| v.is_finite() && *v != 0.0)
                .unwrap_or(5.0);
            let z = (z_num.round() as i64).clamp(4, 18) as f64;
            if parts.len() != 4
                || parts.iter().any(|s| s.is_empty())
                || nums.iter().any(|v| !v.is_finite())
            {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    r#"{"ok":false,"err":"bbox invalid"}"#.to_string(),
                );
            }
            let (s, w, n, e) = (nums[0], nums[1], nums[2], nums[3]);
            if s > n || w > e || s < -90.0 || n > 90.0 || w < -180.0 || e > 180.0 {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    r#"{"ok":false,"err":"bbox out of range"}"#.to_string(),
                );
            }
            let limit = query_param(q, "limit")
                .and_then(|s| s.parse::<usize>().ok())
                .filter(|&v| v > 0)
                .unwrap_or(160);
            let items = query_map_labels_typed(s, w, n, e, Some(z), Some(limit));
            json_resp(
                StatusCode::OK,
                "public, max-age=60, s-maxage=300, stale-while-revalidate=900",
                json!({ "ok": true, "z": z as i64, "items": items }).to_string(),
            )
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
