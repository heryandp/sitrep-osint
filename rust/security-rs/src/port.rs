//! The Rust port of `src/security.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;

use crate::js;
use regex::Regex;
use serde_json::{Value, json};
use std::net::{Ipv4Addr, Ipv6Addr};
use std::sync::LazyLock;
use url::Url;

static PRIVATE_HOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: compile-time constant regex literal.
    Regex::new(
        r"(?i)^(localhost|127\.[0-9]+\.[0-9]+\.[0-9]+|0\.0\.0\.0|10\.[0-9]+\.[0-9]+\.[0-9]+|172\.(1[6-9]|2[0-9]|3[01])\.[0-9]+\.[0-9]+|192\.168\.[0-9]+\.[0-9]+|169\.254\.[0-9]+\.[0-9]+|100\.(6[4-9]|[7-9][0-9]|1[01][0-9]|12[0-7])\.[0-9]+\.[0-9]+|\[?(::1|fe80:.*|fc00:.*|fd[0-9a-f]{2}:.*|::ffff:(127\.|10\.|192\.168\.|172\.(1[6-9]|2[0-9]|3[01])\.|169\.254\.|0\.0\.0\.0))\]?)$",
    )
    .expect("valid PRIVATE_HOST regex")
});

static MAPPED_V4_RE: LazyLock<Regex> = LazyLock::new(|| {
    // INFALLIBLE: compile-time constant regex literal.
    Regex::new(r"(?i)^::ffff:([0-9]+\.[0-9]+\.[0-9]+\.[0-9]+)$").expect("valid MAPPED_V4 regex")
});

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn strip_brackets(s: &str) -> &str {
    let s = s.strip_prefix('[').unwrap_or(s);
    s.strip_suffix(']').unwrap_or(s)
}

fn js_is_truthy(v: &Value) -> bool {
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) => true,
        Value::Object(_) => {
            if js::is_undefined(v) {
                false
            } else if let Some(x) = js::as_number(v) {
                x != 0.0 && !x.is_nan()
            } else if let Some(u) = js::string_units(v) {
                !u.is_empty()
            } else {
                true
            }
        }
    }
}

fn js_to_string(v: &Value) -> String {
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|el| {
                if el.is_null() || js::is_undefined(el) {
                    String::new()
                } else {
                    js_to_string(el)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if js::is_undefined(v) {
                "undefined".into()
            } else if let Some(x) = js::as_number(v) {
                js::number_to_string(x)
            } else if let Some(u) = js::string_units(v) {
                String::from_utf16_lossy(&u)
            } else {
                "[object Object]".into()
            }
        }
    }
}

fn js_or_empty(v: &Value) -> String {
    if js_is_truthy(v) {
        js_to_string(v)
    } else {
        String::new()
    }
}

fn js_to_number(v: &Value) -> f64 {
    match v {
        Value::Null => 0.0,
        Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => {
            let t = js_trim(s);
            if t.is_empty() {
                0.0
            } else if t == "Infinity" || t == "+Infinity" {
                f64::INFINITY
            } else if t == "-Infinity" {
                f64::NEG_INFINITY
            } else {
                t.parse::<f64>().unwrap_or(f64::NAN)
            }
        }
        Value::Array(arr) => match arr.as_slice() {
            [] => 0.0,
            [one] => {
                if one.is_null() || js::is_undefined(one) {
                    0.0
                } else {
                    js_to_number(one)
                }
            }
            _ => f64::NAN,
        },
        Value::Object(_) => {
            if js::is_undefined(v) {
                f64::NAN
            } else {
                js::as_number(v).unwrap_or(f64::NAN)
            }
        }
    }
}

fn net_is_ip(s: &str) -> u8 {
    if s.parse::<Ipv4Addr>().is_ok() {
        4
    } else if s.parse::<Ipv6Addr>().is_ok() {
        6
    } else {
        0
    }
}

pub(crate) fn parse_ipv4_bytes_str(raw: &str) -> Option<[u8; 4]> {
    let s = js_trim(raw);
    let mut iter = s.split('.');
    let mut out = [0u8; 4];
    for slot in &mut out {
        let part = iter.next()?;
        if part.is_empty() || part.len() > 3 || !part.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let n: u16 = part.parse().ok()?;
        if n > 255 {
            return None;
        }
        *slot = n as u8;
    }
    if iter.next().is_some() {
        return None;
    }
    Some(out)
}

