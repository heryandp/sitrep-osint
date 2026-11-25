//! Glue from the Axum front door to `cctv-catalog-rs` (`/api/cams`, `/api/cctv`).

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use cctv_catalog_rs::port::service::{CctvCatalog, CctvCatalogOptions};
use serde_json::json;

pub static CATALOG: LazyLock<CctvCatalog> = LazyLock::new(|| {
    let hist_dir = std::env::var("SITREP_DATA_DIR")
        .or_else(|_| std::env::var("SITREP_HIST_DIR"))
        .unwrap_or_else(|_| "data".to_string());
    let opts = CctvCatalogOptions {
        hist_dir: Some(PathBuf::from(hist_dir)),
        persist_hist: true,
        user_agent: "sitrep-cctv/1.0".to_string(),
        ..Default::default()
    };
    let cat = CctvCatalog::new(opts);
    let _ = cat.load_cctv_hist();
    cat
});

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

pub async fn handle(route: &str, _params: HashMap<String, String>, _req: Request) -> Response {
    match route {
        "GET /api/cctv" => {
            let cat = CATALOG.catalog().await;
            let cc = if cat.warming || cat.probing || cat.cams.is_empty() {
                "public, max-age=2, s-maxage=5, stale-while-revalidate=0"
            } else {
                "public, max-age=60, s-maxage=120, stale-while-revalidate=15"
            };
            match serde_json::to_string(&cat) {
                Ok(s) => json_resp(StatusCode::OK, cc, s),
                Err(e) => json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no-store",
                    json!({ "ok": false, "err": e.to_string() }).to_string(),
                ),
            }
        }
        "GET /api/cams" => {
            let cams = CATALOG.cam_list().await;
            let alive_keys: Vec<String> = cams.iter().map(|c| c.s.clone()).collect();
            let has_cams = !alive_keys.is_empty();
            let cc = if has_cams {
                "public, max-age=60, s-maxage=120, stale-while-revalidate=15"
            } else {
                "public, max-age=2, s-maxage=5, stale-while-revalidate=0"
            };
            let payload = json!({
                "ok": true,
                "alive": alive_keys,
                "total": cams.len(),
                "at": std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0),
            });
            json_resp(StatusCode::OK, cc, payload.to_string())
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
