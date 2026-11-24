//! The Rust port of `src/xpool.js`.

#![forbid(unsafe_code)]

mod pinned;

use std::collections::HashSet;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::js;

const WIB_OFFSET_MS: f64 = 7.0 * 3_600_000.0;
const MAX_DATE_MS: f64 = 8.64e15;
const RATE_LIMIT_FALLBACK_MS: i64 = 5 * 60_000;
const AUTH_ERROR_DISABLE_MS: i64 = 30 * 60_000;

/// Single X account credential and cooldown/disable state.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XAccount {
    pub id: usize,
    #[serde(rename = "auth_token")]
    pub auth_token: String,
    pub ct0: String,
    pub cooldown_until: i64,
    pub disabled_until: i64,
}

impl XAccount {
    pub fn is_healthy(&self, now_ms: i64) -> bool {
        now_ms >= self.cooldown_until && now_ms >= self.disabled_until
    }

    pub fn collect(env: Option<&Value>, cfg_x: Option<&Value>) -> Result<Vec<Self>, String> {
        collect_x_accounts(env, cfg_x)
    }
}

/// Sanitized per-account health snapshot (never exposes `auth_token` or `ct0`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct XAccountStatus {
    pub id: usize,
    pub healthy: bool,
    pub wait_sec: i64,
}

/// Round-robin X account pool with per-account 429 cooldown and 401/403 isolation.
#[derive(Debug, Clone, Default)]
pub struct XPool {
    accounts: Vec<XAccount>,
    cursor: usize,
}

impl XPool {
    pub fn new(accounts: Vec<XAccount>) -> Self {
        Self {
            accounts,
            cursor: 0,
        }
    }

    pub fn from_env(cfg_x: Option<&Value>) -> Result<Self, String> {
        Ok(Self::new(collect_x_accounts(None, cfg_x)?))
    }

    pub fn format_wib_time(ms: f64) -> String {
        format_wib_time_ms(ms)
    }

    pub fn all(&self) -> &[XAccount] {
        &self.accounts
    }

    pub fn size(&self) -> usize {
        self.accounts.len()
    }

    pub fn is_empty(&self) -> bool {
        self.accounts.is_empty()
    }

    pub fn has(&self) -> bool {
        !self.accounts.is_empty()
    }

    pub fn pick(&mut self, now_ms: i64, exclude_ids: Option<&HashSet<usize>>) -> Option<&XAccount> {
        let len = self.accounts.len();
        for i in 0..len {
            let idx = (self.cursor + i) % len;
            let acc = &self.accounts[idx];
            if exclude_ids.is_some_and(|ex| ex.contains(&acc.id)) || !acc.is_healthy(now_ms) {
                continue;
            }
            self.cursor = (idx + 1) % len;
            return Some(&self.accounts[idx]);
        }
        None
    }

    pub fn all_cooling(&self, now_ms: i64) -> bool {
        !self.accounts.is_empty() && self.accounts.iter().all(|a| !a.is_healthy(now_ms))
    }

    pub fn earliest_reset(&self, now_ms: i64) -> i64 {
        self.accounts
            .iter()
            .map(|a| a.cooldown_until.max(a.disabled_until))
            .min()
            .unwrap_or(now_ms)
    }

    pub fn mark_rate_limit(&mut self, account_id: usize, reset_time_ms: i64, now_ms: i64) {
        if let Some(acc) = self.accounts.iter_mut().find(|a| a.id == account_id) {
            let target = if reset_time_ms > now_ms {
                reset_time_ms
            } else {
                now_ms + RATE_LIMIT_FALLBACK_MS
            };
            acc.cooldown_until = acc.cooldown_until.max(target);
        }
    }

    pub fn mark_auth_error(&mut self, account_id: usize, now_ms: i64) {
        if let Some(acc) = self.accounts.iter_mut().find(|a| a.id == account_id) {
            acc.disabled_until = acc.disabled_until.max(now_ms + AUTH_ERROR_DISABLE_MS);
        }
    }

    pub fn status(&self, now_ms: i64) -> Vec<XAccountStatus> {
        self.accounts
            .iter()
            .map(|a| {
                let until = a.cooldown_until.max(a.disabled_until);
                let wait_sec = if until > now_ms {
                    ((until - now_ms) as u64).div_ceil(1000) as i64
                } else {
                    0
                };
                XAccountStatus {
                    id: a.id,
                    healthy: a.is_healthy(now_ms),
                    wait_sec,
                }
            })
            .collect()
    }
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{FEFF}')
}

fn js_truthy(v: &Value) -> bool {
    if v.is_null() || js::is_undefined(v) {
        return false;
    }
    if let Some(n) = js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    match v {
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
        _ => false,
    }
}

