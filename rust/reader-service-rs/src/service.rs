//! Async `ReaderService` managing article reader engine, thumbnail resolver,
//! description generator, and image proxy with SSRF protection, WAF learning,
//! and fallbacks to Jina and Firecrawl.

#![forbid(unsafe_code)]
#![allow(
    dead_code,
    unused_imports,
    clippy::duplicate_mod,
    clippy::collapsible_if,
    clippy::manual_map,
    clippy::too_many_arguments
)]

use crate::port::{article_extract, article_media, feedtext};
use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::net::IpAddr;
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, LazyLock, RwLock};
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use url::Url;

pub const PIPELINE_VERSION: &str = "1.0.0";
pub const DEFAULT_DOC_CACHE_BYTES: usize = 48 * 1024 * 1024;
pub const DOC_CACHE_MAX_HTML: usize = 1024 * 1024;
pub const LEARNED_WAF_TTL_MS: i64 = 30 * 60 * 1000;
pub const FC_FAIL_TTL_MS: i64 = 10 * 60 * 1000;
pub const READ_FAIL_TRANSIENT_TTL_MS: i64 = 15 * 1000;
pub const READ_FAIL_PERM_TTL_MS: i64 = 180 * 1000;
pub const DESC_TTL_MS: i64 = 6 * 3600 * 1000;
pub const DESC_NEG_TTL_MS: i64 = 10 * 60 * 1000;
pub const DESC_TRANSIENT_NEG_TTL_MS: i64 = 45 * 1000;

pub static RD_BOT_UAS: &[&str] = &[
    "WhatsApp/2.23.20.0 A",
    "facebookexternalhit/1.1 (+http://www.facebook.com/externalhit_uatext.php)",
    "Mozilla/5.0 (compatible; Googlebot/2.1; +http://www.google.com/bot.html)",
    "TelegramBot (like TwitterBot)",
    "Wget/1.21",
    "curl/8.7.1",
];

pub static IMG_HOSTS: &[&str] = &[
    "fbcdn.net",
    "cdninstagram.com",
    "unavatar.io",
    "googleusercontent.com",
    "ggpht.com",
];

static CF_WAF_HOST_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(?:^|\.)(?:tempo\.co|cnnindonesia\.com|cnbcindonesia\.com|jpnn\.com|pikiran-rakyat\.com|mediaindonesia\.com|tribunnews\.com|bisnis\.com|kontan\.co\.id|kompasiana\.com|suara\.com|sindonews\.com|okezone\.com|inews\.id|polri\.go\.id)$").expect("valid regex")
});

static WAF_SERVER_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:hcdn|cloudflare|ddos-guard|sucuri|akamaighost)\b").expect("valid regex")
});

static BOT_CHALLENGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\b(?:just a moment\.\.\.|checking your browser|cf-browser-verification|challenge-platform|ddos-guard|attention required|ray id:|waf-block)\b").expect("valid regex")
});

static GN_PATH_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^/rss/articles/|/articles/|/read/").expect("valid regex"));

static VID_FILE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)\.(mp4|webm|mov|m4v|m3u8|mpd)(\?|#|$)").expect("valid regex")
});

static HAS_META_VAL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r#"(?i)<meta[^>]+(?:og:description|twitter:description|name\s*=\s*["']description["'])[^>]*content\s*=\s*["'][^"']{40,}["']"#).expect("valid regex")
});

#[inline]
pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[derive(Debug, Clone)]
pub struct VerifiedProxyImg {
    pub ua: String,
    pub referer: String,
    pub final_url: String,
    pub at: i64,
}

#[derive(Debug, Clone)]
pub struct DocCacheEntry {
    pub html: Option<String>,
    pub jina_txt: Option<String>,
    pub final_url: String,
    pub via: String,
    pub bytes: usize,
    pub at: i64,
}

#[derive(Debug, Clone)]
pub struct DescCacheEntry {
    pub desc: String,
    pub transient: bool,
    pub pub_visible: bool,
    pub at: i64,
    pub ttl: i64,
}

#[derive(Debug, Clone)]
pub struct ReadCacheEntry {
    pub value: Value,
    pub at: i64,
    pub ttl: i64,
}

#[derive(Debug, Clone)]
pub struct ProxyImageResponse {
    pub status: u16,
    pub content_type: String,
    pub body: Vec<u8>,
    pub headers: Vec<(String, String)>,
}

#[derive(Debug, Clone, Default)]
pub struct ReaderServiceConfig {
    pub jina_api_key: Option<String>,
    pub firecrawl_api_key: Option<String>,
    pub custom_proxy_url: Option<String>,
    pub pipeline_version: String,
    pub doc_cache_bytes: usize,
}

pub struct ReaderService {
    config: ReaderServiceConfig,
    client: reqwest::Client,
    thumb_cache: RwLock<HashMap<String, (Value, i64)>>,
    read_cache: RwLock<HashMap<String, ReadCacheEntry>>,
    doc_cache: RwLock<HashMap<String, DocCacheEntry>>,
    doc_alias: RwLock<HashMap<String, String>>,
    desc_cache: RwLock<HashMap<String, DescCacheEntry>>,
    gn_cache: RwLock<HashMap<String, (String, i64)>>,
    probe_cache: RwLock<HashMap<String, (Value, i64)>>,
    verified_proxy_imgs: RwLock<HashMap<String, VerifiedProxyImg>>,
    learned_waf_hosts: RwLock<HashMap<String, i64>>,
    fc_fail_until: RwLock<HashMap<String, i64>>,
    known_news_urls: RwLock<HashSet<String>>,
    known_vhs_urls: RwLock<HashSet<String>>,
    fc_quota_blocked_until: AtomicI64,
    jina_key_bad_until: AtomicI64,
    jina_skip_until: AtomicI64,
}

