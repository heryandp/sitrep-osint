//! Glue from the Axum front door to `market-rs` (`/api/market`, `/api/closes`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use market_rs::port::service::MarketService;
use serde_json::json;

static MARKET: LazyLock<MarketService> = LazyLock::new(|| {
    let hist_dir = std::env::var("SITREP_DATA_DIR")
        .or_else(|_| std::env::var("SITREP_HIST_DIR"))
        .unwrap_or_else(|_| "data".to_string());
    MarketService::new(
        hist_dir,
        "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36",
        None,
        None,
    )
});

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
        "GET /api/market" => match MARKET.market().await {
            Ok(data) => {
                let has_data = data
                    .get("stocks")
                    .and_then(|s| s.get("items"))
                    .and_then(|i| i.as_array())
                    .map(|a| !a.is_empty())
                    .unwrap_or(false)
                    || data
                        .get("ihsg")
                        .and_then(|h| h.get("price"))
                        .map(|p| p.is_number())
                        .unwrap_or(false);
                let cc = if has_data {
                    "public, max-age=15, s-maxage=45, stale-while-revalidate=5"
                } else {
                    "public, max-age=2, s-maxage=5, stale-while-revalidate=0"
                };
                json_resp(StatusCode::OK, cc, data.to_string())
            }
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/closes" => {
            let q = req.uri().query();
            let list_raw = query_param(q, "list").unwrap_or_default();
            let list: Vec<String> = list_raw
                .split(',')
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
            let data = MARKET.closes(&list).await;
            json_resp(
                StatusCode::OK,
                "public, max-age=60, s-maxage=180, stale-while-revalidate=15",
                data.to_string(),
            )
        }
        _ => json_resp(
            StatusCode::NOT_FOUND,
            "no-store",
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
