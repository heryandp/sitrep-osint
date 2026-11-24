//! Glue from the Axum front door to `security-rs` (`/api/auth/me`, `/api/auth/login`, `/api/auth/logout`).

use std::collections::HashMap;

use axum::extract::Request;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use security_rs::port::service::{AuthManager, AuthMode};
use serde_json::json;

use crate::middleware::{apply_rl_headers, auth, auth_context_from_req, client_ip_from_req, limiter};

fn json_response(status: StatusCode, body: String) -> Response {
    (
        status,
        [
            (header::CONTENT_TYPE, "application/json"),
            (header::CACHE_CONTROL, "no-store"),
            (header::REFERRER_POLICY, "same-origin"),
            (header::X_CONTENT_TYPE_OPTIONS, "nosniff"),
        ],
        body,
    )
        .into_response()
}

fn is_same_origin_post(req: &Request) -> bool {
    let host = req
        .headers()
        .get(header::HOST)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if host.is_empty() {
        return false;
    }
    let origin = req
        .headers()
        .get(header::ORIGIN)
        .and_then(|v| v.to_str().ok())
        .map(str::trim);
    let referer = req
        .headers()
        .get(header::REFERER)
        .and_then(|v| v.to_str().ok())
        .map(str::trim);
    let candidate = match (origin, referer) {
        (Some(o), _) if !o.is_empty() && o != "null" => o,
        (_, Some(r)) if !r.is_empty() => r,
        _ => return false,
    };
    let after_scheme = candidate
        .strip_prefix("https://")
        .or_else(|| candidate.strip_prefix("http://"));
    let Some(rest) = after_scheme else {
        return false;
    };
    let cand_host = rest.split('/').next().unwrap_or("").to_ascii_lowercase();
    cand_host == host
}

pub async fn handle(route: &str, _params: HashMap<String, String>, req: Request) -> Response {
    let mgr = auth();
    match route {
        "GET /api/auth/me" => {
            let principal = mgr.authenticate(&auth_context_from_req(&req));
            let mode = match mgr.mode() {
                AuthMode::Session => "session",
                AuthMode::Public => "public",
                AuthMode::Token => "token",
            };
            let user_val = principal.map(|p| {
                json!({
                    "username": p.username,
                    "role": p.role.as_str(),
                    "via": p.via,
                })
            });
            json_response(
                StatusCode::OK,
                json!({ "ok": true, "mode": mode, "user": user_val }).to_string(),
            )
        }
        "POST /api/auth/login" => {
            let ip = client_ip_from_req(&req);
            let rl = limiter().check("login", &ip, false, None);
            if !rl.allowed {
                let mut r = json_response(
                    StatusCode::TOO_MANY_REQUESTS,
                    r#"{"ok":false,"err":"terlalu banyak percobaan login, coba lagi nanti"}"#
                        .to_string(),
                );
                apply_rl_headers(&mut r, rl.limit, rl.remaining);
                return r;
            }
            if !mgr.user_mode() {
                let mut r = json_response(
                    StatusCode::BAD_REQUEST,
                    r#"{"ok":false,"err":"autentikasi multi-user belum diaktifkan"}"#.to_string(),
                );
                apply_rl_headers(&mut r, rl.limit, rl.remaining);
                return r;
            }
            let is_https = auth_context_from_req(&req).is_https;
            let Ok(bytes) = axum::body::to_bytes(req.into_body(), 4096).await else {
                let mut r = json_response(
                    StatusCode::BAD_REQUEST,
                    r#"{"ok":false,"err":"payload JSON tidak valid"}"#.to_string(),
                );
                apply_rl_headers(&mut r, rl.limit, rl.remaining);
                return r;
            };
            let Ok(body) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
                let mut r = json_response(
                    StatusCode::BAD_REQUEST,
                    r#"{"ok":false,"err":"payload JSON tidak valid"}"#.to_string(),
                );
                apply_rl_headers(&mut r, rl.limit, rl.remaining);
                return r;
            };
            let username = body.get("username").and_then(|v| v.as_str()).unwrap_or("");
            let password = body.get("password").and_then(|v| v.as_str()).unwrap_or("");
            match mgr.login(username, password, is_https).await {
                Some(res) => {
                    let mut r = json_response(
                        StatusCode::OK,
                        json!({
                            "ok": true,
                            "user": {
                                "username": res.principal.username,
                                "role": res.principal.role.as_str(),
                                "via": res.principal.via,
                            }
                        })
                        .to_string(),
                    );
                    if let Ok(hv) = HeaderValue::from_str(&res.set_cookie) {
                        r.headers_mut().insert(header::SET_COOKIE, hv);
                    }
                    apply_rl_headers(&mut r, rl.limit, rl.remaining);
                    r
                }
                None => {
                    let mut r = json_response(
                        StatusCode::UNAUTHORIZED,
                        r#"{"ok":false,"err":"username atau password salah"}"#.to_string(),
                    );
                    apply_rl_headers(&mut r, rl.limit, rl.remaining);
                    r
                }
            }
        }
        "POST /api/auth/logout" => {
            if !is_same_origin_post(&req) {
                return json_response(
                    StatusCode::FORBIDDEN,
                    r#"{"ok":false,"err":"origin tidak valid"}"#.to_string(),
                );
            }
            let ctx = auth_context_from_req(&req);
            let raw_tok = ctx
                .cookie_header
                .and_then(|c| AuthManager::parse_cookies(c).remove("sitrep_sess"))
                .unwrap_or_default();
            let set_cookie = mgr.logout(&raw_tok, ctx.is_https);
            let mut r = json_response(StatusCode::OK, r#"{"ok":true}"#.to_string());
            if let Ok(hv) = HeaderValue::from_str(&set_cookie) {
                r.headers_mut().insert(header::SET_COOKIE, hv);
            }
            r
        }
        _ => json_response(
            StatusCode::NOT_FOUND,
            r#"{"ok":false,"err":"not found"}"#.to_string(),
        ),
    }
}