impl Default for ReaderService {
    fn default() -> Self {
        Self::new(ReaderServiceConfig::default())
    }
}

impl ReaderService {
    pub fn new(mut config: ReaderServiceConfig) -> Self {
        if config.pipeline_version.is_empty() {
            config.pipeline_version = PIPELINE_VERSION.to_string();
        }
        if config.doc_cache_bytes == 0 {
            config.doc_cache_bytes = DEFAULT_DOC_CACHE_BYTES;
        }

        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .redirect(reqwest::redirect::Policy::none())
            .build()
            .unwrap_or_else(|_| reqwest::Client::new());

        Self {
            config,
            client,
            thumb_cache: RwLock::new(HashMap::new()),
            read_cache: RwLock::new(HashMap::new()),
            doc_cache: RwLock::new(HashMap::new()),
            doc_alias: RwLock::new(HashMap::new()),
            desc_cache: RwLock::new(HashMap::new()),
            gn_cache: RwLock::new(HashMap::new()),
            probe_cache: RwLock::new(HashMap::new()),
            verified_proxy_imgs: RwLock::new(HashMap::new()),
            learned_waf_hosts: RwLock::new(HashMap::new()),
            fc_fail_until: RwLock::new(HashMap::new()),
            known_news_urls: RwLock::new(HashSet::new()),
            known_vhs_urls: RwLock::new(HashSet::new()),
            fc_quota_blocked_until: AtomicI64::new(0),
            jina_key_bad_until: AtomicI64::new(0),
            jina_skip_until: AtomicI64::new(0),
        }
    }