pub(crate) fn is_private_v4(b: [u8; 4]) -> bool {
    let [a, x, y, _] = b;
    a == 0
        || a == 10
        || (a == 100 && (64..=127).contains(&x))
        || a == 127
        || (a == 169 && x == 254)
        || (a == 172 && (16..=31).contains(&x))
        || (a == 192 && x == 0 && (y == 0 || y == 2))
        || (a == 192 && x == 168)
        || (a == 198 && (x == 18 || x == 19))
        || (a == 198 && x == 51 && y == 100)
        || (a == 203 && x == 0 && y == 113)
        || a >= 224
}

pub(crate) fn expand_ipv6_str(raw: &str) -> Option<[u16; 8]> {
    let trimmed = js_trim(raw).to_ascii_lowercase();
    let unbracketed = strip_brackets(&trimmed);
    let s = unbracketed.split('%').next().unwrap_or("");
    if s.is_empty() {
        return None;
    }
    let addr: Ipv6Addr = s.parse().ok()?;
    Some(addr.segments())
}

pub fn is_private_ip_str(raw: &str) -> bool {
    let trimmed = js_trim(raw).to_ascii_lowercase();
    let unbracketed = strip_brackets(&trimmed);
    let s = unbracketed.split('%').next().unwrap_or("");
    if s.is_empty() {
        return true;
    }
    if let Some(v4) = parse_ipv4_bytes_str(s) {
        return is_private_v4(v4);
    }
    let Some(h) = expand_ipv6_str(s) else {
        return true;
    };

    let is_zero_prefix5 = h[0] == 0 && h[1] == 0 && h[2] == 0 && h[3] == 0 && h[4] == 0;
    if is_zero_prefix5 && h[5] == 0 && h[6] == 0 && (h[7] == 0 || h[7] == 1) {
        return true;
    }
    if is_zero_prefix5 && (h[5] == 0xffff || h[5] == 0) {
        return is_private_v4([
            (h[6] >> 8) as u8,
            (h[6] & 0xff) as u8,
            (h[7] >> 8) as u8,
            (h[7] & 0xff) as u8,
        ]);
    }
    if h[0] == 0 && h[1] == 0 && h[2] == 0 && h[3] == 0xffff && h[4] == 0 && h[5] == 0 {
        return is_private_v4([
            (h[6] >> 8) as u8,
            (h[6] & 0xff) as u8,
            (h[7] >> 8) as u8,
            (h[7] & 0xff) as u8,
        ]);
    }
    if h[0] == 0x0064 && h[1] == 0xff9b {
        if h[2] == 1 {
            return true;
        }
        if h[2] == 0 && h[3] == 0 && h[4] == 0 && h[5] == 0 {
            return is_private_v4([
                (h[6] >> 8) as u8,
                (h[6] & 0xff) as u8,
                (h[7] >> 8) as u8,
                (h[7] & 0xff) as u8,
            ]);
        }
    }
    if h[0] == 0x2002 {
        return is_private_v4([
            (h[1] >> 8) as u8,
            (h[1] & 0xff) as u8,
            (h[2] >> 8) as u8,
            (h[2] & 0xff) as u8,
        ]);
    }
    if h[0] == 0x2001 && (h[1] == 0x0000 || h[1] == 0x0db8) {
        return true;
    }
    if h[0] == 0x0100 && h[1] == 0 && h[2] == 0 && h[3] == 0 {
        return true;
    }
    if (h[0] & 0xfe00) == 0xfc00
        || (h[0] & 0xffc0) == 0xfe80
        || (h[0] & 0xffc0) == 0xfec0
        || (h[0] & 0xff00) == 0xff00
    {
        return true;
    }
    false
}

/// Pure-Rust public URL validator returning `Option<url::Url>` for `rust/app`.
pub fn parse_public_url(raw: &str) -> Option<Url> {
    if raw.encode_utf16().count() > 4096 {
        return None;
    }
    let u = Url::parse(js_trim(raw)).ok()?;
    if u.scheme() != "http" && u.scheme() != "https" {
        return None;
    }
    if !u.username().is_empty() || u.password().is_some_and(|p| !p.is_empty()) {
        return None;
    }
    let raw_host = u.host_str()?;
    let h = strip_brackets(raw_host)
        .trim_end_matches('.')
        .to_ascii_lowercase();
    if h.is_empty()
        || h == "localhost"
        || h.ends_with(".localhost")
        || h.ends_with(".local")
        || h.ends_with(".internal")
    {
        return None;
    }
    if PRIVATE_HOST_RE.is_match(&h) {
        return None;
    }
    if net_is_ip(&h) != 0 && is_private_ip_str(&h) {
        return None;
    }
    Some(u)
}

