//! Glue from the Axum front door to `cctv-gateway-rs`
//! (`/hls`, `/vhs`, `/api/cctv/snap`, `/api/cctv/stats`, `/api/cctv/rum`, `/api/cctv/report`).

use std::collections::HashMap;
use std::sync::LazyLock;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use bytes::Bytes;
use cctv_gateway_rs::port::service::{CctvGateway, GatewayOptions, RumEvent};
use serde_json::{Value, json};
use url::Url;

use super::cctv_catalog::CATALOG;

static GATEWAY: LazyLock<CctvGateway> = LazyLock::new(|| {
    let opts = GatewayOptions::default();
    CctvGateway::new(opts)
});

static HTTP_CLIENT: LazyLock<reqwest::Client> = LazyLock::new(|| {
    reqwest::Client::builder()
        .timeout(Duration::from_secs(12))
        .danger_accept_invalid_certs(true)
        .user_agent("sitrep-cctv/1.0")
        .build()
        .unwrap_or_default()
});

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

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
        "GET /api/cctv/stats" => {
            let gw_stats = GATEWAY.stats();
            let rum = GATEWAY.rum_stats();
            let cors = CATALOG.host_cors().await;
            let payload = json!({
                "ok": true,
                "gateway": gw_stats,
                "hosts": rum,
                "cors": cors,
                "at": now_ms(),
            });
            json_resp(
                StatusCode::OK,
                "public, max-age=5, stale-while-revalidate=30",
                payload.to_string(),
            )
        }
        "POST /api/cctv/rum" => {
            let body_bytes = match axum::body::to_bytes(req.into_body(), 20480).await {
                Ok(b) => b,
                Err(_) => {
                    return json_resp(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "no-store",
                        json!({ "ok": false, "err": "payload terlalu besar" }).to_string(),
                    );
                }
            };
            let events: Vec<RumEvent> = match serde_json::from_slice(&body_bytes) {
                Ok(ev) => ev,
                Err(_) => {
                    return json_resp(
                        StatusCode::BAD_REQUEST,
                        "no-store",
                        json!({ "ok": false, "err": "payload tidak valid" }).to_string(),
                    );
                }
            };
            let n = GATEWAY.rum_ingest(&events, "client", |_| true);
            json_resp(StatusCode::OK, "no-store", json!({ "ok": true, "n": n }).to_string())
        }
        "POST /api/cctv/report" => {
            let body_bytes = match axum::body::to_bytes(req.into_body(), 4096).await {
                Ok(b) => b,
                Err(_) => {
                    return json_resp(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "no-store",
                        json!({ "ok": false, "err": "payload terlalu besar" }).to_string(),
                    );
                }
            };
            let val: Value = match serde_json::from_slice(&body_bytes) {
                Ok(v) => v,
                Err(_) => {
                    return json_resp(
                        StatusCode::BAD_REQUEST,
                        "no-store",
                        json!({ "ok": false, "err": "payload tidak valid" }).to_string(),
                    );
                }
            };
            let u = val.get("u").and_then(|v| v.as_str()).unwrap_or("");
            let ok = val.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            GATEWAY.report(u, ok, "client", |_| true);
            json_resp(StatusCode::OK, "no-store", json!({ "ok": true }).to_string())
        }
        "GET /api/cctv/snap" => {
            let raw_url = query_param(q, "u").unwrap_or_default();
            if raw_url.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "missing u param",
                )
                    .into_response();
            }

            // Check snap cache
            if let Some(buf) = GATEWAY.get_snap_cached(&raw_url) {
                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "image/jpeg"),
                        (header::CACHE_CONTROL, "public, max-age=5, s-maxage=10"),
                        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                    ],
                    buf,
                )
                    .into_response();
            }

            // Direct JPEG bypass for dpuprkotang or direct image urls
            let target_url = CctvGateway::resolve_direct_snap_url(&raw_url).unwrap_or_else(|| raw_url.clone());
            if (target_url.contains("/api/frame.jpeg") || target_url.ends_with(".jpg") || target_url.ends_with(".jpeg"))
                && let Ok(resp) = HTTP_CLIENT.get(&target_url).send().await
                && resp.status().is_success()
                && let Ok(bytes) = resp.bytes().await
            {
                GATEWAY.set_snap_cached(&raw_url, bytes.clone(), 5000);
                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "image/jpeg"),
                        (header::CACHE_CONTROL, "public, max-age=5, s-maxage=10"),
                        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                    ],
                    bytes,
                )
                    .into_response();
            }

            // Subprocess ffmpeg execution (ADR-004)
            let mut cmd = tokio::process::Command::new("ffmpeg");
            cmd.args([
                "-y",
                "-rtsp_transport",
                "tcp",
                "-i",
                &raw_url,
                "-vframes",
                "1",
                "-f",
                "image2",
                "-",
            ]);
            cmd.kill_on_drop(true);

            let res = tokio::time::timeout(Duration::from_secs(5), cmd.output()).await;
            if let Ok(Ok(out)) = res
                && out.status.success()
                && !out.stdout.is_empty()
            {
                let b = Bytes::from(out.stdout);
                GATEWAY.set_snap_cached(&raw_url, b.clone(), 10000);
                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, "image/jpeg"),
                        (header::CACHE_CONTROL, "public, max-age=5, s-maxage=10"),
                        (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
                    ],
                    b,
                )
                    .into_response();
            }

            (
                StatusCode::SERVICE_UNAVAILABLE,
                [(header::CONTENT_TYPE, "text/plain")],
                "snapshot unavailable",
            )
                .into_response()
        }
        "GET /hls" | "GET /vhs" => {
            let raw_url = query_param(q, "u").unwrap_or_default();
            if raw_url.is_empty() {
                return (
                    StatusCode::BAD_REQUEST,
                    [(header::CONTENT_TYPE, "text/plain")],
                    "missing u param",
                )
                    .into_response();
            }

            let canon = CctvGateway::canonical_key(&raw_url);
            if let Some(cached) = GATEWAY.get_cached(&canon) {
                let ct = CctvGateway::sanitize_media_ct("application/octet-stream", &raw_url);
                return (
                    StatusCode::OK,
                    [
                        (header::CONTENT_TYPE, ct),
                        (header::CACHE_CONTROL, "public, max-age=2, s-maxage=5".to_string()),
                        (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
                    ],
                    cached,
                )
                    .into_response();
            }

            let parsed = match Url::parse(&raw_url) {
                Ok(u) => u,
                Err(_) => {
                    return (
                        StatusCode::BAD_REQUEST,
                        [(header::CONTENT_TYPE, "text/plain")],
                        "invalid url",
                    )
                        .into_response();
                }
            };
            let host = parsed.host_str().unwrap_or("").to_string();

            let fetch_fut = async {
                let resp = HTTP_CLIENT
                    .get(&raw_url)
                    .send()
                    .await
                    .map_err(|_| (502u16, "upstream error"))?;
                let status = resp.status().as_u16();
                let ct = resp
                    .headers()
                    .get(header::CONTENT_TYPE)
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("application/octet-stream")
                    .to_string();
                let b = resp
                    .bytes()
                    .await
                    .map_err(|_| (502u16, "upstream read error"))?;
                Ok((status, ct, b))
            };

            let (status_u16, raw_ct, bytes) = match GATEWAY.lane.run(&host, fetch_fut).await {
                Ok(Ok(r)) => r,
                Ok(Err((st, msg))) | Err((st, msg)) => {
                    return (
                        StatusCode::from_u16(st).unwrap_or(StatusCode::BAD_GATEWAY),
                        [(header::CONTENT_TYPE, "text/plain")],
                        msg,
                    )
                        .into_response();
                }
            };

            let status = StatusCode::from_u16(status_u16).unwrap_or(StatusCode::OK);
            let ct = CctvGateway::sanitize_media_ct(&raw_ct, &raw_url);

            let is_playlist = raw_url.contains(".m3u8") || ct.contains("mpegurl");
            let (final_body, ttl_ms) = if is_playlist {
                let text = String::from_utf8_lossy(&bytes);
                let rewritten = CctvGateway::rewrite_playlist(&text, &raw_url, |seg| {
                    format!("/hls?u={}", percent_encoding::percent_encode(seg.as_bytes(), percent_encoding::NON_ALPHANUMERIC))
                });
                let ttl = CctvGateway::playlist_ttl(&text, 1000, 4000);
                (Bytes::from(rewritten), ttl)
            } else {
                (bytes, 45_000)
            };

            GATEWAY.set_cached(&canon, final_body.clone(), ttl_ms);

            let cc = if is_playlist {
                "public, max-age=1, s-maxage=2"
            } else {
                "public, max-age=30, s-maxage=60"
            };

            (
                status,
                [
                    (header::CONTENT_TYPE, ct),
                    (header::CACHE_CONTROL, cc.to_string()),
                    (header::X_CONTENT_TYPE_OPTIONS, "nosniff".to_string()),
                ],
                final_body,
            )
                .into_response()
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
