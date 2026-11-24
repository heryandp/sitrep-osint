//! Rust port of `src/nlp-adapter.js` (`getConfig`, `redactPII`, and `NlpAdapter`).

#![forbid(unsafe_code)]

use std::borrow::Cow;
use std::collections::{HashMap, VecDeque};
use std::sync::LazyLock;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sha1::{Digest, Sha1};

pub const DEFAULT_TIMEOUT_MS: u64 = 2500;
pub const MAX_RETRIES: usize = 1;
pub const CACHE_CAP: usize = 512;
pub const CIRCUIT_FAILURE_THRESHOLD: u32 = 3;
pub const CIRCUIT_OPEN_MS: u64 = 60_000;

type RedactRe = (
    regex::Regex,
    regex::Regex,
    regex::Regex,
    fancy_regex::Regex,
    regex::Regex,
);

static RE: LazyLock<RedactRe> = LazyLock::new(|| {
    // INFALLIBLE: compile-time constant regex literals
    let rx = |p| regex::Regex::new(p).expect("valid regex");
    (
        rx(
            r"(?i-u:https?)://[^\t\n\x0B\x0C\r \u{00A0}\u{1680}\u{2000}-\u{200A}\u{2028}\u{2029}\u{202F}\u{205F}\u{3000}\u{FEFF}]+",
        ),
        rx(r"(?-u:\b)[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Za-z]{2,}(?-u:\b)"),
        rx(r"@[A-Za-z0-9_.]+"),
        fancy_regex::Regex::new(r"(?<![A-Za-z0-9_+])(?:\+?62|0)8[0-9]{7,11}(?![A-Za-z0-9_])")
            .expect("valid phone regex"),
        rx(r"(?-u:\b)[0-9]{12,16}(?-u:\b)"),
    )
});

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| {
        matches!(c, '\t'..='\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'..='\u{200A}'
            | '\u{2028}' | '\u{2029}' | '\u{202F}' | '\u{205F}' | '\u{3000}' | '\u{FEFF}')
    })
}