pub fn rate_key_str(raw_ip: &str) -> String {
    let trimmed = js_trim(raw_ip);
    let raw = strip_brackets(trimmed);
    let fam = net_is_ip(raw);
    if raw.is_empty() || fam == 0 {
        return if raw.is_empty() {
            "unknown".into()
        } else {
            raw.to_string()
        };
    }
    if fam == 4 {
        return raw.to_string();
    }
    if let Some(caps) = MAPPED_V4_RE.captures(raw)
        && let Some(m) = caps.get(1)
    {
        return m.as_str().to_string();
    }
    let Some(full) = expand_ipv6_str(raw) else {
        return raw.to_string();
    };
    format!(
        "{:x}:{:x}:{:x}:{:x}::/64",
        full[0], full[1], full[2], full[3]
    )
}

/// Port of `rdIPv4Bytes` (`src/security.js:17`).
pub fn rd_i_pv4_bytes(ip: Value) -> Result<Value, String> {
    let s = js_or_empty(&ip);
    match parse_ipv4_bytes_str(&s) {
        Some([a, b, c, d]) => Ok(json!([a, b, c, d])),
        None => Ok(Value::Null),
    }
}

/// Port of `rdPrivateBytes` (`src/security.js:24`).
pub fn rd_private_bytes(b: Value) -> Result<Value, String> {
    if !js_is_truthy(&b) {
        return Ok(Value::Bool(true));
    }
    let (a, x, y): (Option<Value>, Option<Value>, Option<Value>) = match &b {
        Value::Array(arr) => (
            arr.first().cloned(),
            arr.get(1).cloned(),
            arr.get(2).cloned(),
        ),
        Value::String(s) => {
            let mut ch = s.chars();
            (
                ch.next().map(|c| Value::String(c.to_string())),
                ch.next().map(|c| Value::String(c.to_string())),
                ch.next().map(|c| Value::String(c.to_string())),
            )
        }
        _ => return Err("TypeError: b is not iterable".into()),
    };

    let eq_num = |opt: &Option<Value>, target: f64| -> bool {
        opt.as_ref()
            .and_then(js::as_number)
            .is_some_and(|n| n == target)
    };
    let num_of = |opt: &Option<Value>| -> f64 { opt.as_ref().map_or(f64::NAN, js_to_number) };

    let xn = num_of(&x);
    let an = num_of(&a);
    let priv_flag = eq_num(&a, 0.0)
        || eq_num(&a, 10.0)
        || (eq_num(&a, 100.0) && (64.0..=127.0).contains(&xn))
        || eq_num(&a, 127.0)
        || (eq_num(&a, 169.0) && eq_num(&x, 254.0))
        || (eq_num(&a, 172.0) && (16.0..=31.0).contains(&xn))
        || (eq_num(&a, 192.0) && eq_num(&x, 0.0) && (eq_num(&y, 0.0) || eq_num(&y, 2.0)))
        || (eq_num(&a, 192.0) && eq_num(&x, 168.0))
        || (eq_num(&a, 198.0) && (eq_num(&x, 18.0) || eq_num(&x, 19.0)))
        || (eq_num(&a, 198.0) && eq_num(&x, 51.0) && eq_num(&y, 100.0))
        || (eq_num(&a, 203.0) && eq_num(&x, 0.0) && eq_num(&y, 113.0))
        || an >= 224.0;
    Ok(Value::Bool(priv_flag))
}

/// Port of `rdPrivateIP` (`src/security.js:72`).
pub fn rd_private_ip(ip: Value) -> Result<Value, String> {
    Ok(Value::Bool(is_private_ip_str(&js_or_empty(&ip))))
}

/// Port of `publicURL` (`src/security.js:114`).
pub fn public_url(raw: Value) -> Result<Value, String> {
    let Some(s) = raw.as_str() else {
        return Ok(Value::Null);
    };
    match parse_public_url(s) {
        Some(_) => Ok(json!({})),
        None => Ok(Value::Null),
    }
}

/// Port of `verifyPassword` (`src/security.js:907`).
pub async fn verify_password(password: Value, stored: Value) -> Result<Value, String> {
    let stored_str = js_or_empty(&stored);
    let pw_str = js_to_string(&password);
    Ok(Value::Bool(service::verify_password_js_compat(
        &pw_str,
        &stored_str,
    )))
}

