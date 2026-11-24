//! Idiomatic Rust security services for `rust/app` Axum middleware:
//! - `AuthManager` (public / session / token modes, HMAC-SHA256 cookies, scrypt verification)
//! - `RateLimiter` (fixed-window per-IP bucketed limiter matching `createRateLimiter`)
//! - `assert_public_url` (async DNS-verified SSRF guard)

#![forbid(unsafe_code)]

use crate::port::{is_private_ip_str, parse_public_url, rate_key_str};
use base64::Engine as _;
use base64::engine::general_purpose::URL_SAFE_NO_PAD;
use hmac::{Hmac, Mac};
use rand::RngCore;
use scrypt::{Params as ScryptParams, scrypt};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{HashMap, VecDeque};
use std::future::Future;
use std::net::IpAddr;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use subtle::ConstantTimeEq;
use url::Url;

type HmacSha256 = Hmac<Sha256>;

pub const DEFAULT_UA: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";

const DUMMY_SCRYPT_HASH: &str = "scrypt$00112233445566778899aabbccddeeff$00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";

const MAX_RATE_KEYS: usize = 20_000;
const REVOKED_SWEEP_THRESHOLD: usize = 1_000;
const REVOKED_HARD_CAP: usize = 25_000;

/// Encode bytes as lowercase hex.
pub fn encode_hex(bytes: &[u8]) -> String {
    const LUT: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for &b in bytes {
        out.push(LUT[(b >> 4) as usize] as char);
        out.push(LUT[(b & 0x0f) as usize] as char);
    }
    out
}