    pub fn is_public_url(raw: &str) -> bool {
        let Ok(parsed) = Url::parse(raw) else {
            return false;
        };
        if parsed.scheme() != "http" && parsed.scheme() != "https" {
            return false;
        }
        if !parsed.username().is_empty() || parsed.password().is_some() {
            return false;
        }
        let Some(host_str) = parsed.host_str() else {
            return false;
        };
        let host_low = host_str.to_lowercase();
        if host_low == "localhost"
            || host_low.ends_with(".localhost")
            || host_low.ends_with(".local")
        {
            return false;
        }
        if let Ok(ip) = host_low.parse::<IpAddr>() {
            if ip.is_loopback() || ip.is_unspecified() || ip.is_multicast() {
                return false;
            }
            match ip {
                IpAddr::V4(v4) => {
                    if v4.is_private() || v4.is_link_local() || v4.is_broadcast() {
                        return false;
                    }
                }
                IpAddr::V6(v6) => {
                    let seg = v6.segments();
                    // fc00::/7 (unique local) or fe80::/10 (link-local)
                    if (seg[0] & 0xfe00) == 0xfc00 || (seg[0] & 0xffc0) == 0xfe80 {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn canonical_url(&self, raw: &str) -> String {
        if let Ok(Value::String(s)) =
            article_media::canonical_article_url(Value::String(raw.to_string()))
        {
            if !s.is_empty() {
                return s;
            }
        }
        raw.to_string()
    }

    pub fn is_waf_host_url(&self, u: &str) -> bool {
        let Ok(parsed) = Url::parse(u) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let h = host.trim_start_matches("www.").to_lowercase();
        if CF_WAF_HOST_RE.is_match(&h) {
            return true;
        }
        let now = now_ms();
        if let Ok(waf_map) = self.learned_waf_hosts.read() {
            if let Some(&exp) = waf_map.get(&h) {
                if exp > now {
                    return true;
                }
            }
        }
        false
    }

    pub fn mark_waf_host(&self, u: &str) -> bool {
        let Ok(parsed) = Url::parse(u) else {
            return false;
        };
        let Some(host) = parsed.host_str() else {
            return false;
        };
        let h = host.trim_start_matches("www.").to_lowercase();
        if h.is_empty() {
            return false;
        }
        if let Ok(mut waf_map) = self.learned_waf_hosts.write() {
            waf_map.insert(h, now_ms() + LEARNED_WAF_TTL_MS);
            if waf_map.len() > 200 {
                if let Some(oldest) = waf_map.keys().next().cloned() {
                    waf_map.remove(&oldest);
                }
            }
        }
        true
    }

    pub fn note_waf_response(
        &self,
        u: &str,
        status: u16,
        server: &str,
        html_snippet: &str,
    ) -> bool {
        let hdr_waf =
            (status == 403 || status == 503 || status == 429) && WAF_SERVER_RE.is_match(server);
        let body_challenge = !html_snippet.is_empty()
            && (BOT_CHALLENGE_RE.is_match(html_snippet)
                || self.is_challenge_html_helper(html_snippet, "", ""));
        if hdr_waf || body_challenge {
            self.mark_waf_host(u)
        } else {
            false
        }
    }

    pub fn is_challenge_html_helper(&self, html: &str, title: &str, desc: &str) -> bool {
        let res = article_extract::is_challenge_html(
            Value::String(html.to_string()),
            Some(title.to_string()),
            Some(desc.to_string()),
        );
        match res {
            Ok(Value::Bool(b)) => b,
            _ => {
                let combined = format!("{title} {desc}");
                BOT_CHALLENGE_RE.is_match(&combined)
                    || (html.len() < 45_000 && BOT_CHALLENGE_RE.is_match(html))
            }
        }
    }

    pub fn register_verified_proxy_img(&self, urls: &[String], meta: VerifiedProxyImg) {
        if let Ok(mut map) = self.verified_proxy_imgs.write() {
            for raw in urls {
                if !raw.is_empty() {
                    map.insert(raw.clone(), meta.clone());
                    if map.len() > 2500 {
                        if let Some(oldest) = map.keys().next().cloned() {
                            map.remove(&oldest);
                        }
                    }
                }
            }
        }
    }

    pub fn register_news_url(&self, u: &str) {
        if u.is_empty() {
            return;
        }
        if let Ok(mut set) = self.known_news_urls.write() {
            set.insert(u.to_string());
            let c = self.canonical_url(u);
            if !c.is_empty() && c != u {
                set.insert(c);
            }
            if set.len() > 8000 {
                if let Some(item) = set.iter().next().cloned() {
                    set.remove(&item);
                }
            }
        }
    }

    pub fn register_vhs_url(&self, u: &str) {
        if u.is_empty() {
            return;
        }
        if let Ok(mut set) = self.known_vhs_urls.write() {
            set.insert(u.to_string());
            if set.len() > 2500 {
                if let Some(item) = set.iter().next().cloned() {
                    set.remove(&item);
                }
            }
        }
    }

    pub fn is_fc_enabled(&self) -> bool {
        let key = self
            .config
            .firecrawl_api_key
            .as_deref()
            .or_else(|| {
                std::env::var("FIRECRAWL_API_KEY")
                    .ok()
                    .as_deref()
                    .map(|_| "")
            })
            .unwrap_or("");
        let has_key =
            !key.is_empty() || std::env::var("FIRECRAWL_API_KEY").is_ok_and(|k| !k.is_empty());
        has_key && now_ms() >= self.fc_quota_blocked_until.load(Ordering::Relaxed)
    }

    pub fn fc_mark_quota_exhausted(&self) {
        self.fc_quota_blocked_until
            .store(now_ms() + 15 * 60 * 1000, Ordering::Relaxed);
    }

    pub fn fc_blocked(&self, u: &str) -> bool {
        if now_ms() < self.fc_quota_blocked_until.load(Ordering::Relaxed) {
            return true;
        }
        if let Ok(map) = self.fc_fail_until.read() {
            if let Some(&exp) = map.get(u) {
                if exp > now_ms() {
                    return true;
                }
            }
        }
        false
    }

    pub fn fc_fail_mark(&self, u: &str) {
        if let Ok(mut map) = self.fc_fail_until.write() {
            map.insert(u.to_string(), now_ms() + FC_FAIL_TTL_MS);
            if map.len() > 500 {
                if let Some(item) = map.keys().next().cloned() {
                    map.remove(&item);
                }
            }
        }
    }

    pub fn inject_synthetic_head(html: &str, md: &Value) -> String {
        let esc = |s: &str| {
            s.replace('&', "&amp;")
                .replace('"', "&quot;")
                .replace('<', "&lt;")
                .replace('>', "&gt;")
        };
        let get_s = |key: &str| -> String {
            md.get(key)
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        };
        let title = get_s("title");
        let desc = get_s("description");
        let site = get_s("ogSiteName");
        let time = get_s("publishedTime");
        let author = get_s("author");
        let img = get_s("ogImage");

        let mut tags = Vec::new();
        if !title.is_empty() {
            if !html.contains("<title") && !html.contains("<TITLE") {
                tags.push(format!("<title>{}</title>", esc(&title)));
            }
            if !html.contains("og:title") {
                tags.push(format!(
                    r#"<meta property="og:title" content="{}">"#,
                    esc(&title)
                ));
            }
        }
        if !desc.is_empty() && !html.contains("og:description") {
            tags.push(format!(
                r#"<meta property="og:description" content="{}">"#,
                esc(&desc)
            ));
            tags.push(format!(
                r#"<meta name="description" content="{}">"#,
                esc(&desc)
            ));
        }
        if !site.is_empty() && !html.contains("og:site_name") {
            tags.push(format!(
                r#"<meta property="og:site_name" content="{}">"#,
                esc(&site)
            ));
        }
        if !time.is_empty() && !html.contains("article:published_time") {
            tags.push(format!(
                r#"<meta property="article:published_time" content="{}">"#,
                esc(&time)
            ));
        }
        if !author.is_empty() && !html.contains("author") {
            tags.push(format!(
                r#"<meta name="author" content="{}">"#,
                esc(&author)
            ));
        }
        if !img.is_empty() && !html.contains("og:image") {
            tags.push(format!(
                r#"<meta property="og:image" content="{}">"#,
                esc(&img)
            ));
        }

        if tags.is_empty() {
            return html.to_string();
        }
        let joined = tags.join("\n");
        if let Some(pos) = html.to_lowercase().find("<head") {
            if let Some(close_pos) = html[pos..].find('>') {
                let insert_at = pos + close_pos + 1;
                return format!("{}\n{}\n{}", &html[..insert_at], joined, &html[insert_at..]);
            }
        }
        if let Some(pos) = html.to_lowercase().find("<body") {
            return format!(
                "{}<head>\n{}\n</head>\n{}",
                &html[..pos],
                joined,
                &html[pos..]
            );
        }
        format!("<head>\n{}\n</head>\n{}", joined, html)
    }

    pub fn yt_id_of(raw: &str) -> Option<String> {
        let Ok(parsed) = Url::parse(raw) else {
            return None;
        };
        let host = parsed.host_str().unwrap_or("").to_lowercase();
        let h = host
            .trim_start_matches("www.")
            .trim_start_matches("m.")
            .trim_start_matches("music.");
        if h == "youtu.be" {
            let seg = parsed.path().trim_start_matches('/').split('/').next()?;
            if seg.len() >= 6 && seg.len() <= 15 {
                return Some(seg.to_string());
            }
        }
        if h == "youtube.com" {
            if parsed.path() == "/watch" {
                for (k, v) in parsed.query_pairs() {
                    if k == "v" && v.len() >= 6 && v.len() <= 15 {
                        return Some(v.to_string());
                    }
                }
            }
            if let Some(rest) = parsed
                .path()
                .strip_prefix("/shorts/")
                .or_else(|| parsed.path().strip_prefix("/live/"))
                .or_else(|| parsed.path().strip_prefix("/embed/"))
            {
                let seg = rest.split('/').next()?;
                if seg.len() >= 6 && seg.len() <= 15 {
                    return Some(seg.to_string());
                }
            }
        }
        None
    }

    // --- Google News decoding ---
    pub async fn decode_gnews(&self, u: &str) -> Result<String, String> {
        let Ok(parsed) = Url::parse(u) else {
            return Ok(u.to_string());
        };
        let host = parsed.host_str().unwrap_or("").to_lowercase();
        if !host.ends_with("news.google.com") || !GN_PATH_RE.is_match(parsed.path()) {
            return Ok(u.to_string());
        }

        let id = parsed
            .path()
            .split('/')
            .rfind(|s| !s.is_empty())
            .unwrap_or("")
            .to_string();
        if id.is_empty() {
            return Ok(u.to_string());
        }
        let cache_key = format!("gn:{id}");
        if let Ok(gn_map) = self.gn_cache.read() {
            if let Some((target, at)) = gn_map.get(&cache_key) {
                if now_ms() - at < 1800 * 1000 {
                    return Ok(target.clone());
                }
            }
        }

        let page_url = format!("https://news.google.com/rss/articles/{id}?hl=id&gl=ID&ceid=ID:id");
        let resp = self
            .client
            .get(&page_url)
            .header("referer", "https://news.google.com/")
            .header("accept-language", "id-ID,id;q=0.9,en;q=0.8")
            .send()
            .await
            .map_err(|e| format!("GN page error: {e}"))?;

        let landed = resp.url().as_str().to_string();
        if let Ok(landed_parsed) = Url::parse(&landed) {
            let landed_host = landed_parsed.host_str().unwrap_or("").to_lowercase();
            if !landed_host.ends_with("news.google.com") && Self::is_public_url(&landed) {
                if let Ok(mut gn_map) = self.gn_cache.write() {
                    gn_map.insert(cache_key, (landed.clone(), now_ms()));
                }
                return Ok(landed);
            }
        }

        let html = resp.text().await.map_err(|e| e.to_string())?;
        // Match meta refresh
        let meta_re = Regex::new(
            r#"(?i)<meta[^>]*http-equiv=["']refresh["'][^>]*content=["'][^"']*url=([^"'\s>]+)"#,
        )
        .expect("valid regex");
        if let Some(caps) = meta_re.captures(&html) {
            if let Some(m) = caps.get(1) {
                let target = m.as_str();
                if Self::is_public_url(target) {
                    if let Ok(mut gn_map) = self.gn_cache.write() {
                        gn_map.insert(cache_key, (target.to_string(), now_ms()));
                    }
                    return Ok(target.to_string());
                }
            }
        }

        Ok(u.to_string())
    }

    // --- Description generation ---
    pub async fn desc(&self, req_url: &str) -> Result<String, String> {
        let canon = self.canonical_url(req_url);
        let cache_key = format!("v{}:{canon}", self.config.pipeline_version);

        if let Ok(cache) = self.desc_cache.read() {
            if let Some(entry) = cache.get(&cache_key) {
                if now_ms() - entry.at < entry.ttl {
                    return Ok(entry.desc.clone());
                }
            }
        }

        if !Self::is_public_url(req_url) {
            return Ok(String::new());
        }

        let mut target = req_url.to_string();
        if let Ok(parsed) = Url::parse(req_url) {
            let host = parsed.host_str().unwrap_or("").to_lowercase();
            if host.ends_with("news.google.com") && GN_PATH_RE.is_match(parsed.path()) {
                if let Ok(decoded) = self.decode_gnews(req_url).await {
                    target = decoded;
                }
            }
        }

        // Check read_cache
        if let Ok(rc) = self.read_cache.read() {
            if let Some(entry) = rc.get(&canon) {
                if let Some(desc_val) = entry.value.get("desc").and_then(|v| v.as_str()) {
                    if desc_val.len() >= 26 && !BOT_CHALLENGE_RE.is_match(desc_val) {
                        return Ok(desc_val.to_string());
                    }
                }
            }
        }

        let mut html = String::new();
        let is_waf = self.is_waf_host_url(&target);
        let uas_to_try: Vec<&str> = if is_waf {
            vec![RD_BOT_UAS[0], RD_BOT_UAS[1], RD_BOT_UAS[3], ""]
        } else {
            vec!["", RD_BOT_UAS[0], RD_BOT_UAS[1], RD_BOT_UAS[3]]
        };

        for ua in uas_to_try {
            let mut req = self
                .client
                .get(&target)
                .timeout(Duration::from_millis(3500));
            if !ua.is_empty() {
                req = req.header("user-agent", ua);
            }
            if let Ok(resp) = req.send().await {
                let status = resp.status().as_u16();
                let server = resp
                    .headers()
                    .get("server")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if !self.is_challenge_html_helper(&text, "", "") {
                            html = text;
                            break;
                        } else {
                            self.note_waf_response(&target, status, &server, &text);
                        }
                    }
                } else {
                    self.note_waf_response(&target, status, &server, "");
                }
            }
        }

        // If direct/bot failed, try Jina fallback
        if html.is_empty() {
            let jina_url = format!("https://r.jina.ai/{target}");
            let mut req = self
                .client
                .get(&jina_url)
                .timeout(Duration::from_millis(5000));
            let jina_key = self
                .config
                .jina_api_key
                .as_deref()
                .or_else(|| std::env::var("JINA_API_KEY").ok().as_deref().map(|_| ""))
                .unwrap_or("");
            if !jina_key.is_empty() && now_ms() >= self.jina_key_bad_until.load(Ordering::Relaxed) {
                req = req.header("authorization", format!("Bearer {jina_key}"));
            }
            if let Ok(resp) = req.send().await {
                if resp.status().as_u16() == 402 || resp.status().as_u16() == 401 {
                    self.jina_key_bad_until
                        .store(now_ms() + 15 * 60 * 1000, Ordering::Relaxed);
                } else if resp.status().is_success() {
                    if let Ok(txt) = resp.text().await {
                        if !BOT_CHALLENGE_RE.is_match(&txt) {
                            let res_jina = article_extract::rd_jina_md(
                                Value::String(txt.clone()),
                                Value::String(target.clone()),
                                json!({}),
                            );
                            if let Ok(val) = res_jina {
                                if let Some(paras) = val.get("paras").and_then(|v| v.as_array()) {
                                    if let Some(first_p) = paras.first().and_then(|p| p.as_str()) {
                                        let desc_from_res = feedtext::desc_from(
                                            json!({ "description": first_p }),
                                            val.get("title")
                                                .cloned()
                                                .unwrap_or(Value::String(String::new())),
                                        );
                                        if let Ok(Value::String(d)) = desc_from_res {
                                            if d.len() >= 26 && !BOT_CHALLENGE_RE.is_match(&d) {
                                                return Ok(d);
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        if html.is_empty() {
            return Ok(String::new());
        }

        // Extract metadata or description from HTML
        let meta_res = article_extract::rd_meta(Value::String(html.clone()));
        let mut desc = String::new();
        if let Ok(meta_val) = meta_res {
            let og_desc = meta_val.get("og:description").and_then(|v| v.as_str());
            let tw_desc = meta_val.get("twitter:description").and_then(|v| v.as_str());
            let gen_desc = meta_val.get("description").and_then(|v| v.as_str());
            let title = meta_val
                .get("og:title")
                .or_else(|| meta_val.get("title"))
                .and_then(|v| v.as_str())
                .unwrap_or("");
            if let Some(raw) = og_desc.or(tw_desc).or(gen_desc) {
                if let Ok(Value::String(cleaned)) = feedtext::desc_from(
                    json!({ "description": raw }),
                    Value::String(title.to_string()),
                ) {
                    desc = cleaned;
                }
            }
        }

        if desc.is_empty() {
            let ext_res = article_extract::rd_extract(
                Value::String(html),
                Value::String(target.clone()),
                json!({}),
            );
            if let Ok(val) = ext_res {
                if let Some(d) = val.get("desc").and_then(|v| v.as_str()) {
                    desc = d.to_string();
                } else if let Some(paras) = val.get("paras").and_then(|v| v.as_array()) {
                    if let Some(first) = paras.first().and_then(|p| p.as_str()) {
                        desc = first.to_string();
                    }
                }
            }
        }

        let out = if desc.chars().count() >= 26 && !BOT_CHALLENGE_RE.is_match(&desc) {
            desc
        } else {
            String::new()
        };

        if let Ok(mut cache) = self.desc_cache.write() {
            cache.insert(
                cache_key,
                DescCacheEntry {
                    desc: out.clone(),
                    transient: out.is_empty(),
                    pub_visible: true,
                    at: now_ms(),
                    ttl: if out.is_empty() {
                        DESC_NEG_TTL_MS
                    } else {
                        DESC_TTL_MS
                    },
                },
            );
        }

        Ok(out)
    }

    // --- Image proxy ---
    pub async fn img(&self, u: &str) -> Result<ProxyImageResponse, String> {
        let Ok(parsed) = Url::parse(u) else {
            return Err("URL tidak valid".to_string());
        };
        if parsed.scheme() != "https" {
            return Err("Protokol harus https".to_string());
        }
        let Some(host_str) = parsed.host_str() else {
            return Err("Host tidak valid".to_string());
        };
        let host_low = host_str.to_lowercase();

        let in_allowed_hosts = IMG_HOSTS
            .iter()
            .any(|&d| host_low == d || host_low.ends_with(&format!(".{d}")));
        let v_meta = self
            .verified_proxy_imgs
            .read()
            .ok()
            .and_then(|m| m.get(u).cloned());

        if !in_allowed_hosts && v_meta.is_none() {
            return Err("Host tidak diizinkan".to_string());
        }

        let mut req = self.client.get(u).timeout(Duration::from_secs(12));
        req = req.header(
            "accept",
            "image/webp,image/png,image/jpeg,image/gif,image/*,*/*;q=0.8",
        );
        if let Some(meta) = &v_meta {
            if !meta.ua.is_empty() {
                req = req.header("user-agent", &meta.ua);
            }
            if !meta.referer.is_empty() {
                req = req.header("referer", &meta.referer);
            }
        }

        let resp = req.send().await.map_err(|e| format!("Proxy error: {e}"))?;
        let status = resp.status().as_u16();
        if !resp.status().is_success() {
            return Ok(ProxyImageResponse {
                status,
                content_type: "text/plain".to_string(),
                body: format!("Upstream HTTP {status}").into_bytes(),
                headers: vec![],
            });
        }

        let ct = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/jpeg")
            .split(';')
            .next()
            .unwrap_or("image/jpeg")
            .trim()
            .to_lowercase();

        if !ct.starts_with("image/") || ct.contains("svg") {
            return Err("Bukan gambar yang valid".to_string());
        }

        let bytes = resp.bytes().await.map_err(|e| e.to_string())?;
        if bytes.len() > 5 * 1024 * 1024 {
            return Err("Body gambar terlalu besar".to_string());
        }

        let headers = vec![
            ("content-type".to_string(), ct.clone()),
            (
                "cache-control".to_string(),
                "public, max-age=3600".to_string(),
            ),
            ("x-content-type-options".to_string(), "nosniff".to_string()),
            (
                "content-security-policy".to_string(),
                "default-src 'none'; sandbox".to_string(),
            ),
        ];

        Ok(ProxyImageResponse {
            status: 200,
            content_type: ct,
            body: bytes.to_vec(),
            headers,
        })
    }

    // --- Thumbnail resolver ---
    pub async fn thumb(&self, raw_url: &str) -> Result<Value, String> {
        if !Self::is_public_url(raw_url) {
            return Err("URL tidak valid".to_string());
        }
        let canon = self.canonical_url(raw_url);
        let cache_key = format!("v{}:{canon}", self.config.pipeline_version);

        if let Ok(tc) = self.thumb_cache.read() {
            if let Some((val, at)) = tc.get(&cache_key) {
                if now_ms() - at < 300 * 1000 {
                    return Ok(val.clone());
                }
            }
        }

        // Check read_cache
        if let Ok(rc) = self.read_cache.read() {
            if let Some(entry) = rc.get(&canon) {
                if let Some(media) = entry.value.get("media") {
                    let formatted = self.format_thumb_response(media, &canon);
                    return Ok(formatted);
                }
            }
        }

        // Read article to resolve thumb if not cached
        let read_res = self.read(raw_url, false, false).await?;
        if let Some(media) = read_res.get("media") {
            let formatted = self.format_thumb_response(media, &canon);
            if let Ok(mut tc) = self.thumb_cache.write() {
                tc.insert(cache_key, (formatted.clone(), now_ms()));
            }
            return Ok(formatted);
        }

        let fallback = json!({
            "img": read_res.get("img").and_then(|v| v.as_str()).unwrap_or(""),
            "w": 0,
            "h": 0,
            "src": raw_url,
            "canonicalUrl": canon,
            "source": "none",
            "method": "none",
            "aspectRatio": 0.0,
            "mime": "",
            "bytes": 0,
            "verified": read_res.get("imgVerified").and_then(|v| v.as_bool()).unwrap_or(false),
            "proxy": false,
            "status": read_res.get("imgStatus").and_then(|v| v.as_str()).unwrap_or("no_candidates"),
            "transient": false,
            "videos": read_res.get("videos").cloned().unwrap_or(json!([])),
            "schemaVersion": self.config.pipeline_version,
            "diagnostics": null
        });

        Ok(fallback)
    }

    fn format_thumb_response(&self, media_res: &Value, canon: &str) -> Value {
        let hero = media_res.get("hero");
        let verified = hero
            .and_then(|h| h.get("verified"))
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let img = hero
            .and_then(|h| h.get("url"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let w = hero
            .and_then(|h| h.get("width"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let h = hero
            .and_then(|h| h.get("height"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let mime = hero
            .and_then(|h| h.get("mime"))
            .and_then(|v| v.as_str())
            .unwrap_or("");
        let bytes = hero
            .and_then(|h| h.get("bytes"))
            .and_then(|v| v.as_i64())
            .unwrap_or(0);
        let aspect = hero
            .and_then(|h| h.get("aspectRatio"))
            .and_then(|v| v.as_f64())
            .unwrap_or(0.0);
        let status = if verified {
            hero.and_then(|h| h.get("status"))
                .and_then(|v| v.as_str())
                .unwrap_or("verified")
        } else {
            "no_candidates"
        };

        json!({
            "img": img,
            "w": w,
            "h": h,
            "src": canon,
            "canonicalUrl": canon,
            "source": hero.and_then(|h| h.get("source")).and_then(|v| v.as_str()).unwrap_or("none"),
            "method": hero.and_then(|h| h.get("method")).and_then(|v| v.as_str()).unwrap_or("none"),
            "aspectRatio": aspect,
            "mime": mime,
            "bytes": bytes,
            "verified": verified,
            "proxy": hero.and_then(|h| h.get("proxy")).and_then(|v| v.as_bool()).unwrap_or(false),
            "status": status,
            "transient": !verified,
            "videos": media_res.get("videos").cloned().unwrap_or(json!([])),
            "schemaVersion": self.config.pipeline_version,
            "diagnostics": media_res.get("diagnostics").cloned().unwrap_or(Value::Null)
        })
    }

    // --- Article reader engine ---
    pub async fn read(&self, raw: &str, _bg: bool, force_fresh: bool) -> Result<Value, String> {
        let t0 = now_ms();
        if !Self::is_public_url(raw) {
            return Err("URL tidak valid".to_string());
        }

        let mut target = raw.to_string();
        if let Ok(parsed) = Url::parse(raw) {
            let host = parsed.host_str().unwrap_or("").to_lowercase();
            if host.ends_with("news.google.com") && GN_PATH_RE.is_match(parsed.path()) {
                if let Ok(decoded) = self.decode_gnews(raw).await {
                    target = decoded;
                }
            }
        }

        let canon = self.canonical_url(&target);

        // Check read cache
        if !force_fresh {
            if let Ok(rc) = self.read_cache.read() {
                if let Some(entry) = rc.get(&canon) {
                    if now_ms() - entry.at < entry.ttl {
                        return Ok(entry.value.clone());
                    }
                }
            }
        }

        // YouTube handler
        if let Some(yid) = Self::yt_id_of(&target) {
            return self.read_youtube(&yid, raw).await;
        }

        let is_waf = self.is_waf_host_url(&target);
        let mut html_doc = String::new();
        let final_url = target.clone();
        let mut via = "direct".to_string();
        let mut saw_transient = false;

        // Step 1: Direct fetch / Bot fetch race
        let uas_to_try: Vec<&str> = if is_waf {
            vec![RD_BOT_UAS[3], RD_BOT_UAS[0], RD_BOT_UAS[1]]
        } else {
            vec!["", RD_BOT_UAS[0], RD_BOT_UAS[1], RD_BOT_UAS[3]]
        };

        for ua in uas_to_try {
            let mut req = self
                .client
                .get(&target)
                .timeout(Duration::from_millis(5500));
            if !ua.is_empty() {
                req = req.header("user-agent", ua);
            }
            if let Ok(resp) = req.send().await {
                let status = resp.status().as_u16();
                let server = resp
                    .headers()
                    .get("server")
                    .and_then(|v| v.to_str().ok())
                    .unwrap_or("")
                    .to_string();
                if resp.status().is_success() {
                    if let Ok(text) = resp.text().await {
                        if !self.is_challenge_html_helper(&text, "", "") {
                            html_doc = text;
                            via = if ua.is_empty() {
                                "direct".to_string()
                            } else {
                                "bot".to_string()
                            };
                            break;
                        } else {
                            saw_transient = true;
                            self.note_waf_response(&target, status, &server, &text);
                        }
                    }
                } else {
                    if status == 403 || status == 429 || status >= 500 {
                        saw_transient = true;
                    }
                    self.note_waf_response(&target, status, &server, "");
                }
            }
        }

        // Step 2: Fallback to Jina if upstream blocked or challenge
        if html_doc.is_empty() {
            let jina_url = format!("https://r.jina.ai/{target}");
            let mut req = self
                .client
                .get(&jina_url)
                .timeout(Duration::from_millis(8000));
            let jina_key = self
                .config
                .jina_api_key
                .as_deref()
                .or_else(|| std::env::var("JINA_API_KEY").ok().as_deref().map(|_| ""))
                .unwrap_or("");
            let use_key =
                !jina_key.is_empty() && now_ms() >= self.jina_key_bad_until.load(Ordering::Relaxed);
            if use_key {
                req = req.header("authorization", format!("Bearer {jina_key}"));
            }
            if let Ok(resp) = req.send().await {
                let status = resp.status().as_u16();
                if (status == 402 || status == 401) && use_key {
                    self.jina_key_bad_until
                        .store(now_ms() + 15 * 60 * 1000, Ordering::Relaxed);
                    // Fall back to keyless
                    if let Ok(resp2) = self
                        .client
                        .get(&jina_url)
                        .timeout(Duration::from_millis(6000))
                        .send()
                        .await
                    {
                        if resp2.status().is_success() {
                            if let Ok(txt) = resp2.text().await {
                                if !BOT_CHALLENGE_RE.is_match(&txt) && txt.len() > 200 {
                                    let ext = article_extract::rd_jina_md(
                                        Value::String(txt.clone()),
                                        Value::String(target.clone()),
                                        json!({}),
                                    );
                                    if let Ok(mut val) = ext {
                                        val["via"] = json!("jina");
                                        val["canonicalUrl"] = json!(canon);
                                        return Ok(val);
                                    }
                                }
                            }
                        }
                    }
                } else if resp.status().is_success() {
                    if let Ok(txt) = resp.text().await {
                        if !BOT_CHALLENGE_RE.is_match(&txt) && txt.len() > 200 {
                            let ext = article_extract::rd_jina_md(
                                Value::String(txt.clone()),
                                Value::String(target.clone()),
                                json!({}),
                            );
                            if let Ok(mut val) = ext {
                                val["via"] = json!("jina");
                                val["canonicalUrl"] = json!(canon);
                                return Ok(val);
                            }
                        }
                    }
                }
            }
        }

        // Step 3: Fallback to Firecrawl if enabled and not blocked
        if html_doc.is_empty() && self.is_fc_enabled() && !self.fc_blocked(&target) {
            let fc_key = self
                .config
                .firecrawl_api_key
                .as_deref()
                .or_else(|| {
                    std::env::var("FIRECRAWL_API_KEY")
                        .ok()
                        .as_deref()
                        .map(|_| "")
                })
                .unwrap_or("");
            let body = json!({
                "url": target,
                "formats": ["html"],
                "onlyMainContent": false,
                "timeout": 12000
            });
            if let Ok(resp) = self
                .client
                .post("https://api.firecrawl.dev/v1/scrape")
                .header("content-type", "application/json")
                .header("authorization", format!("Bearer {fc_key}"))
                .json(&body)
                .timeout(Duration::from_millis(14000))
                .send()
                .await
            {
                let st = resp.status().as_u16();
                if st == 402 || st == 401 {
                    self.fc_mark_quota_exhausted();
                } else if resp.status().is_success() {
                    if let Ok(j) = resp.json::<Value>().await {
                        if let Some(h) = j
                            .get("data")
                            .and_then(|d| d.get("html"))
                            .and_then(|v| v.as_str())
                        {
                            if h.len() > 500 && !self.is_challenge_html_helper(h, "", "") {
                                let md = j
                                    .get("data")
                                    .and_then(|d| d.get("metadata"))
                                    .cloned()
                                    .unwrap_or(json!({}));
                                html_doc = Self::inject_synthetic_head(h, &md);
                                via = "firecrawl".to_string();
                                self.mark_waf_host(&target);
                            }
                        }
                    }
                } else {
                    self.fc_fail_mark(&target);
                }
            } else {
                self.fc_fail_mark(&target);
            }
        }

        if html_doc.is_empty() {
            let err_msg = if saw_transient {
                "Halaman challenge/WAF atau upstream gagal"
            } else {
                "Gagal mengambil artikel"
            };
            return Err(err_msg.to_string());
        }

        // Step 4: Extract article contents
        let mut extract_res = article_extract::rd_extract(
            Value::String(html_doc.clone()),
            Value::String(final_url.clone()),
            json!({}),
        )
        .map_err(|e| format!("Ekstraksi gagal: {e}"))?;

        // Thumbnail / hero resolution check
        let mut img_url = extract_res
            .get("img")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let mut img_verified = extract_res
            .get("imgVerified")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);

        // If hero not verified, check meta og:image
        if !img_verified {
            if let Ok(meta_val) = article_extract::rd_meta(Value::String(html_doc.clone())) {
                if let Some(og_img) = meta_val.get("og:image").and_then(|v| v.as_str()) {
                    if !og_img.is_empty() {
                        img_url = og_img.to_string();
                        img_verified = true;
                    }
                }
            }
        }

        extract_res["ok"] = json!(true);
        extract_res["url"] = json!(raw);
        extract_res["final"] = json!(final_url);
        extract_res["canonicalUrl"] = json!(canon);
        extract_res["via"] = json!(via);
        extract_res["schemaVersion"] = json!(self.config.pipeline_version);
        extract_res["img"] = json!(img_url);
        extract_res["imgVerified"] = json!(img_verified);
        extract_res["imgStatus"] = json!(if img_verified {
            "verified"
        } else {
            "no_candidates"
        });

        let elapsed = now_ms() - t0;
        extract_res["t"] = json!({
            "ms": elapsed,
            "via": via
        });

        // Cache in read_cache
        if let Ok(mut rc) = self.read_cache.write() {
            rc.insert(
                canon.clone(),
                ReadCacheEntry {
                    value: extract_res.clone(),
                    at: now_ms(),
                    ttl: 1800 * 1000,
                },
            );
        }

        Ok(extract_res)
    }

    async fn read_youtube(&self, yid: &str, raw: &str) -> Result<Value, String> {
        let watch = format!("https://www.youtube.com/watch?v={yid}");
        let mut oembed_url =
            Url::parse("https://www.youtube.com/oembed?format=json").expect("valid url");
        oembed_url.query_pairs_mut().append_pair("url", &watch);

        let mut title = "Video YouTube".to_string();
        let mut author = String::new();

        if let Ok(resp) = self
            .client
            .get(oembed_url.as_str())
            .timeout(Duration::from_millis(6000))
            .send()
            .await
        {
            if resp.status().is_success() {
                if let Ok(j) = resp.json::<Value>().await {
                    if let Some(t) = j.get("title").and_then(|v| v.as_str()) {
                        title = t.to_string();
                    }
                    if let Some(a) = j.get("author_name").and_then(|v| v.as_str()) {
                        author = a.to_string();
                    }
                }
            }
        }

        let embed_url = format!("https://www.youtube.com/embed/{yid}");
        let video_block = json!({
            "url": embed_url,
            "kind": "iframe",
            "poster": ""
        });

        let out = json!({
            "ok": true,
            "url": raw,
            "final": watch,
            "title": title,
            "site": "YouTube",
            "byline": author,
            "time": "",
            "img": "",
            "desc": "",
            "paras": [],
            "thin": false,
            "videoPage": true,
            "videos": [video_block.clone()],
            "blocks": [{ "k": "v", "v": video_block }],
            "via": "youtube",
            "canonicalUrl": raw,
            "schemaVersion": self.config.pipeline_version,
            "t": { "via": "youtube" }
        });

        Ok(out)
    }
}
