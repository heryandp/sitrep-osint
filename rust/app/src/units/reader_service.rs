//! Glue from the Axum front door to `reader-service-rs`
//! (`/api/read`, `/api/thumb`, `/api/desc`, `/img`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use reader_service_rs::port::service::{ReaderService, ReaderServiceConfig};
use serde_json::{Value, json};

static READER: LazyLock<ReaderService> = LazyLock::new(|| {
    let cfg = ReaderServiceConfig {
        jina_api_key: std::env::var("JINA_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty()),
        firecrawl_api_key: std::env::var("FIRECRAWL_API_KEY")
            .ok()
            .filter(|s| !s.trim().is_empty()),
        custom_proxy_url: std::env::var("CUSTOM_PROXY_URL")
            .ok()
            .filter(|s| !s.trim().is_empty()),
        pipeline_version: String::new(),
        doc_cache_bytes: 0,
    };
    ReaderService::new(cfg)
});

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
        "GET /api/read" => {
            let raw_url = query_param(q, "u").unwrap_or_default();
            if raw_url.is_empty() {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    json!({ "ok": false, "err": "parameter u (URL) diperlukan" }).to_string(),
                );
            }
            if !ReaderService::is_public_url(&raw_url) {
                return json_resp(
                    StatusCode::FORBIDDEN,
                    "no-store",
                    json!({ "ok": false, "err": "host tidak diizinkan" }).to_string(),
                );
            }
            let bg = query_param(q, "bg").as_deref() == Some("1");
            let fresh = query_param(q, "fresh").as_deref() == Some("1");

            match READER.read(&raw_url, bg, fresh).await {
                Ok(val) => {
                    let is_thin = val.get("thin").and_then(|v| v.as_bool()).unwrap_or(false);
                    let is_fb = val
                        .get("isFallback")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false)
                        || val
                            .get("wireFallback")
                            .and_then(|v| v.as_bool())
                            .unwrap_or(false);
                    let cc = if val.get("ok").and_then(|v| v.as_bool()).unwrap_or(false)
                        && !is_thin
                        && !is_fb
                    {
                        "public, max-age=300, s-maxage=600, stale-while-revalidate=30"
                    } else {
                        "no-store"
                    };
                    json_resp(StatusCode::OK, cc, val.to_string())
                }
                Err(err) => json_resp(
                    StatusCode::OK,
                    "no-store",
                    json!({ "ok": false, "err": err, "transient": true }).to_string(),
                ),
            }
        }
        "GET /api/thumb" => {
            let raw_url = query_param(q, "u").unwrap_or_default();
            if raw_url.is_empty() {
                return json_resp(
                    StatusCode::BAD_REQUEST,
                    "no-store",
                    json!({ "ok": false, "err": "parameter u (URL) diperlukan" }).to_string(),
                );
            }
            match READER.thumb(&raw_url).await {
                Ok(val) => {
                    let verified = val
                        .get("verified")
                        .and_then(|v| v.as_bool())
                        .unwrap_or(false);
                    let cc = if verified {
                        "public, max-age=300, s-maxage=600, stale-while-revalidate=30"
                    } else {
                        "no-store"
                    };
                    json_resp(StatusCode::OK, cc, val.to_string())
                }
                Err(err) => json_resp(
                    StatusCode::OK,
                    "no-store",
                    json!({ "ok": false, "err": err, "transient": true }).to_string(),
                ),
            }
        }
        "GET /api/desc" | "POST /api/desc" => {
            let mut urls = Vec::new();
            if route.starts_with("POST") {
                let bytes = match axum::body::to_bytes(req.into_body(), 16384).await {
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
                    && let Some(Value::Array(arr)) = map.get("urls")
                {
                    for v in arr {
                        if let Some(s) = v.as_str() {
                            urls.push(s.to_string());
                        }
                    }
                }
            } else if let Some(single) = query_param(q, "url") {
                urls.push(single);
            }

            urls.truncate(30);
            let mut descs = serde_json::Map::new();
            let retryable = serde_json::Map::new();

            for u in urls {
                match READER.desc(&u).await {
                    Ok(d) => {
                        descs.insert(u, Value::String(d));
                    }
                    Err(_) => {
                        descs.insert(u, Value::String(String::new()));
                    }
                }
            }

            json_resp(
                StatusCode::OK,
                "public, max-age=60, s-maxage=120, stale-while-revalidate=30",
                json!({
                    "ok": true,
                    "descs": descs,
                    "retryable": retryable
                })
                .to_string(),
            )
        }
        "GET /img" => {
            let raw_url = query_param(q, "u").unwrap_or_default();
            if raw_url.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "missing u param",
                )
                    .into_response();
            }

            if !raw_url.starts_with("https://") || !ReaderService::is_public_url(&raw_url) {
                return (
                    StatusCode::FORBIDDEN,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "host tidak diizinkan",
                )
                    .into_response();
            }

            match READER.img(&raw_url).await {
                Ok(img_resp) => {
                    let status = StatusCode::from_u16(img_resp.status).unwrap_or(StatusCode::OK);
                    let mut res = (
                        status,
                        [
                            (header::CONTENT_TYPE, img_resp.content_type),
                            (header::REFERRER_POLICY, "no-referrer".to_string()),
                            (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
                        ],
                        img_resp.body,
                    )
                        .into_response();

                    for (k, v) in img_resp.headers {
                        if let (Ok(name), Ok(val)) = (
                            axum::http::HeaderName::from_bytes(k.as_bytes()),
                            axum::http::HeaderValue::from_str(&v),
                        ) {
                            res.headers_mut().insert(name, val);
                        }
                    }
                    res
                }
                Err(err) => (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "text/plain")],
                    err,
                )
                    .into_response(),
            }
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
