//! The Rust port of `src/article-extract.js`.

#![forbid(unsafe_code)]

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde_json::{Map, Value, json};
use std::borrow::Cow;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::LazyLock;
use url::Url;

fn sub_one(t: &mut String, r: &Regex, rep: &str) {
    if let Cow::Owned(next) = r.replace(t, rep) {
        *t = next;
    }
}

fn sub_all(t: &mut String, r: &Regex, rep: &str) {
    if let Cow::Owned(next) = r.replace_all(t, rep) {
        *t = next;
    }
}

fn is_js_ws(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\x0B' | '\x0C' | '\r' | ' ' | '\u{00A0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200A}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202F}'
                | '\u{205F}'
                | '\u{3000}'
                | '\u{FEFF}'
    )
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(is_js_ws)
}

fn js_trim_end(s: &str) -> &str {
    s.trim_end_matches(is_js_ws)
}

fn collapse_ws(s: &str, trim: bool) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_ws = trim;
    for c in s.chars() {
        if is_js_ws(c) {
            if !prev_ws {
                if !trim {
                    out.push(' ');
                }
                prev_ws = true;
            }
        } else {
            if trim && prev_ws && !out.is_empty() {
                out.push(' ');
            }
            out.push(c);
            prev_ws = false;
        }
    }
    out
}

fn js_escape_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        if matches!(
            c,
            '.' | '*' | '+' | '?' | '^' | '$' | '{' | '}' | '(' | ')' | '|' | '[' | ']' | '\\'
        ) {
            out.push('\\');
        }
        out.push(c);
    }
    out
}

fn re(pat: &str) -> Regex {
    // INFALLIBLE: compile-time constant regex pattern
    Regex::new(pat).expect("valid static regex")
}

#[allow(dead_code)]
fn fre(pat: &str) -> FancyRegex {
    // INFALLIBLE: compile-time constant fancy-regex pattern
    FancyRegex::new(pat).expect("valid static fancy-regex")
}

fn is_truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    if let Some(n) = crate::js::as_number(v) {
        return !n.is_nan() && n != 0.0;
    }
    if let Some(u) = crate::js::string_units(v) {
        return !u.is_empty();
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Array(_) | Value::Object(_) => true,
        _ => false,
    }
}

fn js_to_string(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".to_string();
    }
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Null => "null".to_string(),
        Value::Bool(b) => if *b { "true" } else { "false" }.to_string(),
        Value::Array(arr) => arr
            .iter()
            .map(|e| {
                if e.is_null() || crate::js::is_undefined(e) {
                    String::new()
                } else {
                    js_to_string(e)
                }
            })
            .collect::<Vec<_>>()
            .join(","),
        Value::Object(_) => {
            if let Some(u) = crate::js::string_units(v) {
                String::from_utf16_lossy(&u)
            } else {
                "[object Object]".to_string()
            }
        }
        _ => String::new(),
    }
}

fn js_falsy_or_string(v: &Value) -> String {
    if is_truthy(v) {
        js_to_string(v)
    } else {
        String::new()
    }
}

fn js_nullish_or_string(v: &Value) -> String {
    if v.is_null() || crate::js::is_undefined(v) {
        String::new()
    } else {
        js_to_string(v)
    }
}

static ENT: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("amp", "&");
    m.insert("lt", "<");
    m.insert("gt", ">");
    m.insert("quot", "\"");
    m.insert("apos", "'");
    m.insert("nbsp", " ");
    m.insert("hellip", "…");
    m.insert("mdash", "—");
    m.insert("ndash", "–");
    m.insert("lsquo", "‘");
    m.insert("rsquo", "’");
    m.insert("ldquo", "“");
    m.insert("rdquo", "”");
    m.insert("laquo", "«");
    m.insert("raquo", "»");
    m.insert("bull", "•");
    m.insert("middot", "·");
    m.insert("copy", "©");
    m.insert("reg", "®");
    m.insert("trade", "™");
    m.insert("ntilde", "ñ");
    m.insert("Ntilde", "Ñ");
    m.insert("eacute", "é");
    m.insert("Eacute", "É");
    m.insert("egrave", "è");
    m.insert("agrave", "à");
    m.insert("aacute", "á");
    m.insert("iacute", "í");
    m.insert("oacute", "ó");
    m.insert("uacute", "ú");
    m.insert("uuml", "ü");
    m.insert("ouml", "ö");
    m.insert("auml", "ä");
    m.insert("ccedil", "ç");
    m.insert("acirc", "â");
    m.insert("ecirc", "ê");
    m.insert("ocirc", "ô");
    m.insert("icirc", "î");
    m.insert("ucirc", "û");
    m.insert("szlig", "ß");
    m.insert("oslash", "ø");
    m.insert("aring", "å");
    m.insert("aelig", "æ");
    m.insert("iexcl", "¡");
    m.insert("iquest", "¿");
    m.insert("plusmn", "±");
    m.insert("frac12", "½");
    m.insert("shy", "");
    m.insert("ensp", " ");
    m.insert("emsp", " ");
    m.insert("thinsp", " ");
    m.insert("zwnj", "");
    m.insert("zwj", "");
    m.insert("deg", "°");
    m.insert("times", "×");
    m.insert("divide", "÷");
    m.insert("euro", "€");
    m.insert("pound", "£");
    m.insert("yen", "¥");
    m.insert("sect", "§");
    m.insert("para", "¶");
    m
});

fn rd_chr(n: i64) -> String {
    if (0..=0x10ffff).contains(&n)
        && let Some(c) = char::from_u32(n as u32)
    {
        return c.to_string();
    }
    String::new()
}

static RE_ENT_HEX: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)&#x([0-9a-f]+);"));
static RE_ENT_DEC: LazyLock<Regex> = LazyLock::new(|| re(r"&#(\d+);"));
static RE_ENT_NAMED: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)&([a-z][a-z0-9]+);"));

fn decode_entities_str(s: &str) -> String {
    let s = RE_ENT_HEX.replace_all(s, |caps: &regex::Captures| {
        let h = &caps[1];
        if let Ok(n) = i64::from_str_radix(h, 16) {
            rd_chr(n)
        } else {
            String::new()
        }
    });
    let s = RE_ENT_DEC.replace_all(&s, |caps: &regex::Captures| {
        let d = &caps[1];
        if let Ok(n) = d.parse::<i64>() {
            rd_chr(n)
        } else {
            String::new()
        }
    });
    let s = RE_ENT_NAMED.replace_all(&s, |caps: &regex::Captures| {
        let full = &caps[0];
        let n = caps[1].to_ascii_lowercase();
        if let Some(val) = ENT.get(n.as_str()) {
            val.to_string()
        } else {
            full.to_string()
        }
    });
    s.into_owned()
}

static RE_BR: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<br\s*/?>"));
static RE_BLOCK_CLOSE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)</(?:p|div|li|h[1-6]|blockquote|figcaption|tr|section|article)>"));
static RE_ANY_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"<[^>]*>"));
static RE_NBSP: LazyLock<Regex> = LazyLock::new(|| re(r"\u{00a0}"));
static RE_SPACES: LazyLock<Regex> = LazyLock::new(|| re(r"[ \t\x0c\x0b]+"));
static RE_LINE_SPACES: LazyLock<Regex> = LazyLock::new(|| re(r" *\n *"));
static RE_MULTI_NL: LazyLock<Regex> = LazyLock::new(|| re(r"\n{2,}"));

fn rd_clean_str(s: &str) -> String {
    let t = RE_BR.replace_all(s, "\n");
    let t = RE_BLOCK_CLOSE.replace_all(&t, "\n");
    let t = RE_ANY_TAG.replace_all(&t, " ");
    let t = decode_entities_str(&t);
    let t = RE_NBSP.replace_all(&t, " ");
    let t = RE_SPACES.replace_all(&t, " ");
    let t = RE_LINE_SPACES.replace_all(&t, "\n");
    let t = RE_MULTI_NL.replace_all(&t, "\n");
    t.trim().to_string()
}

static RE_WS_PLUS: LazyLock<Regex> = LazyLock::new(|| re(r"\s+"));

fn rd_inline_str(s: &str) -> String {
    let cleaned = rd_clean_str(s);
    RE_WS_PLUS.replace_all(&cleaned, " ").trim().to_string()
}

fn rd_host_str(u: &str) -> String {
    if let Ok(url) = Url::parse(u)
        && let Some(host) = url.host_str()
    {
        return host.strip_prefix("www.").unwrap_or(host).to_string();
    }
    String::new()
}

static RE_PROTOCOL_BLACKLIST: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(?:data:|javascript:|blob:|mailto:|tel:|about:)"));

fn rd_abs_str(u: &str, base: &str) -> String {
    if u.is_empty() {
        return String::new();
    }
    let raw = decode_entities_str(u.trim());
    let raw = raw.trim();
    if raw.is_empty() || RE_PROTOCOL_BLACKLIST.is_match(raw) {
        return String::new();
    }
    let base_url = Url::parse(base).ok();
    if let Ok(x) = Url::options().base_url(base_url.as_ref()).parse(raw) {
        let scheme = x.scheme();
        if scheme.eq_ignore_ascii_case("http") || scheme.eq_ignore_ascii_case("https") {
            return x.to_string();
        }
    }
    String::new()
}

static RE_PARSE_ATTRS_BODY_START: LazyLock<Regex> = LazyLock::new(|| re(r"^<\s*[a-zA-Z0-9:-]+"));
static RE_PARSE_ATTRS_BODY_END: LazyLock<Regex> = LazyLock::new(|| re(r"/?\s*>$"));
static RE_PARSE_ATTRS_ATTR: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*(?:=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?"#)
});

fn parse_attrs_map(tag_str: &str) -> Map<String, Value> {
    let mut o = Map::new();
    let body = RE_PARSE_ATTRS_BODY_START.replace(tag_str, "");
    let body = RE_PARSE_ATTRS_BODY_END.replace(&body, "");
    for cap in RE_PARSE_ATTRS_ATTR.captures_iter(&body) {
        let k = cap[1].to_ascii_lowercase();
        let raw_val = cap
            .get(2)
            .or_else(|| cap.get(3))
            .or_else(|| cap.get(4))
            .map(|m| m.as_str())
            .unwrap_or("");
        o.insert(k, Value::String(decode_entities_str(raw_val)));
    }
    o
}

static BOT_CHALLENGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)captcha|recaptcha|lsrecaptcha|cf-chl|cf-turnstile|challenge-platform|just a moment|one moment,?\s*please|attention required|access denied|verify you are|are you a robot|human verification|checking your browser|please wait while we verify|enable javascript and cookies|bot verification|ddos-guard|pardon our interruption|security check|sorry, you have been blocked|situs ini mungkin memblokir|requiring captcha",
    )
});

static ERROR_PAGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:404\s*(?:not\s*found|error)?|page\s*not\s*found|error\s*40[34]|403\s*forbidden|50[0234]\s*(?:bad gateway|service unavailable|gateway time-?out)?|halaman\s+tidak\s+ditemukan|artikel\s+tidak\s+ditemukan|maaf,?\s+halaman\s+yang\s+anda\s+cari\s+tidak\s+ditemukan|oops!?\s*that\s+page\s+can'?t\s+be\s+found)$",
    )
});

static RE_HTML_COMMENT: LazyLock<Regex> = LazyLock::new(|| re(r"<!--[\s\S]*?-->"));
static RE_SCRIPT_TAG: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<script\b[^>]*>[\s\S]*?</script>"));
static RE_STYLE_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<style\b[^>]*>[\s\S]*?</style>"));

fn is_challenge_html_bool(html: &str, title: &str, desc: &str) -> bool {
    let check = format!("{title} {desc}");
    if BOT_CHALLENGE_RE.is_match(&check) {
        return true;
    }
    if html.is_empty() {
        return false;
    }
    let visible = RE_HTML_COMMENT.replace_all(html, " ");
    let visible = RE_SCRIPT_TAG.replace_all(&visible, " ");
    let visible = RE_STYLE_TAG.replace_all(&visible, " ");
    if visible.len() < 45000 && BOT_CHALLENGE_RE.is_match(&visible) {
        return true;
    }
    let text = RE_ANY_TAG.replace_all(&visible, " ");
    let text = RE_WS_PLUS.replace_all(&text, " ");
    let text = text.trim();
    text.len() < 160 && BOT_CHALLENGE_RE.is_match(html)
}

static RE_META_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<meta\b[^>]*>"));
static RE_INVALID_META_VAL: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(undefined|null|nan|\[object object\])$"));

fn rd_meta_map(html: &str) -> Map<String, Value> {
    let mut out = Map::new();
    for m in RE_META_TAG.find_iter(html) {
        let a = parse_attrs_map(m.as_str());
        let k = a
            .get("property")
            .or_else(|| a.get("name"))
            .or_else(|| a.get("itemprop"))
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        if k.is_empty() {
            continue;
        }
        let content = a.get("content").and_then(|v| v.as_str()).unwrap_or("");
        if content.is_empty() {
            continue;
        }
        let val = rd_inline_str(content);
        if !val.is_empty() && !RE_INVALID_META_VAL.is_match(&val) && !out.contains_key(&k) {
            out.insert(k, Value::String(val));
        }
    }
    out
}

static RE_JSON_LD: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)<script[^>]*type=["']application/ld\+json["'][^>]*>([\s\S]*?)</script>"#)
});
static RE_CDATA: LazyLock<Regex> = LazyLock::new(|| re(r"^\s*<!\[CDATA\[|\]\]>\s*$"));

fn rd_ld_vec(html: &str) -> Vec<Value> {
    let mut out = Vec::new();
    for cap in RE_JSON_LD.captures_iter(html) {
        let raw = RE_CDATA.replace_all(&cap[1], "");
        let raw = raw.trim();
        if raw.is_empty() {
            continue;
        }
        if let Ok(j) = serde_json::from_str::<Value>(raw) {
            let mut stack = VecDeque::new();
            if let Value::Array(arr) = j {
                stack.extend(arr);
            } else {
                stack.push_back(j);
            }
            while let Some(n) = stack.pop_front() {
                match n {
                    Value::Array(arr) => {
                        for item in arr.into_iter().rev() {
                            stack.push_front(item);
                        }
                    }
                    Value::Object(ref obj) => {
                        out.push(n.clone());
                        if let Some(Value::Array(graph)) = obj.get("@graph") {
                            stack.extend(graph.clone());
                        }
                        if let Some(me @ Value::Object(_)) = obj.get("mainEntity") {
                            stack.push_back(me.clone());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    out
}

fn rd_name_val(v: &Value) -> String {
    if !is_truthy(v) {
        return String::new();
    }
    match v {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            if let Some(first) = arr.first() {
                rd_name_val(first)
            } else {
                String::new()
            }
        }
        Value::Object(obj) => {
            if let Some(name) = obj.get("name").filter(|x| is_truthy(x)) {
                js_to_string(name)
            } else if let Some(hl) = obj.get("headline").filter(|x| is_truthy(x)) {
                js_to_string(hl)
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}

static RE_SLASH_WS_END: LazyLock<Regex> = LazyLock::new(|| re(r"/\s*$"));

fn find_matching_close_bytes(
    html: &str,
    open_end_idx: usize,
    tag_name: &str,
) -> Option<(usize, usize)> {
    if open_end_idx >= html.len() {
        return None;
    }
    let pat = format!(r"(?i)<(/?)({tag_name})(?:\s*|([\s/][^>]*))>");
    let regex = Regex::new(&pat).ok()?;
    let mut depth = 1usize;
    for m in regex.find_iter(&html[open_end_idx..]) {
        let abs_start = open_end_idx + m.start();
        let abs_end = open_end_idx + m.end();
        let match_str = m.as_str();
        let is_close = match_str.starts_with("</") || match_str.starts_with("< /");
        let self_close =
            !is_close && RE_SLASH_WS_END.is_match(&match_str[..match_str.len().saturating_sub(1)]);
        if self_close {
            continue;
        }
        if is_close {
            depth -= 1;
            if depth == 0 {
                return Some((abs_start, abs_end));
            }
        } else {
            depth += 1;
        }
    }
    None
}

static RE_NOSCRIPT_IMAGES: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<noscript\b[^>]*>([\s\S]*?)</noscript\s*>"));
static RE_IMG_AMP: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<(?:img|amp-img)\b"));
static RE_H1: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<h1\b"));
static RE_FIGURE_IMG: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<(?:figure|img|amp-img)\b"));
static RE_NAV: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<nav\b"));

fn strip_elements_by_tag_impl(
    s: &str,
    tags: &[String],
    unwrap_noscript: bool,
    preserve_header_figures: bool,
) -> String {
    let mut s_curr = s.to_string();
    if unwrap_noscript {
        s_curr = RE_NOSCRIPT_IMAGES
            .replace_all(&s_curr, |caps: &regex::Captures| {
                let inner = &caps[1];
                if RE_IMG_AMP.is_match(inner) {
                    inner.to_string()
                } else {
                    " ".to_string()
                }
            })
            .into_owned();
    }
    if tags.is_empty() {
        return s_curr;
    }
    let tag_set = tags.join("|");
    let pat = format!(r"(?i)<({tag_set})\b([^>]*)>");
    let Ok(open_re) = Regex::new(&pat) else {
        return s_curr;
    };

    let mut out = String::with_capacity(s_curr.len());
    let mut last = 0;

    let mut search_from = 0;
    while let Some(m) = open_re.find(&s_curr[search_from..]) {
        let start = search_from + m.start();
        let open_end = search_from + m.end();
        let caps = match open_re.captures(&s_curr[start..open_end]) {
            Some(c) => c,
            None => {
                search_from = open_end;
                continue;
            }
        };
        let tag = caps[1].to_ascii_lowercase();
        let attr_str = caps.get(2).map(|x| x.as_str()).unwrap_or("");

        if RE_SLASH_WS_END.is_match(attr_str) {
            out.push_str(&s_curr[last..start]);
            out.push(' ');
            last = open_end;
            search_from = open_end;
            continue;
        }

        if let Some((inner_end, outer_end)) = find_matching_close_bytes(&s_curr, open_end, &tag) {
            if tag == "header" && preserve_header_figures {
                let inner = &s_curr[open_end..inner_end];
                if RE_H1.is_match(inner) && RE_FIGURE_IMG.is_match(inner) && !RE_NAV.is_match(inner)
                {
                    search_from = open_end;
                    continue;
                }
            }
            out.push_str(&s_curr[last..start]);
            out.push(' ');
            last = outer_end;
            search_from = outer_end;
        } else {
            search_from = open_end;
        }
    }
    out.push_str(&s_curr[last..]);
    out
}

static ARTICLE_BODY_KEEP_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?:detail-desc|article-content|article-body|content-detail|detail-content|post-content|entry-content|berita-content|field-name-body|txt-berita|detail_body|news-content|content-body|post-body|article__body|detail__body-text|itp_bodycontent|detail__body|txt-article|detail-text|article-content-body|read__content|read__article|story-body|artikel-isi|isi-berita)",
    )
});

static JUNK_BOX_TOKEN_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?:^|[\s_-])(?:reactions?|rating|emoticons?|emoji|votes?|polls?|like-box|dislike|social-share|article-tools|post-tools|share-tools|share-box|share-bar|share-buttons?|follow-us|komentar|comments?(?:-box|-list|-section|-area)?|disqus|author-(?:box|card|img|image|photo|profile|info|bio|wrap|detail)|wartawan|penulis|byline|redaksi|editor-box|profile-(?:box|img|image)|related(?:-(?:news|articles?|posts?|stories|box|wrap|list|section))?|baca-juga|baca_juga|bacajuga|berita-terkait|artikel-terkait|linksisip|sisip|recommend(?:ed|ations?)?|most-read|most-popular|injected|widget|latest-(?:news|post|stories)|popular(?:-(?:news|post|box))?|trending|berita-lain|more-(?:news|stories)|also-read|you-may|jnews_|td-related|jeg_(?:block|sidebar)|advert(?:isement|orial)?|iklan|ad-(?:slot|box|unit|banner|container|wrap)|ads-(?:box|slot|wrap|container)|sponsor(?:ed)?|stripbanner|ticker|breaking-news|see-also|summary-article|read-page--summary|inbetween|banner_article_slide|newsletter|subscribe(?:-box|-form)?|cookie(?:-banner|-notice|-consent|-popup)?|gdpr|breadcrumb|pagination|page-nav|tag-list|tags-box|topik-terkait|sr-only|visually-hidden|screen-reader|assistive)(?:$|[\s_-])",
    )
});

