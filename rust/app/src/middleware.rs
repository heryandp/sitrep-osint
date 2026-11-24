//! Shared `AuthManager`, `RateLimiter`, and Axum rate-limit middleware for `djp-sitrep`.

use std::sync::LazyLock;

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::middleware::Next;
use axum::response::{IntoResponse, Response};
use security_rs::port::service::{AuthConfig, AuthManager, RateLimiter, RequestAuthContext, Role};

static AUTH: LazyLock<AuthManager> = LazyLock::new(|| {
    let mut cfg = AuthConfig::from_env();
    let explicit_zero = std::env::var("SITREP_PUBLIC").ok().as_deref() == Some("0");
    if !explicit_zero && !cfg.public_deploy && cfg.users_spec.trim().is_empty() && !cfg.explicit_token
    {
        cfg.public_deploy = true;
    }
    AuthManager::new(cfg).unwrap_or_else(|_| AuthManager::new(AuthConfig::default()).expect("default auth"))
});

static LIMITER: LazyLock<RateLimiter> = LazyLock::new(RateLimiter::new);

pub fn auth() -> &'static AuthManager {
    &AUTH
}

pub fn limiter() -> &'static RateLimiter {
    &LIMITER
}

pub fn client_ip_from_req(req: &Request) -> String {
    let trust_proxy = auth().trust_proxy();
    if trust_proxy {
        if std::env::var("SITREP_TRUST_CF").ok().as_deref() == Some("1")
            && let Some(cf) = req
                .headers()
                .get("cf-connecting-ip")
                .and_then(|v| v.to_str().ok())
                .map(str::trim)
                .filter(|s| !s.is_empty())
        {
            return cf.to_string();
        }
        if let Some(xff) = req
            .headers()
            .get("x-forwarded-for")
            .and_then(|v| v.to_str().ok())
            && let Some(first) = xff.split(',').next().map(str::trim).filter(|s| !s.is_empty())
        {
            return first.to_string();
        }
    }
    "127.0.0.1".to_string()
}

pub fn is_gated_path(path: &str) -> bool {
    (path.starts_with("/api/") && path != "/api/ping" && !path.starts_with("/api/auth/"))
        || path == "/img"
        || path == "/hls"
        || path == "/vhs"
        || path.starts_with("/tomtom/tile/")
}

pub fn is_heavy_request(path: &str, query: Option<&str>) -> bool {
    let has_fresh = query.is_some_and(|q| {
        q.split('&').any(|pair| {
            let mut it = pair.splitn(2, '=');
            it.next() == Some("fresh") && it.next().is_some_and(|v| !v.is_empty())
        })
    });
    path == "/img"
        || path == "/hls"
        || path == "/vhs"
        || path == "/api/thumb"
        || path == "/api/read"
        || path == "/api/pajak/summarize"
        || path == "/api/pajak/brief"
        || path.starts_with("/api/copernicus/tile/")
        || has_fresh
}

pub fn auth_context_from_req<'a>(req: &'a Request) -> RequestAuthContext<'a> {
    let token_header = req
        .headers()
        .get("x-sitrep-token")
        .and_then(|v| v.to_str().ok());
    let cookie_header = req.headers().get(header::COOKIE).and_then(|v| v.to_str().ok());
    let query_token = req.uri().query().and_then(|q| {
        q.split('&').find_map(|pair| {
            let mut it = pair.splitn(2, '=');
            if it.next() == Some("token") {
                it.next().filter(|v| !v.is_empty())
            } else {
                None
            }
        })
    });
    let is_https = auth().trust_proxy()
        && req
            .headers()
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.eq_ignore_ascii_case("https"));
    RequestAuthContext {
        token_header,
        query_token,
        cookie_header,
        is_https,
    }
}

/// Authoritative Rust rate-limit layer so native Rust routes and proxied Node routes
/// share a single consistent `x-ratelimit-limit` / `x-ratelimit-remaining` counter.
pub async fn rate_limit_middleware(req: Request, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let query = req.uri().query().map(ToString::to_string);
    if !is_gated_path(&path) {
        return next.run(req).await;
    }

    let principal = auth().authenticate(&auth_context_from_req(&req));
    let Some(principal) = principal else {
        let (status, msg) = if auth().user_mode() {
            (StatusCode::UNAUTHORIZED, r#"{"ok":false,"err":"autentikasi diperlukan"}"#)
        } else {
            (StatusCode::FORBIDDEN, r#"{"ok":false,"err":"token tidak valid"}"#)
        };
        return (
            status,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "same-origin"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            msg,
        )
            .into_response();
    };

    let ip = client_ip_from_req(&req);
    let bucket = if is_heavy_request(&path, query.as_deref()) {
        "heavy"
    } else {
        "api"
    };
    let exempt = auth().has_role(Some(&principal), Role::Operator);
    let decision = limiter().check(bucket, &ip, exempt, None);

    if !decision.allowed {
        let mut resp = (
            StatusCode::TOO_MANY_REQUESTS,
            [
                (header::CONTENT_TYPE, "application/json"),
                (header::CACHE_CONTROL, "no-store"),
                (header::REFERRER_POLICY, "same-origin"),
                (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
            ],
            r#"{"ok":false,"err":"terlalu banyak request, coba lagi sebentar"}"#,
        )
            .into_response();
        apply_rl_headers(&mut resp, decision.limit, decision.remaining);
        return resp;
    }

    let mut resp = next.run(req).await;
    if !decision.exempt {
        apply_rl_headers(&mut resp, decision.limit, decision.remaining);
    }
    resp
}

pub fn apply_rl_headers(resp: &mut Response, limit: u32, remaining: u32) {
    if let Ok(v) = HeaderValue::from_str(&limit.to_string()) {
        resp.headers_mut().insert("x-ratelimit-limit", v);
    }
    if let Ok(v) = HeaderValue::from_str(&remaining.to_string()) {
        resp.headers_mut().insert("x-ratelimit-remaining", v);
    }
}
