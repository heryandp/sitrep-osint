//! Glue from the Axum front door to `news-service-rs` (`/api/news`, `/api/breaking`, `/api/incidents`, `/api/situation`, `/api/pajak/clusters`).

use std::collections::HashMap;
use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{StatusCode, header};
use axum::response::{IntoResponse, Response};
use news_service_rs::port::service::NewsService;
use serde_json::json;

static NEWS: LazyLock<NewsService> = LazyLock::new(NewsService::new);

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
    let q = req.uri().query();
    match route {
        "GET /api/news" => match NEWS.news().await {
            Ok(data) => json_resp(
                StatusCode::OK,
                "public, max-age=25, s-maxage=60, stale-while-revalidate=10",
                data.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/breaking" => match NEWS.breaking().await {
            Ok(data) => json_resp(
                StatusCode::OK,
                "public, max-age=20, s-maxage=60, stale-while-revalidate=10",
                data.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/incidents" => match NEWS.incidents().await {
            Ok(data) => json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=90, stale-while-revalidate=15",
                data.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/situation" => match NEWS.situation().await {
            Ok(data) => json_resp(
                StatusCode::OK,
                "public, max-age=30, s-maxage=90, stale-while-revalidate=15",
                data.to_string(),
            ),
            Err(e) => json_resp(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no-store",
                json!({ "ok": false, "err": e }).to_string(),
            ),
        },
        "GET /api/pajak/clusters" => {
            let win = query_param(q, "win");
            let theme = query_param(q, "theme");
            let scope = query_param(q, "scope");
            match NEWS
                .pajak_clusters(win.as_deref(), theme.as_deref(), scope.as_deref(), None)
                .await
            {
                Ok(data) => json_resp(
                    StatusCode::OK,
                    "public, max-age=12, s-maxage=30, stale-while-revalidate=5",
                    data.to_string(),
                ),
                Err(e) => json_resp(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "no-store",
                    json!({ "ok": false, "err": e }).to_string(),
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