static SIDEBAR_NEG_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)(?:no|without|has|with)-sidebar|sidebar-layout"));
static RE_SIDEBAR: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(?:^|[\s_-])sidebar(?:$|[\s_-])"));
static RE_BANNER_PROMO: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)(?:^|[\s_-])(?:banner|promo)(?:$|[\s_-])"));
static RE_BANNER_NEG: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)(?:hero|lead|article|main|cover)"));
static RE_HIDDEN_ATTR: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)\shidden(?:\s|=|$)|aria-hidden\s*=\s*["']true["']"#));

fn is_junk_container_attr(attr_str: &str) -> bool {
    if RE_HIDDEN_ATTR.is_match(attr_str) {
        return true;
    }
    let fake_tag = format!("<div {attr_str}>");
    let a = parse_attrs_map(&fake_tag);
    let mut parts = Vec::new();
    for k in [
        "class",
        "id",
        "role",
        "data-testid",
        "aria-label",
        "data-component",
    ] {
        if let Some(v) = a.get(k).and_then(|x| x.as_str()).filter(|s| !s.is_empty()) {
            parts.push(v);
        }
    }
    let combined = parts.join(" ");
    if combined.is_empty() {
        return false;
    }
    if ARTICLE_BODY_KEEP_RE.is_match(&combined) {
        return false;
    }
    if JUNK_BOX_TOKEN_RE.is_match(&combined) {
        return true;
    }
    if RE_SIDEBAR.is_match(&combined) && !SIDEBAR_NEG_RE.is_match(&combined) {
        return true;
    }
    if RE_BANNER_PROMO.is_match(&combined) && !RE_BANNER_NEG.is_match(&combined) {
        return true;
    }
    false
}

static RE_JUNK_CONTAINER_OPEN: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<(div|span|p|section|ul|ol|table|figure|aside|dl)\b([^>]*)>"));

fn strip_junk_containers_str(html: &str) -> String {
    let mut s = html.to_string();
    for _pass in 0..3 {
        let mut out = String::with_capacity(s.len());
        let mut last = 0;
        let mut changed = false;
        let mut search_from = 0;

        while let Some(m) = RE_JUNK_CONTAINER_OPEN.find(&s[search_from..]) {
            let start = search_from + m.start();
            let open_end = search_from + m.end();
            let caps = match RE_JUNK_CONTAINER_OPEN.captures(&s[start..open_end]) {
                Some(c) => c,
                None => {
                    search_from = open_end;
                    continue;
                }
            };
            let tag = caps[1].to_ascii_lowercase();
            let attr_str = caps.get(2).map(|x| x.as_str()).unwrap_or("");

            if !is_junk_container_attr(attr_str) {
                search_from = open_end;
                continue;
            }

            if RE_SLASH_WS_END.is_match(attr_str) {
                out.push_str(&s[last..start]);
                out.push(' ');
                last = open_end;
                search_from = open_end;
                changed = true;
                continue;
            }

            if let Some((_inner_end, outer_end)) = find_matching_close_bytes(&s, open_end, &tag) {
                if (outer_end - start) as f64 > (s.len() as f64) * 0.75 {
                    search_from = open_end;
                    continue;
                }
                out.push_str(&s[last..start]);
                out.push(' ');
                last = outer_end;
                search_from = outer_end;
                changed = true;
            } else {
                search_from = open_end;
            }
        }
        out.push_str(&s[last..]);
        s = out;
        if !changed {
            break;
        }
    }
    s
}

static RE_A_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<a\b([^>]*)>([\s\S]*?)</a\s*>"));
static RE_HREF_ATTR: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)href\s*=\s*["']([^"']+)"#));
static RE_IMG_EXT: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\.(?:jpe?g|png|webp|gif|avif)(\?|#|$)"));
static RE_H1_6: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<h[1-6]\b"));

fn strip_card_links_str(s: &str, base: &str) -> String {
    let mut base_path = String::new();
    if let Ok(u) = Url::parse(base) {
        base_path = u.path().to_string();
    }
    RE_A_TAG
        .replace_all(s, |caps: &regex::Captures| {
            let full = &caps[0];
            let at = &caps[1];
            let inner = &caps[2];
            if !RE_IMG_AMP.is_match(inner) {
                return full.to_string();
            }
            let href = RE_HREF_ATTR
                .captures(at)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");
            if href.is_empty() || href.starts_with('#') || RE_IMG_EXT.is_match(href) {
                return full.to_string();
            }
            let txt = RE_ANY_TAG.replace_all(inner, " ");
            let txt = RE_WS_PLUS.replace_all(&txt, " ");
            let txt = txt.trim();
            if RE_H1_6.is_match(inner) || txt.len() > 25 {
                return " ".to_string();
            }
            let base_url = Url::parse(base).ok();
            if let Ok(h) = Url::options().base_url(base_url.as_ref()).parse(href)
                && h.path().len() > 8
                && h.path() != base_path
            {
                return " ".to_string();
            }
            full.to_string()
        })
        .into_owned()
}

static RE_STRIP_LINK_BLOCKS_HEADER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)<(h[1-6]|span|strong|b|p|em|i|label)\b[^>]*>\s*(?:baca|simak|lihat|baca juga|simak juga|lihat juga|selengkapnya|berita terkait|artikel terkait)\s*:?\s*</[a-zA-Z0-9]+\s*>",
    )
});
static RE_BLOCK_CONTAINER_OPEN: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<(p|li|h[1-6]|div|span|td)\b([^>]*)>"));
static RE_BLOCK_STOPPERS: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)</?(?:p|li|div|ul|ol|h[1-6]|section|article|table|tr)\b"));
static RE_HAS_A_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<a\b"));

fn strip_link_blocks_str(s: &str) -> String {
    let out = RE_STRIP_LINK_BLOCKS_HEADER.replace_all(s, " ").into_owned();

    // Parse non-nested element blocks with tags in (p|li|h[1-6]|div|span|td)
    let mut result = String::with_capacity(out.len());
    let mut last = 0;
    let mut search_from = 0;

    while let Some(m) = RE_BLOCK_CONTAINER_OPEN.find(&out[search_from..]) {
        let start = search_from + m.start();
        let open_end = search_from + m.end();
        let caps = match RE_BLOCK_CONTAINER_OPEN.captures(&out[start..open_end]) {
            Some(c) => c,
            None => {
                search_from = open_end;
                continue;
            }
        };
        let tag = caps[1].to_ascii_lowercase();
        let attr_str = caps.get(2).map(|x| x.as_str()).unwrap_or("");
        if RE_SLASH_WS_END.is_match(attr_str) {
            search_from = open_end;
            continue;
        }

        if let Some((inner_end, outer_end)) = find_matching_close_bytes(&out, open_end, &tag) {
            let inner = &out[open_end..inner_end];
            if !RE_BLOCK_STOPPERS.is_match(inner) && RE_HAS_A_TAG.is_match(inner) {
                let plain = RE_ANY_TAG.replace_all(inner, " ");
                let plain = RE_WS_PLUS.replace_all(&plain, " ");
                let plain = plain.trim();
                if !plain.is_empty() && plain.len() <= 240 {
                    let mut linked = 0usize;
                    for a_cap in RE_A_TAG.captures_iter(inner) {
                        let a_inner = &a_cap[2];
                        let a_txt = RE_ANY_TAG.replace_all(a_inner, " ");
                        let a_txt = RE_WS_PLUS.replace_all(&a_txt, " ");
                        linked += a_txt.trim().len();
                    }
                    if (linked as f64) / (plain.len() as f64) >= 0.8 {
                        result.push_str(&out[last..start]);
                        result.push(' ');
                        last = outer_end;
                        search_from = outer_end;
                        continue;
                    }
                }
            }
            search_from = open_end;
        } else {
            search_from = open_end;
        }
    }
    result.push_str(&out[last..]);
    result
}

static RD_CONT_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)<(article|div|section|main)\b[^>]*(?:(?:class|id)\s*=\s*["'][^"']*(?:detail-desc|article-content|article-body|content-detail|detail-content|post-content|entry-content|berita-content|field-name-body|txt-berita|detail_body|news-content|content-body|post-body|article__body|detail__body-text|itp_bodycontent|detail__body|txt-article|side-article|detail-text|article-content-body|read__content|read__article|story-body|artikel-isi|isi-berita)[^"']*["']|itemprop\s*=\s*["']articleBody["'])[^>]*>"#,
    )
});

static RD_LEAD_MEDIA_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)<(div|figure|section)\b[^>]*(?:class|id)\s*=\s*["'][^"']*(?:detail__media|detail-image|photo__wrap|photo-cover|featured-image|post-thumbnail|article-photo|cover-photo|img-article|media-cover|max-card__image|artimg)[^"']*["'][^>]*>"#,
    )
});

static RD_LEAD_IMG_TAG_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r#"(?i)<img\b[^>]*(?:class|id)\s*=\s*["'][^"']*(?:featured[-_]?img|featured[-_]?image|article[-_]?featured|post[-_]?thumbnail|hero[-_]?img|detail[-_]?img|cover[-_]?img|wp-post-image)[^"']*["'][^>]*>"#,
    )
});

static RE_ARTICLE_OPEN: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<(article)\b[^>]*>"));
static RE_MAIN_OPEN: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<(main)\b[^>]*>"));

struct MatchedElement {
    html: String,
    full: String,
    index: usize,
}

fn extract_matched_elements(html: &str, open_re: &Regex) -> Vec<MatchedElement> {
    let mut out = Vec::new();
    let mut search_from = 0;
    while let Some(m) = open_re.find(&html[search_from..]) {
        let start = search_from + m.start();
        let open_end = search_from + m.end();
        let caps = match open_re.captures(&html[start..open_end]) {
            Some(c) => c,
            None => {
                search_from = open_end;
                continue;
            }
        };
        let tag = caps[1].to_ascii_lowercase();
        if let Some((inner_end, outer_end)) = find_matching_close_bytes(html, open_end, &tag) {
            let u16_idx = crate::utf16::utf16_offset(html, start);
            out.push(MatchedElement {
                html: html[open_end..inner_end].to_string(),
                full: html[start..outer_end].to_string(),
                index: u16_idx,
            });
        }
        search_from = open_end;
    }
    out
}

static STRIP_TAGS_BODY: &[&str] = &[
    "script", "style", "svg", "form", "nav", "aside", "footer", "header", "button", "select",
    "textarea", "template", "dialog", "menu",
];
static STRIP_TAGS_LEAD: &[&str] = &["script", "style", "svg", "form", "nav", "aside", "footer"];

fn extract_containers_map(html: &str) -> (Vec<Value>, Vec<Value>) {
    let mut body_containers = Vec::new();
    let mut lead_media_containers = Vec::new();

    for el in extract_matched_elements(html, &RD_CONT_RE) {
        body_containers.push(json!({
            "kind": "selector",
            "html": el.html,
            "index": el.index,
        }));
    }
    for el in extract_matched_elements(html, &RE_ARTICLE_OPEN) {
        body_containers.push(json!({
            "kind": "article",
            "html": el.html,
            "index": el.index,
        }));
    }
    for el in extract_matched_elements(html, &RE_MAIN_OPEN) {
        body_containers.push(json!({
            "kind": "main",
            "html": el.html,
            "index": el.index,
        }));
    }
    for el in extract_matched_elements(html, &RD_LEAD_MEDIA_RE) {
        lead_media_containers.push(Value::String(el.full));
    }
    let lead_tags = STRIP_TAGS_LEAD
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let lead_scan = strip_elements_by_tag_impl(html, &lead_tags, false, false);
    for m in RD_LEAD_IMG_TAG_RE.find_iter(&lead_scan) {
        lead_media_containers.push(Value::String(m.as_str().to_string()));
    }
    let body_tags = STRIP_TAGS_BODY
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    let cleaned_full = strip_elements_by_tag_impl(html, &body_tags, true, false);
    body_containers.push(json!({
        "kind": "full",
        "html": cleaned_full,
        "index": 0,
    }));

    (body_containers, lead_media_containers)
}

static VID_IFRAME: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?:youtube\.com/(?:embed|shorts)/|youtube-nocookie\.com/(?:embed|shorts)/|youtu\.be/|player\.vimeo\.com/|facebook\.com/plugins/video|/embed/|/player/|/video-embed/)",
    )
});
static VID_FILE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\.(mp4|webm|mov|m4v|m3u8|mpd)(\?|#|$)"));
static RE_YT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?:youtube\.com/(?:embed|shorts)/|youtu\.be/|youtube-nocookie\.com/(?:embed|shorts)/)([A-Za-z0-9_-]{6,15})",
    )
});

fn rd_yt_str(u: &str) -> String {
    if let Some(caps) = RE_YT.captures(u) {
        format!("https://www.youtube.com/embed/{}", &caps[1])
    } else {
        String::new()
    }
}

