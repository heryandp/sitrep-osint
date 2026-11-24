//! Glue from the Axum front door to `copernicus-rs` (`/api/copernicus/status`, `/api/copernicus/tile/{*path}`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{HeaderMap, HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use copernicus_rs::port::service::{CopernicusConfig, CopernicusProxy};
use serde_json::json;

static COPERNICUS: LazyLock<CopernicusProxy> = LazyLock::new(|| {
    let hist_dir = std::env::var("SITREP_DATA_DIR")
        .or_else(|_| std::env::var("SITREP_HIST_DIR"))
        .unwrap_or_else(|_| "data".to_string());
    let cache_dir = std::path::PathBuf::from(hist_dir).join("copernicus-tiles");
    let config = CopernicusConfig {
        client_id: std::env::var("CDSE_CLIENT_ID")
            .or_else(|_| std::env::var("COPERNICUS_CLIENT_ID"))
            .unwrap_or_default(),
        client_secret: std::env::var("CDSE_CLIENT_SECRET")
            .or_else(|_| std::env::var("COPERNICUS_CLIENT_SECRET"))
            .unwrap_or_default(),
        cache_dir,
        daily_cap: 1200,
        ..Default::default()
    };
    CopernicusProxy::new(config)
});

pub async fn handle(route: &str, params: HashMap<String, String>, req: Request) -> Response {
    match route {
        "GET /api/copernicus/status" => {
            let st = COPERNICUS.status().await;
            match serde_json::to_string(&st) {
                Ok(s) => {
                    let clean = s.replace("\"cacheTtlHours\":24.0", "\"cacheTtlHours\":24");
                    (
                        StatusCode::OK,
                        [
                            (header::CONTENT_TYPE, "application/json"),
                            (
                                header::CACHE_CONTROL,
                                "public, max-age=15, stale-while-revalidate=60",
                            ),
                            (header::REFERRER_POLICY, "same-origin"),
                            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                        ],
                        clean,
                    )
                        .into_response()
                }
                Err(e) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    [
                        (header::CONTENT_TYPE, "application/json"),
                        (header::CACHE_CONTROL, "no-store"),
                        (header::REFERRER_POLICY, "same-origin"),
                        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                    ],
                    json!({ "ok": false, "err": e.to_string() }).to_string(),
                )
                    .into_response(),
            }
        }
        "GET /api/copernicus/tile/{*path}" => {
            let path = params.get("path").map(|s| s.as_str()).unwrap_or("");
            let clean = path.strip_suffix(".png").unwrap_or(path);
            let parts: Vec<&str> = clean.split('/').collect();
            if parts.len() != 3 {
                return (StatusCode::NOT_FOUND, HeaderMap::new(), "tile not found").into_response();
            }
            let (Ok(z), Ok(x), Ok(y)) = (
                parts[0].parse::<i64>(),
                parts[1].parse::<i64>(),
                parts[2].parse::<i64>(),
            ) else {
                return (StatusCode::NOT_FOUND, HeaderMap::new(), "tile not found").into_response();
            };

            let if_none_match = req
                .headers()
                .get(header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok());

            let tile = COPERNICUS.fetch_tile(z, x, y).await;
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
            headers.insert(
                "x-s2-cache",
                HeaderValue::from_str(&tile.cache_state).unwrap_or(HeaderValue::from_static("UNKNOWN")),
            );

            if let Some(ref et) = tile.etag {
                if let Ok(val) = HeaderValue::from_str(et) {
                    headers.insert(header::ETAG, val);
                }
                if tile.status == 200 && if_none_match == Some(et) {
                    headers.insert(
                        header::CACHE_CONTROL,
                        HeaderValue::from_static("public, max-age=86400, stale-while-revalidate=86400"),
                    );
                    return (StatusCode::NOT_MODIFIED, headers).into_response();
                }
            }

            let cc = if tile.status == 200 {
                if tile.stale {
                    "public, max-age=3600, stale-while-revalidate=3600"
                } else {
                    "public, max-age=86400, stale-while-revalidate=86400"
                }
            } else {
                "no-store"
            };
            headers.insert(header::CACHE_CONTROL, HeaderValue::from_static(cc));

            let status = StatusCode::from_u16(tile.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            (status, headers, tile.buf).into_response()
        }
        _ => (StatusCode::NOT_FOUND, HeaderMap::new(), "not found").into_response(),
    }
}