fn js_to_string(val: &Value, falsy_empty: bool) -> String {
    let fmt_n = |x: f64| match falsy_empty && (x == 0.0 || x.is_nan()) {
        true => String::new(),
        false => crate::js::number_to_string(x),
    };
    let falsy = |s: &str| if falsy_empty { "" } else { s }.to_string();
    match val {
        Value::Null => falsy("null"),
        Value::Bool(true) => "true".into(),
        Value::Bool(false) => falsy("false"),
        Value::Number(n) => fmt_n(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(arr) => arr
            .iter()
            .map(|v| match v.is_null() || crate::js::is_undefined(v) {
                true => String::new(),
                false => js_to_string(v, false),
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) if crate::js::is_undefined(val) => falsy("undefined"),
        Value::Object(_) => {
            crate::js::as_number(val).map_or_else(|| "[object Object]".into(), fmt_n)
        }
    }
}

fn js_str_to_number(s: &str) -> f64 {
    let t = js_trim(s);
    match t {
        "" => return 0.0,
        "Infinity" | "+Infinity" => return f64::INFINITY,
        "-Infinity" => return f64::NEG_INFINITY,
        _ => {}
    }
    for (p, q, r) in [("0x", "0X", 16), ("0b", "0B", 2), ("0o", "0O", 8)] {
        if let Some(rest) = t.strip_prefix(p).or_else(|| t.strip_prefix(q)) {
            return u128::from_str_radix(rest, r).map_or(f64::NAN, |v| v as f64);
        }
    }
    let low = t.to_ascii_lowercase();
    if low.contains("inf") || low.contains("nan") {
        f64::NAN
    } else {
        t.parse().unwrap_or(f64::NAN)
    }
}

fn js_to_number(val: &Value) -> f64 {
    match val {
        Value::Null | Value::Bool(false) => 0.0,
        Value::Bool(true) => 1.0,
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        Value::String(s) => js_str_to_number(s),
        Value::Array(_) => js_str_to_number(&js_to_string(val, false)),
        Value::Object(_) if crate::js::is_undefined(val) => f64::NAN,
        Value::Object(_) => crate::js::as_number(val).unwrap_or(f64::NAN),
    }
}

fn clamp_timeout(raw: f64) -> f64 {
    (if raw.is_nan() || raw == 0.0 {
        DEFAULT_TIMEOUT_MS as f64
    } else {
        raw
    })
    .clamp(500.0, 10_000.0)
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NlpConfig {
    pub enabled: bool,
    pub provider: String,
    pub endpoint: String,
    pub api_key: String,
    pub model: String,
    pub timeout_ms: u64,
}

impl NlpConfig {
    pub fn from_env_lookup(mut get: impl FnMut(&str) -> Option<String>) -> Self {
        let mut s = |k: &str| js_trim(&get(k).unwrap_or_default()).to_string();
        let provider = s("SITREP_NLP_PROVIDER").to_lowercase();
        let endpoint = s("SITREP_NLP_ENDPOINT");
        let api_key = s("SITREP_NLP_API_KEY");
        let model = s("SITREP_NLP_MODEL");
        let raw = get("SITREP_NLP_TIMEOUT_MS").map_or(f64::NAN, |v| js_str_to_number(&v));
        let enabled = !provider.is_empty() && !endpoint.is_empty() && !api_key.is_empty();
        Self {
            enabled,
            provider,
            endpoint,
            api_key,
            model,
            timeout_ms: clamp_timeout(raw) as u64,
        }
    }
}

/// Port of `getConfig` (`src/nlp-adapter.js:9`).
pub fn get_config(env: Value) -> Result<Value, String> {
    let (cfg, t) = match env {
        ref v if crate::js::is_undefined(v) => {
            let c = NlpConfig::from_env_lookup(|k| std::env::var(k).ok());
            let t = c.timeout_ms as f64;
            (c, t)
        }
        Value::Null => {
            return Err(
                "TypeError: Cannot read properties of null (reading 'SITREP_NLP_PROVIDER')".into(),
            );
        }
        Value::Object(map) => {
            let mut c = NlpConfig::from_env_lookup(|k| map.get(k).map(|v| js_to_string(v, true)));
            let t = clamp_timeout(
                map.get("SITREP_NLP_TIMEOUT_MS")
                    .map_or(f64::NAN, js_to_number),
            );
            c.timeout_ms = t as u64;
            (c, t)
        }
        _ => (
            NlpConfig::from_env_lookup(|_| None),
            DEFAULT_TIMEOUT_MS as f64,
        ),
    };
    Ok(json!({
        "enabled": cfg.enabled, "provider": cfg.provider, "endpoint": cfg.endpoint,
        "apiKey": cfg.api_key, "model": cfg.model, "timeoutMs": crate::js::number(t),
    }))
}

fn redact_pii_str(input: &str) -> String {
    let (re_url, re_email, re_user, re_phone, re_id) = &*RE;
    let mut s = Cow::Borrowed(input);
    if s.contains("://") {
        s = Cow::Owned(re_url.replace_all(&s, "[URL]").into_owned());
    }
    if s.contains('@') {
        if s.contains('.') {
            s = Cow::Owned(re_email.replace_all(&s, "[EMAIL]").into_owned());
        }
        if s.contains('@') {
            s = Cow::Owned(re_user.replace_all(&s, "[USER]").into_owned());
        }
    }
    if s.contains("08") || s.contains("628") {
        s = Cow::Owned(re_phone.replace_all(&s, "[PHONE]").into_owned());
    }
    if s.len() >= 12 && s.as_bytes().iter().any(u8::is_ascii_digit) {
        s = Cow::Owned(re_id.replace_all(&s, "[ID]").into_owned());
    }
    js_trim(&s).to_string()
}

/// Port of `redactPII` (`src/nlp-adapter.js:19`).
pub fn redact_pii(text: Value) -> Result<Value, String> {
    Ok(Value::String(redact_pii_str(&js_to_string(&text, true))))
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NlpOverride {
    pub intent: String,
    pub confidence: Option<f64>,
    #[serde(default)]
    pub is_metaphor: bool,
    #[serde(default)]
    pub is_sarcasm: bool,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NlpStats {
    pub calls: u64,
    pub cache_hits: u64,
    pub successes: u64,
    pub failures: u64,
    pub fallbacks: u64,
    pub circuit_skips: u64,
    pub circuit_open: bool,
    pub cache_size: usize,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DisambiguateResult {
    pub used_external: bool,
    pub cached: bool,
    pub reason: Option<String>,
    pub error: Option<String>,
    pub override_result: Option<NlpOverride>,
}

/// Stateful NLP disambiguation adapter with SHA-1 FIFO cache (cap 512) and 3-failure circuit breaker.
#[derive(Debug, Clone, Default)]
pub struct NlpAdapter {
    config: NlpConfig,
    transport_enabled: bool,
    cache: HashMap<String, NlpOverride>,
    cache_order: VecDeque<String>,
    consecutive_failures: u32,
    circuit_open_until: u64,
    stats: NlpStats,
}

impl NlpAdapter {
    pub fn new(config: NlpConfig, transport_enabled: bool) -> Self {
        Self {
            config,
            transport_enabled,
            ..Default::default()
        }
    }

    pub fn config(&self) -> &NlpConfig {
        &self.config
    }

    pub fn redact_pii(input: &str) -> String {
        redact_pii_str(input)
    }

    pub fn cache_key(model: &str, redacted: &str) -> String {
        Sha1::digest(format!("{model}|{redacted}"))
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    pub fn is_available(&self, now_ms: u64) -> bool {
        self.config.enabled && self.transport_enabled && now_ms >= self.circuit_open_until
    }

    pub fn disambiguate_with<F>(
        &mut self,
        raw_text: &str,
        feed_lane: Option<&str>,
        now_ms: u64,
        mut invoke: F,
    ) -> DisambiguateResult
    where
        F: FnMut(&NlpConfig, &str) -> Result<NlpOverride, String>,
    {
        let fb = |r: &str, error| DisambiguateResult {
            reason: Some(r.into()),
            error,
            ..Default::default()
        };
        if !self.config.enabled || !self.transport_enabled {
            self.stats.fallbacks += 1;
            return fb("disabled_offline_fallback", None);
        }
        if now_ms < self.circuit_open_until {
            self.stats.circuit_skips += 1;
            self.stats.fallbacks += 1;
            return fb("circuit_open_fallback", None);
        }
        if matches!(feed_lane, Some("incident_signals" | "rejected")) {
            return fb("not_borderline", None);
        }

        let redacted = redact_pii_str(raw_text);
        let key = Self::cache_key(&self.config.model, &redacted);
        if let Some(hit) = self.cache.get(&key).cloned() {
            self.stats.cache_hits += 1;
            return DisambiguateResult {
                used_external: true,
                cached: true,
                override_result: Some(hit),
                ..Default::default()
            };
        }

        self.stats.calls += 1;
        let mut last_err = String::new();
        for _ in 0..=MAX_RETRIES {
            match invoke(&self.config, &redacted) {
                Ok(out) => {
                    self.consecutive_failures = 0;
                    self.stats.successes += 1;
                    let ov = NlpOverride {
                        intent: out.intent.chars().take(40).collect(),
                        confidence: out
                            .confidence
                            .filter(|c| c.is_finite())
                            .map(|c| c.clamp(0.0, 1.0)),
                        ..out
                    };
                    if !self.cache.contains_key(&key) {
                        if self.cache.len() >= CACHE_CAP
                            && let Some(old) = self.cache_order.pop_front()
                        {
                            self.cache.remove(&old);
                        }
                        self.cache_order.push_back(key.clone());
                    }
                    self.cache.insert(key, ov.clone());
                    return DisambiguateResult {
                        used_external: true,
                        override_result: Some(ov),
                        ..Default::default()
                    };
                }
                Err(e) => last_err = e,
            }
        }

        self.consecutive_failures += 1;
        self.stats.failures += 1;
        self.stats.fallbacks += 1;
        if self.consecutive_failures >= CIRCUIT_FAILURE_THRESHOLD {
            self.circuit_open_until = now_ms.saturating_add(CIRCUIT_OPEN_MS);
        }
        fb(
            "error_deterministic_fallback",
            Some(last_err.chars().take(120).collect()),
        )
    }

    pub fn get_stats(&self, now_ms: u64) -> NlpStats {
        NlpStats {
            circuit_open: now_ms < self.circuit_open_until,
            cache_size: self.cache.len(),
            ..self.stats
        }
    }

    pub fn reset(&mut self) {
        *self = Self::new(self.config.clone(), self.transport_enabled);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn adapter() -> NlpAdapter {
        let cfg = NlpConfig {
            enabled: true,
            provider: "openai-compat".into(),
            endpoint: "https://example.invalid/v1/classify".into(),
            api_key: "dummy".into(),
            model: "sitrep-v1".into(),
            timeout_ms: 600,
        };
        NlpAdapter::new(cfg, true)
    }

    #[test]
    fn offline_borderline_and_circuit_breaker() {
        let mut off = NlpAdapter::new(NlpConfig::from_env_lookup(|_| None), true);
        assert!(!off.is_available(1_000));
        let r = off.disambiguate_with("banjir?", None, 1_000, |_, _| Err("no".into()));
        assert_eq!(r.reason.as_deref(), Some("disabled_offline_fallback"));

        let mut a = adapter();
        let r2 = a.disambiguate_with("banjir", Some("incident_signals"), 1_000, |_, _| {
            Err("no".into())
        });
        assert_eq!(r2.reason.as_deref(), Some("not_borderline"));

        let mut tries = 0;
        for i in 0..4 {
            let res = a.disambiguate_with(&format!("m {i}"), None, 10_000, |_, _| {
                tries += 1;
                Err("NLP HTTP 503".into())
            });
            assert!(!res.used_external);
        }
        let st = a.get_stats(10_000);
        assert_eq!(
            (tries, st.circuit_open, st.failures, st.circuit_skips),
            (6, true, 3, 1)
        );
        assert!(!a.is_available(69_999) && a.is_available(70_000));
        a.reset();
        assert_eq!(a.get_stats(10_000), NlpStats::default());
    }

    #[test]
    fn sha1_cache_hits_and_evicts_at_512() {
        let mut a = adapter();
        let mut calls = 0;
        for _ in 0..2 {
            let r = a.disambiguate_with("WA 081234567890", None, 1_000, |_, red| {
                calls += 1;
                assert_eq!(red, "WA [PHONE]");
                Ok(NlpOverride {
                    intent: "x".repeat(60),
                    confidence: Some(1.7),
                    is_metaphor: true,
                    is_sarcasm: false,
                })
            });
            let ov = r.override_result.expect("override");
            assert_eq!(
                (r.used_external, ov.intent.len(), ov.confidence),
                (true, 40, Some(1.0))
            );
        }
        assert_eq!((calls, a.get_stats(1_000).cache_hits), (1, 1));
        for i in 0..520 {
            a.disambiguate_with(&format!("u {i}"), None, 2_000, |_, _| {
                Ok(NlpOverride {
                    intent: "ctx".into(),
                    ..Default::default()
                })
            });
        }
        assert_eq!(a.get_stats(2_000).cache_size, CACHE_CAP);
    }
}