/// Port of `parseUsersSpec` (`src/security.js:921`).
pub fn parse_users_spec(spec: Value) -> Result<Value, String> {
    let entries = service::parse_users_entries(&js_or_empty(&spec))?;
    let pairs: Vec<Value> = entries
        .into_iter()
        .map(|u| {
            json!([
                u.username,
                {
                    "username": u.username,
                    "role": u.role.as_str(),
                    "hash": u.hash
                }
            ])
        })
        .collect();
    Ok(json!({ "$map": pairs }))
}

fn role_rank_exact(s: &str) -> u8 {
    match s {
        "viewer" => 1,
        "operator" => 2,
        "admin" => 3,
        _ => 0,
    }
}

/// Port of `hasRole` (`src/security.js:952`).
pub fn has_role(principal: Value, min_role: Value) -> Result<Value, String> {
    if !js_is_truthy(&principal) {
        return Ok(Value::Bool(false));
    }
    let role_str = principal
        .as_object()
        .and_then(|o| o.get("role"))
        .map(js_to_string)
        .unwrap_or_default();
    let min_str = js_to_string(&min_role);
    let rank = role_rank_exact(&role_str);
    let req_rank = match role_rank_exact(&min_str) {
        0 => 99,
        r => r,
    };
    Ok(Value::Bool(rank >= req_rank))
}

/// Port of `rateKey` (`src/security.js:1005`).
pub fn rate_key(ip: Value) -> Result<Value, String> {
    Ok(Value::String(rate_key_str(&js_or_empty(&ip))))
}

fn env_trusted_hops() -> f64 {
    let raw = std::env::var("SITREP_TRUSTED_HOPS").unwrap_or_default();
    let t = js_trim(&raw);
    let n = if t.is_empty() {
        0.0
    } else {
        t.parse::<f64>().unwrap_or(f64::NAN)
    };
    if n.is_finite() && n != 0.0 { n } else { 1.0 }
}

/// Port of `getClientIp` (`src/security.js:980`).
pub fn get_client_ip(
    req: Value,
    trust_proxy: Option<bool>,
    trusted_hops: Option<Value>,
) -> Result<Value, String> {
    let trust_proxy = trust_proxy.unwrap_or(false);
    let hops_num: Option<f64> = match &trusted_hops {
        None => Some(env_trusted_hops()),
        Some(v) if js::is_undefined(v) => Some(env_trusted_hops()),
        Some(v) => js::as_number(v).filter(|n| n.is_finite()),
    };

    if trust_proxy
        && js_is_truthy(&req)
        && let Some(headers) = req.as_object().and_then(|o| o.get("headers"))
        && js_is_truthy(headers)
    {
        let h_obj = headers.as_object();
        let cf_raw_val = h_obj
            .and_then(|o| o.get("cf-connecting-ip"))
            .map(js_or_empty)
            .unwrap_or_default();
        let cf_raw = strip_brackets(js_trim(&cf_raw_val));
        let cf_valid = if !cf_raw.is_empty() && net_is_ip(cf_raw) != 0 {
            cf_raw
        } else {
            ""
        };
        let via_cf = h_obj
            .and_then(|o| o.get("cf-ray"))
            .is_some_and(js_is_truthy);
        let trust_cf_env = std::env::var("SITREP_TRUST_CF").ok().as_deref() == Some("1");
        if !cf_valid.is_empty() && (trust_cf_env || via_cf) {
            return Ok(Value::String(cf_valid.to_string()));
        }

        if let Some(xff_val) = h_obj.and_then(|o| o.get("x-forwarded-for"))
            && js_is_truthy(xff_val)
        {
            let xff_str = js_to_string(xff_val);
            let ips: Vec<String> = xff_str
                .split(',')
                .map(|s| strip_brackets(js_trim(s)).to_string())
                .filter(|ip| net_is_ip(ip) != 0)
                .collect();
            if !ips.is_empty() {
                let hops = match hops_num {
                    Some(n) => {
                        let f = n.floor();
                        if f < 1.0 {
                            1usize
                        } else if f >= ips.len() as f64 {
                            ips.len()
                        } else {
                            f as usize
                        }
                    }
                    None => 1usize,
                };
                return Ok(Value::String(ips[ips.len().saturating_sub(hops)].clone()));
            }
        }

        if !cf_valid.is_empty() {
            return Ok(Value::String(cf_valid.to_string()));
        }

        let xr_val = h_obj
            .and_then(|o| o.get("x-real-ip"))
            .map(js_or_empty)
            .unwrap_or_default();
        let xr = strip_brackets(js_trim(&xr_val));
        if !xr.is_empty() && net_is_ip(xr) != 0 {
            return Ok(Value::String(xr.to_string()));
        }
    }

    if js_is_truthy(&req)
        && let Some(sock) = req.as_object().and_then(|o| o.get("socket"))
        && js_is_truthy(sock)
        && let Some(rem) = sock.as_object().and_then(|o| o.get("remoteAddress"))
        && js_is_truthy(rem)
    {
        return Ok(rem.clone());
    }
    Ok(Value::String("unknown".into()))
}

