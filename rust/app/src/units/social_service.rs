//! Glue from the Axum front door to `social-service-rs`
//! (`/api/x`, `/api/threads`, `/api/social/diagnostics`, `/api/social/explain`, `/api/xdiag`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use serde_json::{Value, json};
use social_service_rs::port::service::SocialService;

static SOCIAL: LazyLock<SocialService> = LazyLock::new(SocialService::new);

fn query_param(query: Option<&str>, key: &str) -> Option<String> {
    let q = query?;
    for pair in q.split('&') {
        let mut it = pair.splitn(2, '=');
        if it.next() == Some(key) {
            let val = it.next().unwrap_or("");
            return Some(
                percent_encoding::percent_decode_str(val)
                    .decode_utf8_lossy()
                    .to_string(),
            );
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
    let q = req.uri().query();
    match route {
        "GET /api/x" => match SOCIAL.api_x() {
            Ok(val) => json_resp(
                StatusCode::OK,
                "public, max-age=25, s-maxage=60, stale-while-revalidate=10",
                val.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/threads" => match SOCIAL.api_threads() {
            Ok(val) => json_resp(
                StatusCode::OK,
                "public, max-age=25, s-maxage=60, stale-while-revalidate=10",
                val.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/social/diagnostics" => match SOCIAL.api_social_diagnostics() {
            Ok(val) => json_resp(
                StatusCode::OK,
                "public, max-age=20, s-maxage=40, stale-while-revalidate=10",
                val.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/social/explain" | "POST /api/social/explain" => {
            let mut text = query_param(q, "text").unwrap_or_default();
            if route.starts_with("POST") {
                let bytes = match axum::body::to_bytes(req.into_body(), 8192).await {
                    Ok(b) => b,
                    Err(_) => {
                        return json_resp(
                            StatusCode::PAYLOAD_TOO_LARGE,
                            "no-store",
                            json!({ "ok": false, "err": "payload terlalu besar" }).to_string(),
                        );
                    }
                };
                if let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(&bytes)
                    && let Some(Value::String(s)) = map.get("text")
                {
                    text = s.clone();
                }
            }

            match SOCIAL.api_social_explain(&text) {
                Ok(val) => json_resp(StatusCode::OK, "no-store", val.to_string()),
                Err(e) => json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no-store",
                    json!({ "ok": false, "err": e }).to_string(),
                ),
            }
        }
        "GET /api/xdiag" => match SOCIAL.api_xdiag() {
            Ok(val) => json_resp(StatusCode::OK, "no-store", val.to_string()),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