fn js_to_string(v: &Value) -> String {
    if v.is_null() {
        return "null".into();
    }
    if js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = js::as_number(v) {
        return js::number_to_string(n);
    }
    match v {
        Value::Bool(b) => b.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|e| {
                if e.is_null() || js::is_undefined(e) {
                    String::new()
                } else {
                    js_to_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        _ => "[object Object]".into(),
    }
}

fn js_or_empty(v: Option<&Value>) -> String {
    v.filter(|val| js_truthy(val))
        .map_or_else(String::new, js_to_string)
}

fn js_str_to_number(s: &str) -> f64 {
    let t = js_trim(s);
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    for (lo, hi, r) in [("0x", "0X", 16), ("0b", "0B", 2), ("0o", "0O", 8)] {
        if let Some(rest) = t.strip_prefix(lo).or_else(|| t.strip_prefix(hi)) {
            return u128::from_str_radix(rest, r).map_or(f64::NAN, |n| n as f64);
        }
    }
    let ok = t
        .bytes()
        .all(|b| b.is_ascii_digit() || matches!(b, b'+' | b'-' | b'.' | b'e' | b'E'));
    if ok {
        t.parse::<f64>().unwrap_or(f64::NAN)
    } else {
        f64::NAN
    }
}

fn js_to_number(v: &Value) -> f64 {
    if v.is_null() {
        return 0.0;
    }
    if js::is_undefined(v) {
        return f64::NAN;
    }
    if let Some(n) = js::as_number(v) {
        return n;
    }
    match v {
        Value::Bool(b) => f64::from(*b),
        Value::String(s) => js_str_to_number(s),
        Value::Array(arr) => match arr.as_slice() {
            [] => 0.0,
            [one] if one.is_null() || js::is_undefined(one) => 0.0,
            [one] => js_str_to_number(&js_to_string(one)),
            _ => f64::NAN,
        },
        _ => f64::NAN,
    }
}

fn push_pair(out: &mut Vec<(String, String)>, tok: Option<&Value>, ct0: Option<&Value>) {
    let (a, c) = (
        js_trim(&js_or_empty(tok)).to_string(),
        js_trim(&js_or_empty(ct0)).to_string(),
    );
    if !a.is_empty() && !c.is_empty() {
        out.push((a, c));
    }
}

fn split_list(v: Option<&Value>) -> Vec<String> {
    js_or_empty(v)
        .split(',')
        .map(|s| js_trim(s).to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn collect_x_accounts(env: Option<&Value>, cfg_x: Option<&Value>) -> Result<Vec<XAccount>, String> {
    if env.is_some_and(Value::is_null) {
        return Err("TypeError: Cannot read properties of null (reading 'X_AUTH_TOKEN')".into());
    }
    let use_process_env = env.is_none_or(js::is_undefined);
    let env_obj = env.and_then(Value::as_object);
    let get_env = |key: &str| {
        if use_process_env {
            std::env::var(key).ok().map(Value::String)
        } else {
            env_obj.and_then(|m| m.get(key).cloned())
        }
    };
    let cfg_obj = cfg_x.and_then(Value::as_object);

    let mut candidates = Vec::new();
    let env_toks = split_list(get_env("X_AUTH_TOKEN").as_ref());
    let env_cts = split_list(get_env("X_CT0").as_ref());
    if !env_toks.is_empty() && !env_cts.is_empty() {
        candidates.extend(env_toks.into_iter().zip(env_cts));
    } else if let Some(cfg) = cfg_obj {
        let (tok, ct0) = (cfg.get("auth_token"), cfg.get("ct0"));
        if tok.is_some_and(js_truthy) && ct0.is_some_and(js_truthy) {
            push_pair(&mut candidates, tok, ct0);
        }
    }

    for n in 2..=10 {
        let tok = get_env(&format!("X_AUTH_TOKEN_{n}"));
        let ct0 = get_env(&format!("X_CT0_{n}"));
        push_pair(&mut candidates, tok.as_ref(), ct0.as_ref());
    }

    if let Some(accs) = cfg_obj
        .and_then(|c| c.get("accounts"))
        .and_then(Value::as_array)
    {
        for obj in accs.iter().filter_map(Value::as_object) {
            push_pair(&mut candidates, obj.get("auth_token"), obj.get("ct0"));
        }
    }

    let (mut seen, mut out) = (HashSet::new(), Vec::new());
    for (auth_token, ct0) in candidates {
        if seen.insert(auth_token.clone()) {
            out.push(XAccount {
                id: out.len() + 1,
                auth_token,
                ct0,
                cooldown_until: 0,
                disabled_until: 0,
            });
        }
    }
    Ok(out)
}

/// Port of `buildXAccounts` (`src/xpool.js:3`).
pub fn build_x_accounts(env: Value, cfg_x: Value) -> Result<Value, String> {
    let accounts = collect_x_accounts(Some(&env), Some(&cfg_x))?;
    serde_json::to_value(accounts).map_err(|e| e.to_string())
}

fn format_wib_time_ms(ms: f64) -> String {
    let base = if ms == 0.0 || ms.is_nan() {
        pinned::now_ms()
    } else {
        ms
    };
    let shifted = base + WIB_OFFSET_MS;
    if !shifted.is_finite() || shifted.abs() > MAX_DATE_MS {
        return "NaN:NaN".into();
    }
    let ms_in_day = (shifted.trunc() as i64).rem_euclid(86_400_000);
    format!(
        "{:02}:{:02}",
        ms_in_day / 3_600_000,
        (ms_in_day % 3_600_000) / 60_000
    )
}

/// Port of `formatWibTime` (`src/xpool.js:114`).
pub fn format_wib_time(ms: Value) -> Result<Value, String> {
    Ok(Value::String(format_wib_time_ms(js_to_number(&ms))))
}