static RE_IFRAME_START: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^<iframe"));
static RE_SRC_ATTR: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\bsrc\s*=\s*["']([^"']+)"#));
static RE_SOURCE_TAG_SRC: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)<source\b[^>]*\bsrc\s*=\s*["']([^"']+)"#));
static RE_POSTER_ATTR: LazyLock<Regex> = LazyLock::new(|| re(r#"(?i)\bposter\s*=\s*["']([^"']+)"#));

fn rd_video_el(tag: &str, final_url: &str) -> Option<Value> {
    if RE_IFRAME_START.is_match(tag) {
        let a = parse_attrs_map(tag);
        let src = a.get("src").and_then(|v| v.as_str()).unwrap_or("");
        let u = rd_abs_str(src, final_url);
        if u.is_empty() {
            return None;
        }
        let yt = rd_yt_str(&u);
        if !yt.is_empty() {
            return Some(json!({ "url": yt, "kind": "iframe", "poster": "" }));
        }
        if VID_IFRAME.is_match(&u) && !VID_FILE.is_match(&u) {
            return Some(json!({ "url": u, "kind": "iframe", "poster": "" }));
        }
        return None;
    }
    let src = RE_SRC_ATTR
        .captures(tag)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .or_else(|| {
            RE_SOURCE_TAG_SRC
                .captures(tag)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
        })
        .unwrap_or("");
    let u = rd_abs_str(src, final_url);
    if u.is_empty() {
        return None;
    }
    let poster = RE_POSTER_ATTR
        .captures(tag)
        .and_then(|c| c.get(1))
        .map(|m| m.as_str())
        .unwrap_or("");
    Some(json!({ "url": u, "kind": "video", "poster": rd_abs_str(poster, final_url) }))
}

static RE_DUMMY_VIDEO: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)/(?:uploads/video|videos?|media)/?$"));

fn is_dummy_video_url(u: &str) -> bool {
    let s = u
        .split('?')
        .next()
        .unwrap_or("")
        .split('#')
        .next()
        .unwrap_or("");
    RE_DUMMY_VIDEO.is_match(s) && !VID_FILE.is_match(u) && !VID_IFRAME.is_match(u)
}

static RE_YT_ID_MATCH: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)["']?youtube_id["']?\s*:\s*["']([A-Za-z0-9_-]{6,15})["']"#));
static RE_NEWS_URL: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)["']?(?:news_url|url)["']?\s*:\s*["']([^"']+)["']"#));
static RE_NEWS_IMG: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)["']?(?:news_image|poster|thumbnail)["']?\s*:\s*["']([^"']+)["']"#));
static RE_TITLE_KEY: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)["']?title["']?\s*:\s*["']([^"']+)["']"#));
static RE_CONTENT_BT: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)["']?content["']?\s*:\s*`([\s\S]*?)`"#));
static RE_CONTENT_QT: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)["']?content["']?\s*:\s*(?:"((?:\\.|[^"\\])*)"|'((?:\\.|[^'\\])*)')"#)
});
static RE_HASHTAGS: LazyLock<Regex> = LazyLock::new(|| re(r"#[A-Za-z0-9_]+"));
static RE_META_ITEMPROP_EMBED: LazyLock<Regex> = LazyLock::new(|| {
    re(r#"(?i)<meta[^>]+itemprop\s*=\s*["']embedURL["'][^>]+content\s*=\s*["']([^"']+)["']"#)
});
static RE_EMBED_URL_JSON: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)"embedUrl"\s*:\s*"(https?:\\?/\\?/[^"]+)""#));

struct ScriptVideosResult {
    videos: Vec<Value>,
    script_body: String,
}

#[allow(dead_code)]
struct YtEntry {
    yt_id: String,
    n_url: String,
    n_img: String,
    v_title: String,
    content: String,
    slug_hit: bool,
}

fn rd_extract_script_videos_impl(s_html: &str, final_url: &str, title: &str) -> ScriptVideosResult {
    let mut vids = Vec::new();
    let mut script_body = String::new();
    let mut slug_tokens = Vec::new();
    if let Ok(u) = Url::parse(final_url) {
        slug_tokens = u
            .path()
            .split('/')
            .filter(|p| !p.is_empty() && p.len() >= 6)
            .map(|s| s.to_string())
            .collect();
    }
    let yt_slug = slug_tokens
        .iter()
        .find(|p| p.to_ascii_lowercase().starts_with("yt-"))
        .or_else(|| slug_tokens.last())
        .map(|s| s.as_str())
        .unwrap_or("");

    let mut yt_entries = Vec::new();
    for cap in RE_YT_ID_MATCH.captures_iter(s_html) {
        let m = cap.get(0).unwrap();
        let yt_id = cap[1].to_string();
        let m_idx = m.start();
        let start = s_html[..m_idx].rfind('{').unwrap_or(0);
        let mut end = s_html[m_idx..].find("},").map(|i| m_idx + i);
        let alt_end = s_html[m_idx..].find("}];").map(|i| m_idx + i);
        if let Some(ae) = alt_end
            && (end.is_none() || ae < end.unwrap())
        {
            end = Some(ae);
        }
        let end_idx = match end {
            Some(e) if e - start <= 12000 => e,
            _ => (m_idx + 5000).min(s_html.len()),
        };
        let obj = &s_html[start..end_idx];
        let n_url = RE_NEWS_URL
            .captures(obj)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let n_img = RE_NEWS_IMG
            .captures(obj)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim().to_string())
            .unwrap_or_default();
        let raw_v_title = RE_TITLE_KEY
            .captures(obj)
            .and_then(|c| c.get(1))
            .map(|m| m.as_str().trim())
            .unwrap_or("");
        let v_title = rd_clean_str(raw_v_title);
        let mut content = String::new();
        if let Some(bt) = RE_CONTENT_BT.captures(obj) {
            content = bt[1].to_string();
        } else if let Some(qt) = RE_CONTENT_QT.captures(obj) {
            let inner = qt
                .get(1)
                .or_else(|| qt.get(2))
                .map(|m| m.as_str())
                .unwrap_or("");
            content = inner.replace(r"\n", "\n").replace(r#"\""#, "\"");
        }
        let slug_hit = !yt_slug.is_empty()
            && !n_url.is_empty()
            && n_url
                .to_ascii_lowercase()
                .contains(&yt_slug.to_ascii_lowercase());
        yt_entries.push(YtEntry {
            yt_id,
            n_url,
            n_img,
            v_title,
            content,
            slug_hit,
        });
    }

    if !yt_entries.is_empty() {
        let chosen = yt_entries
            .iter()
            .find(|e| e.slug_hit)
            .or_else(|| {
                if yt_entries.len() == 1 {
                    yt_entries.first()
                } else {
                    None
                }
            })
            .or_else(|| {
                yt_entries.iter().find(|e| {
                    !title.is_empty()
                        && !e.v_title.is_empty()
                        && rd_on_topic_bool(title, "", &[&e.v_title])
                })
            });
        if let Some(c) = chosen {
            vids.push(json!({
                "url": format!("https://www.youtube.com/embed/{}", c.yt_id),
                "kind": "iframe",
                "poster": rd_abs_str(&c.n_img, final_url),
            }));
            if c.content.trim().len() >= 40 {
                let cleaned = RE_HASHTAGS.replace_all(&c.content, "");
                script_body = cleaned.trim().to_string();
            }
        }
    }

    for cap in RE_META_ITEMPROP_EMBED.captures_iter(s_html) {
        let u = rd_abs_str(&cap[1], final_url);
        if !u.is_empty() {
            let yt = rd_yt_str(&u);
            if !yt.is_empty() {
                vids.push(json!({ "url": yt, "kind": "iframe", "poster": "" }));
            } else if VID_IFRAME.is_match(&u) {
                vids.push(json!({ "url": u, "kind": "iframe", "poster": "" }));
            }
        }
    }

    for cap in RE_EMBED_URL_JSON.captures_iter(s_html) {
        let raw = cap[1].replace(r"\/", "/");
        let u = rd_abs_str(&raw, final_url);
        if !u.is_empty() {
            let yt = rd_yt_str(&u);
            if !yt.is_empty() {
                vids.push(json!({ "url": yt, "kind": "iframe", "poster": "" }));
            } else if VID_IFRAME.is_match(&u) {
                vids.push(json!({ "url": u, "kind": "iframe", "poster": "" }));
            }
        }
    }

    ScriptVideosResult {
        videos: vids,
        script_body,
    }
}

static RE_MEDIA_KIND_VIDEO: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)videoplayback|googlevideo|\.m3u8"));
static RE_MEDIA_KIND_IFRAME_NEG: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)/(?:berita|news|article|read)/"));

fn media_kind(u: &str) -> &'static str {
    if VID_FILE.is_match(u) || RE_MEDIA_KIND_VIDEO.is_match(u) {
        "video"
    } else if VID_IFRAME.is_match(u) && !RE_MEDIA_KIND_IFRAME_NEG.is_match(u) {
        "iframe"
    } else {
        "probe"
    }
}

static RE_VIDEO_TAG: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<video\b[^>]*>([\s\S]*?)</video>"));
static RE_SRC_ATTR_DOUBLE_OR_SINGLE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)\bsrc=["']([^"']+)"#));
static RE_POSTER_ATTR_DOUBLE_OR_SINGLE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)\bposter=["']([^"']+)"#));
static RE_SOURCE_TAG_SRC_DOUBLE_OR_SINGLE: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)<source\b[^>]*\bsrc=["']([^"']+)"#));
static RE_IFRAME_SRC_TAG: LazyLock<Regex> =
    LazyLock::new(|| re(r#"(?i)<iframe\b[^>]*src=["']([^"']+)["']"#));

fn rd_videos_vec(html: &str, final_url: &str, dom: bool, title: &str) -> Vec<Value> {
    let mut out = Vec::new();
    let mut seen = HashSet::new();

    let mut add = |mut u: String, mut kind: String, poster: &str| {
        u = rd_abs_str(&u, final_url);
        if u.is_empty() || is_dummy_video_url(&u) {
            return;
        }
        let yt = rd_yt_str(&u);
        if !yt.is_empty() {
            u = yt;
            kind = "iframe".to_string();
        }
        let k = format!("{}|{}", u.split('?').next().unwrap_or(""), kind);
        if seen.contains(&k) {
            return;
        }
        seen.insert(k);
        out.push(json!({
            "url": u,
            "kind": kind,
            "poster": rd_abs_str(poster, final_url),
        }));
    };

    let meta = rd_meta_map(html);
    for k in [
        "og:video",
        "og:video:url",
        "og:video:secure_url",
        "og:video:mp4",
        "twitter:player:stream",
    ] {
        if let Some(u_val) = meta.get(k).and_then(|v| v.as_str()) {
            let u = rd_abs_str(u_val, final_url);
            if !u.is_empty() {
                add(u.clone(), media_kind(&u).to_string(), "");
            }
        }
    }
    if let Some(tp_val) = meta.get("twitter:player").and_then(|v| v.as_str()) {
        let tplayer = rd_abs_str(tp_val, final_url);
        if !tplayer.is_empty() && VID_IFRAME.is_match(&tplayer) {
            add(tplayer, "iframe".to_string(), "");
        }
    }

    if dom {
        for cap in RE_VIDEO_TAG.captures_iter(html) {
            let tag = &cap[0];
            let inner = &cap[1];
            let src = RE_SRC_ATTR_DOUBLE_OR_SINGLE
                .captures(tag)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .or_else(|| {
                    RE_SOURCE_TAG_SRC_DOUBLE_OR_SINGLE
                        .captures(inner)
                        .and_then(|c| c.get(1))
                        .map(|m| m.as_str())
                })
                .unwrap_or("");
            let poster = RE_POSTER_ATTR_DOUBLE_OR_SINGLE
                .captures(tag)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");
            add(src.to_string(), "video".to_string(), poster);
        }
        for cap in RE_IFRAME_SRC_TAG.captures_iter(html) {
            let u = rd_abs_str(&cap[1], final_url);
            let yt = rd_yt_str(&u);
            if !yt.is_empty() {
                add(yt, "iframe".to_string(), "");
                continue;
            }
            if VID_IFRAME.is_match(&u) && !VID_FILE.is_match(&u) {
                add(u, "iframe".to_string(), "");
            }
        }
    }

    for n in rd_ld_vec(html) {
        let n_type = n.get("@type").and_then(|v| v.as_str()).unwrap_or("");
        if !n_type.to_ascii_lowercase().contains("video") {
            continue;
        }
        let v_title = format!(
            "{} {}",
            n.get("name").and_then(|v| v.as_str()).unwrap_or(""),
            n.get("description").and_then(|v| v.as_str()).unwrap_or("")
        );
        if !title.is_empty()
            && !v_title.trim().is_empty()
            && !rd_on_topic_bool(title, "", &[&v_title])
        {
            continue;
        }
        let u = n
            .get("contentUrl")
            .or_else(|| n.get("embedUrl"))
            .and_then(|v| v.as_str())
            .map(|s| rd_abs_str(s, final_url))
            .unwrap_or_default();
        if !u.is_empty() {
            let thumb = n.get("thumbnailUrl").and_then(|v| v.as_str()).unwrap_or("");
            add(u.clone(), media_kind(&u).to_string(), thumb);
        }
    }

    let script_res = rd_extract_script_videos_impl(html, final_url, title);
    for sv in script_res.videos {
        let u = sv
            .get("url")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let k = sv
            .get("kind")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        let p = sv.get("poster").and_then(|v| v.as_str()).unwrap_or("");
        add(u, k, p);
    }

    out.into_iter().take(4).collect()
}

static RD_SKIP: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:baca juga|simak juga|lihat juga|berita terkait|artikel terkait|advertisement|iklan|scroll(?:\s+untuk|\s+to)?|lanjutkan(?:\s+membaca)?|ikuti(?:\s+kami|\s+saluran|\s+channel|\s+whatsapp)?|bagikan|komentar|rekomendasi|topik(?:\s+terkait)?|tags?|sumber|foto|video|infografis|follow|newsletter|subscribe|langganan|whatsapp\s+channel|halaman\s+selanjutnya|artikel\s+selanjutnya)\b",
    )
});
static RD_CREDIT: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\b(?:foto|istimewa|dok\.?|ilustrasi)\s*:"));
static RD_BOILER: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)baca berita terkini|berita terkini dan terpercaya|dapatkan (?:berita|update|info)|untuk mendapatkan berita|follow (?:channel|akun|kami)|ikuti (?:kami|saluran|channel|akun)|gabung (?:di )?(?:channel|grup|saluran)|lorem ipsum|all rights reserved|hak cipta|download aplikasi|unduh aplikasi|informasi seputar .{0,60}(?:bisa|dapat) diakses|bergabunglah|klik (?:di sini|link)|whatsapp channel|google news|dilarang (?:keras )?(?:mengambil|menyalin|mengutip|mengcopy)|crawling|pengindeksan otomatis|download sekarang|menyajikan berita terhangat|tinggalkan komentar|^©|^copyright|unduh .{0,30}app|install aplikasi|dapatkan notifikasi|aktifkan notifikasi|laporkan (?:iklan|konten)|unblock notifications|real[- ]time updates|know more|sign up for|subscribe to|berlangganan (?:untuk|newsletter|pos|email)|sponsored|advertisement|sumber (?:utama|pilihan|terpercaya) di google|preferred source|tambahkan .{0,40}sumber|tas darurat.{0,3}ada apa|isi komentar|tanggung jawab (?:pengguna|penulis komentar)|promo rp|jajan pakai|diskon hingga|beli sekarang|daftar sekarang|^(?:sumber|source)\s*:|^(?:editor|penulis|reporter|redaktur|kontributor|pewarta|fotografer)\s*:|(?:window|modal|dialog|jendela|log)\s*(?:ini|log)?[^.\n]{0,50}(?:ditutup|close)|(?:tekan|press|aktifkan|activate)[^.\n]{0,40}(?:escape|\besc\b|tombol tutup|close button|tombol close)|dapat ditutup dengan|situs ini menggunakan cookie|we use cookies|cookie policy|kebijakan privasi|terima semua cookie|accept all cookies",
    )
});
static RD_END: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:baca (?:juga|berita|selengkapnya)|simak (?:juga|berita)|berita (?:terkait|lainnya|terpopuler|populer|rekomendasi)|artikel (?:terkait|lainnya|populer|selanjutnya)|terpopuler|rekomendasi(?: untuk anda)?|tag:|topik:|related (?:news|articles?|posts?)|read (?:more|also)|you (?:may|might) (?:also )?like|karin adalah|apresiasi spesial|gabung membership|kamu bisa bertanya tentang|kontribusi (?:ini|yang) |tambahkan .{0,40}(?:sebagai|jadi) sumber|syarat dan ketentuan|disclaimer\b|komentar\s*(?:\(\d+\))?$|tulis komentar)",
    )
});

static RE_CAPTION1: LazyLock<Regex> = LazyLock::new(|| re(r"^[A-Z][A-Z0-9 ,'.&-]{5,45}\s[-–:]\s"));
static RE_CAPTION2: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\([^()]*(?:/|commons|photo|credit|foto)[^()]*\)\.?$"));
static RE_CAPTION3: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)^(?:ilustrasi|foto ilustrasi|foto|dok\.?|dokumentasi|gambar|suasana|caption)\s*[:\-–]")
});
static RE_CAPTION4: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)\((?:foto|antara|dok|sumber|istimewa|reuters|afp|ap|getty)[^)]*\)?\.?$")
});
static RE_CAPTION5: LazyLock<Regex> = LazyLock::new(|| re(r"^[\w.-]+\.[a-z]{2,}(?:/\S*)?…?$"));

fn rd_captionish(t: &str) -> bool {
    t.len() < 220
        && (RE_CAPTION1.is_match(t)
            || (t.len() < 180 && RE_CAPTION2.is_match(t))
            || RE_CAPTION3.is_match(t)
            || RE_CAPTION4.is_match(t)
            || RE_CAPTION5.is_match(t))
}

static RE_PCT_HEX: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)%[0-9A-F]{2}"));
static RE_URL_LINE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:https?://|www\.)\S+$"));
static RE_GARBAGE_CHARS: LazyLock<Regex> = LazyLock::new(|| re(r"[\\{}<>|]"));

fn rd_garbage(t: &str) -> bool {
    RE_PCT_HEX.find_iter(t).count() >= 3
        || RE_URL_LINE.is_match(t)
        || RE_GARBAGE_CHARS.find_iter(t).count() > 6
}