/// Decode hex matching Node.js `Buffer.from(hex, 'hex')` (truncates trailing odd nibble).
pub fn decode_node_hex(hex: &str) -> Vec<u8> {
    let bytes = hex.as_bytes();
    let pairs = bytes.len() / 2;
    let mut out = Vec::with_capacity(pairs);
    for i in 0..pairs {
        let hi = hex_nibble(bytes[2 * i]);
        let lo = hex_nibble(bytes[2 * i + 1]);
        match (hi, lo) {
            (Some(h), Some(l)) => out.push((h << 4) | l),
            _ => break,
        }
    }
    out
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Constant-time UTF-8 string comparison using `subtle::ConstantTimeEq`.
pub fn timing_equal_str(a: &str, b: &str) -> bool {
    let ba = a.as_bytes();
    let bb = b.as_bytes();
    ba.len() == bb.len() && bool::from(ba.ct_eq(bb))
}

/// Generate a random hex string of `num_bytes` cryptographically secure bytes.
pub fn random_hex(num_bytes: usize) -> String {
    let mut buf = vec![0u8; num_bytes];
    rand::rng().fill_bytes(&mut buf);
    encode_hex(&buf)
}

/// Hash a password with `scrypt` (N=16384, r=8, p=1, dkLen=64) in `scrypt$<saltHex>$<hashHex>` format.
pub fn hash_password(password: &str) -> Result<String, String> {
    let mut salt = [0u8; 16];
    rand::rng().fill_bytes(&mut salt);
    // INFALLIBLE: log_n=14, r=8, p=1, len=64 are valid compile-time constants.
    let params = ScryptParams::new(14, 8, 1, 64).map_err(|e| e.to_string())?;
    let mut dk = [0u8; 64];
    scrypt(password.as_bytes(), &salt, &params, &mut dk).map_err(|e| e.to_string())?;
    Ok(format!("scrypt${}${}", encode_hex(&salt), encode_hex(&dk)))
}

/// Verify `scrypt$<saltHex>$<hashHex>` (3-part, exact `src/security.js` contract).
pub fn verify_password_js_compat(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    if parts.len() != 3 || parts[0] != "scrypt" {
        return false;
    }
    let (salt_hex, hash_hex) = (parts[1], parts[2]);
    if salt_hex.is_empty()
        || hash_hex.is_empty()
        || !salt_hex.bytes().all(|b| b.is_ascii_hexdigit())
        || !hash_hex.bytes().all(|b| b.is_ascii_hexdigit())
    {
        return false;
    }
    let salt = decode_node_hex(salt_hex);
    let expected = decode_node_hex(hash_hex);
    let Ok(params) = ScryptParams::new(14, 8, 1, 64) else {
        return false;
    };
    let mut derived = [0u8; 64];
    if scrypt(password.as_bytes(), &salt, &params, &mut derived).is_err() {
        return false;
    }
    derived.len() == expected.len() && bool::from(derived.ct_eq(&expected))
}

/// Verify both 3-part `scrypt$<salt>$<hash>` and 6-part `scrypt$<N>$<r>$<p>$<salt>$<hash>`.
pub fn verify_password_sync(password: &str, stored: &str) -> bool {
    let parts: Vec<&str> = stored.split('$').collect();
    match parts.as_slice() {
        ["scrypt", _, _] => verify_password_js_compat(password, stored),
        ["scrypt", n_str, r_str, p_str, salt_hex, hash_hex] => {
            if salt_hex.is_empty()
                || hash_hex.is_empty()
                || !salt_hex.bytes().all(|b| b.is_ascii_hexdigit())
                || !hash_hex.bytes().all(|b| b.is_ascii_hexdigit())
            {
                return false;
            }
            let (Ok(n), Ok(r), Ok(p)) = (
                n_str.parse::<u64>(),
                r_str.parse::<u32>(),
                p_str.parse::<u32>(),
            ) else {
                return false;
            };
            if n < 2 || !n.is_power_of_two() || n > (1 << 20) || r == 0 || p == 0 {
                return false;
            }
            let log_n = n.trailing_zeros() as u8;
            let salt = decode_node_hex(salt_hex);
            let expected = decode_node_hex(hash_hex);
            if expected.is_empty() || expected.len() > 256 {
                return false;
            }
            let Ok(params) = ScryptParams::new(log_n, r, p, expected.len()) else {
                return false;
            };
            let mut derived = vec![0u8; expected.len()];
            if scrypt(password.as_bytes(), &salt, &params, &mut derived).is_err() {
                return false;
            }
            bool::from(derived.ct_eq(&expected))
        }
        _ => false,
    }
}

/// Async password verification offloaded onto Tokio's blocking pool.
pub async fn verify_password_async(password: String, stored: String) -> bool {
    tokio::task::spawn_blocking(move || verify_password_sync(&password, &stored))
        .await
        .unwrap_or(false)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Viewer = 1,
    Operator = 2,
    Admin = 3,
}

impl Role {
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "viewer" => Some(Self::Viewer),
            "operator" => Some(Self::Operator),
            "admin" => Some(Self::Admin),
            _ => None,
        }
    }

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Viewer => "viewer",
            Self::Operator => "operator",
            Self::Admin => "admin",
        }
    }

    pub const fn rank(self) -> u8 {
        self as u8
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum AuthMode {
    Public,
    Session,
    Token,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserRecord {
    pub username: String,
    pub role: Role,
    pub hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Principal {
    pub username: String,
    pub role: Role,
    pub via: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub jti: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub exp: Option<i64>,
}

/// Parse `SITREP_USERS` into an insertion-ordered list of `UserRecord`s.
pub fn parse_users_entries(spec: &str) -> Result<Vec<UserRecord>, String> {
    let mut order: Vec<String> = Vec::new();
    let mut map: HashMap<String, UserRecord> = HashMap::new();

    for raw_ent in spec.split([';', '\n', ',']) {
        let ent = raw_ent.trim();
        if ent.is_empty() {
            continue;
        }
        let Some(idx1) = ent.find(':') else {
            continue;
        };
        if idx1 == 0 {
            continue;
        }
        let username = ent[..idx1].trim();
        if !is_valid_username(username) {
            continue;
        }
        let rest = &ent[idx1 + 1..];
        let (role, secret_part) = match rest.find(':') {
            Some(idx2) if idx2 > 0 => {
                if let Some(r) = Role::parse(&rest[..idx2]) {
                    (r, rest[idx2 + 1..].trim())
                } else {
                    (Role::Admin, rest.trim())
                }
            }
            _ => (Role::Admin, rest.trim()),
        };
        if secret_part.is_empty() {
            continue;
        }
        let hash = if secret_part.starts_with("scrypt$") {
            secret_part.to_string()
        } else {
            hash_password(secret_part)?
        };
        let key = username.to_string();
        if !map.contains_key(&key) {
            order.push(key.clone());
        }
        map.insert(
            key.clone(),
            UserRecord {
                username: key,
                role,
                hash,
            },
        );
    }

    Ok(order.into_iter().filter_map(|k| map.remove(&k)).collect())
}

pub(crate) fn is_valid_username(u: &str) -> bool {
    !u.is_empty()
        && u.len() <= 64
        && u.bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
}

#[derive(Debug, Clone)]
pub struct AuthConfig {
    pub api_token: Option<String>,
    pub explicit_token: bool,
    pub users_spec: String,
    pub public_deploy: bool,
    pub auth_secret: String,
    pub session_ttl_sec: u64,
    pub trust_proxy: bool,
    pub token_role: Role,
    pub allow_query_token: Option<bool>,
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self {
            api_token: None,
            explicit_token: false,
            users_spec: String::new(),
            public_deploy: false,
            auth_secret: String::new(),
            session_ttl_sec: 43_200,
            trust_proxy: false,
            token_role: Role::Admin,
            allow_query_token: None,
        }
    }
}

impl AuthConfig {
    /// Load auth configuration from environment variables matching `server.js`.
    pub fn from_env() -> Self {
        let env_token = std::env::var("SITREP_TOKEN")
            .ok()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty());
        let explicit_token = env_token.is_some();
        let users_spec = std::env::var("SITREP_USERS").unwrap_or_default();
        let auth_secret = std::env::var("SITREP_AUTH_SECRET").unwrap_or_default();
        let public_env = std::env::var("SITREP_PUBLIC").ok().as_deref() == Some("1");
        let cloud_paas = [
            "RAILWAY_ENVIRONMENT",
            "RENDER",
            "FLY_APP_NAME",
            "VERCEL",
            "HEROKU_APP_NAME",
            "K_SERVICE",
        ]
        .iter()
        .any(|k| std::env::var(k).ok().is_some_and(|v| !v.is_empty()));
        let public_deploy = public_env || cloud_paas;
        let trust_proxy = std::env::var("SITREP_TRUST_PROXY").ok().as_deref() == Some("1");
        let token_role = std::env::var("SITREP_TOKEN_ROLE")
            .ok()
            .and_then(|s| Role::parse(&s))
            .unwrap_or(Role::Admin);
        let disable_query =
            std::env::var("SITREP_DISABLE_QUERY_TOKEN").ok().as_deref() == Some("1");

        Self {
            api_token: env_token,
            explicit_token,
            users_spec,
            public_deploy,
            auth_secret,
            session_ttl_sec: 43_200,
            trust_proxy,
            token_role,
            allow_query_token: if disable_query { Some(false) } else { None },
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct RequestAuthContext<'a> {
    pub token_header: Option<&'a str>,
    pub query_token: Option<&'a str>,
    pub cookie_header: Option<&'a str>,
    pub is_https: bool,
}

#[derive(Debug, Clone)]
pub struct LoginResult {
    pub principal: Principal,
    pub token: String,
    pub set_cookie: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct SessionPayload {
    u: String,
    r: String,
    iat: i64,
    exp: i64,
    jti: String,
}

#[derive(Default)]
struct RevokedStore {
    map: HashMap<String, i64>,
    order: VecDeque<String>,
}

pub struct AuthManager {
    api_token: String,
    explicit_token: bool,
    users: HashMap<String, UserRecord>,
    user_mode: bool,
    public_mode: bool,
    signing_key: [u8; 32],
    session_ttl_sec: u64,
    trust_proxy: bool,
    token_role: Role,
    allow_query_token: bool,
    revoked: Mutex<RevokedStore>,
}

impl AuthManager {
    pub fn new(cfg: AuthConfig) -> Result<Self, String> {
        let api_token = cfg
            .api_token
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| random_hex(16));
        let user_list = parse_users_entries(&cfg.users_spec)?;
        let users: HashMap<String, UserRecord> = user_list
            .into_iter()
            .map(|u| (u.username.clone(), u))
            .collect();
        let user_mode = !users.is_empty();
        let public_mode = !user_mode && cfg.public_deploy;

        if user_mode && cfg.auth_secret.len() < 32 {
            return Err(
                "SITREP_AUTH_SECRET wajib minimal 32 karakter saat SITREP_USERS (mode session) aktif"
                    .into(),
            );
        }

        let signing_key = if cfg.auth_secret.len() >= 32 {
            let digest = Sha256::digest(cfg.auth_secret.as_bytes());
            let mut key = [0u8; 32];
            key.copy_from_slice(&digest);
            key
        } else {
            let mut key = [0u8; 32];
            rand::rng().fill_bytes(&mut key);
            key
        };

        let session_ttl_sec = cfg.session_ttl_sec.max(300);
        let allow_query_token = cfg.allow_query_token.unwrap_or_else(|| {
            !public_mode
                && !user_mode
                && std::env::var("SITREP_DISABLE_QUERY_TOKEN").ok().as_deref() != Some("1")
        });

        Ok(Self {
            api_token,
            explicit_token: cfg.explicit_token,
            users,
            user_mode,
            public_mode,
            signing_key,
            session_ttl_sec,
            trust_proxy: cfg.trust_proxy,
            token_role: cfg.token_role,
            allow_query_token,
            revoked: Mutex::new(RevokedStore::default()),
        })
    }

    pub fn mode(&self) -> AuthMode {
        if self.user_mode {
            AuthMode::Session
        } else if self.public_mode {
            AuthMode::Public
        } else {
            AuthMode::Token
        }
    }

    pub fn user_mode(&self) -> bool {
        self.user_mode
    }

    pub fn public_mode(&self) -> bool {
        self.public_mode
    }

    pub fn allow_query_token(&self) -> bool {
        self.allow_query_token
    }

    pub fn trust_proxy(&self) -> bool {
        self.trust_proxy
    }

    pub fn api_token(&self) -> &str {
        &self.api_token
    }

    pub fn has_role(&self, principal: Option<&Principal>, min_role: Role) -> bool {
        principal.is_some_and(|p| p.role.rank() >= min_role.rank())
    }

    pub fn sign_session(&self, username: &str, role: Role) -> (String, i64, String) {
        self.sign_session_at(username, role, now_sec())
    }

    pub fn sign_session_at(&self, username: &str, role: Role, now_s: i64) -> (String, i64, String) {
        let exp = now_s + self.session_ttl_sec as i64;
        let jti = random_hex(12);
        let payload = SessionPayload {
            u: username.to_string(),
            r: role.as_str().to_string(),
            iat: now_s,
            exp,
            jti: jti.clone(),
        };
        // INFALLIBLE: SessionPayload only contains strings and integers.
        let json = serde_json::to_string(&payload).unwrap_or_default();
        let b64 = URL_SAFE_NO_PAD.encode(json.as_bytes());
        let sig = self.hmac_b64(&b64);
        (format!("{b64}.{sig}"), exp, jti)
    }

    fn hmac_b64(&self, data: &str) -> String {
        // INFALLIBLE: HMAC-SHA256 accepts any key length.
        let mut mac = match HmacSha256::new_from_slice(&self.signing_key) {
            Ok(m) => m,
            Err(_) => return String::new(),
        };
        mac.update(data.as_bytes());
        URL_SAFE_NO_PAD.encode(mac.finalize().into_bytes())
    }

    pub fn verify_session_token(&self, raw_tok: &str) -> Option<Principal> {
        self.verify_session_token_at(raw_tok, now_sec())
    }

    pub fn verify_session_token_at(&self, raw_tok: &str, now_s: i64) -> Option<Principal> {
        let dot = raw_tok.find('.')?;
        if dot == 0 {
            return None;
        }
        let b64 = &raw_tok[..dot];
        let sig = &raw_tok[dot + 1..];
        let expected = self.hmac_b64(b64);
        if !timing_equal_str(sig, &expected) {
            return None;
        }
        let decoded = URL_SAFE_NO_PAD.decode(b64.as_bytes()).ok()?;
        let payload: SessionPayload = serde_json::from_slice(&decoded).ok()?;
        Role::parse(&payload.r)?;
        if payload.exp <= now_s {
            return None;
        }
        let user = self.users.get(&payload.u)?;
        if !payload.jti.is_empty()
            && let Ok(guard) = self.revoked.lock()
            && guard.map.contains_key(&payload.jti)
        {
            return None;
        }
        Some(Principal {
            username: payload.u,
            role: user.role,
            via: "session".into(),
            jti: Some(payload.jti),
            exp: Some(payload.exp),
        })
    }

    pub fn revoke_token(&self, raw_tok: &str) {
        self.revoke_token_at(raw_tok, now_ms());
    }

    pub fn revoke_token_at(&self, raw_tok: &str, now_millis: i64) {
        let Some(p) = self.verify_session_token_at(raw_tok, now_millis / 1000) else {
            return;
        };
        let (Some(jti), Some(exp)) = (p.jti, p.exp) else {
            return;
        };
        let Ok(mut guard) = self.revoked.lock() else {
            return;
        };
        if !guard.map.contains_key(&jti) {
            guard.order.push_back(jti.clone());
        }
        guard.map.insert(jti, exp * 1000);
        if guard.map.len() > REVOKED_SWEEP_THRESHOLD {
            guard.map.retain(|_, exp_ms| *exp_ms > now_millis);
            let map_ref = &guard.map;
            let kept: VecDeque<String> = guard
                .order
                .iter()
                .filter(|k| map_ref.contains_key(*k))
                .cloned()
                .collect();
            guard.order = kept;
            while guard.map.len() > REVOKED_HARD_CAP {
                let Some(oldest) = guard.order.pop_front() else {
                    break;
                };
                guard.map.remove(&oldest);
            }
        }
    }

    pub fn authenticate(&self, ctx: &RequestAuthContext<'_>) -> Option<Principal> {
        let hdr_tok = ctx.token_header.unwrap_or("");
        let qry_tok = if self.allow_query_token {
            ctx.query_token.unwrap_or("")
        } else {
            ""
        };
        let token_matches = timing_equal_str(hdr_tok, &self.api_token)
            || timing_equal_str(qry_tok, &self.api_token);

        if !self.user_mode {
            if self.public_mode {
                if self.explicit_token && token_matches {
                    return Some(Principal {
                        username: "api-token".into(),
                        role: self.token_role,
                        via: "token".into(),
                        jti: None,
                        exp: None,
                    });
                }
                return Some(Principal {
                    username: "public".into(),
                    role: Role::Viewer,
                    via: "public".into(),
                    jti: None,
                    exp: None,
                });
            }
            if token_matches {
                return Some(Principal {
                    username: "local".into(),
                    role: Role::Admin,
                    via: "token".into(),
                    jti: None,
                    exp: None,
                });
            }
            return None;
        }

        if let Some(cookie_hdr) = ctx.cookie_header {
            let cookies = Self::parse_cookies(cookie_hdr);
            if let Some(raw_tok) = cookies.get("sitrep_sess")
                && let Some(principal) = self.verify_session_token(raw_tok)
            {
                return Some(principal);
            }
        }

        if self.explicit_token && token_matches {
            return Some(Principal {
                username: "api-token".into(),
                role: self.token_role,
                via: "token".into(),
                jti: None,
                exp: None,
            });
        }
        None
    }

    pub async fn verify_credentials(&self, username: &str, password: &str) -> Option<Principal> {
        let uname = username.trim();
        let Some(rec) = self.users.get(uname) else {
            let dummy_pw = if password.is_empty() { "x" } else { password };
            let _ =
                verify_password_async(dummy_pw.to_string(), DUMMY_SCRYPT_HASH.to_string()).await;
            return None;
        };
        let ok = verify_password_async(password.to_string(), rec.hash.clone()).await;
        ok.then(|| Principal {
            username: rec.username.clone(),
            role: rec.role,
            via: "session".into(),
            jti: None,
            exp: None,
        })
    }

    pub async fn login(
        &self,
        username: &str,
        password: &str,
        is_https: bool,
    ) -> Option<LoginResult> {
        let mut principal = self.verify_credentials(username, password).await?;
        let (token, exp, jti) = self.sign_session(&principal.username, principal.role);
        principal.jti = Some(jti);
        principal.exp = Some(exp);
        let set_cookie = self.session_cookie_header(&token, self.session_ttl_sec, is_https);
        Some(LoginResult {
            principal,
            token,
            set_cookie,
        })
    }

    pub fn logout(&self, raw_tok: &str, is_https: bool) -> String {
        if !raw_tok.is_empty() {
            self.revoke_token(raw_tok);
        }
        self.session_cookie_header("", 0, is_https)
    }

    pub fn session_cookie_header(
        &self,
        token_value: &str,
        max_age_sec: u64,
        is_https: bool,
    ) -> String {
        let mut out = format!(
            "sitrep_sess={token_value}; Path=/; HttpOnly; SameSite=Lax; Max-Age={max_age_sec}"
        );
        if is_https {
            out.push_str("; Secure");
        }
        out
    }

    pub fn parse_cookies(raw: &str) -> HashMap<String, String> {
        let mut out = HashMap::new();
        for part in raw.split(';') {
            let Some(eq) = part.find('=') else {
                continue;
            };
            if eq == 0 {
                continue;
            }
            let k = part[..eq].trim();
            let v_raw = part[eq + 1..].trim();
            let v = percent_decode(v_raw);
            out.insert(k.to_string(), v);
        }
        out
    }

    pub fn inject_html_meta(&self, html: &str, principal: Option<&Principal>) -> String {
        let meta = if self.public_mode {
            r#"<meta name="sitrep-public" content="1">"#.to_string()
        } else if !self.user_mode {
            format!(
                r#"<meta name="sitrep-token" content="{}">"#,
                esc_html(&self.api_token)
            )
        } else if let Some(p) = principal {
            format!(
                r#"<meta name="sitrep-user" content="{}">"#,
                esc_html(&format!("{}:{}", p.username, p.role.as_str()))
            )
        } else {
            return html.to_string();
        };

        if let Some(pos) = find_head_tag_end(html) {
            let mut out = String::with_capacity(html.len() + meta.len());
            out.push_str(&html[..pos]);
            out.push_str(&meta);
            out.push_str(&html[pos..]);
            out
        } else {
            format!("{meta}{html}")
        }
    }

    pub fn render_login_html(&self) -> &'static str {
        concat!(
            "<!doctype html>",
            r#"<html lang="id"><head><meta charset="utf-8">"#,
            r#"<meta name="viewport" content="width=device-width,initial-scale=1">"#,
            "<title>SITREP ... Autentikasi Diperlukan</title>",
            "<style>",
            "*{box-sizing:border-box;margin:0;padding:0}",
            r#"body{min-height:100vh;display:flex;align-items:center;justify-content:center;background:#0a0d12;color:#e6edf3;font:14px/1.5 -apple-system,BlinkMacSystemFont,"Segoe UI",Roboto,monospace}"#,
            ".card{width:100%;max-width:360px;background:#11161f;border:1px solid #263042;border-radius:10px;padding:28px;box-shadow:0 16px 40px rgba(0,0,0,.45)}",
            "h1{font-size:18px;letter-spacing:.08em;text-transform:uppercase;color:#f0f6fc;margin-bottom:6px}",
            "p.sub{color:#8b949e;font-size:12px;margin-bottom:20px}",
            "label{display:block;font-size:12px;color:#9ea7b3;margin-bottom:6px}",
            "input{width:100%;padding:10px 12px;margin-bottom:14px;border-radius:6px;border:1px solid #2d3748;background:#090c10;color:#f0f6fc;font:14px inherit}",
            "input:focus{outline:none;border-color:#58a6ff}",
            "button{width:100%;padding:10px 14px;border:0;border-radius:6px;background:#238636;color:#fff;font:600 14px inherit;cursor:pointer}",
            "button:hover{background:#2ea043}",
            ".err{min-height:18px;color:#f85149;font-size:12px;margin-bottom:10px}",
            "</style></head><body>",
            r#"<form class="card" id="f">"#,
            "<h1>SITREP</h1>",
            r#"<p class="sub">Masuk dengan akun operator untuk mengakses dashboard OSINT.</p>"#,
            r#"<div class="err" id="err"></div>"#,
            r#"<label for="u">Username</label>"#,
            r#"<input id="u" name="username" autocomplete="username" required autofocus>"#,
            r#"<label for="p">Password</label>"#,
            r#"<input id="p" name="password" type="password" autocomplete="current-password" required>"#,
            r#"<button type="submit">Masuk</button>"#,
            "</form>",
            "<script>",
            r#"const f=document.getElementById("f"),err=document.getElementById("err");"#,
            r#"f.addEventListener("submit",async e=>{e.preventDefault();err.textContent="";"#,
            r#"try{const r=await fetch("/api/auth/login",{method:"POST",credentials:"same-origin",headers:{"content-type":"application/json"},"#,
            r#"body:JSON.stringify({username:f.username.value,password:f.password.value})});"#,
            r#"const j=await r.json().catch(()=>({}));"#,
            r#"if(!r.ok||!j.ok){err.textContent=j.err||"Login gagal ("+r.status+")";return;}"#,
            r#"location.reload();}catch{err.textContent="Koneksi gagal";}});"#,
            "</script></body></html>"
        )
    }
}

fn find_head_tag_end(html: &str) -> Option<usize> {
    let lower = html.to_ascii_lowercase();
    let start = lower.find("<head")?;
    let rest = &html[start + 5..];
    let first_after = rest.as_bytes().first().copied();
    if let Some(b) = first_after
        && b != b'>'
        && !b.is_ascii_whitespace()
    {
        return None;
    }
    let close = rest.find('>')?;
    Some(start + 5 + close + 1)
}

fn esc_html(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '"' => out.push_str("&quot;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            _ => out.push(c),
        }
    }
    out
}

fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let (Some(h), Some(l)) = (hex_nibble(bytes[i + 1]), hex_nibble(bytes[i + 2]))
        {
            out.push((h << 4) | l);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8(out).unwrap_or_else(|_| s.to_string())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RateRule {
    pub limit: u32,
    pub window_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RateDecision {
    pub allowed: bool,
    pub limit: u32,
    pub remaining: u32,
    pub retry_after_sec: u64,
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub exempt: bool,
}

struct BucketEntry {
    count: u32,
    reset_at_ms: u64,
}

#[derive(Default)]
struct LimiterState {
    store: HashMap<String, BucketEntry>,
    order: VecDeque<String>,
}

pub struct RateLimiter {
    rules: HashMap<String, RateRule>,
    default_rule: RateRule,
    state: Mutex<LimiterState>,
}

impl Default for RateLimiter {
    fn default() -> Self {
        Self::new()
    }
}

impl RateLimiter {
    pub fn new() -> Self {
        Self::with_buckets(HashMap::new())
    }

    pub fn with_buckets(extra: HashMap<String, RateRule>) -> Self {
        let api_rule = RateRule {
            limit: 1200,
            window_ms: 60_000,
        };
        let mut rules = HashMap::from([
            (
                "login".into(),
                RateRule {
                    limit: 10,
                    window_ms: 300_000,
                },
            ),
            (
                "heavy".into(),
                RateRule {
                    limit: 600,
                    window_ms: 60_000,
                },
            ),
            ("api".into(), api_rule),
        ]);
        rules.extend(extra);
        let default_rule = rules.get("api").copied().unwrap_or(api_rule);
        Self {
            rules,
            default_rule,
            state: Mutex::new(LimiterState::default()),
        }
    }

    pub fn size(&self) -> usize {
        self.state.lock().map_or(0, |g| g.store.len())
    }

    pub fn check(
        &self,
        bucket_name: &str,
        ip: &str,
        exempt: bool,
        rule_override: Option<RateRule>,
    ) -> RateDecision {
        self.check_at(
            bucket_name,
            ip,
            exempt,
            rule_override,
            now_ms().max(0) as u64,
        )
    }

    pub fn check_at(
        &self,
        bucket_name: &str,
        ip: &str,
        exempt: bool,
        rule_override: Option<RateRule>,
        now_millis: u64,
    ) -> RateDecision {
        let rule = rule_override
            .or_else(|| self.rules.get(bucket_name).copied())
            .unwrap_or(self.default_rule);
        if exempt {
            return RateDecision {
                allowed: true,
                limit: rule.limit,
                remaining: rule.limit,
                retry_after_sec: 0,
                exempt: true,
            };
        }

        let key = format!("{bucket_name}:{}", rate_key_str(ip));
        let Ok(mut guard) = self.state.lock() else {
            return RateDecision {
                allowed: true,
                limit: rule.limit,
                remaining: rule.limit.saturating_sub(1),
                retry_after_sec: 1,
                exempt: false,
            };
        };

        if guard.store.len() > MAX_RATE_KEYS {
            guard.store.retain(|_, v| v.reset_at_ms > now_millis);
            let store_ref = &guard.store;
            let kept: VecDeque<String> = guard
                .order
                .iter()
                .filter(|k| store_ref.contains_key(*k))
                .cloned()
                .collect();
            guard.order = kept;
            while guard.store.len() > MAX_RATE_KEYS {
                match guard.order.front() {
                    Some(k) if k.starts_with("login:") => break,
                    Some(_) => {
                        if let Some(oldest) = guard.order.pop_front() {
                            guard.store.remove(&oldest);
                        }
                    }
                    None => break,
                }
            }
        }

        let needs_reset = guard
            .store
            .get(&key)
            .is_none_or(|e| e.reset_at_ms <= now_millis);
        if needs_reset {
            if !guard.store.contains_key(&key) {
                guard.order.push_back(key.clone());
            }
            guard.store.insert(
                key.clone(),
                BucketEntry {
                    count: 0,
                    reset_at_ms: now_millis.saturating_add(rule.window_ms),
                },
            );
        }

        let (count, reset_at_ms) = match guard.store.get_mut(&key) {
            Some(ent) => {
                ent.count = ent.count.saturating_add(1);
                (ent.count, ent.reset_at_ms)
            }
            None => (1, now_millis.saturating_add(rule.window_ms)),
        };

        let allowed = count <= rule.limit;
        let remaining = rule.limit.saturating_sub(count);
        let delta_ms = reset_at_ms.saturating_sub(now_millis);
        let retry_after_sec = delta_ms.div_ceil(1000).max(1);
        RateDecision {
            allowed,
            limit: rule.limit,
            remaining,
            retry_after_sec,
            exempt: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedPublicUrl {
    pub url: Url,
    pub host: String,
    pub addrs: Vec<IpAddr>,
}

/// Async SSRF guard: validates URL scheme/host/credentials and resolves DNS via `tokio::net::lookup_host`,
/// rejecting if any resolved IP falls into a private/special IPv4 or IPv6 range.
pub async fn assert_public_url(raw: &str) -> Result<ResolvedPublicUrl, String> {
    assert_public_url_with(raw, |host, port| async move {
        let iter = tokio::net::lookup_host((host.as_str(), port))
            .await
            .map_err(|_| "host tidak ditemukan".to_string())?;
        Ok(iter.map(|sa| sa.ip()).collect())
    })
    .await
}

/// Generic testable SSRF guard accepting a custom async DNS resolver.
pub async fn assert_public_url_with<F, Fut>(
    raw: &str,
    resolver: F,
) -> Result<ResolvedPublicUrl, String>
where
    F: FnOnce(String, u16) -> Fut,
    Fut: Future<Output = Result<Vec<IpAddr>, String>>,
{
    let u = Url::parse(raw.trim()).map_err(|_| "URL tidak valid".to_string())?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return Err("URL tidak valid".into());
    }
    if !u.username().is_empty() || u.password().is_some_and(|p| !p.is_empty()) {
        return Err("URL berkredensial ditolak".into());
    }
    let Some(raw_host) = u.host_str() else {
        return Err("host lokal ditolak".into());
    };
    let host = {
        let s = raw_host.strip_prefix('[').unwrap_or(raw_host);
        s.strip_suffix(']').unwrap_or(s)
    }
    .trim_end_matches('.')
    .to_ascii_lowercase();

    if parse_public_url(raw).is_none() {
        if host.parse::<IpAddr>().is_ok() {
            return Err("alamat privat ditolak".into());
        }
        return Err("host lokal ditolak".into());
    }

    if let Ok(ip) = host.parse::<IpAddr>() {
        if is_private_ip_str(&host) {
            return Err("alamat privat ditolak".into());
        }
        return Ok(ResolvedPublicUrl {
            url: u,
            host,
            addrs: vec![ip],
        });
    }

    let port = u.port_or_known_default().unwrap_or(80);
    let raw_addrs = resolver(host.clone(), port).await?;
    let mut addrs: Vec<IpAddr> = Vec::with_capacity(raw_addrs.len());
    for ip in raw_addrs {
        if !addrs.contains(&ip) {
            addrs.push(ip);
        }
    }
    if addrs.is_empty() {
        return Err("host tidak ditemukan".into());
    }
    if addrs.iter().any(|ip| is_private_ip_str(&ip.to_string())) {
        return Err("alamat privat ditolak".into());
    }
    Ok(ResolvedPublicUrl {
        url: u,
        host,
        addrs,
    })
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_millis() as i64)
}

fn now_sec() -> i64 {
    now_ms() / 1000
}
