//! Glue from the Axum front door to `tomtom-rs` (`/api/tomtom/*`, `/tomtom/tile/{*path}`).

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::json;
use tomtom_rs::port::service::{LatLon, TomTomService};

static TOMTOM: LazyLock<TomTomService> = LazyLock::new(|| {
    let key = std::env::var("TOMTOM_API_KEY")
        .or_else(|_| std::env::var("TOMTOM_KEY"))
        .ok();
    TomTomService::new(key)
});

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

pub async fn handle(route: &str, params: HashMap<String, String>, req: Request) -> Response {
    let q = req.uri().query();
    match route {
        "GET /api/tomtom/status" => {
            let configured = TOMTOM.tt_has();
            let has_env_key = std::env::var("TOMTOM_API_KEY")
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false)
                || std::env::var("TOMTOM_KEY")
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false);
            let source = if has_env_key { "env" } else { "none" };
            json_resp(
                StatusCode::OK,
                "no-store",
                json!({ "ok": true, "keyConfigured": configured, "source": source, "at": now_ms() }).to_string(),
            )
        }
        "GET /api/tomtom/flow" => {
            let lat_opt = query_param(q, "lat").and_then(|s| s.parse::<f64>().ok());
            let lon_opt = query_param(q, "lon").and_then(|s| s.parse::<f64>().ok());
            let zoom = query_param(q, "zoom").and_then(|s| s.parse::<f64>().ok());

            let (Some(lat), Some(lon)) = (lat_opt, lon_opt) else {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    r#"{"ok":false,"err":"lat/lon wajib"}"#.to_string(),
                );
            };

            let res = TOMTOM.flow(lat, lon, zoom).await;
            json_resp(StatusCode::OK, "no-store", res.to_string())
        }
        "GET /api/tomtom/incidents" => {
            let bbox = query_param(q, "bbox");
            let bbox_str = if let Some(b) = bbox {
                b
            } else if let (Some(lat_s), Some(lon_s)) = (query_param(q, "lat"), query_param(q, "lon")) {
                let lat = lat_s.parse::<f64>().unwrap_or(0.0);
                let lon = lon_s.parse::<f64>().unwrap_or(0.0);
                let d = query_param(q, "d")
                    .and_then(|s| s.parse::<f64>().ok())
                    .unwrap_or(0.25)
                    .clamp(0.05, 2.5);
                format!("{},{},{},{}", lon - d, lat - d, lon + d, lat + d)
            } else {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    r#"{"ok":false,"err":"bbox wajib (minLon,minLat,maxLon,maxLat) atau lat+lon"}"#.to_string(),
                );
            };

            let lang = query_param(q, "lang");
            let res = TOMTOM.incidents(&bbox_str, lang.as_deref()).await;
            json_resp(StatusCode::OK, "no-store", res.to_string())
        }
        "GET /api/tomtom/route" => {
            let from_s = query_param(q, "from");
            let to_s = query_param(q, "to");
            let mode = query_param(q, "mode");

            let parse_coord = |s: Option<String>| {
                let s = s?;
                let mut it = s.split(',');
                let lat = it.next()?.trim().parse::<f64>().ok()?;
                let lon = it.next()?.trim().parse::<f64>().ok()?;
                Some(LatLon { lat, lon })
            };

            let (Some(from), Some(to)) = (parse_coord(from_s), parse_coord(to_s)) else {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    r#"{"ok":false,"err":"from=lat,lon dan to=lat,lon wajib"}"#.to_string(),
                );
            };

            let res = TOMTOM.route(from, to, mode.as_deref()).await;
            json_resp(StatusCode::OK, "no-store", res.to_string())
        }
        "GET /tomtom/tile/{*path}" => {
            let path = params.get("path").map(|s| s.as_str()).unwrap_or("");
            let clean = path.strip_suffix(".png").unwrap_or(path);
            let parts: Vec<&str> = clean.split('/').collect();
            if parts.len() != 4 {
                return (StatusCode::NOT_FOUND, HeaderMap::new(), "tile not found").into_response();
            }

            let kind = parts[0];
            let (z_raw, x_raw, y_raw) = (parts[1], parts[2], parts[3]);
            let if_none_match = req
                .headers()
                .get(header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok());

            let tile = TOMTOM.tile(kind, z_raw, x_raw, y_raw, if_none_match).await;
            let mut headers = HeaderMap::new();
            headers.insert(
                header::CONTENT_TYPE,
                HeaderValue::from_str(&tile.content_type)
                    .unwrap_or(HeaderValue::from_static("image/png")),
            );
            headers.insert(header::REFERRER_POLICY, HeaderValue::from_static("same-origin"));
            headers.insert(
                header::X_CONTENT_TYPE_OPTIONS,
                HeaderValue::from_static("nosniff"),
            );

            if !tile.etag.is_empty()
                && let Ok(val) = HeaderValue::from_str(&tile.etag)
            {
                headers.insert(header::ETAG, val);
            }

            if tile.not_modified {
                return (StatusCode::NOT_MODIFIED, headers).into_response();
            }

            let status = StatusCode::from_u16(tile.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, headers, tile.body).into_response()
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