static RE_END_PUNCT: LazyLock<Regex> = LazyLock::new(|| re(r#"[.!?"”')]$"#));
static RE_START_NUM: LazyLock<Regex> = LazyLock::new(|| re(r"^[\d\-–•*·]"));
static RE_HAS_ALPHA: LazyLock<Regex> = LazyLock::new(|| re(r"[A-Za-z]"));
static RE_TITLE_START_WORD: LazyLock<Regex> = LazyLock::new(|| re(r#"^[A-Z0-9“"(]"#));

fn rd_headlineish(t: &str) -> bool {
    if t.len() > 160 || RE_END_PUNCT.is_match(t) || RE_START_NUM.is_match(t) {
        return false;
    }
    let words: Vec<&str> = t
        .split_whitespace()
        .filter(|x| RE_HAS_ALPHA.is_match(x))
        .collect();
    if words.len() < 4 {
        return false;
    }
    let title_cased = words
        .iter()
        .filter(|x| RE_TITLE_START_WORD.is_match(x))
        .count();
    (title_cased as f64) / (words.len() as f64) >= 0.6
}

static RD_STOP: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    "yang dan untuk dengan dari pada dalam akan atau juga tidak ini itu oleh para telah sudah bisa saat usai jadi lebih hingga serta kata kini ada sebagai antara bagi agar kembali ungkap soal terkait dalam atas bawah tentang karena agar bahwa seperti kami kita mereka kamu anda"
        .split_whitespace()
        .collect()
});

fn topic_tokens_vec(s: &str) -> Vec<String> {
    let lower = s.to_lowercase();
    let mut cleaned = String::with_capacity(lower.len());
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() || ('\u{00c0}'..='\u{024f}').contains(&c) || c == ' ' {
            cleaned.push(c);
        } else {
            cleaned.push(' ');
        }
    }
    cleaned
        .split_whitespace()
        .filter(|w| w.len() >= 4 && !RD_STOP.contains(*w))
        .map(|w| w.to_string())
        .collect()
}

fn rd_on_topic_bool(title: &str, desc: &str, paras: &[&str]) -> bool {
    let mut tt = Vec::new();
    let mut seen = HashSet::new();
    for tok in topic_tokens_vec(title) {
        if seen.insert(tok.clone()) {
            tt.push(tok);
        }
    }
    if tt.len() < 2 || paras.is_empty() {
        return true;
    }
    let sample = paras.iter().take(14).copied().collect::<Vec<_>>().join(" ");
    let combined = format!("{sample} {desc}");
    let body_toks = topic_tokens_vec(&combined).join(" ");
    let body = format!(" {body_toks} ");
    let hit = tt
        .iter()
        .filter(|w| body.contains(&format!(" {w} ")))
        .count();
    hit >= 2.min(tt.len()) && (hit as f64) / (tt.len() as f64) >= 0.2
}

static RE_NON_WORD: LazyLock<Regex> = LazyLock::new(|| re(r"\W+"));
static RE_HOME_BREADCRUMB: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(?:home|beranda|news|nasional|daerah)\s*[/›>»]"));
static RE_ELLIPSIS_END: LazyLock<Regex> = LazyLock::new(|| re(r"(?:…|\.\.\.)$"));

fn prune_blocks_impl(
    blocks: &[Value],
    title: &str,
    wire: &HashSet<String>,
) -> (Vec<Value>, usize, usize) {
    let me = RE_NON_WORD
        .replace_all(&title.to_lowercase(), " ")
        .trim()
        .to_string();
    let nrm = |t: &str| -> String {
        RE_NON_WORD
            .replace_all(&t.to_lowercase(), " ")
            .trim()
            .to_string()
    };

    let mut out: Vec<Value> = Vec::new();
    let mut boilerplate_dropped = 0usize;
    let mut duplicate_dropped = 0usize;
    let mut seen_para = HashSet::new();

    for b in blocks {
        let k = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
        if k != "p" && k != "h" && k != "q" && k != "li" {
            out.push(b.clone());
            continue;
        }
        let text = b.get("t").and_then(|v| v.as_str()).unwrap_or("").trim();
        if text.is_empty() {
            continue;
        }
        if BOT_CHALLENGE_RE.is_match(text) {
            boilerplate_dropped += 1;
            continue;
        }
        if RD_END.is_match(text)
            && out
                .iter()
                .filter(|x| {
                    let xk = x.get("k").and_then(|v| v.as_str()).unwrap_or("");
                    xk == "p" || xk == "q" || xk == "li"
                })
                .count()
                >= 2
        {
            break;
        }
        if RD_BOILER.is_match(text) && text.len() < 280 {
            boilerplate_dropped += 1;
            continue;
        }
        if k != "h" && (rd_captionish(text) || rd_garbage(text)) {
            boilerplate_dropped += 1;
            continue;
        }
        if RE_HOME_BREADCRUMB.is_match(text) {
            boilerplate_dropped += 1;
            continue;
        }
        let n = nrm(text);
        if me.len() > 20 && (n == me || (n.contains(&me) && n.len() < me.len() + 40)) {
            duplicate_dropped += 1;
            continue;
        }
        if n != me && n.len() > 25 {
            let mut hit = false;
            for w in wire {
                if n == *w || (n.contains(w) && n.len() < w.len() + 25) {
                    hit = true;
                    break;
                }
            }
            if hit {
                boilerplate_dropped += 1;
                continue;
            }
        }
        let dk = if n.len() > 150 { &n[..150] } else { &n };
        if seen_para.contains(dk) {
            duplicate_dropped += 1;
            continue;
        }
        seen_para.insert(dk.to_string());
        out.push(b.clone());
    }

    let ps: Vec<String> = out
        .iter()
        .filter(|x| {
            let k = x.get("k").and_then(|v| v.as_str()).unwrap_or("");
            k == "p" || k == "q"
        })
        .filter_map(|x| x.get("t").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();

    out.retain(|b| {
        let k = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
        let t = b.get("t").and_then(|v| v.as_str()).unwrap_or("");
        if (k != "p" && k != "q") || !RE_ELLIPSIS_END.is_match(t) {
            return true;
        }
        let head = RE_ELLIPSIS_END.replace(t, "");
        let head = head.trim().to_lowercase();
        if head.len() > 20
            && ps
                .iter()
                .any(|o| o != t && o.to_lowercase().contains(&head))
        {
            duplicate_dropped += 1;
            return false;
        }
        true
    });

    let mut keep = vec![true; out.len()];
    let mut i = 0;
    while i < out.len() {
        let ki = out[i].get("k").and_then(|v| v.as_str()).unwrap_or("");
        let ti = out[i].get("t").and_then(|v| v.as_str()).unwrap_or("");
        if ki == "p" && rd_headlineish(ti) {
            let mut j = i;
            while j < out.len() {
                let kj = out[j].get("k").and_then(|v| v.as_str()).unwrap_or("");
                let tj = out[j].get("t").and_then(|v| v.as_str()).unwrap_or("");
                if kj != "p" || rd_headlineish(tj) {
                    j += 1;
                } else {
                    break;
                }
            }
            let run = out[i..j]
                .iter()
                .filter(|x| x.get("k").and_then(|v| v.as_str()) == Some("p"))
                .count();
            if run >= 2 {
                for q in i..j {
                    if out[q].get("k").and_then(|v| v.as_str()) == Some("p") {
                        keep[q] = false;
                        boilerplate_dropped += 1;
                    }
                }
            }
            i = j;
        } else {
            i += 1;
        }
    }

    let final_blocks = out
        .into_iter()
        .enumerate()
        .filter(|(idx, _)| keep[*idx])
        .map(|(_, b)| b)
        .collect();

    (final_blocks, boilerplate_dropped, duplicate_dropped)
}

fn split_srcset(srcset_raw: &str) -> Vec<String> {
    let trimmed = srcset_raw.trim();
    if trimmed.is_empty() {
        return Vec::new();
    }
    let mut parts = Vec::new();
    let mut last = 0;
    let s = trimmed;
    let bytes = s.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b',' {
            let comma_idx = i;
            // Check condition 1: preceding is \s\d+(\.\d+)?[wx]\s*
            let before = &s[last..comma_idx];
            let before_trimmed = before.trim_end();
            let cond1 = if let Some(last_char) = before_trimmed.chars().last() {
                if matches!(last_char, 'w' | 'W' | 'x' | 'X') {
                    let num_part = &before_trimmed[..before_trimmed.len() - last_char.len_utf8()];
                    let ws_idx = num_part.rfind(|c: char| c.is_whitespace());
                    if let Some(w) = ws_idx {
                        let num_str = &num_part[w + 1..];
                        !num_str.is_empty()
                            && num_str.chars().all(|c| c.is_ascii_digit() || c == '.')
                    } else {
                        false
                    }
                } else {
                    false
                }
            } else {
                false
            };

            // Check condition 2: ,\s+(?=(https?:)?//|/)
            let after = &s[comma_idx + 1..];
            let ws_len = after.len() - after.trim_start().len();
            let cond2 = if ws_len > 0 {
                let rest = &after[ws_len..];
                rest.starts_with('/')
                    || rest.starts_with("//")
                    || (rest.starts_with("http://")
                        || rest.starts_with("https://")
                        || rest.starts_with("HTTP://")
                        || rest.starts_with("HTTPS://"))
            } else {
                false
            };

            if cond1 || cond2 {
                parts.push(before.trim().to_string());
                let after_ws_idx = comma_idx + 1;
                let skip_ws = &s[after_ws_idx..];
                let ws_count = skip_ws.len() - skip_ws.trim_start().len();
                last = after_ws_idx + ws_count;
                i = last;
                continue;
            }
        }
        i += 1;
    }
    if last < len {
        let tail = s[last..].trim();
        if !tail.is_empty() {
            parts.push(tail.to_string());
        }
    }
    parts
}

static RE_SRCSET_PART: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(\S+)(?:\s+(\d+(?:\.\d+)?)(w|x))?$"));

struct SrcsetCandidate {
    url: String,
    score: f64,
    val: f64,
    unit: String,
}

fn pick_best_srcset_impl(srcset_raw: &str, base: &str) -> String {
    let parts = split_srcset(srcset_raw);
    let mut candidates = Vec::new();

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(m) = RE_SRCSET_PART.captures(trimmed) {
            let raw_url = &m[1];
            let url = rd_abs_str(raw_url, base);
            if url.is_empty() {
                continue;
            }
            let val = m
                .get(2)
                .and_then(|v| v.as_str().parse::<f64>().ok())
                .unwrap_or(0.0);
            let unit = m
                .get(3)
                .map(|v| v.as_str().to_ascii_lowercase())
                .unwrap_or_default();
            let score = if unit == "w" {
                val
            } else if unit == "x" {
                val * 600.0
            } else {
                100.0
            };
            candidates.push(SrcsetCandidate {
                url,
                score,
                val,
                unit,
            });
        }
    }
    if candidates.is_empty() {
        return String::new();
    }

    let mut w_good: Vec<&SrcsetCandidate> = candidates
        .iter()
        .filter(|c| c.unit == "w" && c.val >= 700.0 && c.val <= 1800.0)
        .collect();
    if !w_good.is_empty() {
        w_good.sort_by(|a, b| {
            b.val
                .partial_cmp(&a.val)
                .unwrap_or(std::cmp::Ordering::Equal)
        });
        return w_good[0].url.clone();
    }
    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    candidates[0].url.clone()
}

static RE_IMAGE_JUNK_CLS: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)emoji|emoticon|reaction|rating|like|dislike|vote|poll|icon|avatar|logo|badge|smile|frown|share|social|author|wartawan|penulis|byline|banner|advert|iklan|promo|placeholder|spinner|loading",
    )
});

static RE_IMAGE_JUNK_ALT: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:like|dislike|vote|rate|bagus|jelek|setuju|tidak setuju|share|comment|komentar|emoji|logo|favicon|icon|avatar|iklan|advertisement)",
    )
});

static RE_IMG_PLACEHOLDER_URL: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)(?:placeholder|blank|spacer|transparent|1x1|loading|lazy)\."));

struct ExtractedTagImage {
    url: String,
    alt: String,
    attrs: Map<String, Value>,
}

fn extract_tag_image_impl(tag: &str, final_url: &str) -> Option<ExtractedTagImage> {
    let a = parse_attrs_map(tag);
    let cls = format!(
        "{} {} {}",
        a.get("class").and_then(|v| v.as_str()).unwrap_or(""),
        a.get("id").and_then(|v| v.as_str()).unwrap_or(""),
        a.get("role").and_then(|v| v.as_str()).unwrap_or("")
    )
    .to_ascii_lowercase();
    if RE_IMAGE_JUNK_CLS.is_match(&cls) {
        return None;
    }
    let alt_raw = a
        .get("alt")
        .or_else(|| a.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let alt = rd_inline_str(alt_raw);
    if RE_IMAGE_JUNK_ALT.is_match(&alt) {
        return None;
    }

    let raw_w = a.get("width").and_then(|v| v.as_str()).unwrap_or("").trim();
    let raw_h = a
        .get("height")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let w = if !raw_w.ends_with('%') {
        raw_w.parse::<f64>().unwrap_or(0.0) as i64
    } else {
        0
    };
    let h = if !raw_h.ends_with('%') {
        raw_h.parse::<f64>().unwrap_or(0.0) as i64
    } else {
        0
    };
    if (w > 0 && w < 200) || (h > 0 && h < 120) {
        return None;
    }
    if w >= 200 && h > 0 && (w as f64) / (h as f64) > 3.2 {
        return None;
    }

    let mut cand_urls = Vec::new();
    for k in [
        "data-src",
        "data-lazy-src",
        "data-original",
        "data-lazy",
        "data-url",
        "data-srcset",
        "srcset",
        "src",
    ] {
        if let Some(val) = a.get(k).and_then(|v| v.as_str()) {
            if k.ends_with("srcset") {
                let u = pick_best_srcset_impl(val, final_url);
                if !u.is_empty() {
                    cand_urls.push(u);
                }
            } else {
                let u = rd_abs_str(val, final_url);
                if !u.is_empty() && !RE_IMG_PLACEHOLDER_URL.is_match(&u) {
                    cand_urls.push(u);
                }
            }
        }
    }
    let url = cand_urls.into_iter().next()?;
    let u_clean = url.split('?').next().unwrap_or("");
    let f_clean = final_url.split('?').next().unwrap_or("");
    if u_clean == f_clean {
        return None;
    }
    Some(ExtractedTagImage { url, alt, attrs: a })
}

static STRIP_TAGS_BLOCKS: &[&str] = &[
    "script", "style", "svg", "form", "nav", "header", "footer", "aside", "button", "select",
    "textarea", "template", "dialog", "menu",
];
static RE_IFRAME_OR_VIDEO: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)<iframe\b[^>]*>[\s\S]*?</iframe\s*>|<video\b[^>]*>[\s\S]*?</video\s*>")
});
static RE_FIGURE_TAG: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<figure\b[^>]*>([\s\S]*?)</figure\s*>"));
static RE_FIGCAPTION: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<figcaption\b[^>]*>([\s\S]*?)</figcaption>"));
static RE_H2_4: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)<h2\b[^>]*>([\s\S]*?)</h2\s*>|<h3\b[^>]*>([\s\S]*?)</h3\s*>|<h4\b[^>]*>([\s\S]*?)</h4\s*>",
    )
});
static RE_BLOCKQUOTE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)<blockquote\b[^>]*>([\s\S]*?)</blockquote\s*>"));
static RE_LI_TAG: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)<li\b[^>]*>([\s\S]*?)</li\s*>"));
static RE_MARK_LINKED: LazyLock<Regex> =
    LazyLock::new(|| re(r"\u{0001}([^\u{0001}\u{0002}]*)\u{0002}"));
static RE_DEL_MARK: LazyLock<Regex> = LazyLock::new(|| re(r"[\u{0001}\u{0002}]"));
static RE_RD_VI_MARK: LazyLock<Regex> = LazyLock::new(|| re(r"@@RD[VI]\d+@@"));
static RE_COPYRIGHT_START: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(?:copyright|©|all rights reserved)"));

struct ImageBlock {
    url: String,
    cap: String,
    alt: String,
    attrs: Map<String, Value>,
    inside_figure: bool,
}

fn rd_blocks_impl(
    html: &str,
    final_url: &str,
) -> (Vec<Value>, Vec<String>, Vec<Value>, Vec<Value>) {
    let mut blocks = Vec::new();
    let mut paras = Vec::new();
    let mut vids: Vec<Value> = Vec::new();
    let mut vseen = HashSet::new();
    let mut pseen = HashSet::new();

    let mut s = RE_HTML_COMMENT.replace_all(html, " ").into_owned();
    let strip_tags = STRIP_TAGS_BLOCKS
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>();
    s = strip_elements_by_tag_impl(&s, &strip_tags, true, true);

    // Strip secondary article tags
    {
        let mut out = String::with_capacity(s.len());
        let mut last = 0;
        let mut first = true;
        let mut search_from = 0;
        while let Some(m) = RE_ARTICLE_OPEN.find(&s[search_from..]) {
            let start = search_from + m.start();
            let open_end = search_from + m.end();
            if let Some((_inner_end, outer_end)) =
                find_matching_close_bytes(&s, open_end, "article")
            {
                if first {
                    first = false;
                    search_from = open_end;
                    continue;
                }
                out.push_str(&s[last..start]);
                out.push(' ');
                last = outer_end;
                search_from = outer_end;
            } else {
                search_from = open_end;
            }
        }
        if last > 0 {
            out.push_str(&s[last..]);
            s = out;
        }
    }

    s = strip_junk_containers_str(&s);

    s = RE_IFRAME_OR_VIDEO
        .replace_all(&s, |caps: &regex::Captures| {
            let m = &caps[0];
            if vids.len() >= 4 {
                return " ".to_string();
            }
            let v = match rd_video_el(m, final_url) {
                Some(val) => val,
                None => return " ".to_string(),
            };
            let u = v.get("url").and_then(|x| x.as_str()).unwrap_or("");
            let k = u.split('?').next().unwrap_or("").to_string();
            if vseen.contains(&k) {
                return " ".to_string();
            }
            vseen.insert(k);
            vids.push(v);
            format!("\n@@RDV{}@@\n", vids.len() - 1)
        })
        .into_owned();

    s = strip_card_links_str(&s, final_url);
    s = strip_link_blocks_str(&s);

    let mut imgs: Vec<ImageBlock> = Vec::new();
    let mut iseen = HashSet::new();

    let mut add_img = |tag: &str, cap: &str, inside_figure: bool| -> String {
        let im = match extract_tag_image_impl(tag, final_url) {
            Some(i) => i,
            None => return " ".to_string(),
        };
        let norm_key = im.url.split('?').next().unwrap_or("").to_ascii_lowercase();
        if iseen.contains(&norm_key) || imgs.len() >= 12 {
            return " ".to_string();
        }
        iseen.insert(norm_key);
        let mut c = rd_inline_str(cap);
        if c.is_empty() && im.alt.len() >= 16 {
            c = im.alt.clone();
        }
        let cap_truncated = if c.len() > 220 {
            c[..220].to_string()
        } else {
            c
        };
        imgs.push(ImageBlock {
            url: im.url,
            cap: cap_truncated,
            alt: im.alt,
            attrs: im.attrs,
            inside_figure,
        });
        format!("\n@@RDI{}@@\n", imgs.len() - 1)
    };

    s = RE_FIGURE_TAG
        .replace_all(&s, |caps: &regex::Captures| {
            let inner = &caps[1];
            let t = match RE_IMG_AMP.find(inner) {
                Some(m) => {
                    let end = inner[m.start()..]
                        .find('>')
                        .map(|i| m.start() + i + 1)
                        .unwrap_or(m.end());
                    &inner[m.start()..end]
                }
                None => return " ".to_string(),
            };
            let cap = RE_FIGCAPTION
                .captures(inner)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");
            let cap_plain = RE_ANY_TAG.replace_all(cap, " ");
            add_img(t, &cap_plain, true)
        })
        .into_owned();

    // Replace standalone img / amp-img
    let mut s_new = String::with_capacity(s.len());
    let mut search_from = 0;
    while let Some(m) = RE_IMG_AMP.find(&s[search_from..]) {
        let start = search_from + m.start();
        let end = s[start..]
            .find('>')
            .map(|i| start + i + 1)
            .unwrap_or(s.len());
        s_new.push_str(&s[search_from..start]);
        let rep = add_img(&s[start..end], "", false);
        s_new.push_str(&rep);
        search_from = end;
    }
    s_new.push_str(&s[search_from..]);
    s = s_new;

    s = RE_H2_4
        .replace_all(&s, |caps: &regex::Captures| {
            let inner = caps
                .get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .map(|m| m.as_str())
                .unwrap_or("");
            format!("\n@@RDH@@{inner}\n")
        })
        .into_owned();
    s = RE_BLOCKQUOTE
        .replace_all(&s, |caps: &regex::Captures| {
            format!("\n@@RDQ@@{}\n", &caps[1])
        })
        .into_owned();
    s = RE_LI_TAG
        .replace_all(&s, |caps: &regex::Captures| {
            format!("\n@@RDL@@{}\n", &caps[1])
        })
        .into_owned();
    s = RE_A_TAG
        .replace_all(&s, |caps: &regex::Captures| {
            let t = &caps[2];
            let clean = RE_ANY_TAG.replace_all(t, " ");
            format!("\u{0001}{clean}\u{0002}")
        })
        .into_owned();

    let cleaned_all = rd_clean_str(&s);
    for seg_raw in cleaned_all.split('\n') {
        let mut seg = seg_raw;
        let mut kind = "p";
        if let Some(rest) = seg.strip_prefix("@@RDH@@") {
            kind = "h";
            seg = rest;
        } else if let Some(rest) = seg.strip_prefix("@@RDQ@@") {
            kind = "q";
            seg = rest;
        } else if let Some(rest) = seg.strip_prefix("@@RDL@@") {
            kind = "li";
            seg = rest;
        }

        if seg.contains('\u{0001}') {
            let mut linked = 0usize;
            for lm in RE_MARK_LINKED.captures_iter(seg) {
                let inner = &lm[1];
                let plain = RE_RD_VI_MARK.replace_all(inner, " ");
                linked += plain.trim().len();
            }
            let plain = RE_RD_VI_MARK.replace_all(seg, " ");
            let plain = RE_DEL_MARK.replace_all(&plain, "");
            let plain = RE_WS_PLUS.replace_all(&plain, " ");
            let plain = plain.trim();
            if !plain.is_empty()
                && plain.len() <= 600
                && (linked as f64) / (plain.len() as f64) >= 0.8
            {
                continue;
            }
        }

        let seg_del = RE_DEL_MARK.replace_all(seg, "");
        let parts = split_rd_vi_marks(&seg_del);

        let mut i = 0;
        while i < parts.len() {
            let mod3 = i % 3;
            if mod3 == 1 {
                let mkind = &parts[i];
                let n_str = parts.get(i + 1).map(|s| s.as_str()).unwrap_or("");
                let n = n_str.parse::<usize>().unwrap_or(usize::MAX);
                i += 1;
                if mkind == "V" {
                    if let Some(v) = vids.get(n) {
                        blocks.push(json!({ "k": "v", "v": v.clone() }));
                    }
                } else if mkind == "I"
                    && let Some(im) = imgs.get(n)
                {
                    blocks.push(json!({
                        "k": "i",
                        "img": {
                            "url": im.url,
                            "cap": im.cap,
                            "alt": im.alt,
                            "attrs": im.attrs,
                            "insideFigure": im.inside_figure,
                        }
                    }));
                }
                i += 1;
                continue;
            }
            if mod3 == 2 {
                i += 1;
                continue;
            }
            let part_str = &parts[i];
            let t = RE_WS_PLUS.replace_all(part_str, " ");
            let t = t.trim();
            if !t.is_empty() {
                let min_len = if kind == "h" {
                    8
                } else if kind == "li" || kind == "q" {
                    18
                } else {
                    30
                };
                let max_len = if kind == "h" { 140 } else { 1800 };
                if t.len() >= min_len
                    && t.len() <= max_len
                    && !RD_SKIP.is_match(t)
                    && !(t.len() < 120 && RD_CREDIT.is_match(t))
                    && !RE_COPYRIGHT_START.is_match(t)
                {
                    let mut k_norm = RE_NON_WORD.replace_all(&t.to_lowercase(), " ").to_string();
                    if k_norm.len() > 140 {
                        k_norm.truncate(140);
                    }
                    if !pseen.contains(&k_norm) {
                        pseen.insert(k_norm);
                        if kind != "h" {
                            paras.push(t.to_string());
                        }
                        blocks.push(json!({ "k": kind, "t": t }));
                        if paras.len() >= 50 {
                            break;
                        }
                    }
                }
            }
            i += 1;
        }
        if paras.len() >= 50 {
            break;
        }
    }

    let imgs_val = imgs
        .into_iter()
        .map(|im| {
            json!({
                "url": im.url,
                "cap": im.cap,
                "alt": im.alt,
                "attrs": im.attrs,
                "insideFigure": im.inside_figure,
            })
        })
        .collect();

    (blocks, paras, vids, imgs_val)
}

