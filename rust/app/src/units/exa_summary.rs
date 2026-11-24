//! Glue from the Axum front door to `exa-summary-rs` (`/api/exa/status`, `/api/pajak/summarize`, `/api/pajak/brief`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use exa_summary_rs::port::service::{ExaSummaryService, ServiceConfig};
use serde_json::{Value, json};

static EXA: LazyLock<ExaSummaryService> = LazyLock::new(|| {
    let hist_dir = std::env::var("SITREP_DATA_DIR")
        .or_else(|_| std::env::var("SITREP_HIST_DIR"))
        .unwrap_or_else(|_| "data".to_string());
    let config = ServiceConfig {
        hist_dir: std::path::PathBuf::from(hist_dir),
        api_key: std::env::var("EXA_API_KEY")
            .or_else(|_| std::env::var("EXA_KEY"))
            .unwrap_or_default(),
        ..Default::default()
    };
    ExaSummaryService::new(config)
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

pub async fn handle(route: &str, _params: HashMap<String, String>, req: Request) -> Response {
    match route {
        "GET /api/exa/status" => {
            let st = EXA.status().await;
            match serde_json::to_string(&st) {
                Ok(s) => json_resp(StatusCode::OK, "no-store", s),
                Err(e) => json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no-store",
                    json!({ "ok": false, "err": e.to_string() }).to_string(),
                ),
            }
        }
        "POST /api/pajak/summarize" => {
            let body_bytes = match axum::body::to_bytes(req.into_body(), 16384).await {
                Ok(b) => b,
                Err(_) => {
                    return json_resp(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "no-store",
                        json!({ "ok": false, "err": "payload terlalu besar" }).to_string(),
                    );
                }
            };

            let parsed: Value = match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(_) => {
                    return json_resp(
                        StatusCode::BAD_REQUEST,
                        "no-store",
                        json!({ "ok": false, "err": "payload JSON tidak valid" }).to_string(),
                    );
                }
            };

            let url = parsed.get("url").and_then(|v| v.as_str()).unwrap_or("").trim();
            let urls: Vec<String> = parsed
                .get("urls")
                .and_then(|v| v.as_array())
                .map(|a| {
                    a.iter()
                        .filter_map(|x| x.as_str().map(|s| s.trim().to_string()))
                        .take(5)
                        .collect()
                })
                .unwrap_or_default();
            let title = parsed.get("title").and_then(|v| v.as_str()).unwrap_or("").trim();
            let desc = parsed.get("desc").and_then(|v| v.as_str()).unwrap_or("").trim();
            let theme_label = parsed.get("themeLabel").and_then(|v| v.as_str()).unwrap_or("").trim();
            let cluster_id = parsed.get("clusterId").and_then(|v| v.as_str()).unwrap_or("").trim();

            if url.is_empty() && urls.is_empty() && title.is_empty() {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    json!({ "ok": false, "err": "url atau title wajib diisi" }).to_string(),
                );
            }

            let all_urls = if !url.is_empty() {
                let mut u = vec![url.to_string()];
                for x in urls {
                    if !u.contains(&x) {
                        u.push(x);
                    }
                }
                u
            } else {
                urls
            };

            match EXA
                .summarize_advanced(&all_urls, title, desc, false, theme_label, cluster_id, None)
                .await
            {
                Ok(val) => json_resp(StatusCode::OK, "no-store", val.to_string()),
                Err(err) => json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no-store",
                    json!({ "ok": false, "err": err }).to_string(),
                ),
            }
        }
        "POST /api/pajak/brief" => {
            let body_bytes = match axum::body::to_bytes(req.into_body(), 8192).await {
                Ok(b) => b,
                Err(_) => {
                    return json_resp(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "no-store",
                        json!({ "ok": false, "err": "payload terlalu besar" }).to_string(),
                    );
                }
            };

            let parsed: Value = match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(_) => {
                    return json_resp(
                        StatusCode::BAD_REQUEST,
                        "no-store",
                        json!({ "ok": false, "err": "payload JSON tidak valid" }).to_string(),
                    );
                }
            };

            let topic = parsed.get("theme").or_else(|| parsed.get("topic")).and_then(|v| v.as_str()).unwrap_or("");
            match EXA.brief(topic, None).await {
                Ok(val) => json_resp(StatusCode::OK, "no-store", val.to_string()),
                Err(err) => json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no-store",
                    json!({ "ok": false, "err": err }).to_string(),
                ),
            }
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