fn is_https_req_impl(req: &Value, trust_proxy: bool) -> bool {
    if js_is_truthy(req)
        && let Some(sock) = req.as_object().and_then(|o| o.get("socket"))
        && js_is_truthy(sock)
        && sock
            .as_object()
            .and_then(|o| o.get("encrypted"))
            .is_some_and(js_is_truthy)
    {
        return true;
    }
    if trust_proxy
        && js_is_truthy(req)
        && let Some(headers) = req.as_object().and_then(|o| o.get("headers"))
        && js_is_truthy(headers)
    {
        let xfp_raw = headers
            .as_object()
            .and_then(|o| o.get("x-forwarded-proto"))
            .map(js_or_empty)
            .unwrap_or_default();
        let first = xfp_raw.split(',').next().unwrap_or("");
        if js_trim(first).eq_ignore_ascii_case("https") {
            return true;
        }
    }
    false
}

/// Port of `isHttpsReq` (`src/security.js:1016`).
pub fn is_https_req(req: Value, trust_proxy: Option<bool>) -> Result<Value, String> {
    Ok(Value::Bool(is_https_req_impl(
        &req,
        trust_proxy.unwrap_or(false),
    )))
}

fn url_host_with_port(u: &Url) -> Option<String> {
    let host = u.host_str()?;
    Some(match u.port() {
        Some(p) => format!("{host}:{p}"),
        None => host.to_string(),
    })
}

/// Port of `isSameOriginPost` (`src/security.js:1033`).
pub fn is_same_origin_post(req: Value, trust_proxy: Option<bool>) -> Result<Value, String> {
    if req.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'headers')".into());
    }
    if js::is_undefined(&req) {
        return Err("TypeError: Cannot read properties of undefined (reading 'headers')".into());
    }
    let headers_val = req.as_object().and_then(|o| o.get("headers"));
    let Some(headers_v) = headers_val else {
        return Err("TypeError: Cannot read properties of undefined (reading 'origin')".into());
    };
    if headers_v.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'origin')".into());
    }
    if js::is_undefined(headers_v) {
        return Err("TypeError: Cannot read properties of undefined (reading 'origin')".into());
    }

    let h_obj = headers_v.as_object();
    let origin = h_obj.and_then(|o| o.get("origin"));
    let referer = h_obj.and_then(|o| o.get("referer"));
    let host = h_obj.and_then(|o| o.get("host"));

    let Some(host_v) = host.filter(|v| js_is_truthy(v)) else {
        return Ok(Value::Bool(false));
    };
    let host_lower = js_to_string(host_v).to_ascii_lowercase();
    let trust_proxy = trust_proxy.unwrap_or(false);
    let expected_proto = if is_https_req_impl(&req, trust_proxy) {
        "https"
    } else {
        "http"
    };

    let matches_origin_url = |raw: &Value| -> bool {
        let s = js_to_string(raw);
        let Ok(u) = Url::parse(&s) else {
            return false;
        };
        let Some(u_host) = url_host_with_port(&u) else {
            return false;
        };
        if u_host.to_ascii_lowercase() != host_lower {
            return false;
        }
        !trust_proxy || u.scheme() == expected_proto
    };

    if let Some(o) = origin
        && js_is_truthy(o)
    {
        return Ok(Value::Bool(matches_origin_url(o)));
    }
    if let Some(r) = referer
        && js_is_truthy(r)
    {
        return Ok(Value::Bool(matches_origin_url(r)));
    }
    let sfs = h_obj
        .and_then(|o| o.get("sec-fetch-site"))
        .map(js_or_empty)
        .unwrap_or_default()
        .to_ascii_lowercase();
    Ok(Value::Bool(sfs == "same-origin"))
}