static RE_RD_VI_SPLIT: LazyLock<Regex> = LazyLock::new(|| re(r"@@RD([VI])(\d+)@@"));

fn split_rd_vi_marks(s: &str) -> Vec<String> {
    let mut parts = Vec::new();
    let mut last = 0;
    for cap in RE_RD_VI_SPLIT.captures_iter(s) {
        let m = cap.get(0).unwrap();
        parts.push(s[last..m.start()].to_string());
        parts.push(cap[1].to_string());
        parts.push(cap[2].to_string());
        last = m.end();
    }
    parts.push(s[last..].to_string());
    parts
}

static RE_SENTENCE_SPLIT: LazyLock<Regex> = LazyLock::new(|| re(r#"[.!?]\s+[A-Z"“]"#));

fn split_article_body_text(body: &str) -> Vec<String> {
    let raw = body.trim();
    if raw.is_empty() {
        return Vec::new();
    }
    let cleaned = if RE_BLOCK_STOPPERS.is_match(raw) || RE_BR.is_match(raw) {
        rd_clean_str(raw)
    } else {
        decode_entities_str(raw).replace("\r\n", "\n")
    };
    let mut parts: Vec<String> = cleaned
        .split('\n')
        .map(|x| RE_WS_PLUS.replace_all(x, " ").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if parts.len() <= 1 && cleaned.len() > 420 {
        let single = RE_WS_PLUS.replace_all(&cleaned, " ").trim().to_string();
        let mut sents = Vec::new();
        let mut last = 0;
        for m in RE_SENTENCE_SPLIT.find_iter(&single) {
            let split_pos = m.start() + 1; // after punctuation
            sents.push(single[last..split_pos].trim().to_string());
            let next_start = single[split_pos..]
                .find(|c: char| !c.is_whitespace())
                .map(|i| split_pos + i)
                .unwrap_or(single.len());
            last = next_start;
        }
        if last < single.len() {
            sents.push(single[last..].trim().to_string());
        }

        let mut grouped = Vec::new();
        let mut cur = String::new();
        for st in sents {
            if st.is_empty() {
                continue;
            }
            if cur.is_empty() {
                cur = st;
            } else {
                cur = format!("{cur} {st}");
            }
            if cur.len() >= 240 {
                grouped.push(cur);
                cur = String::new();
            }
        }
        if !cur.is_empty() {
            if !grouped.is_empty() && cur.len() < 60 {
                let last_idx = grouped.len() - 1;
                grouped[last_idx] = format!("{} {}", grouped[last_idx], cur);
            } else {
                grouped.push(cur);
            }
        }
        parts = grouped;
    }

    parts
        .into_iter()
        .filter(|t| t.len() >= 25 && t.len() <= 1800)
        .take(50)
        .collect()
}

fn round_to_3(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0
}

#[derive(Clone)]
struct TextQuality {
    paragraph_count: usize,
    total_chars: usize,
    headline_relevance: f64,
    duplicate_ratio: f64,
    boilerplate_ratio: f64,
    is_on_topic: bool,
    is_challenge_or_error: bool,
    is_full_article: bool,
}

impl TextQuality {
    fn to_value(&self) -> Value {
        json!({
            "paragraphCount": self.paragraph_count,
            "totalChars": self.total_chars,
            "headlineRelevance": self.headline_relevance,
            "duplicateRatio": self.duplicate_ratio,
            "boilerplateRatio": self.boilerplate_ratio,
            "isOnTopic": self.is_on_topic,
            "isChallengeOrError": self.is_challenge_or_error,
            "isFullArticle": self.is_full_article,
        })
    }
}

fn compute_text_quality_impl(
    title: &str,
    expected_title: Option<&str>,
    _url: &str,
    desc: &str,
    blocks: &[Value],
    boilerplate_dropped: usize,
    duplicate_dropped: usize,
) -> TextQuality {
    let text_blocks: Vec<&Value> = blocks
        .iter()
        .filter(|b| {
            let k = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
            k == "p" || k == "q" || k == "li"
        })
        .collect();
    let paras: Vec<&str> = text_blocks
        .iter()
        .filter_map(|b| b.get("t").and_then(|v| v.as_str()))
        .collect();
    let paragraph_count = paras.len();
    let total_chars = paras.iter().map(|p| p.len()).sum::<usize>();
    let total_raw = paragraph_count + boilerplate_dropped + duplicate_dropped;
    let duplicate_ratio = if total_raw > 0 {
        round_to_3(duplicate_dropped as f64 / total_raw as f64)
    } else {
        0.0
    };
    let boilerplate_ratio = if total_raw > 0 {
        round_to_3(boilerplate_dropped as f64 / total_raw as f64)
    } else {
        0.0
    };

    let ref_title = expected_title.unwrap_or(title);
    let mut tt = Vec::new();
    let mut tt_seen = HashSet::new();
    for tok in topic_tokens_vec(ref_title) {
        if tt_seen.insert(tok.clone()) {
            tt.push(tok);
        }
    }

    let mut headline_relevance = 1.0;
    if tt.len() >= 2 && !paras.is_empty() {
        let sample = paras.iter().take(14).copied().collect::<Vec<_>>().join(" ");
        let combined = format!("{sample} {desc}");
        let body_toks = topic_tokens_vec(&combined).join(" ");
        let body = format!(" {body_toks} ");
        let hit = tt
            .iter()
            .filter(|w| body.contains(&format!(" {w} ")))
            .count();
        headline_relevance = round_to_3(hit as f64 / tt.len() as f64);
    } else if paras.is_empty() {
        headline_relevance = 0.0;
    }

    let mut expected_match = true;
    if let Some(exp) = expected_title
        && !exp.is_empty()
        && !title.is_empty()
    {
        let mut exp_toks = Vec::new();
        let mut exp_seen = HashSet::new();
        for tok in topic_tokens_vec(exp) {
            if exp_seen.insert(tok.clone()) {
                exp_toks.push(tok);
            }
        }
        if exp_toks.len() >= 2 {
            let sample = paras.iter().take(6).copied().collect::<Vec<_>>().join(" ");
            let combined = format!("{title} {sample}");
            let page_toks = topic_tokens_vec(&combined).join(" ");
            let page_body = format!(" {page_toks} ");
            let hits = exp_toks
                .iter()
                .filter(|w| page_body.contains(&format!(" {w} ")))
                .count();
            if hits == 0 || (exp_toks.len() >= 3 && (hits as f64 / exp_toks.len() as f64) < 0.2) {
                expected_match = false;
            }
        }
    }

    let on_own_topic = rd_on_topic_bool(title, desc, &paras);
    let is_on_topic = expected_match && on_own_topic;
    let p_sample = paras.iter().take(3).copied().collect::<Vec<_>>().join(" ");
    let combined_sample = format!("{title} {desc} {p_sample}");
    let is_challenge_or_error = BOT_CHALLENGE_RE.is_match(&combined_sample)
        || (paragraph_count <= 2 && ERROR_PAGE_RE.is_match(title.trim()));
    let is_full_article = !is_challenge_or_error
        && is_on_topic
        && ((paragraph_count >= 2 && total_chars >= 180)
            || (paragraph_count >= 1 && total_chars >= 450));

    TextQuality {
        paragraph_count,
        total_chars,
        headline_relevance,
        duplicate_ratio,
        boilerplate_ratio,
        is_on_topic,
        is_challenge_or_error,
        is_full_article,
    }
}

static SITE_BRANDS: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    let mut m = HashMap::new();
    m.insert("metrotvnews.com", "MetroTV");
    m.insert("detik.com", "Detik");
    m.insert("kompas.com", "Kompas");
    m.insert("kompas.id", "Kompas");
    m.insert("cnnindonesia.com", "CNN Indonesia");
    m.insert("cnbcindonesia.com", "CNBC Indonesia");
    m.insert("antaranews.com", "Antara");
    m.insert("tempo.co", "Tempo");
    m.insert("liputan6.com", "Liputan6");
    m.insert("republika.co.id", "Republika");
    m.insert("tribunnews.com", "Tribunnews");
    m.insert("tribun-timur.com", "Tribun Timur");
    m.insert("tribun-bali.com", "Tribun Bali");
    m.insert("tribun-medan.com", "Tribun Medan");
    m.insert("tribun-video.com", "Tribun Video");
    m.insert("kumparan.com", "Kumparan");
    m.insert("sindonews.com", "Sindonews");
    m.insert("okezone.com", "Okezone");
    m.insert("jawapos.com", "Jawa Pos");
    m.insert("viva.co.id", "VIVA");
    m.insert("suara.com", "Suara");
    m.insert("jpnn.com", "JPNN");
    m.insert("bbc.com", "BBC Indonesia");
    m.insert("bbci.co.uk", "BBC Indonesia");
    m.insert("dw.com", "DW Indonesia");
    m.insert("cna.id", "CNA Indonesia");
    m.insert("voi.id", "VOI");
    m.insert("tvonenews.com", "tvOne");
    m.insert("inews.id", "iNews");
    m.insert("beritasatu.com", "BeritaSatu");
    m.insert("merdeka.com", "Merdeka");
    m.insert("mediaindonesia.com", "Media Indonesia");
    m.insert("rmol.id", "RMOL");
    m.insert("bisnis.com", "Bisnis Indonesia");
    m.insert("kontan.co.id", "Kontan");
    m.insert("katadata.co.id", "Katadata");
    m.insert("tirto.id", "Tirto");
    m.insert("bsinews.id", "BSI News");
    m.insert("idntimes.com", "IDN Times");
    m.insert("brilio.net", "Brilio");
    m.insert("youtube.com", "YouTube");
    m.insert("youtu.be", "YouTube");
    m
});

static RE_LOOKS_URL: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:https?:)?//|^www\."));
static RE_LOOKS_DOM: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^[a-z0-9-]+(?:\.[a-z0-9-]+)*\.[a-z]{2,}(?:[/:?#]|$)"));
static RE_HOST_PREFIX: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:www|m)\."));
static RE_TRIBUN_HOST: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^tribun-([a-z0-9-]+)\.[a-z.]+$"));

fn host_of_site(v: &str) -> String {
    let s = v.trim();
    if s.is_empty() {
        return String::new();
    }
    let looks_url = RE_LOOKS_URL.is_match(s);
    let looks_dom = RE_LOOKS_DOM.is_match(s);
    if !looks_url && !looks_dom {
        return String::new();
    }
    let full = if s.to_ascii_lowercase().starts_with("http://")
        || s.to_ascii_lowercase().starts_with("https://")
    {
        s.to_string()
    } else if s.starts_with("//") {
        format!("https:{s}")
    } else {
        format!("https://{s}")
    };
    if let Ok(u) = Url::parse(&full)
        && let Some(h) = u.host_str()
    {
        let lower = h.to_ascii_lowercase();
        return RE_HOST_PREFIX.replace(&lower, "").to_string();
    }
    String::new()
}

fn brand_of_host(h: &str) -> String {
    if h.is_empty() {
        return String::new();
    }
    if let Some(brand) = SITE_BRANDS.get(h) {
        return brand.to_string();
    }
    for (dom, brand) in SITE_BRANDS.iter() {
        let suffix = format!(".{dom}");
        if h.ends_with(&suffix) {
            return brand.to_string();
        }
    }
    String::new()
}

fn clean_site_name_impl(raw_site: &str, fallback_url: &str) -> String {
    let s = raw_site.trim();
    if !s.is_empty() {
        let h = host_of_site(s);
        if !h.is_empty() {
            let mapped = brand_of_host(&h);
            if !mapped.is_empty() {
                return mapped;
            }
            if let Some(caps) = RE_TRIBUN_HOST.captures(&h) {
                let name = caps[1]
                    .split('-')
                    .map(|w| {
                        let mut c = w.chars();
                        match c.next() {
                            None => String::new(),
                            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
                        }
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                return format!("Tribun {name}");
            }
            return h;
        }
        return s.to_string();
    }
    let fh = host_of_site(fallback_url);
    let brand = brand_of_host(&fh);
    if !brand.is_empty() {
        brand
    } else if !fh.is_empty() {
        fh
    } else {
        String::new()
    }
}

static RE_TITLE_BREAKING: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:(?:breaking\s+news|berita\s+video|video|foto|live|eksklusif|update|terkini)\s*[-–—:]\s*)+",
    )
});
static RE_TITLE_DC: LazyLock<Regex> = LazyLock::new(|| re(r"\s*::\s*(\S.{1,60}?)\s*$"));
static RE_TITLE_SEP1: LazyLock<Regex> = LazyLock::new(|| re(r"\s*[|–—]\s*([^|–—]{2,45})\s*$"));
static RE_TITLE_SEP2: LazyLock<FancyRegex> =
    LazyLock::new(|| fre(r"\s+[-–—]{1,2}\s+([^–—\s](?:(?!\s+[-–—]\s+).){1,45})\s*$"));
static RE_TITLE_NON_ALPHANUM: LazyLock<Regex> = LazyLock::new(|| re(r"[^a-z0-9]+"));
static RE_TITLE_TLD: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\.[a-z]{2,8}$"));

fn strip_pub_tail(s: &str, pub_trimmed: &str, pk: &str) -> String {
    if pub_trimmed.is_ascii() {
        let end = js_trim_end(s);
        let plen = pub_trimmed.len();
        if end.len() >= plen {
            let at = end.len() - plen;
            if end.is_char_boundary(at) && end[at..].eq_ignore_ascii_case(pub_trimmed) {
                let before_pub = js_trim_end(&end[..at]);
                let before_seps = before_pub.trim_end_matches(['-', '–', '—', '|', '•', '·']);
                if before_seps.len() < before_pub.len() {
                    return js_trim(before_seps).to_string();
                }
            }
        }
        return js_trim(s).to_string();
    }
    let pat = format!(r"(?i)(?:\s+[-–—]+\s*|\s*[-–—|•·]+\s*){pk}\s*$");
    Regex::new(&pat)
        .map(|r| js_trim(&r.replace(s, "")).to_string())
        .unwrap_or_else(|_| js_trim(s).to_string())
}

static TITLE_SRC_TAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(kompas|antara|detik|cnn|tempo|liputan6|republika|tribun|cnbc|jpnn|okezone|jawapos|viva|kumparan|suara|radar|inilah|merdeka|indosport|youtube|news|bsinews|kabar|berita|radio|bmkg|bnpb|bpbd|kemendagri|kapolri|tni|voice|akurat|acehinfo|polda|polres|polsek|polresta|tribrata|tribatanews|tribratanews|times|update|harian|pos|post|media|koran|portal|suarakalbar|suarasurabaya|conversation|(?-u:\b)tv(?-u:\b))",
    )
});
static TITLE_DOMAIN_TAIL_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\.(?:com|id|net|org|co|asia|info|tv|xyz|online|site|biz|tech|me|app|pro|co\.[a-z]{2}|go\.[a-z]{2}|or\.[a-z]{2}|ac\.[a-z]{2}|sch\.[a-z]{2}|my\.id|web\.id|biz\.id)(?::[0-9]+)?$",
    )
});
static TITLE_DOUBLE_COLON_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\s*::\s*\S"));
static TITLE_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:(?:breaking\s+news|berita\s+video|video|foto|live|eksklusif|update|terkini)\s*[-–—:]\s*)+",
    )
});
static TITLE_URL_TAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)\s+[-–—]+\s*(?:https?://|www\.)\S+$"));
static TITLE_TAIL_SPLIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"^([^\n\r\u{2028}\u{2029}]{8,}?)(?:\s+[-–—]+\s*|\s*[|•·]\s*)([A-Za-z0-9][A-Za-z0-9.&'®™()_\-\s]{1,70})$",
    )
});
static TITLE_TLD_RE2: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)\.[a-z]{2,8}$"));
static TITLE_GOV_PUB_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(r"(?i)^(?:pemerintah|pemkab|pemkot|polresta|polres|polsek|polda|diskominfo|humas)(?-u:\b)")
});

