//! Glue from the Axum front door to `uptime-service-rs` (`GET /api/pajak/uptime`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use uptime_service_rs::port::UptimeService;

static UPTIME: LazyLock<UptimeService> = LazyLock::new(UptimeService::new);

fn query_has_fresh(query: Option<&str>) -> bool {
    let Some(q) = query else { return false };
    q.split('&').any(|pair| {
        let mut it = pair.splitn(2, '=');
        let key = it.next().unwrap_or("");
        let val = it.next().unwrap_or("");
        key == "fresh" && matches!(val, "1" | "true")
    })
}

fn is_public_mode() -> bool {
    if std::env::var("SITREP_PUBLIC").is_ok_and(|v| v == "0") {
        return false;
    }
    if std::env::var("SITREP_PUBLIC").is_ok_and(|v| v == "1") {
        return true;
    }
    let has_users = std::env::var("SITREP_USERS").is_ok_and(|v| !v.trim().is_empty());
    let has_token = std::env::var("SITREP_TOKEN").is_ok_and(|v| !v.trim().is_empty());
    !has_users && !has_token
}

/// Handles `GET /api/pajak/uptime` natively in Rust via `uptime_service_rs::port::UptimeService`.
pub async fn handle(_route: &str, _params: HashMap<String, String>, req: Request) -> Response {
    let fresh = query_has_fresh(req.uri().query());
    if fresh && is_public_mode() {
        return (
            StatusCode::FORBIDDEN,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "same-origin"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            r#"{"ok":false,"err":"akses ditolak (butuh role operator)"}"#,
        )
            .into_response();
    }

    let payload = UPTIME.check_all(fresh).await;
    match serde_json::to_string(&payload) {
        Ok(body) => (
            StatusCode::OK,
            [
                (header::CONTENT_TYPE, "application/json"),
                (
                    header::CACHE_CONTROL,
                    "public, max-age=10, s-maxage=30, stale-while-revalidate=90",
                ),
                (header::REFERRER_POLICY, "same-origin"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            body,
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "same-origin"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            format!(
                r#"{{"ok":false,"err":{}}}"#,
                serde_json::json!(err.to_string())
            ),
        )
            .into_response(),
    }
}