fn feedtext_clean_title(t: &str, pub_str: &str) -> String {
    let mut s0 = js_trim(t).to_string();
    if let Some(pp) = s0.find(" | ")
        && crate::utf16::utf16_offset(&s0, pp) > 15
    {
        s0 = js_trim(&s0[..pp]).to_string();
    }
    if s0.contains("::")
        && let Some(m) = TITLE_DOUBLE_COLON_RE.find(&s0)
        && crate::utf16::utf16_offset(&s0, m.start()) > 15
    {
        s0 = js_trim(&s0[..m.start()]).to_string();
    }
    sub_one(&mut s0, &TITLE_PREFIX_RE, "");
    s0 = js_trim(&s0).to_string();

    if !pub_str.is_empty() {
        let pub_trimmed = js_trim(pub_str);
        let pk = js_escape_regex(pub_trimmed);
        if pk.encode_utf16().count() >= 3 {
            s0 = strip_pub_tail(&s0, pub_trimmed, &pk);
        }
    }

    let mut s = s0;
    if s.contains('.') || s.contains("://") {
        sub_one(&mut s, &TITLE_URL_TAIL_RE, "");
        s = js_trim(&s).to_string();
    }
    for _ in 0..4 {
        let Some(caps) = TITLE_TAIL_SPLIT_RE.captures(&s) else {
            break;
        };
        let head = &caps[1];
        let tail = js_trim(&caps[2]);
        let words: Vec<String> = tail
            .split(is_js_ws)
            .filter(|w| !w.is_empty())
            .map(|w| w.replace(['®', '™'], ""))
            .filter(|w| !w.is_empty())
            .collect();
        if words.len() > 6 {
            break;
        }
        let is_domain = TITLE_DOMAIN_TAIL_RE.is_match(tail) || TITLE_TLD_RE2.is_match(tail);
        let is_known = TITLE_SRC_TAIL_RE.is_match(tail);
        let is_title_case = words.len() <= 5
            && words.iter().all(|w| {
                w.as_bytes()
                    .first()
                    .is_some_and(|b| b.is_ascii_uppercase() || b.is_ascii_digit())
            });
        let is_gov_pub = TITLE_GOV_PUB_RE.is_match(tail);
        let is_short_slug = words.len() <= 3
            && tail.encode_utf16().count() <= 30
            && !tail.contains([',', ';', '?', '!']);
        let is_pub = is_domain
            || is_known
            || is_title_case
            || is_gov_pub
            || (is_short_slug
                && words.iter().all(|w| {
                    w.as_bytes()
                        .first()
                        .is_some_and(|b| b.is_ascii_lowercase() || b.is_ascii_digit())
                }));
        if !is_pub {
            break;
        }
        s = js_trim(head).to_string();
    }
    s
}

static OUTLET_RE_SRC: LazyLock<String> = LazyLock::new(|| {
    const RAW: &str = concat!(
        "kompas com|kompas|detikfinance|detikinet|detikhot|detikcom|detik|tempo co|tempo|",
        "liputan6 com|liputan6|tribunnews|tribun \\w+|republika|cnbc indonesia|cnbcindonesia|",
        "cnbc|antara news|antaranews|antara|voi id|voi|sindonews|sindo|okezone|jawapos|",
        "viva co id|viva|suara com|suara|jpnn|bbc news indonesia|bbc indonesia|bbc|",
        "dw indonesia|dw|cna indonesia|cna|inews id|inews|tvone|tvri|bisnis com|bisnis|",
        "kontan co id|kontan|katadata co id|katadata|kumparan|tirto id|tirto|merdeka com|",
        "merdeka|beritasatu com|beritasatu|rmol id|rmol|idntimes com|idntimes|idn times|",
        "harian jogja|radar \\w+|hops id|mediaindonesia|media indonesia|rri|tagar|",
        "cnn indonesia|cnnindonesia|money|finance detik|finance|tv one|tvone|jakartaglobe|",
        "the jakarta post|waspada|suaramerdeka"
    );
    let mut outlets: Vec<&str> = RAW.split('|').collect();
    outlets.sort_by_key(|a| std::cmp::Reverse(a.len()));
    let joined = outlets
        .into_iter()
        .map(|s| s.replace(r"\w+", "[A-Za-z0-9_]+"))
        .collect::<Vec<_>>()
        .join("|");
    let dom = r"\S*\.(?:com|co|co\.id|co\.uk|or\.id|go\.id|ac\.id|sch\.id|net|org|tv|info|media|news|id)(?-u:\b)";
    format!("(?:{joined}|{dom})")
});

static OUTLET_TAIL_RE: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)[-–—|•·\s]*{}\s*$", *OUTLET_RE_SRC)));
static OUTLET_HEAD_RE: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)^\s*{}\s*[-:–—.]\s+", *OUTLET_RE_SRC)));
static PUBLISHER_ONLY_RE: LazyLock<Regex> =
    LazyLock::new(|| re(&format!(r"(?i)^\s*{}\s*$", *OUTLET_RE_SRC)));

static MEDIA_CREDIT_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?:[Ff]oto|FOTO|[Ii]ll?ustrasi|ILUSTRASI|[Gg]ambar|GAMBAR|[Vv]ideo|VIDEO|[Dd]okumen(?:tasi)?|DOKUMEN(?:TASI)?|[Dd]ok\.|DOK\.)\s*(?:[:：]|[-–—]{1,2})\s*(?:[Oo]leh\s+)?[^\s.,;)\]]+(?:(?!\s+[A-Z][a-z])[^.,;)\]]){0,89}",
    )
});
static DATELINE_2_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| fre(r"^[A-Z][A-Z0-9 .&'()\-]{1,40}?\s+[-–—]{1,2}\s+(?=[A-Z])"));

static CTA_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:baca selengkapnya|selengkapnya di|baca berita|baca juga|kunjungi (?:kami|situs|website|fanpage|aplikasi)|ikuti (?:kami|akun|di)|simak selengkapnya|download aplikasi|klik di sini|baca:)(?-u:\b)",
    )
});
static PHONE_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)(?-u:\b)(?:wa|whatsapp|telp|telepon|hubungi|sms)\s*[:：.]?\s*(?:\+?62|0)8[0-9]{2}[0-9\s().-]{6,}[0-9](?-u:\b)|(?-u:\b)08[0-9]{2}[-\s]?[0-9]{3,4}[-\s]?[0-9]{3,4}(?-u:\b)",
    )
});
static SUMBER_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)sumber\s*(?:[:：]|[-–—]{1,2})\s*[^.,;!)\]\-–—]{0,60}"));
static MEDIA_PREFIX_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?:[Ff]oto|FOTO|[Ii]ll?ustrasi|ILUSTRASI|[Gg]ambar|GAMBAR|[Vv]ideo|VIDEO|[Dd]okumen(?:tasi)?|DOKUMEN(?:TASI)?|[Dd]ok\.|DOK\.)\s*(?:[:：]|[-–—]{1,2})",
    )
});
static PAREN_CREDIT_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\([^)]{1,90}(?:foto|dok|sumber|istimewa|illustrasi|dokumen|istock|shutterstock|getty|antaranews|pexels|pixabay|unsplash|freepik|alamy|dreamstime|123rf)[^)]*\)",
    )
});
static AGENCY_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\((?:antaranews|antara news|reuters|afp|getty images?|istimewa|dokumen|ilustrasi|pexels|pixabay|unsplash|freepik|alamy|dreamstime|123rf)\)",
    )
});
static URL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)(?:https?://|www\.)\S+"));
static DATELINE_1_RE: LazyLock<Regex> =
    LazyLock::new(|| re(r"^[A-Z][A-Z0-9 .,&'()\-]{0,90}\s*(?:--|–|—)\s+"));
static SUMBER_HEAD_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^(?:sumber\s*(?:--|–|—|-\s))+"));
static ELLIPSIS_TAIL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?:\s*(?:\.{3}|…))+\s*$"));
static TRAIL_DASH_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\s+[-–—|]\s*$"));
static TRAIL_COLON_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\s+[:;]\s*$"));
static EMPTY_PAREN_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\(\s*\)"));
static EMPTY_BRACK_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\[\s*\]"));
static SPACE_PUNCT_RE: LazyLock<Regex> = LazyLock::new(|| re(r"[ \t]+([,.;:!?])"));
static DOUBLE_DOT_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\.\s*\."));
static SPACE_SEP_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\s+([,;:])"));
static MULTI_DASH_RE: LazyLock<Regex> = LazyLock::new(|| re(r"(?:\s+[-–—]+){2,}\s*"));
static DANGLING_STOP_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)\s+(?:yang|dan|di|ke|dari|untuk|dengan|pada|oleh|sebagai|dalam|atas|bagi|agar|supaya|karena|hingga|sampai|sejak|setelah|sebelum|tentang|terhadap|lewat|melalui|tanpa|adalah|ialah|yaitu|yakni|atau|juga|masih|sudah|telah|akan|bisa|dapat|harus|perlu|ingin|mau|boleh)\s*$",
    )
});
static WS_TAIL_RE: LazyLock<Regex> = LazyLock::new(|| re(r"\s+\S*$"));
static CHROME_HEAD_RE: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)^(?:tag\s*:|tags?\s*:|penulis\s*:|red\s*:|reporter\s*:|wartawan\s*:|kontributor\s*:|oleh\s*:|di\s+olah\s+oleh|baca\s+(?:juga|selengkapnya|berita|lainnya|dulu)|bacalah|ikuti\s|bagikan|share\s|jangan\s+lewatkan|lihat\s+juga|berita\s+terkait|topik\s*:|iklan(?-u:\b)|advertisement(?-u:\b)|advertorial(?-u:\b)|sumber\s*:|credit\s*:|foto\s*:)",
    )
});

fn clean_desc_text(input: &str) -> String {
    if input.is_empty() {
        return String::new();
    }
    let mut t = collapse_ws(input, true);

    loop {
        let prev_len = t.len();
        if t.as_bytes().first().is_some_and(u8::is_ascii_uppercase) && t.contains(['-', '–', '—'])
        {
            sub_one(&mut t, &DATELINE_1_RE, "");
            if let Cow::Owned(next) = DATELINE_2_RE.replace(&t, "") {
                t = next;
            }
        }
        sub_one(&mut t, &SUMBER_HEAD_RE, "");
        if t.len() == prev_len || t.is_empty() {
            break;
        }
    }

    sub_all(&mut t, &SUMBER_RE, " ");
    if MEDIA_PREFIX_RE.is_match(&t)
        && let Cow::Owned(next) = MEDIA_CREDIT_RE.replace_all(&t, " ")
    {
        t = next;
    }
    if t.contains('(') {
        sub_all(&mut t, &PAREN_CREDIT_RE, " ");
        sub_all(&mut t, &AGENCY_PAREN_RE, " ");
    }
    sub_one(&mut t, &OUTLET_HEAD_RE, "");
    if let Some(m) = CTA_RE.find(&t) {
        t.truncate(m.start());
    }
    if t.contains("://") || t.contains('.') {
        sub_all(&mut t, &URL_RE, " ");
    }
    if t.as_bytes().iter().any(u8::is_ascii_digit) {
        sub_all(&mut t, &PHONE_RE, " ");
    }
    for _ in 0..8 {
        let next = js_trim_end(&OUTLET_TAIL_RE.replace(&t, "")).to_string();
        if next == t {
            break;
        }
        t = next;
    }
    if js_trim_end(&t).ends_with(['.', '…']) {
        sub_one(&mut t, &ELLIPSIS_TAIL_RE, "");
    }
    t = collapse_ws(&t, false);
    let trimmed_lead = t.trim_start_matches(|c: char| {
        is_js_ws(c) || matches!(c, ':' | ';' | ',' | '.' | '-' | '–' | '—' | '|' | '•' | '·')
    });
    if trimmed_lead.len() != t.len() {
        t = trimmed_lead.to_string();
    }
    if js_trim_end(&t).ends_with(['-', '–', '—', '|']) {
        sub_one(&mut t, &TRAIL_DASH_RE, "");
    }
    if js_trim_end(&t).ends_with([':', ';']) {
        sub_one(&mut t, &TRAIL_COLON_RE, "");
    }
    if t.contains('(') {
        sub_all(&mut t, &EMPTY_PAREN_RE, "");
    }
    if t.contains('[') {
        sub_all(&mut t, &EMPTY_BRACK_RE, "");
    }
    sub_all(&mut t, &SPACE_PUNCT_RE, "$1");
    if t.contains('.') {
        sub_all(&mut t, &DOUBLE_DOT_RE, ".");
    }
    if t.contains([',', ';', ':']) {
        sub_all(&mut t, &SPACE_SEP_RE, "$1");
    }
    if t.contains(['-', '–', '—']) {
        sub_all(&mut t, &MULTI_DASH_RE, " — ");
    }
    for _ in 0..3 {
        let prev_len = t.len();
        sub_one(&mut t, &DANGLING_STOP_RE, "");
        if t.len() == prev_len {
            break;
        }
    }
    t = js_trim(&t).to_string();

    if t.contains(['(', ')']) {
        let mut depth: usize = 0;
        let mut balanced = String::with_capacity(t.len());
        for c in t.chars() {
            if c == '(' {
                depth += 1;
                balanced.push(c);
            } else if c == ')' {
                if depth > 0 {
                    depth -= 1;
                    balanced.push(c);
                }
            } else {
                balanced.push(c);
            }
        }
        if depth > 0 {
            let mut rdepth: usize = 0;
            for (i, b) in balanced.bytes().enumerate().rev() {
                if b == b')' {
                    rdepth += 1;
                } else if b == b'(' {
                    if rdepth == 0 {
                        balanced.truncate(i);
                        break;
                    }
                    rdepth -= 1;
                }
            }
        }
        t = balanced;
    }

    if t.contains('(') {
        sub_all(&mut t, &EMPTY_PAREN_RE, "");
    }
    if t.contains('[') {
        sub_all(&mut t, &EMPTY_BRACK_RE, "");
    }
    t = collapse_ws(&t, true);

    if t.chars().count() > 520 {
        let mut keep: String = t.chars().take(517).collect();
        if let Some(m) = WS_TAIL_RE.find(&keep)
            && m.start() > 0
        {
            keep.truncate(m.start());
        }
        let end = keep
            .trim_end_matches(|c: char| {
                is_js_ws(c)
                    || matches!(
                        c,
                        ',' | '.' | ';' | ':' | '!' | '?' | '-' | '–' | '—' | '|' | '•' | '·'
                    )
            })
            .len();
        keep.truncate(end);
        keep.push('…');
        t = keep;
    }

    if BOT_CHALLENGE_RE.is_match(&t) {
        String::new()
    } else {
        t
    }
}

fn norm_title_for_desc(t: &str) -> String {
    let lower = t.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut sep = false;
    for c in lower.chars() {
        if c.is_ascii_alphanumeric() {
            if sep && !out.is_empty() {
                out.push(' ');
            }
            out.push(c);
            sep = false;
        } else {
            sep = true;
        }
    }
    out
}

fn feedtext_desc_from(desc_str: &str, title_str: &str) -> String {
    let s = clean_desc_text(desc_str);
    let title_norm = norm_title_for_desc(title_str);
    if s.is_empty()
        || s.chars().count() < 26
        || (!title_norm.is_empty() && norm_title_for_desc(&s) == title_norm)
        || PUBLISHER_ONLY_RE.is_match(&s)
        || CHROME_HEAD_RE.is_match(&s)
        || BOT_CHALLENGE_RE.is_match(&s)
    {
        return String::new();
    }
    s
}

fn clean_article_title_str(raw_title: &str, site: &str, final_url: &str) -> String {
    let mut title = rd_clean_str(raw_title);
    title = RE_TITLE_BREAKING.replace(&title, "").trim().to_string();

    let norm = |x: &str| {
        RE_TITLE_NON_ALPHANUM
            .replace_all(&x.to_lowercase(), "")
            .to_string()
    };

    if let Some(caps) = RE_TITLE_DC.captures(&title)
        && !site.is_empty()
    {
        let tail = norm(&caps[1]);
        let s = norm(site);
        if !tail.is_empty() && (s.contains(&tail) || tail.contains(&s)) {
            let m = caps.get(0).unwrap();
            title = title[..m.start()].trim().to_string();
        }
    }

    let sep_match = if let Some(caps) = RE_TITLE_SEP1.captures(&title) {
        let m = caps.get(0).unwrap();
        Some((norm(&caps[1]), m.start()))
    } else if let Ok(Some(caps)) = RE_TITLE_SEP2.captures(&title) {
        let m = caps.get(0).unwrap();
        let tail_str = caps.get(1).map(|x| x.as_str()).unwrap_or("");
        Some((norm(tail_str), m.start()))
    } else {
        None
    };

    if let Some((tail, m_start)) = sep_match {
        let host = rd_host_str(final_url);
        let host_no_tld = RE_TITLE_TLD.replace(&host, "").to_string();
        let cands = [norm(site), norm(&host), norm(&host_no_tld)];
        let valid_cands: Vec<&String> = cands.iter().filter(|x| x.len() >= 3).collect();
        if !tail.is_empty()
            && valid_cands
                .iter()
                .any(|s| **s == tail || s.contains(&tail) || tail.contains(*s))
        {
            title = title[..m_start].trim().to_string();
        }
    }

    feedtext_clean_title(&title, site)
}

// ---------------------------------------------------------------------------
// 26 Ported Functions
// ---------------------------------------------------------------------------

/// Port of `isChallengeHtml` (`src/article-extract.js:78`).
pub fn is_challenge_html(
    html: serde_json::Value,
    title: Option<String>,
    desc: Option<String>,
) -> Result<serde_json::Value, String> {
    let title_str = title.unwrap_or_default();
    let desc_str = desc.unwrap_or_default();
    let s_html = js_falsy_or_string(&html);
    Ok(Value::Bool(is_challenge_html_bool(
        &s_html, &title_str, &desc_str,
    )))
}

/// Port of `decodeEntities` (`src/article-extract.js:28`).
pub fn decode_entities(s: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw = js_nullish_or_string(&s);
    Ok(Value::String(decode_entities_str(&raw)))
}

/// Port of `rdClean` (`src/article-extract.js:35`).
pub fn rd_clean(s: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw = js_nullish_or_string(&s);
    Ok(Value::String(rd_clean_str(&raw)))
}

/// Port of `rdInline` (`src/article-extract.js:49`).
pub fn rd_inline(s: serde_json::Value) -> Result<serde_json::Value, String> {
    let raw = js_nullish_or_string(&s);
    Ok(Value::String(rd_inline_str(&raw)))
}

/// Port of `rdHost` (`src/article-extract.js:50`).
pub fn rd_host(u: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_to_string(&u);
    Ok(Value::String(rd_host_str(&s)))
}

/// Port of `rdAbs` (`src/article-extract.js:63`).
pub fn rd_abs(u: serde_json::Value, base: serde_json::Value) -> Result<serde_json::Value, String> {
    if !is_truthy(&u) {
        return Ok(Value::String(String::new()));
    }
    let u_str = js_to_string(&u);
    let base_str = js_to_string(&base);
    Ok(Value::String(rd_abs_str(&u_str, &base_str)))
}

/// Port of `parseAttrs` (`src/article-extract.js:52`).
pub fn parse_attrs(tag_str: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_falsy_or_string(&tag_str);
    let m = parse_attrs_map(&s);
    Ok(Value::Object(m))
}

/// Port of `pickBestSrcset` (`src/article-extract.js:583`).
pub fn pick_best_srcset(
    srcset_raw: serde_json::Value,
    base: serde_json::Value,
) -> Result<serde_json::Value, String> {
    if !is_truthy(&srcset_raw) {
        return Ok(Value::String(String::new()));
    }
    let raw = js_to_string(&srcset_raw);
    let base_str = js_nullish_or_string(&base);
    Ok(Value::String(pick_best_srcset_impl(&raw, &base_str)))
}

/// Port of `rdMeta` (`src/article-extract.js:91`).
pub fn rd_meta(html: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_falsy_or_string(&html);
    let m = rd_meta_map(&s);
    Ok(Value::Object(m))
}

/// Port of `rdLd` (`src/article-extract.js:105`).
pub fn rd_ld(html: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_falsy_or_string(&html);
    let v = rd_ld_vec(&s);
    Ok(Value::Array(v))
}

/// Port of `rdName` (`src/article-extract.js:126`).
pub fn rd_name(v: serde_json::Value) -> Result<serde_json::Value, String> {
    Ok(Value::String(rd_name_val(&v)))
}

/// Port of `findMatchingClose` (`src/article-extract.js:134`).
pub fn find_matching_close(
    html: serde_json::Value,
    open_end_idx: serde_json::Value,
    tag_name: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let s = js_to_string(&html);
    let idx_num = crate::js::as_number(&open_end_idx).unwrap_or(0.0);
    let u16_idx = if idx_num < 0.0 || idx_num.is_nan() {
        0
    } else {
        idx_num as usize
    };
    let tag = js_to_string(&tag_name);
    let byte_idx = crate::utf16::byte_offset(&s, u16_idx);
    if let Some((inner_end_byte, outer_end_byte)) = find_matching_close_bytes(&s, byte_idx, &tag) {
        let inner_u16 = crate::utf16::utf16_offset(&s, inner_end_byte);
        let outer_u16 = crate::utf16::utf16_offset(&s, outer_end_byte);
        Ok(json!({
            "innerEnd": inner_u16,
            "outerEnd": outer_u16,
        }))
    } else {
        Ok(Value::Null)
    }
}

/// Port of `stripElementsByTag` (`src/article-extract.js:151`).
pub fn strip_elements_by_tag(
    html: serde_json::Value,
    tags: serde_json::Value,
    opts: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let (unwrap_noscript, preserve_header_figures) = match &opts {
        Value::Null => {
            return Err(
                "Cannot read properties of null (reading 'unwrapNoscriptImages')".to_string(),
            );
        }
        v if crate::js::is_undefined(v) => (false, false),
        Value::Object(o) => {
            let u = o.get("unwrapNoscriptImages").is_some_and(is_truthy);
            let p = o.get("preserveHeaderFigures").is_some_and(is_truthy);
            (u, p)
        }
        _ => (false, false),
    };
    let tags_vec = match tags {
        Value::Null => return Err("Cannot read properties of null (reading 'join')".to_string()),
        Value::Array(arr) => arr.iter().map(js_to_string).collect::<Vec<_>>(),
        _ => return Err("tags.join is not a function".to_string()),
    };
    let s = js_falsy_or_string(&html);
    let out = strip_elements_by_tag_impl(&s, &tags_vec, unwrap_noscript, preserve_header_figures);
    Ok(Value::String(out))
}

/// Port of `stripJunkContainers` (`src/article-extract.js:211`).
pub fn strip_junk_containers(html: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_falsy_or_string(&html);
    Ok(Value::String(strip_junk_containers_str(&s)))
}

/// Port of `stripCardLinks` (`src/article-extract.js:248`).
pub fn strip_card_links(
    s: serde_json::Value,
    base: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let s_str = js_falsy_or_string(&s);
    let base_str = js_nullish_or_string(&base);
    Ok(Value::String(strip_card_links_str(&s_str, &base_str)))
}

/// Port of `stripLinkBlocks` (`src/article-extract.js:265`).
pub fn strip_link_blocks(s: serde_json::Value) -> Result<serde_json::Value, String> {
    let s_str = js_falsy_or_string(&s);
    Ok(Value::String(strip_link_blocks_str(&s_str)))
}

/// Port of `extractContainers` (`src/article-extract.js:303`).
pub fn extract_containers(html: serde_json::Value) -> Result<serde_json::Value, String> {
    let s = js_falsy_or_string(&html);
    let (body_containers, lead_media_containers) = extract_containers_map(&s);
    Ok(json!({
        "bodyContainers": body_containers,
        "leadMediaContainers": lead_media_containers,
    }))
}

/// Port of `rdBlocks` (`src/article-extract.js:646`).
pub fn rd_blocks(
    html: serde_json::Value,
    final_url: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let s = js_falsy_or_string(&html);
    let url_str = js_nullish_or_string(&final_url);
    let (blocks, paras, videos, images) = rd_blocks_impl(&s, &url_str);
    Ok(json!({
        "blocks": blocks,
        "paras": paras,
        "videos": videos,
        "images": images,
    }))
}

/// Port of `rdVideos` (`src/article-extract.js:420`).
pub fn rd_videos(
    html: serde_json::Value,
    final_url: serde_json::Value,
    dom: Option<bool>,
    title: Option<String>,
    on_topic_fn: Option<serde_json::Value>,
) -> Result<serde_json::Value, String> {
    let _ = on_topic_fn;
    let s = js_falsy_or_string(&html);
    let url_str = js_nullish_or_string(&final_url);
    let dom_bool = dom.unwrap_or(true);
    let title_str = title.unwrap_or_default();
    let vids = rd_videos_vec(&s, &url_str, dom_bool, &title_str);
    Ok(Value::Array(vids))
}

/// Port of `rdOnTopic` (`src/article-extract.js:503`).
pub fn rd_on_topic(
    title: serde_json::Value,
    desc: serde_json::Value,
    paras: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let title_str = js_falsy_or_string(&title);
    let desc_str = js_falsy_or_string(&desc);
    if let Value::Array(arr) = paras {
        let p_strs: Vec<String> = arr.iter().map(js_to_string).collect();
        let p_refs: Vec<&str> = p_strs.iter().map(|s| s.as_str()).collect();
        Ok(Value::Bool(rd_on_topic_bool(
            &title_str, &desc_str, &p_refs,
        )))
    } else {
        Ok(Value::Bool(true))
    }
}

/// Port of `topicTokens` (`src/article-extract.js:495`).
pub fn topic_tokens(s: serde_json::Value) -> Result<serde_json::Value, String> {
    let s_str = js_falsy_or_string(&s);
    let toks = topic_tokens_vec(&s_str);
    let vals: Vec<Value> = toks.into_iter().map(Value::String).collect();
    Ok(Value::Array(vals))
}

/// Port of `pruneBlocks` (`src/article-extract.js:511`).
pub fn prune_blocks(
    blocks: serde_json::Value,
    title: serde_json::Value,
    wire_titles_set: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let blocks_arr = match blocks {
        Value::Array(arr) => arr,
        Value::String(s) => s.chars().map(|c| Value::String(c.to_string())).collect(),
        _ => return Err("blocks is not iterable".to_string()),
    };
    let title_str = js_nullish_or_string(&title);
    let mut wire_set = HashSet::new();
    if let Value::Array(arr) = wire_titles_set {
        for item in arr {
            let s = js_to_string(&item);
            let n = RE_NON_WORD
                .replace_all(&s.to_lowercase(), " ")
                .trim()
                .to_string();
            wire_set.insert(n);
        }
    }
    let (final_blocks, boilerplate_dropped, duplicate_dropped) =
        prune_blocks_impl(&blocks_arr, &title_str, &wire_set);
    Ok(json!({
        "blocks": final_blocks,
        "boilerplateDropped": boilerplate_dropped,
        "duplicateDropped": duplicate_dropped,
    }))
}

/// Port of `computeTextQuality` (`src/article-extract.js:782`).
pub fn compute_text_quality(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'title' of 'object null' as it is null.".to_string(),
        );
    }
    let empty_blocks = Vec::new();
    let (title, expected_title, url, desc, blocks, boilerplate_dropped, duplicate_dropped) =
        if let Value::Object(ref o) = arg0 {
            let t = o.get("title").and_then(|v| v.as_str()).unwrap_or("");
            let exp = o.get("expectedTitle").and_then(|v| v.as_str());
            let u = o.get("url").and_then(|v| v.as_str()).unwrap_or("");
            let d = o.get("desc").and_then(|v| v.as_str()).unwrap_or("");
            let blk = o
                .get("blocks")
                .and_then(|v| v.as_array())
                .unwrap_or(&empty_blocks);
            let bp = o
                .get("boilerplateDropped")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            let dp = o
                .get("duplicateDropped")
                .and_then(|v| v.as_u64())
                .unwrap_or(0) as usize;
            (t, exp, u, d, blk.as_slice(), bp, dp)
        } else {
            ("", None, "", "", empty_blocks.as_slice(), 0, 0)
        };
    let q = compute_text_quality_impl(
        title,
        expected_title,
        url,
        desc,
        blocks,
        boilerplate_dropped,
        duplicate_dropped,
    );
    Ok(q.to_value())
}

/// Port of `cleanArticleTitle` (`src/article-extract.js:836`).
pub fn clean_article_title(
    raw_title: serde_json::Value,
    site: serde_json::Value,
    final_url: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let raw_str = js_falsy_or_string(&raw_title);
    let site_str = js_falsy_or_string(&site);
    let url_str = js_falsy_or_string(&final_url);
    Ok(Value::String(clean_article_title_str(
        &raw_str, &site_str, &url_str,
    )))
}

/// Port of `rdExtract` (`src/article-extract.js:858`).
pub fn rd_extract(
    html: serde_json::Value,
    final_url: serde_json::Value,
    opts: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let expected_title = match &opts {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'expectedTitle')".to_string());
        }
        v if crate::js::is_undefined(v) => None,
        Value::Object(o) => o
            .get("expectedTitle")
            .and_then(|v| v.as_str())
            .map(|s| s.to_string()),
        _ => None,
    };
    let mut wire_set = HashSet::new();
    if let Value::Object(ref o) = opts
        && let Some(Value::Array(arr)) = o.get("wireTitles")
    {
        for item in arr {
            let s = js_to_string(item);
            let n = RE_NON_WORD
                .replace_all(&s.to_lowercase(), " ")
                .trim()
                .to_string();
            wire_set.insert(n);
        }
    }

    let s_html = js_falsy_or_string(&html);
    let final_url_str = js_nullish_or_string(&final_url);
    let meta = rd_meta_map(&s_html);
    let ld = rd_ld_vec(&s_html);

    let art = ld.iter().find(|n| {
        let t = n.get("@type").and_then(|v| v.as_str()).unwrap_or("");
        re(r"(?i)article|posting|news").is_match(t)
    });
    let vid = ld.iter().find(|n| {
        let t = n.get("@type").and_then(|v| v.as_str()).unwrap_or("");
        re(r"(?i)video").is_match(t)
    });

    let pub_name = art
        .and_then(|a| {
            a.get("publisher").and_then(|p| {
                if let Some(s) = p.as_str() {
                    Some(s.to_string())
                } else {
                    p.get("name")
                        .and_then(|v| v.as_str())
                        .map(|s| s.to_string())
                }
            })
        })
        .unwrap_or_default();

    let raw_site = meta
        .get("og:site_name")
        .and_then(|v| v.as_str())
        .unwrap_or(&pub_name);
    let site = clean_site_name_impl(raw_site, &final_url_str);

    let mut raw_title = meta
        .get("og:title")
        .or_else(|| meta.get("twitter:title"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            art.and_then(|a| a.get("headline"))
                .map(rd_name_val)
                .unwrap_or_default()
        });

    if raw_title.is_empty()
        && let Some(caps) = re(r"(?i)<title[^>]*>([\s\S]*?)</title>").captures(&s_html)
    {
        raw_title = caps[1].to_string();
    }
    if raw_title.is_empty()
        && let Some(caps) = re(r"(?i)<h1\b[^>]*>([\s\S]*?)</h1>").captures(&s_html)
    {
        raw_title = rd_clean_str(&caps[1]);
    }

    let mut title = clean_article_title_str(&raw_title, &site, &final_url_str);
    let host = rd_host_str(&final_url_str);
    if (title.is_empty() || title == host)
        && let Some(ref exp) = expected_title
    {
        title = clean_article_title_str(exp, &site, &final_url_str);
    }

    let raw_desc = meta
        .get("og:description")
        .or_else(|| meta.get("twitter:description"))
        .or_else(|| meta.get("description"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            art.and_then(|a| a.get("description"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        });

    let byline = meta
        .get("author")
        .or_else(|| meta.get("article:author"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            art.and_then(|a| a.get("author"))
                .map(rd_name_val)
                .unwrap_or_default()
        });

    let time = meta
        .get("article:published_time")
        .or_else(|| meta.get("datepublished"))
        .and_then(|v| v.as_str())
        .map(|s| s.to_string())
        .unwrap_or_else(|| {
            art.and_then(|a| a.get("datePublished"))
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string()
        });

    if is_challenge_html_bool(&s_html, &title, &raw_desc) {
        let t_out = if title.is_empty() { host } else { title };
        return Ok(json!({
            "ok": false,
            "url": final_url,
            "final": final_url,
            "title": t_out,
            "site": site,
            "byline": byline,
            "time": time,
            "img": "",
            "desc": "",
            "paras": [],
            "thin": true,
            "isFallback": false,
            "textSource": "challenge",
            "videos": [],
            "blocks": [],
            "extraction": {
                "method": "blocked",
                "isFallback": false,
                "quality": {
                    "paragraphCount": 0,
                    "totalChars": 0,
                    "headlineRelevance": 0,
                    "duplicateRatio": 0,
                    "boilerplateRatio": 0,
                    "isOnTopic": false,
                    "isChallengeOrError": true,
                    "isFullArticle": false,
                }
            }
        }));
    }

    let (body_containers, _lead_media) = extract_containers_map(&s_html);

    struct Candidate {
        method: &'static str,
        blocks: Vec<Value>,
        videos: Vec<Value>,
        quality: TextQuality,
        score: f64,
    }

    let mut candidates: Vec<Candidate> = Vec::new();

    for cand in body_containers {
        let kind = cand.get("kind").and_then(|v| v.as_str()).unwrap_or("");
        let cand_html = cand.get("html").and_then(|v| v.as_str()).unwrap_or("");
        let (raw_blocks, _raw_paras, raw_vids, _raw_imgs) =
            rd_blocks_impl(cand_html, &final_url_str);
        let (pruned_blocks, bp_dropped, dp_dropped) =
            prune_blocks_impl(&raw_blocks, &title, &wire_set);
        let q = compute_text_quality_impl(
            &title,
            expected_title.as_deref(),
            &final_url_str,
            &raw_desc,
            &pruned_blocks,
            bp_dropped,
            dp_dropped,
        );
        let kind_bonus = match kind {
            "selector" => 20.0,
            "article" => 12.0,
            "main" => 5.0,
            _ => 0.0,
        };
        let score = (if q.is_on_topic { 120.0 } else { -200.0 })
            + (q.paragraph_count.min(15) as f64) * 10.0
            + ((q.total_chars as f64 / 180.0).min(35.0))
            + kind_bonus
            - q.boilerplate_ratio * 40.0
            - q.duplicate_ratio * 40.0;
        let method = match kind {
            "selector" => "container_selector",
            "article" => "semantic_article",
            "main" => "semantic_main",
            _ => "html_density",
        };
        let is_full = q.is_full_article;
        let p_count = q.paragraph_count;
        let t_chars = q.total_chars;
        candidates.push(Candidate {
            method,
            blocks: pruned_blocks,
            videos: raw_vids,
            quality: q,
            score,
        });
        if kind != "full" && is_full && p_count >= 4 && t_chars >= 700 {
            break;
        }
    }

    let script_vid = rd_extract_script_videos_impl(&s_html, &final_url_str, &title);
    let art_body = art
        .and_then(|a| a.get("articleBody"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let art_desc = art
        .and_then(|a| a.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let vid_desc = vid
        .and_then(|v| v.get("description"))
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let extra_sources: &[(&'static str, &str)] = &[
        ("jsonld_articleBody", art_body),
        (
            "jsonld_description",
            if art_desc.len() > 160 { art_desc } else { "" },
        ),
        (
            "jsonld_video_desc",
            if vid_desc.len() > 80 { vid_desc } else { "" },
        ),
        ("script_video_transcript", &script_vid.script_body),
    ];

    for (m_name, body_text) in extra_sources {
        if body_text.trim().is_empty() {
            continue;
        }
        let ps = split_article_body_text(body_text);
        if ps.is_empty() {
            continue;
        }
        let ps_blocks: Vec<Value> = ps
            .into_iter()
            .map(|t| json!({ "k": "p", "t": t }))
            .collect();
        let (pruned_blocks, bp_dropped, dp_dropped) =
            prune_blocks_impl(&ps_blocks, &title, &wire_set);
        let q = compute_text_quality_impl(
            &title,
            expected_title.as_deref(),
            &final_url_str,
            &raw_desc,
            &pruned_blocks,
            bp_dropped,
            dp_dropped,
        );
        let bonus = if *m_name == "jsonld_articleBody" || *m_name == "script_video_transcript" {
            15.0
        } else {
            0.0
        };
        let score = (if q.is_on_topic { 120.0 } else { -200.0 })
            + (q.paragraph_count.min(15) as f64) * 10.0
            + ((q.total_chars as f64 / 180.0).min(35.0))
            + bonus
            - q.boilerplate_ratio * 40.0
            - q.duplicate_ratio * 40.0;
        let cand_vids = if *m_name == "script_video_transcript" {
            script_vid.videos.clone()
        } else {
            Vec::new()
        };
        let mut cand_blocks = Vec::new();
        if let Some(first_v) = cand_vids.first() {
            cand_blocks.push(json!({ "k": "v", "v": first_v }));
        }
        cand_blocks.extend(pruned_blocks);
        candidates.push(Candidate {
            method: m_name,
            blocks: cand_blocks,
            videos: cand_vids,
            quality: q,
            score,
        });
    }

    candidates.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let winner_idx = candidates.iter().position(|c| {
        c.quality.is_on_topic && !c.quality.is_challenge_or_error && c.quality.paragraph_count > 0
    });

    let mut winner = winner_idx.map(|idx| candidates.remove(idx));
    let mut text_source = winner.as_ref().map(|w| w.method).unwrap_or("none");
    let mut is_fallback = winner
        .as_ref()
        .map(|w| !w.quality.is_full_article)
        .unwrap_or(true);

    if winner.is_none() || is_fallback {
        let cleaned_meta_desc = feedtext_desc_from(&rd_inline_str(&raw_desc), &title);

        if winner.is_none()
            && !cleaned_meta_desc.is_empty()
            && !BOT_CHALLENGE_RE.is_match(&cleaned_meta_desc)
        {
            let fb_blocks = vec![json!({ "k": "p", "t": cleaned_meta_desc })];
            let mut q = compute_text_quality_impl(
                &title,
                expected_title.as_deref(),
                &final_url_str,
                &cleaned_meta_desc,
                &fb_blocks,
                0,
                0,
            );
            q.is_full_article = false;
            winner = Some(Candidate {
                method: "meta_description",
                blocks: fb_blocks,
                videos: Vec::new(),
                quality: q,
                score: 0.0,
            });
            text_source = "meta_description";
            is_fallback = true;
        } else if winner.as_ref().is_some_and(|w| !w.quality.is_full_article) {
            is_fallback = true;
        }
    }

    let final_blocks_raw = winner
        .as_ref()
        .map(|w| w.blocks.clone())
        .unwrap_or_default();
    let final_quality = winner
        .as_ref()
        .map(|w| &w.quality)
        .cloned()
        .unwrap_or_else(|| {
            compute_text_quality_impl(
                &title,
                expected_title.as_deref(),
                &final_url_str,
                &raw_desc,
                &[],
                0,
                0,
            )
        });

    let tnorm = rd_inline_str(&title).to_lowercase();
    let filtered_blocks: Vec<Value> = final_blocks_raw
        .into_iter()
        .filter(|b| {
            let k = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
            let t = b.get("t").and_then(|v| v.as_str()).unwrap_or("");
            !(k == "p" && !tnorm.is_empty() && t.to_lowercase() == tnorm)
        })
        .collect();

    let mut fin_blocks = Vec::new();
    let mut joined = Vec::new();
    let mut total = 0;
    for b in filtered_blocks {
        let k = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
        let t = b.get("t").and_then(|v| v.as_str()).unwrap_or("");
        if k == "p" || k == "h" || k == "q" || k == "li" {
            if total + t.len() > 18000 {
                continue;
            }
            total += t.len();
            if k != "h" {
                joined.push(t.to_string());
            }
        }
        fin_blocks.push(b);
    }

    let blk_videos = winner
        .as_ref()
        .map(|w| w.videos.clone())
        .unwrap_or_default();
    let all_v = rd_videos_vec(&s_html, &final_url_str, false, &title);
    let vkey = |v: &Value| {
        v.get("url")
            .and_then(|x| x.as_str())
            .unwrap_or("")
            .split('?')
            .next()
            .unwrap_or("")
            .to_string()
    };
    let mut pkeys: HashSet<String> = blk_videos.iter().map(vkey).collect();
    let mut videos = blk_videos;
    for v in all_v {
        let k = vkey(&v);
        if !pkeys.contains(&k) {
            pkeys.insert(k);
            videos.push(v);
        }
    }
    videos.truncate(4);

    let desc_from_res = feedtext_desc_from(&rd_inline_str(&raw_desc), &title);

    let mut clean_desc = if !desc_from_res.is_empty() {
        desc_from_res
    } else {
        rd_inline_str(&raw_desc)
    };
    if BOT_CHALLENGE_RE.is_match(&clean_desc) {
        clean_desc = String::new();
    }
    if (clean_desc.len() < 25 || re(r"(?i)^[-–—\s]*[A-Za-z0-9.\s]+$").is_match(&clean_desc))
        && let Some(first_p) = joined.iter().find(|p| p.len() >= 40)
    {
        clean_desc = if first_p.len() > 300 {
            first_p[..300].to_string()
        } else {
            first_p.clone()
        };
    }

    let off_topic = !candidates.is_empty() && candidates.iter().all(|c| !c.quality.is_on_topic);

    let mut title_out = if title.is_empty() || title == host {
        if let Some(ref exp) = expected_title {
            clean_article_title_str(exp, &site, &final_url_str)
        } else {
            title
        }
    } else {
        title
    };
    if title_out.is_empty() {
        title_out = host;
    }
    if title_out.is_empty() {
        title_out = "Tanpa judul".to_string();
    }

    Ok(json!({
        "ok": !off_topic && !final_quality.is_challenge_or_error,
        "offTopic": off_topic,
        "url": final_url,
        "final": final_url,
        "title": title_out,
        "site": site,
        "byline": byline,
        "time": time,
        "img": "",
        "desc": clean_desc,
        "paras": joined,
        "thin": !final_quality.is_full_article,
        "isFallback": is_fallback,
        "textSource": text_source,
        "videos": videos,
        "blocks": fin_blocks,
        "extraction": {
            "method": text_source,
            "isFallback": is_fallback,
            "quality": final_quality.to_value(),
        }
    }))
}

static RE_MD_IMG: LazyLock<Regex> = LazyLock::new(|| {
    re(r"\[!\[([^\]]*)\]\((https?://[^)\s]+)\)\]\(([^)\s]*)\)|!\[([^\]]*)\]\((https?://[^)\s]+)\)")
});
static RE_MD_ANY_IMG: LazyLock<Regex> = LazyLock::new(|| re(r"!\[[^\]]*\]\([^)]*\)"));
static RE_MD_LINK: LazyLock<Regex> = LazyLock::new(|| re(r"\[([^\]]*)\]\([^)]*\)"));
static RE_MD_FORMATTING: LazyLock<Regex> = LazyLock::new(|| re(r"(\*\*|__|\*|_|`|~~)"));
static RE_MD_IGNORE_LINE: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^(?:markdown content:|url source:|published time:)"));
static RE_MD_TITLE_LINE: LazyLock<Regex> = LazyLock::new(|| re(r"(?i)^title:\s*(.+)"));
static RE_MD_HEADING_STOP: LazyLock<Regex> = LazyLock::new(|| {
    re(
        r"(?i)terpopuler|terkait|baca juga|rekomendasi|berita lain|terbaru|trending|populer|lainnya|related|more (?:news|stories)|latest|you may|komentar|comments?",
    )
});
static RE_MD_LIST_ITEM: LazyLock<Regex> = LazyLock::new(|| re(r"^(?:[-*]|\d+\.)\s+"));
static RE_MD_IMG_CAP_CLEAN: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)^image\s+\d+\s*[:\-–—]?\s*|^image\s*[:\-–—]\s*"));
static RE_MD_IMG_JUNK_CAP: LazyLock<Regex> =
    LazyLock::new(|| re(r"(?i)logo|icon|share|whatsapp|facebook|twitter|menu|search|badge"));

fn emit_md_text(
    mut txt: String,
    kind: &'static str,
    paras: &mut Vec<String>,
    blocks: &mut Vec<Value>,
    seen: &mut HashSet<String>,
    done: &mut bool,
) {
    if *done {
        return;
    }
    txt = RE_MD_ANY_IMG.replace_all(&txt, "").to_string();
    let mut link_len = 0;
    for lm in RE_MD_LINK.captures_iter(&txt) {
        link_len += lm[1].len();
    }
    let stripped = RE_MD_LINK.replace_all(&txt, "$1").to_string();
    if (link_len as f64) / (1.max(stripped.len()) as f64) >= 0.7 {
        return;
    }
    let clean = RE_MD_FORMATTING.replace_all(&stripped, "");
    let clean = RE_WS_PLUS.replace_all(&clean, " ");
    let clean = clean.trim();
    let min_len = if kind == "h" {
        8
    } else if kind == "q" || kind == "li" {
        20
    } else {
        35
    };
    let max_len = if kind == "h" { 140 } else { 1800 };
    if clean.len() < min_len || clean.len() > max_len {
        return;
    }
    if RD_SKIP.is_match(clean) || BOT_CHALLENGE_RE.is_match(clean) {
        return;
    }
    let mut k = RE_NON_WORD
        .replace_all(&clean.to_lowercase(), " ")
        .to_string();
    if k.len() > 140 {
        k.truncate(140);
    }
    if seen.contains(&k) {
        return;
    }
    seen.insert(k);
    if kind != "h" {
        paras.push(clean.to_string());
    }
    blocks.push(json!({ "k": kind, "t": clean }));
    if paras.len() >= 50 {
        *done = true;
    }
}

/// Port of `rdJinaMd` (`src/article-extract.js:1017`).
pub fn rd_jina_md(
    text: serde_json::Value,
    url: serde_json::Value,
    opts: serde_json::Value,
) -> Result<serde_json::Value, String> {
    let s_text = js_nullish_or_string(&text);
    let url_str = js_nullish_or_string(&url);

    let check_len = s_text.len().min(5000);
    if BOT_CHALLENGE_RE.is_match(&s_text[..check_len]) {
        let site = rd_host_str(&url_str);
        let title = if site.is_empty() {
            "Tanpa judul".to_string()
        } else {
            site.clone()
        };
        return Ok(json!({
            "ok": false,
            "offTopic": false,
            "url": url,
            "final": url,
            "title": title,
            "site": site,
            "byline": "",
            "time": "",
            "img": "",
            "desc": "",
            "paras": [],
            "thin": true,
            "isFallback": false,
            "textSource": "challenge",
            "videos": [],
            "blocks": [],
            "via": "jina",
            "extraction": {
                "method": "blocked",
                "isFallback": false,
                "quality": {
                    "paragraphCount": 0,
                    "totalChars": 0,
                    "headlineRelevance": 0,
                    "duplicateRatio": 0,
                    "boilerplateRatio": 0,
                    "isOnTopic": false,
                    "isChallengeOrError": true,
                    "isFullArticle": false,
                }
            }
        }));
    }

    // Notice error case: opts is null
    let wire_titles = match &opts {
        Value::Null => {
            return Err("Cannot read properties of null (reading 'wireTitles')".to_string());
        }
        v if crate::js::is_undefined(v) => Value::Null,
        Value::Object(o) => o.get("wireTitles").cloned().unwrap_or(Value::Null),
        _ => Value::Null,
    };
    let expected_title = match &opts {
        Value::Object(o) => o.get("expectedTitle").and_then(|v| v.as_str()),
        _ => None,
    };

    let mut paras = Vec::new();
    let mut seen = HashSet::new();
    let mut blocks = Vec::new();
    let mut title = String::new();
    let mut done = false;

    let mut cur_link_path = String::new();
    if let Ok(u) = Url::parse(&url_str) {
        cur_link_path = u.path().to_string();
    }

    let cap_of = |alt: &str| -> String {
        let c = RE_MD_IMG_CAP_CLEAN.replace(alt, "");
        let c = rd_inline_str(&c);
        if c.len() >= 6 && !RE_MD_IMG_JUNK_CAP.is_match(&c) {
            if c.len() > 220 {
                c[..220].to_string()
            } else {
                c
            }
        } else {
            String::new()
        }
    };

    for raw in s_text.split('\n') {
        if done {
            break;
        }
        let t = raw.trim();
        if t.is_empty() {
            continue;
        }
        if RE_MD_IGNORE_LINE.is_match(t) {
            continue;
        }
        if let Some(tm) = RE_MD_TITLE_LINE.captures(t) {
            if title.is_empty() && !BOT_CHALLENGE_RE.is_match(&tm[1]) {
                title = tm[1].trim().to_string();
            }
            continue;
        }
        if t.starts_with("```") {
            continue;
        }
        if t.starts_with('#') {
            let hd = t.trim_start_matches('#').trim();
            if title.is_empty() && !BOT_CHALLENGE_RE.is_match(hd) {
                title = hd.to_string();
            } else if paras.len() >= 2 && RE_MD_HEADING_STOP.is_match(hd) {
                break;
            } else if !paras.is_empty() {
                emit_md_text(
                    hd.to_string(),
                    "h",
                    &mut paras,
                    &mut blocks,
                    &mut seen,
                    &mut done,
                );
            }
            continue;
        }
        if t.starts_with('>') {
            let qt = t.trim_start_matches('>').trim();
            emit_md_text(
                qt.to_string(),
                "q",
                &mut paras,
                &mut blocks,
                &mut seen,
                &mut done,
            );
            continue;
        }
        if RE_MD_LIST_ITEM.is_match(t) {
            let li = RE_MD_LIST_ITEM.replace(t, "");
            emit_md_text(
                li.to_string(),
                "li",
                &mut paras,
                &mut blocks,
                &mut seen,
                &mut done,
            );
            continue;
        }

        struct LineImg {
            url: String,
            cap: String,
            alt: String,
            off: usize,
        }
        let mut line_imgs = Vec::new();

        let mut lt = String::with_capacity(t.len());
        let mut last_idx = 0;
        for cap in RE_MD_IMG.captures_iter(t) {
            let Some(m) = cap.get(0) else {
                continue;
            };
            let off = m.start();
            let u1 = cap.get(2).map(|x| x.as_str());
            let u2 = cap.get(5).map(|x| x.as_str());
            let raw_u = u1.or(u2).unwrap_or("");
            let iu = rd_abs_str(raw_u, &url_str);

            let a1 = cap.get(1).map(|x| x.as_str());
            let a2 = cap.get(4).map(|x| x.as_str());
            let ia = a1.or(a2).unwrap_or("");

            let link1 = cap.get(3).map(|x| x.as_str()).unwrap_or("");
            let mut skip_img = false;
            if !link1.is_empty() {
                let base_url = Url::parse(&url_str).ok();
                if let Ok(lx) = Url::options().base_url(base_url.as_ref()).parse(link1) {
                    let is_img_link = RE_IMG_EXT.is_match(lx.path());
                    if !is_img_link && lx.path().len() > 8 && lx.path() != cur_link_path {
                        skip_img = true;
                    }
                }
            }

            if !skip_img && !iu.is_empty() {
                line_imgs.push(LineImg {
                    url: iu,
                    cap: cap_of(ia),
                    alt: rd_inline_str(ia),
                    off,
                });
            }
            lt.push_str(&t[last_idx..m.start()]);
            lt.push(' ');
            last_idx = m.end();
        }
        lt.push_str(&t[last_idx..]);

        if line_imgs.is_empty() {
            emit_md_text(lt, "p", &mut paras, &mut blocks, &mut seen, &mut done);
            continue;
        }

        let txt_len = RE_WS_PLUS.replace_all(&lt, " ").trim().len();
        let first_off = line_imgs[0].off;
        let threshold = 20.max((t.len() as f64 * 0.4) as usize);

        let push_line_imgs = |blocks: &mut Vec<Value>, is_done: bool| {
            if !is_done {
                for im in &line_imgs {
                    blocks.push(json!({
                        "k": "i",
                        "img": {
                            "url": im.url,
                            "cap": im.cap,
                            "alt": im.alt,
                        }
                    }));
                }
            }
        };

        if txt_len == 0 || first_off <= threshold {
            push_line_imgs(&mut blocks, done);
            emit_md_text(lt, "p", &mut paras, &mut blocks, &mut seen, &mut done);
        } else {
            emit_md_text(lt, "p", &mut paras, &mut blocks, &mut seen, &mut done);
            push_line_imgs(&mut blocks, done);
        }
    }

    title = clean_article_title_str(&title, &rd_host_str(&url_str), &url_str);

    let mut wire_set = HashSet::new();
    if let Value::Array(arr) = wire_titles {
        for item in arr {
            let s = js_to_string(&item);
            let n = RE_NON_WORD
                .replace_all(&s.to_lowercase(), " ")
                .trim()
                .to_string();
            wire_set.insert(n);
        }
    }

    let (pruned_blocks, bp_dropped, dp_dropped) = prune_blocks_impl(&blocks, &title, &wire_set);

    let final_paras: Vec<String> = pruned_blocks
        .iter()
        .filter(|b| {
            let k = b.get("k").and_then(|v| v.as_str()).unwrap_or("");
            k == "p" || k == "q" || k == "li"
        })
        .filter_map(|b| b.get("t").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .collect();

    let quality = compute_text_quality_impl(
        &title,
        expected_title,
        &url_str,
        "",
        &pruned_blocks,
        bp_dropped,
        dp_dropped,
    );

    let desc = final_paras
        .first()
        .map(|p| {
            if p.len() > 300 {
                p[..300].to_string()
            } else {
                p.clone()
            }
        })
        .unwrap_or_default();

    let site_clean = clean_site_name_impl("", &url_str);
    let mut title_out = title;
    if title_out.is_empty() {
        title_out = if !site_clean.is_empty() {
            site_clean.clone()
        } else {
            "Tanpa judul".to_string()
        };
    }

    Ok(json!({
        "ok": !quality.is_challenge_or_error && quality.is_on_topic,
        "offTopic": !quality.is_on_topic,
        "url": url,
        "final": url,
        "title": title_out,
        "site": site_clean,
        "byline": "",
        "time": "",
        "img": "",
        "desc": desc,
        "paras": final_paras,
        "thin": !quality.is_full_article,
        "isFallback": !quality.is_full_article,
        "textSource": "jina_markdown",
        "videos": [],
        "blocks": pruned_blocks,
        "via": "jina",
        "extraction": {
            "method": "jina_markdown",
            "isFallback": !quality.is_full_article,
            "quality": quality.to_value(),
        }
    }))
}
