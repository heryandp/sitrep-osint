//! The Rust port of `src/article-media.js`.

#![forbid(unsafe_code)]

use serde_json::{Value, json};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::sync::LazyLock;

const PIPELINE_VERSION: i64 = 4;

static TRACKING_PARAM_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)^(?:utm_[a-z0-9_]+|fbclid|gclid|dclid|msclkid|yclid|twclid|igshid|mc_cid|mc_eid|_ga|_gl|ga_[a-z_]+|guce_referrer(?:_sig)?|ns_[a-z_]+|oc|ved|usg|sa|spm|fb_action_ids|fb_action_types|fb_source|ref_src|ref_url)$").expect("TRACKING_PARAM_RE")
});

static RD_IMG_JUNK: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)logo|logotype|wordmark|brandmark|favicon|apple-touch-icon|android-chrome|mstile|sprite|avatar|badge|placeholder|blank|pixel|spacer|advert|/ads?/|[-_/.]ads?[-_/.]|banner|emoji|emoticon|button|share|loading|lazy\.|default[-_]|no[-_]?image|not[-_]?found|gravatar|pravatar|thumb(?:nail)?s?/small|image[-_]?404|404[-_]?image").expect("RD_IMG_JUNK")
});

static RD_JUNK_URL: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    fancy_regex::Regex::new(r"(?i)appstore|playstore|googleplay|gettheapp|framebarasset|asset-template|(?:^|/)slide[-_]|assets_frontend/images|/assets/(?:medsos|img|images|icons?)/(?:ui|icons?|buttons?|social|medsos)?|share[-_ ]?preview|social[-_ ]?(?:share|card|icon)|og[-_]?(?:default|image)\b|partials/|widgets-nocache|analytics_page_request|inpicture_widget|wartawan|scorecard|criteo|doubleclick|googlesyndication|googleadservices|adservice|hotjar|facebook\.net|img\.youtube\.com|(^|/)ic_[a-z0-9]|en-flag|id-flag|(?:^|/)thumb_(?:small|mini|tiny|xs|sm|default|placeholder|icon|avatar|\d{1,4}\.)|/mini/|/small/|/img/site/|foot[-_ ]?network|foto[-_/]bank(?!/originals)|fotobank|/(?:tautan|lampiran)/|image[-_]?404|404[-_]?image|ai[-_ ]?image[-_ ]?generator|/avatars?/|/authors?/|/penulis/|/icons?/|/logos?/").expect("RD_JUNK_URL")
});

static RD_JUNK_BASE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)^(?:google|gnews|googlenews|telegram|threads|bluesky|pinterest|line|ig|fb|twitter|x|tiktok|instagram|youtube|wa|whatsapp|linkedin|googleplay|appstore|playstore|en-flag|id-flag|favicon|logo|icon|avatar|placeholder|blank|spacer|pixel|spinner|loading|default)([-_. ]|$)").expect("RD_JUNK_BASE")
});

static UNSUPPORTED_MIME_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)^(?:image/svg\+xml|image/x-icon|image/vnd\.microsoft\.icon|text/|application/(?:json|xml|xhtml\+xml|javascript)|video/|audio/)").expect("UNSUPPORTED_MIME_RE")
});

static NON_EDITORIAL_DOM_CLASS: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)(?:^|[\s_-])(?:logo|brand|favicon|icon|avatar|author|wartawan|penulis|byline|profile|social|share|badge|emoji|emoticon|reaction|rating|like|dislike|vote|poll|banner|advert|iklan|promo|placeholder|spacer|pixel|blank|sprite|watermark|related|sidebar|widget)(?:$|[\s_-])").expect("NON_EDITORIAL_DOM_CLASS")
});

static NON_EDITORIAL_FILENAME: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)(?:^|[-_.])(logo|logotype|wordmark|favicon|apple-touch-icon|sprite|watermark|placeholder|avatar|icons?|default|image[-_]?404|404[-_]?image|no[-_]?image|not[-_]?found)(?:[-_.\d]|$)").expect("NON_EDITORIAL_FILENAME")
});

static NON_EDITORIAL_ALT_START: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)^(?:logo|favicon|icon|avatar|share|whatsapp|facebook|twitter|telegram|linkedin|advertisement|iklan|banner|placeholder|emoji|reaction|like|dislike|menu|search|no[-_ ]?image)\b").expect("NON_EDITORIAL_ALT_START")
});

static NON_EDITORIAL_ALT_SUBSTR: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)\b(?:publisher logo|site logo|author photo|foto penulis|foto wartawan|ai[-_ ]?image[-_ ]?generator|follow us|bagikan ke)\b").expect("NON_EDITORIAL_ALT_SUBSTR")
});

static RD_CONT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r#"(?i)<(article|div|section|main)\b[^>]*(?:(?:class|id)\s*=\s*["'][^"']*(?:detail-desc|article-content|article-body|content-detail|detail-content|post-content|entry-content|berita-content|field-name-body|txt-berita|detail_body|news-content|content-body|post-body|article__body|detail__body-text|itp_bodycontent|detail__body|txt-article|side-article|detail-text|article-content-body|read__content|read__article|story-body|artikel-isi|isi-berita)[^"']*["']|itemprop\s*=\s*["']articleBody["'])[^>]*>"#).expect("RD_CONT_RE")
});

static RD_LEAD_MEDIA_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r#"(?i)<(div|figure|section)\b[^>]*(?:class|id)\s*=\s*["'][^"']*(?:detail__media|detail-image|photo__wrap|photo-cover|featured-image|post-thumbnail|article-photo|cover-photo|img-article|media-cover|max-card__image|artimg)[^"']*["'][^>]*>"#).expect("RD_LEAD_MEDIA_RE")
});

static RD_LEAD_IMG_TAG_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r#"(?i)<img\b[^>]*(?:class|id)\s*=\s*["'][^"']*(?:featured[-_]?img|featured[-_]?image|article[-_]?featured|post[-_]?thumbnail|hero[-_]?img|detail[-_]?img|cover[-_]?img|wp-post-image)[^"']*["'][^>]*>"#).expect("RD_LEAD_IMG_TAG_RE")
});

static ARTICLE_BODY_KEEP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)(?:detail-desc|article-content|article-body|content-detail|detail-content|post-content|entry-content|berita-content|field-name-body|txt-berita|detail_body|news-content|content-body|post-body|article__body|detail__body-text|itp_bodycontent|detail__body|txt-article|detail-text|article-content-body|read__content|read__article|story-body|artikel-isi|isi-berita)").expect("ARTICLE_BODY_KEEP_RE")
});

static WA_IMAGE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)^(?:whatsapp|wa)[-_ ]image").expect("WA_IMAGE")
});

static NOT_IMAGE_HEAD: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"^(?:<!doctype|<html|<head|<svg|<\?xml|\{)").expect("NOT_IMAGE_HEAD")
});

static JUNK_BOX_TOKEN_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)(?:^|[\s_-])(?:reactions?|rating|emoticons?|emoji|votes?|polls?|like-box|dislike|social-share|article-tools|post-tools|share-tools|share-box|share-bar|share-buttons?|follow-us|komentar|comments?(?:-box|-list|-section|-area)?|disqus|author-(?:box|card|img|image|photo|profile|info|bio|wrap|detail)|wartawan|penulis|byline|redaksi|editor-box|profile-(?:box|img|image)|related(?:-(?:news|articles?|posts?|stories|box|wrap|list|section))?|baca-juga|baca_juga|bacajuga|berita-terkait|artikel-terkait|linksisip|sisip|recommend(?:ed|ations?)?|most-read|most-popular|injected|widget|latest-(?:news|post|stories)|popular(?:-(?:news|post|box))?|trending|berita-lain|more-(?:news|stories)|also-read|you-may|jnews_|td-related|jeg_(?:block|sidebar)|advert(?:isement|orial)?|iklan|ad-(?:slot|box|unit|banner|container|wrap)|ads-(?:box|slot|wrap|container)|sponsor(?:ed)?|stripbanner|ticker|breaking-news|see-also|summary-article|read-page--summary|inbetween|banner_article_slide|newsletter|subscribe(?:-box|-form)?|cookie(?:-banner|-notice|-consent|-popup)?|gdpr|breadcrumb|pagination|page-nav|tag-list|tags-box|topik-terkait|sr-only|visually-hidden|screen-reader|assistive)(?:$|[\s_-])").expect("JUNK_BOX_TOKEN_RE")
});

static SIDEBAR_NEG_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)(?:no|without|has|with)-sidebar|sidebar-layout")
        .expect("SIDEBAR_NEG_RE")
});

static BOT_CHALLENGE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
    // INFALLIBLE: static regex syntax verified
    regex::Regex::new(r"(?i)captcha|recaptcha|lsrecaptcha|cf-chl|cf-turnstile|challenge-platform|just a moment|one moment,?\s*please|attention required|access denied|verify you are|are you a robot|human verification|checking your browser|please wait while we verify|enable javascript and cookies|bot verification|ddos-guard|pardon our interruption|security check|sorry, you have been blocked|situs ini mungkin memblokir|requiring captcha").expect("BOT_CHALLENGE_RE")
});

fn js_string_of(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(0.0)),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let mut parts = Vec::with_capacity(arr.len());
            for item in arr {
                if item.is_null() || crate::js::is_undefined(item) {
                    parts.push(String::new());
                } else {
                    parts.push(js_string_of(item));
                }
            }
            parts.join(",")
        }
        Value::Object(obj) => {
            if crate::js::is_undefined(val) {
                "undefined".to_string()
            } else if let Some(n) = crate::js::as_number(val) {
                crate::js::number_to_string(n)
            } else if let Some(units) = crate::js::string_units(val) {
                String::from_utf16_lossy(&units)
            } else if let Some(arr) = obj
                .get("$buffer")
                .or_else(|| obj.get("$bytes"))
                .and_then(|v| v.as_array())
            {
                let parts: Vec<String> = arr
                    .iter()
                    .map(|b| b.as_u64().unwrap_or(0).to_string())
                    .collect();
                parts.join(",")
            } else {
                "[object Object]".to_string()
            }
        }
    }
}

fn js_string_or_falsy_empty(val: &Value) -> String {
    if val.is_null() || crate::js::is_undefined(val) {
        return String::new();
    }
    if let Some(b) = val.as_bool() {
        if !b {
            return String::new();
        }
        return "true".to_string();
    }
    if let Some(n) = val.as_f64() {
        if n == 0.0 {
            return String::new();
        }
        return crate::js::number_to_string(n);
    }
    js_string_of(val)
}

fn rd_chr(n: u32) -> String {
    if n <= 0x10ffff {
        std::char::from_u32(n)
            .map(|c| c.to_string())
            .unwrap_or_default()
    } else {
        String::new()
    }
}

fn decode_entities(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '&' {
            let mut ent = String::new();
            while let Some(&next) = chars.peek() {
                if next == ';' {
                    chars.next();
                    break;
                }
                if next.is_ascii_alphanumeric() || next == '#' {
                    ent.push(chars.next().unwrap());
                    if ent.len() > 10 {
                        break;
                    }
                } else {
                    break;
                }
            }
            if ent.starts_with("#x") || ent.starts_with("#X") {
                if let Ok(n) = u32::from_str_radix(&ent[2..], 16) {
                    out.push_str(&rd_chr(n));
                    continue;
                }
            } else if let Some(stripped) = ent.strip_prefix('#') {
                if let Ok(n) = stripped.parse::<u32>() {
                    out.push_str(&rd_chr(n));
                    continue;
                }
            } else {
                let lower = ent.to_lowercase();
                let rep = match lower.as_str() {
                    "amp" => Some("&"),
                    "lt" => Some("<"),
                    "gt" => Some(">"),
                    "quot" => Some("\""),
                    "apos" => Some("'"),
                    "nbsp" => Some(" "),
                    "hellip" => Some("…"),
                    "mdash" => Some("—"),
                    "ndash" => Some("–"),
                    "lsquo" => Some("‘"),
                    "rsquo" => Some("’"),
                    "ldquo" => Some("“"),
                    "rdquo" => Some("”"),
                    "laquo" => Some("«"),
                    "raquo" => Some("»"),
                    "bull" => Some("•"),
                    "middot" => Some("·"),
                    "copy" => Some("©"),
                    "reg" => Some("®"),
                    "trade" => Some("™"),
                    "ntilde" => Some("ñ"),
                    "eacute" => Some("é"),
                    "egrave" => Some("è"),
                    "agrave" => Some("à"),
                    "aacute" => Some("á"),
                    "iacute" => Some("í"),
                    "oacute" => Some("ó"),
                    "uacute" => Some("ú"),
                    "uuml" => Some("ü"),
                    "ouml" => Some("ö"),
                    "auml" => Some("ä"),
                    "ccedil" => Some("ç"),
                    "acirc" => Some("â"),
                    "ecirc" => Some("ê"),
                    "ocirc" => Some("ô"),
                    "icirc" => Some("î"),
                    "ucirc" => Some("û"),
                    "szlig" => Some("ß"),
                    "oslash" => Some("ø"),
                    "aring" => Some("å"),
                    "aelig" => Some("æ"),
                    "iexcl" => Some("¡"),
                    "iquest" => Some("¿"),
                    "plusmn" => Some("±"),
                    "frac12" => Some("½"),
                    "shy" => Some(""),
                    "ensp" => Some(" "),
                    "emsp" => Some(" "),
                    "thinsp" => Some(" "),
                    "zwnj" => Some(""),
                    "zwj" => Some(""),
                    "deg" => Some("°"),
                    "times" => Some("×"),
                    "divide" => Some("÷"),
                    "euro" => Some("€"),
                    "pound" => Some("£"),
                    "yen" => Some("¥"),
                    "sect" => Some("§"),
                    "para" => Some("¶"),
                    _ => None,
                };
                if let Some(r) = rep {
                    out.push_str(r);
                    continue;
                }
            }
            out.push('&');
            out.push_str(&ent);
        } else {
            out.push(c);
        }
    }
    out
}

fn rd_clean(s: &str) -> String {
    // INFALLIBLE: static regex syntax verified
    static BR_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<br\s*/?>").unwrap());
    static CLOSE_BLOCK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)</(p|div|li|h[1-6]|blockquote|figcaption|tr|section|article)>")
            .unwrap()
    });
    static TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<[^>]*>").unwrap());
    static SPACES_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"[ \t\x0C\x0B]+").unwrap());
    static LINE_SPACES_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r" *\n *").unwrap());
    static MULTI_NL_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\n{2,}").unwrap());

    let s1 = BR_RE.replace_all(s, "\n");
    let s2 = CLOSE_BLOCK_RE.replace_all(&s1, "\n");
    let s3 = TAG_RE.replace_all(&s2, " ");
    let s4 = decode_entities(&s3);
    let s5 = s4.replace('\u{00A0}', " ");
    let s6 = SPACES_RE.replace_all(&s5, " ");
    let s7 = LINE_SPACES_RE.replace_all(&s6, "\n");
    let s8 = MULTI_NL_RE.replace_all(&s7, "\n");
    s8.trim().to_string()
}

fn js_trim(s: &str) -> &str {
    s.trim_matches(|c: char| c.is_whitespace() || c == '\u{feff}')
}

fn rd_inline(s: &str) -> String {
    // INFALLIBLE: static regex syntax verified
    static WS_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\s+").unwrap());
    js_trim(&WS_RE.replace_all(&rd_clean(s), " ")).to_string()
}

fn parse_attrs(tag_str: &str) -> serde_json::Map<String, Value> {
    // INFALLIBLE: static regex syntax verified
    static STRIP_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^<\s*[a-zA-Z0-9:-]+").unwrap());
    static STRIP_END_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"/?\s*>$").unwrap());
    static ATTR_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"([a-zA-Z_:][-a-zA-Z0-9_:.]*)\s*(?:=\s*(?:"([^"]*)"|'([^']*)'|([^\s"'=<>`]+)))?"#,
        )
        .unwrap()
    });

    let s1 = STRIP_TAG_RE.replace(tag_str, "");
    let body = STRIP_END_RE.replace(&s1, "");
    let mut out = serde_json::Map::new();
    for cap in ATTR_RE.captures_iter(&body) {
        let key = cap[1].to_lowercase();
        let raw_val = cap
            .get(2)
            .or_else(|| cap.get(3))
            .or_else(|| cap.get(4))
            .map(|m| m.as_str())
            .unwrap_or("");
        out.insert(key, Value::String(decode_entities(raw_val)));
    }
    out
}

#[inline]
fn get_attr<'a>(a: &'a serde_json::Map<String, Value>, key: &str) -> Option<&'a str> {
    a.get(key).and_then(|v| v.as_str())
}

fn rd_abs(u: &str, base: &str) -> String {
    if u.is_empty() {
        return String::new();
    }
    let raw = decode_entities(u.trim());
    if raw.is_empty() {
        return String::new();
    }
    // INFALLIBLE: static regex syntax verified
    static PROTO_SKIP: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^(?:data:|javascript:|blob:|mailto:|tel:|about:)").unwrap()
    });
    if PROTO_SKIP.is_match(&raw) {
        return String::new();
    }
    let base_url = match url::Url::parse(base) {
        Ok(b) => b,
        Err(_) => return String::new(),
    };
    match base_url.join(&raw) {
        Ok(u) => {
            if u.scheme() == "http" || u.scheme() == "https" {
                u.to_string()
            } else {
                String::new()
            }
        }
        Err(_) => String::new(),
    }
}

/// Port of `canonicalArticleUrl` (`src/article-media.js:10`).
pub fn canonical_article_url(raw: Value) -> Result<Value, String> {
    let s = match raw.as_str() {
        Some(s) if !s.is_empty() => s.trim(),
        _ => return Ok(json!("")),
    };
    let mut u = match url::Url::parse(s) {
        Ok(u) => u,
        Err(_) => return Ok(json!("")),
    };
    if u.scheme() != "http" && u.scheme() != "https" {
        return Ok(json!(""));
    }
    u.set_fragment(None);

    let query_pairs: Vec<(String, String)> = u
        .query_pairs()
        .filter(|(k, _)| !TRACKING_PARAM_RE.is_match(k))
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();

    if query_pairs.is_empty() {
        u.set_query(None);
    } else {
        let mut ser = u.query_pairs_mut();
        ser.clear();
        for (k, v) in query_pairs {
            ser.append_pair(&k, &v);
        }
    }

    // INFALLIBLE: static regex syntax verified
    static SLASH_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"/{2,}").unwrap());
    let path = SLASH_RE.replace_all(u.path(), "/").to_string();
    let final_path = if path.len() > 1 && path.ends_with('/') && u.query().is_none() {
        &path[..path.len() - 1]
    } else {
        &path
    };
    u.set_path(final_path);
    Ok(json!(u.to_string()))
}

/// Port of `regDomain` (`src/article-media.js:34`).
pub fn reg_domain(u: Value) -> Result<Value, String> {
    let raw = match u.as_str() {
        Some(s) => s,
        None => return Ok(json!("")),
    };
    let parsed = match url::Url::parse(raw) {
        Ok(p) => p,
        Err(_) => return Ok(json!("")),
    };
    let host = match parsed.host_str() {
        Some(h) => h.to_lowercase(),
        None => return Ok(json!("")),
    };
    let host_clean = host.strip_prefix("www.").unwrap_or(&host);
    let parts: Vec<&str> = host_clean.split('.').collect();
    if parts.len() <= 2 {
        return Ok(json!(parts.join(".")));
    }
    let sld = parts[parts.len() - 2];
    // INFALLIBLE: static regex syntax verified
    static SLD_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^(?:co|or|go|ac|sch|net|web|my|biz|mil|com|edu|gov|org)$").unwrap()
    });
    if SLD_RE.is_match(sld) && parts.last().map_or(0, |p| p.len()) == 2 {
        Ok(json!(parts[parts.len() - 3..].join(".")))
    } else {
        Ok(json!(parts[parts.len() - 2..].join(".")))
    }
}

/// Port of `resolveUrl` (`src/article-media.js:48`).
pub fn resolve_url(raw: Value, base: Value) -> Result<Value, String> {
    if raw.is_null() || crate::js::is_undefined(&raw) {
        return Ok(json!(""));
    }
    if let Some(b) = raw.as_bool()
        && !b
    {
        return Ok(json!(""));
    }
    if let Some(n) = raw.as_f64()
        && n == 0.0
    {
        return Ok(json!(""));
    }
    let raw_str = js_string_of(&raw);
    let base_str = match base.as_str() {
        Some(s) => s,
        None => return Ok(json!("")),
    };
    Ok(json!(rd_abs(&raw_str, base_str)))
}

/// Port of `upgradeImage` (`src/article-media.js:52`).
pub fn upgrade_image(u: Value) -> Result<Value, String> {
    let s = match u.as_str() {
        Some(s) => s,
        None => return Ok(u),
    };
    let mut x = match url::Url::parse(s) {
        Ok(x) => x,
        Err(_) => return Ok(u),
    };
    let h = x.host_str().unwrap_or("").to_lowercase();

    // 1. Google user content
    // INFALLIBLE: static regex syntax verified
    static GUC_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(^|\.)googleusercontent\.com$").unwrap());
    if GUC_RE.is_match(&h) {
        static GUC_S: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"=s\d+(?:-[^?]*)?$").unwrap());
        static GUC_W: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"=w\d+(?:-h\d+)?(?:-[^?]*)?$").unwrap());
        let res = GUC_S.replace(s, "=s0");
        let res2 = GUC_W.replace(&res, "=w1600");
        return Ok(json!(res2.to_string()));
    }

    // 2. detik
    // INFALLIBLE: static regex syntax verified
    static DETIK_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(^|\.)detik\.(?:net\.id|com)$").unwrap());
    if DETIK_RE.is_match(&h) {
        static DETIK_EXT: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"(?i)\.(?:svg|gif|ico)(?:$|[?#])").unwrap());
        if DETIK_EXT.is_match(x.path()) {
            return Ok(json!(s));
        }
        set_search_param_min(&mut x, "w", 1200);
        set_search_param_min(&mut x, "q", 90);
        return Ok(json!(x.to_string()));
    }

    // 3. antaranews
    // INFALLIBLE: static regex syntax verified
    static ANTARA_HOST: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(^|\.)antaranews\.com$").unwrap());
    static ANTARA_CACHE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"/cache/\d+x\d+/").unwrap());
    if ANTARA_HOST.is_match(&h) && ANTARA_CACHE.is_match(x.path()) {
        let res = ANTARA_CACHE.replace(s, "/cache/1200x800/");
        return Ok(json!(res.to_string()));
    }

    // 4. tribunnews
    // INFALLIBLE: static regex syntax verified
    static TRIBUN_HOST: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(^|\.)tribunnews\.com$").unwrap());
    static TRIBUN_PATH_1: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^/[^/]+=/\d+x\d+/(?:filters:[^/]+/)?(.+)$").unwrap()
    });
    static TRIBUN_PATH_2: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^/(?:\d+x\d+/)?(?:filters:[^/]+/)?((?:[a-z0-9_-]+/)?foto/bank/.+)$")
            .unwrap()
    });
    if TRIBUN_HOST.is_match(&h) {
        let path = x.path();
        if let Some(caps) = TRIBUN_PATH_1
            .captures(path)
            .or_else(|| TRIBUN_PATH_2.captures(path))
            && let Some(m) = caps.get(1)
        {
            return Ok(json!(format!("https://asset-2.tstatic.net/{}", m.as_str())));
        }
    }

    // 5. bsinews
    // INFALLIBLE: static regex syntax verified
    static BSI_HOST: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(^|\.)(?:cdn\.)?bsinews\.id$").unwrap());
    if BSI_HOST.is_match(&h) {
        let w: i64 = get_search_param_int(&x, "width");
        if w < 1200 {
            set_search_param(&mut x, "width", "1200");
            set_search_param(&mut x, "height", "674");
            set_search_param(&mut x, "quality", "80");
            return Ok(json!(x.to_string()));
        }
    }

    Ok(json!(s))
}

fn get_search_param_int(u: &url::Url, key: &str) -> i64 {
    for (k, v) in u.query_pairs() {
        if k == key {
            return v.parse::<i64>().unwrap_or(0);
        }
    }
    0
}

fn set_search_param(u: &mut url::Url, key: &str, val: &str) {
    let mut pairs: Vec<(String, String)> = u
        .query_pairs()
        .map(|(k, v)| (k.into_owned(), v.into_owned()))
        .collect();
    let mut found = false;
    for (k, v) in &mut pairs {
        if k == key {
            *v = val.to_string();
            found = true;
            break;
        }
    }
    if !found {
        pairs.push((key.to_string(), val.to_string()));
    }
    let mut ser = u.query_pairs_mut();
    ser.clear();
    for (k, v) in pairs {
        ser.append_pair(&k, &v);
    }
}

fn set_search_param_min(u: &mut url::Url, key: &str, min_val: i64) {
    let cur = get_search_param_int(u, key);
    if cur == 0 || cur < min_val {
        set_search_param(u, key, &min_val.to_string());
    }
}

/// Port of `imageKey` (`src/article-media.js:87`).
pub fn image_key(u: Value) -> Result<Value, String> {
    let s = js_string_or_falsy_empty(&u);
    let split_q = s.split('?').next().unwrap_or("");
    let split_h = split_q.split('#').next().unwrap_or("").to_lowercase();

    // INFALLIBLE: static regex syntax verified
    static EXT_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\.\w{2,5}$").unwrap());
    static TRIBUN_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^https?://asset(?:-\d+)?\.(?:tribunnews\.com|tstatic\.net)/(?:[^/]+=/\d+x\d+/)?(?:filters:[^/]+/)?").unwrap()
    });
    static DIM_PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"/(?:thumb|fill|fit|resize|crop|cache)/?\d{2,4}(?:x|/)\d{2,4}/").unwrap()
    });
    static TRANSFORM_PATH_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"/[^/]*\b(?:w_\d+|c_fill|c_scale|f_auto|q_auto[^/]*)[^/]*/").unwrap()
    });

    static BASE_THUMB: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^(?:thumb|tn|sm|md|lg)[-_]").unwrap());
    static BASE_DIMS: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"[-_]\d{2,4}x\d{2,4}$").unwrap());
    static BASE_RATIO: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"[-_](?:169|43|11|16_9|4_3)$").unwrap());
    static BASE_LABEL: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"[-_](?:small|medium|large|thumb|thumbnail|original|full)$").unwrap()
    });
    static BASE_GENERIC: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"^(?:image|photo|foto|img|picture|default|thumb|cover|poster|index|whatsapp[-_]image)\d*$").unwrap()
    });

    let mut p = EXT_RE.replace(&split_h, "").to_string();
    p = TRIBUN_RE.replace(&p, "https://tribun-asset/").to_string();
    p = DIM_PATH_RE.replace_all(&p, "/").to_string();
    p = TRANSFORM_PATH_RE.replace_all(&p, "/").to_string();

    let raw_base = p.split('/').next_back().unwrap_or("");
    let b1 = BASE_THUMB.replace(raw_base, "");
    let b2 = BASE_DIMS.replace(&b1, "");
    let b3 = BASE_RATIO.replace(&b2, "");
    let base = BASE_LABEL.replace(&b3, "").to_string();

    if base.len() >= 8
        && base.chars().any(|c| c.is_ascii_lowercase())
        && !BASE_GENERIC.is_match(&base)
    {
        return Ok(json!(base));
    }
    Ok(json!(p))
}

/// Port of `dirKey` (`src/article-media.js:104`).
pub fn dir_key(u: Value) -> Result<Value, String> {
    let s = js_string_or_falsy_empty(&u);
    let split_q = s.split('?').next().unwrap_or("");
    let split_h = split_q.split('#').next().unwrap_or("");
    let seg: Vec<&str> = split_h.split('/').filter(|x| !x.is_empty()).collect();
    if seg.len() > 1 {
        Ok(json!(seg[seg.len() - 2]))
    } else {
        Ok(json!(""))
    }
}

/// Port of `rdSmallHint` (`src/article-media.js:109`).
pub fn rd_small_hint(u: Value) -> Result<Value, String> {
    let s = js_string_or_falsy_empty(&u);
    let path = s.split('?').next().unwrap_or("");

    // INFALLIBLE: static regex syntax verified
    static DIMS_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
        fancy_regex::Regex::new(r"(?:^|[/_=.-])(\d{2,4})x(\d{2,4})(?=[/_.-]|$)").unwrap()
    });
    for c in DIMS_RE.captures_iter(path).flatten() {
        let w = c
            .get(1)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        let h = c
            .get(2)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        if w < 400 || h < 200 {
            return Ok(json!(true));
        }
    }

    static FILL_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)/(?:fill|fit|resize|crop|w)/(\d{2,4})/(\d{2,4})/").unwrap()
    });
    if let Some(cap) = FILL_RE.captures(path) {
        let w = cap
            .get(1)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        let h = cap
            .get(2)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        if w < 400 || h < 200 {
            return Ok(json!(true));
        }
    }

    static QUERY_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
        fancy_regex::Regex::new(r"(?i)[?&](?:w|width|size)=(\d{2,4})(?=&|$)").unwrap()
    });
    if let Ok(Some(cap)) = QUERY_RE.captures(&s) {
        let w = cap
            .get(1)
            .and_then(|m| m.as_str().parse::<i64>().ok())
            .unwrap_or(0);
        if w < 400 {
            return Ok(json!(true));
        }
    }

    Ok(json!(false))
}

/// Port of `rdJunkUrl` (`src/article-media.js:126`).
pub fn rd_junk_url(u: Value) -> Result<Value, String> {
    let s = js_string_or_falsy_empty(&u);
    let path = s.split('?').next().unwrap_or("");
    if RD_JUNK_URL.is_match(path).unwrap_or(false) {
        return Ok(json!(true));
    }
    let base = path.split('/').next_back().unwrap_or("");
    // INFALLIBLE: static regex syntax verified
    static WA_TELEGRAM_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"(?i)^(?:whatsapp|wa|telegram)[-_ ](?:image|foto|img|pic|photo|doc|video)",
        )
        .unwrap()
    });
    if WA_TELEGRAM_RE.is_match(base) {
        return Ok(json!(false));
    }
    static SOCIAL_ICON_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(?:^|/)(?:i-(?:facebook|twitter|x|whatsapp|telegram|instagram|youtube|pinterest|linkedin|tiktok)|(?:wa|whatsapp|fb|ig|twitter|telegram|pinterest|linkedin|youtube|tiktok)[-_](?:icon|share|logo|btn|komunitas))[^/]*$").unwrap()
    });
    if SOCIAL_ICON_RE.is_match(path) {
        return Ok(json!(true));
    }
    Ok(json!(RD_JUNK_BASE.is_match(base)))
}

/// Port of `classifyCandidateUrl` (`src/article-media.js:135`).
pub fn classify_candidate_url(url_val: Value, ctx: Option<Value>) -> Result<Value, String> {
    let url_str = match url_val.as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(json!({ "ok": false, "reason": "invalid_url" })),
    };
    let parsed = match url::Url::parse(url_str) {
        Ok(p) => p,
        Err(_) => return Ok(json!({ "ok": false, "reason": "invalid_url" })),
    };
    if parsed.scheme() != "http" && parsed.scheme() != "https" {
        return Ok(json!({ "ok": false, "reason": "invalid_protocol" }));
    }
    let path = parsed.path();
    // INFALLIBLE: static regex syntax verified
    static VECTOR_EXT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)\.(?:svg|ico|cur)$").unwrap());
    if VECTOR_EXT.is_match(path) {
        return Ok(json!({ "ok": false, "reason": "unsupported_vector_or_icon_ext" }));
    }
    let role = ctx
        .as_ref()
        .and_then(|c| c.get("role"))
        .and_then(|r| r.as_str())
        .unwrap_or("");
    if role == "hero" && path.to_lowercase().ends_with(".gif") {
        return Ok(json!({ "ok": false, "reason": "gif_rejected_for_hero" }));
    }
    if rd_junk_url(Value::String(url_str.to_string()))?
        .as_bool()
        .unwrap_or(false)
    {
        return Ok(json!({ "ok": false, "reason": "junk_url_pattern" }));
    }
    let fn_str = path.split('/').next_back().unwrap_or("");
    if NON_EDITORIAL_FILENAME.is_match(fn_str) {
        return Ok(json!({ "ok": false, "reason": "non_editorial_filename" }));
    }
    if RD_IMG_JUNK.is_match(path) {
        return Ok(json!({ "ok": false, "reason": "non_editorial_path" }));
    }
    let up_val = upgrade_image(Value::String(url_str.to_string()))?;
    let up_str = up_val.as_str().unwrap_or(url_str);
    let hint_orig = rd_small_hint(Value::String(url_str.to_string()))?
        .as_bool()
        .unwrap_or(false);
    if hint_orig {
        let hint_up = if up_str == url_str {
            true
        } else {
            rd_small_hint(Value::String(up_str.to_string()))?
                .as_bool()
                .unwrap_or(false)
        };
        if hint_up {
            return Ok(json!({ "ok": false, "reason": "small_dimension_url_hint" }));
        }
    }

    Ok(json!({ "ok": true }))
}

/// Port of `classifyCandidateAttrs` (`src/article-media.js:156`).
pub fn classify_candidate_attrs(attrs_val: Value) -> Result<Value, String> {
    if attrs_val.is_null() {
        return Err("Cannot read properties of null (reading 'class')".to_string());
    }
    if crate::js::is_undefined(&attrs_val) {
        return Ok(json!({ "ok": true }));
    }
    let obj = match attrs_val.as_object() {
        Some(o) => o,
        None => return Ok(json!({ "ok": true })),
    };

    let class_str = obj.get("class").map(js_string_of).unwrap_or_default();
    let id_str = obj.get("id").map(js_string_of).unwrap_or_default();
    let role_str = obj.get("role").map(js_string_of).unwrap_or_default();
    let cls = format!("{class_str} {id_str} {role_str}").to_lowercase();
    if NON_EDITORIAL_DOM_CLASS.is_match(&cls) {
        return Ok(json!({ "ok": false, "reason": "non_editorial_dom_class" }));
    }

    let alt = obj
        .get("alt")
        .or_else(|| obj.get("title"))
        .map(js_string_of)
        .unwrap_or_default();
    let alt_trimmed = alt.trim();
    if !alt_trimmed.is_empty()
        && !WA_IMAGE.is_match(alt_trimmed)
        && (NON_EDITORIAL_ALT_START.is_match(alt_trimmed)
            || NON_EDITORIAL_ALT_SUBSTR.is_match(alt_trimmed))
    {
        return Ok(json!({ "ok": false, "reason": "non_editorial_alt_text" }));
    }

    let raw_w = obj.get("width").map(js_string_of).unwrap_or_default();
    let raw_h = obj.get("height").map(js_string_of).unwrap_or_default();
    let raw_w_trim = raw_w.trim();
    let raw_h_trim = raw_h.trim();
    let w = if !raw_w_trim.ends_with('%') {
        parse_js_int(raw_w_trim)
    } else {
        0
    };
    let h = if !raw_h_trim.ends_with('%') {
        parse_js_int(raw_h_trim)
    } else {
        0
    };

    if (w > 0 && w < 200) || (h > 0 && h < 120) {
        return Ok(json!({ "ok": false, "reason": "small_html_attr_dimensions" }));
    }
    if w >= 200 && h > 0 && (w as f64 / h as f64) > 3.2 {
        return Ok(json!({ "ok": false, "reason": "extreme_banner_html_attrs" }));
    }

    Ok(json!({ "ok": true }))
}

fn parse_js_int(s: &str) -> i64 {
    let trimmed = s.trim();
    let mut digits = String::new();
    for (i, c) in trimmed.chars().enumerate() {
        if (i == 0 && (c == '+' || c == '-')) || c.is_ascii_digit() {
            digits.push(c);
        } else {
            break;
        }
    }
    digits.parse::<i64>().unwrap_or(0)
}

fn parse_isobmff_size(buf: &[u8]) -> Option<(u32, u32)> {
    let ispe = b"ispe";
    if let Some(pos) = buf.windows(4).position(|w| w == ispe)
        && pos >= 4
        && pos + 16 <= buf.len()
    {
        let w = u32::from_be_bytes(buf[pos + 8..pos + 12].try_into().unwrap());
        let h = u32::from_be_bytes(buf[pos + 12..pos + 16].try_into().unwrap());
        if w > 0 && w < 30000 && h > 0 && h < 30000 {
            return Some((w, h));
        }
    }
    None
}

fn sniff_image_size(buf: &[u8]) -> Option<(u32, u32, &'static str)> {
    if buf.len() < 12 {
        return None;
    }

    // 1. PNG: 89 50 4E 47
    if buf.len() >= 24 && buf[0] == 0x89 && &buf[1..4] == b"PNG" {
        let w = u32::from_be_bytes(buf[16..20].try_into().unwrap());
        let h = u32::from_be_bytes(buf[20..24].try_into().unwrap());
        return Some((w, h, "image/png"));
    }

    // 2. GIF
    if buf.len() >= 10 && (&buf[0..6] == b"GIF87a" || &buf[0..6] == b"GIF89a") {
        let w = u16::from_le_bytes(buf[6..8].try_into().unwrap()) as u32;
        let h = u16::from_le_bytes(buf[8..10].try_into().unwrap()) as u32;
        return Some((w, h, "image/gif"));
    }

    // 3. JPEG: FF D8
    if buf[0] == 0xFF && buf[1] == 0xD8 {
        let mut i = 2;
        while i + 3 < buf.len() {
            if buf[i] != 0xFF {
                i += 1;
                continue;
            }
            let m = buf[i + 1];
            if m == 0xFF {
                i += 1;
                continue;
            }
            if m == 0x00 || m == 0xD8 || m == 0x01 || (0xD0..=0xD7).contains(&m) {
                i += 2;
                continue;
            }
            if m == 0xD9 || m == 0xDA {
                break;
            }
            if i + 4 > buf.len() {
                break;
            }
            let len = u16::from_be_bytes(buf[i + 2..i + 4].try_into().unwrap()) as usize;
            if len < 2 {
                break;
            }
            if (0xC0..=0xCF).contains(&m) && m != 0xC4 && m != 0xC8 && m != 0xCC {
                if i + 9 > buf.len() {
                    break;
                }
                let h = u16::from_be_bytes(buf[i + 5..i + 7].try_into().unwrap()) as u32;
                let w = u16::from_be_bytes(buf[i + 7..i + 9].try_into().unwrap()) as u32;
                return Some((w, h, "image/jpeg"));
            }
            i += 2 + len;
        }
    }

    // 4. WEBP: RIFF ... WEBP
    if buf.len() >= 25 && &buf[0..4] == b"RIFF" && &buf[8..12] == b"WEBP" {
        let t = &buf[12..16];
        if t == b"VP8X" && buf.len() >= 30 {
            let w = 1 + (buf[24] as u32 | ((buf[25] as u32) << 8) | ((buf[26] as u32) << 16));
            let h = 1 + (buf[27] as u32 | ((buf[28] as u32) << 8) | ((buf[29] as u32) << 16));
            return Some((w, h, "image/webp"));
        }
        if t == b"VP8 " && buf.len() >= 30 {
            let needle = [0x9d, 0x01, 0x2a];
            if let Some(pos) = buf.windows(3).position(|w| w == needle)
                && pos > 0
                && pos + 7 <= buf.len()
            {
                let w =
                    (u16::from_le_bytes(buf[pos + 3..pos + 5].try_into().unwrap()) & 0x3fff) as u32;
                let h =
                    (u16::from_le_bytes(buf[pos + 5..pos + 7].try_into().unwrap()) & 0x3fff) as u32;
                return Some((w, h, "image/webp"));
            }
        }
        if t == b"VP8L" && buf.len() >= 25 {
            let b = u32::from_le_bytes(buf[21..25].try_into().unwrap());
            let w = 1 + (b & 0x3fff);
            let h = 1 + ((b >> 14) & 0x3fff);
            return Some((w, h, "image/webp"));
        }
    }

    // 5. AVIF / HEIC: ftyp at offset 4
    if buf.len() >= 16 && &buf[4..8] == b"ftyp" {
        let brand = std::str::from_utf8(&buf[8..12]).unwrap_or("");
        // INFALLIBLE: static regex syntax verified
        static AVIF_BRAND: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"(?i)^(?:avif|avis|mif1|heic|heix)$").unwrap());
        if AVIF_BRAND.is_match(brand)
            && let Some((w, h)) = parse_isobmff_size(buf)
        {
            return Some((w, h, "image/avif"));
        }
    }

    // 6. BMP: BM
    if buf.len() >= 26 && &buf[0..2] == b"BM" {
        let w = i32::from_le_bytes(buf[18..22].try_into().unwrap());
        let h = i32::from_le_bytes(buf[22..26].try_into().unwrap()).abs();
        if w > 0 && h > 0 {
            return Some((w as u32, h as u32, "image/bmp"));
        }
    }

    None
}

fn extract_buffer_bytes(buf_val: &Value) -> Option<Vec<u8>> {
    if let Some(arr) = buf_val.as_array() {
        return Some(
            arr.iter()
                .filter_map(|x| x.as_u64().map(|b| b as u8))
                .collect(),
        );
    }
    if let Some(obj) = buf_val.as_object()
        && let Some(arr) = obj
            .get("$buffer")
            .or_else(|| obj.get("$bytes"))
            .and_then(|v| v.as_array())
    {
        return Some(
            arr.iter()
                .filter_map(|x| x.as_u64().map(|b| b as u8))
                .collect(),
        );
    }
    None
}

/// Port of `imageSize` (`src/article-media.js:187`).
pub fn image_size(buf: Value) -> Result<Value, String> {
    let bytes = match extract_buffer_bytes(&buf) {
        Some(b) => b,
        None => return Ok(Value::Null),
    };
    match sniff_image_size(&bytes) {
        Some((w, h, mime)) => Ok(json!({ "w": w, "h": h, "mime": mime })),
        None => Ok(Value::Null),
    }
}

/// Port of `inspectImageBuffer` (`src/article-media.js:246`).
pub fn inspect_image_buffer(buf: Value, content_type: Option<String>) -> Result<Value, String> {
    let raw_ct = content_type.unwrap_or_default();
    let first_part = raw_ct.split(';').next().unwrap_or("");
    let ct = js_trim(first_part).to_lowercase();

    if !ct.is_empty() && UNSUPPORTED_MIME_RE.is_match(&ct) {
        return Ok(json!({ "ok": false, "reason": "unsupported_mime", "mime": ct }));
    }
    if !ct.is_empty() && !ct.starts_with("image/") && !ct.contains("octet-stream") {
        return Ok(json!({ "ok": false, "reason": "unsupported_mime", "mime": ct }));
    }

    // Check if buf is falsy
    if buf.is_null() || crate::js::is_undefined(&buf) {
        return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
    }
    if let Some(b) = buf.as_bool() {
        if !b {
            return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
        }
        return Err("buf.subarray is not a function".to_string());
    }
    if let Some(n) = buf.as_f64() {
        if n == 0.0 {
            return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
        }
        return Err("buf.subarray is not a function".to_string());
    }
    if let Some(s) = buf.as_str() {
        if s.is_empty() {
            return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
        }
        if s.len() < 12 {
            return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
        }
        return Err("buf.subarray is not a function".to_string());
    }
    if let Some(arr) = buf.as_array() {
        if arr.len() < 12 {
            return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
        }
        return Err("buf.subarray is not a function".to_string());
    }

    // Must be object with $buffer or $bytes
    let bytes = match extract_buffer_bytes(&buf) {
        Some(b) => b,
        None => return Err("buf.subarray is not a function".to_string()),
    };

    if bytes.len() < 12 {
        return Ok(json!({ "ok": false, "reason": "empty_or_truncated_image" }));
    }

    let head_slice = &bytes[..bytes.len().min(256)];
    let head_text = String::from_utf8_lossy(head_slice)
        .trim_start()
        .to_lowercase();
    if NOT_IMAGE_HEAD.is_match(&head_text) {
        let reason = if head_text.starts_with("<svg") || head_text.contains("<svg") {
            "svg_payload"
        } else {
            "not_an_image_payload"
        };
        return Ok(json!({ "ok": false, "reason": reason }));
    }

    if bytes[0] == 0x00
        && bytes[1] == 0x00
        && (bytes[2] == 0x01 || bytes[2] == 0x02)
        && bytes[3] == 0x00
    {
        return Ok(json!({ "ok": false, "reason": "favicon_ico_payload", "mime": "image/x-icon" }));
    }

    let dim = sniff_image_size(&bytes);
    let (w, h, sniffed_mime) = match dim {
        Some((w, h, m)) if w > 0 && h > 0 => (w, h, m),
        _ => return Ok(json!({ "ok": false, "reason": "unrecognized_or_corrupt_image" })),
    };

    let mut hasher = Sha1::new();
    hasher.update(&bytes[..bytes.len().min(32768)]);
    let fingerprint = format!("{:x}", hasher.finalize());

    let final_mime = if !sniffed_mime.is_empty() {
        sniffed_mime
    } else if !ct.is_empty() {
        ct.as_str()
    } else {
        "image/unknown"
    };

    Ok(json!({
        "ok": true,
        "w": w,
        "h": h,
        "mime": final_mime,
        "fingerprint": fingerprint,
    }))
}

/// Port of `validateImageMetrics` (`src/article-media.js:278`).
pub fn validate_image_metrics(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err("Cannot read properties of null (reading 'role')".to_string());
    }
    let obj = match arg0.as_object() {
        Some(o) => o,
        None => return Ok(json!({ "ok": false, "reason": "missing_dimensions" })),
    };

    let dim = match obj.get("dim") {
        Some(d) => d,
        None => return Ok(json!({ "ok": false, "reason": "missing_dimensions" })),
    };
    let w = dim.get("w").and_then(|v| v.as_i64()).unwrap_or(0);
    let h = dim.get("h").and_then(|v| v.as_i64()).unwrap_or(0);
    if w <= 0 || h <= 0 {
        return Ok(json!({ "ok": false, "reason": "missing_dimensions" }));
    }

    let role = obj.get("role").and_then(|v| v.as_str()).unwrap_or("hero");
    let source = obj.get("source").and_then(|v| v.as_str()).unwrap_or("og");
    let bytes = obj.get("bytes").and_then(|v| v.as_i64()).unwrap_or(0);
    let mime = obj
        .get("mime")
        .map(js_string_of)
        .unwrap_or_default()
        .to_lowercase();
    let inside_figure = obj
        .get("insideFigure")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if mime.contains("svg") || mime.contains("icon") || mime.contains("bmp") {
        return Ok(json!({ "ok": false, "reason": "unsupported_image_format", "detail": mime }));
    }

    if mime == "image/gif" && (role == "hero" || bytes < 25000 || w < 450 || h < 250) {
        return Ok(
            json!({ "ok": false, "reason": "gif_asset_rejected", "detail": format!("{w}x{h},{bytes}B") }),
        );
    }

    let is_large_modern = (mime == "image/webp" || mime == "image/avif") && w >= 600 && h >= 315;
    let min_bytes = if is_large_modern {
        3200
    } else if role == "hero" {
        5000
    } else {
        4500
    };
    if bytes > 0 && bytes < min_bytes {
        return Ok(
            json!({ "ok": false, "reason": "file_too_small", "detail": format!("{bytes}B < {min_bytes}B") }),
        );
    }

    let ar_str = format!("{:.3}", w as f64 / h as f64);
    let ar = ar_str.parse::<f64>().unwrap_or(0.0);

    if role == "hero" {
        if w < 480 || h < 250 {
            return Ok(
                json!({ "ok": false, "reason": "dimensions_too_small", "detail": format!("{w}x{h}") }),
            );
        }
        if ar > 2.6 || ar < 0.72 {
            return Ok(
                json!({ "ok": false, "reason": "invalid_aspect_ratio", "detail": format!("ar={ar}") }),
            );
        }
        let is_square = (0.88..=1.14).contains(&ar);
        if is_square && !(source == "body_lead" && inside_figure && w >= 500) {
            return Ok(
                json!({ "ok": false, "reason": "square_logo_or_icon_risk", "detail": format!("{w}x{h}") }),
            );
        }
    } else {
        if w < 400 || h < 200 {
            return Ok(
                json!({ "ok": false, "reason": "body_dimensions_too_small", "detail": format!("{w}x{h}") }),
            );
        }
        if ar > 2.8 || ar < 0.45 {
            return Ok(
                json!({ "ok": false, "reason": "body_invalid_aspect_ratio", "detail": format!("ar={ar}") }),
            );
        }
    }

    Ok(json!({ "ok": true, "aspectRatio": ar }))
}

struct CloseMatch {
    inner_end: usize,
    outer_end: usize,
}

fn find_matching_close(html: &str, open_end_idx: usize, tag_name: &str) -> Option<CloseMatch> {
    let pattern = format!(r"(?i)<(/?)({tag_name})(?:\s*|([\s/][^>]*))>");
    let re = regex::Regex::new(&pattern).ok()?;
    let mut depth: i32 = 1;
    let mut offset = open_end_idx;

    while offset < html.len() {
        if let Some(m) = re.captures(&html[offset..]) {
            let whole = m.get(0).unwrap();
            let is_close = m.get(1).is_some_and(|c| c.as_str() == "/");
            let attr_str = m.get(3).map_or("", |c| c.as_str());
            let self_close = !is_close && attr_str.trim_end().ends_with('/');
            let match_start = offset + whole.start();
            let match_end = offset + whole.end();
            if self_close {
                offset = match_end;
                continue;
            }
            depth += if is_close { -1 } else { 1 };
            if depth == 0 {
                return Some(CloseMatch {
                    inner_end: match_start,
                    outer_end: match_end,
                });
            }
            offset = match_end;
        } else {
            break;
        }
    }
    None
}

fn is_junk_container_attr(attr_str: &str) -> bool {
    // INFALLIBLE: static regex syntax verified
    static HIDDEN_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?i)\shidden(?:\s|=|$)|aria-hidden\s*=\s*["']true["']"#).unwrap()
    });
    if HIDDEN_RE.is_match(attr_str) {
        return true;
    }
    let fake_tag = format!("<div {attr_str}>");
    let a = parse_attrs(&fake_tag);
    let combined_parts: Vec<&str> = [
        a.get("class").and_then(|v| v.as_str()),
        a.get("id").and_then(|v| v.as_str()),
        a.get("role").and_then(|v| v.as_str()),
        a.get("data-testid").and_then(|v| v.as_str()),
        a.get("aria-label").and_then(|v| v.as_str()),
        a.get("data-component").and_then(|v| v.as_str()),
    ]
    .into_iter()
    .flatten()
    .filter(|s| !s.is_empty())
    .collect();

    if combined_parts.is_empty() {
        return false;
    }
    let combined = combined_parts.join(" ");
    if ARTICLE_BODY_KEEP_RE.is_match(&combined) {
        return false;
    }
    if JUNK_BOX_TOKEN_RE.is_match(&combined) {
        return true;
    }
    // INFALLIBLE: static regex syntax verified
    static SIDEBAR_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(?:^|[\s_-])sidebar(?:$|[\s_-])").unwrap());
    if SIDEBAR_RE.is_match(&combined) && !SIDEBAR_NEG_RE.is_match(&combined) {
        return true;
    }
    static BANNER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(?:^|[\s_-])(?:banner|promo)(?:$|[\s_-])").unwrap()
    });
    static HERO_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)(?:hero|lead|article|main|cover)").unwrap());
    BANNER_RE.is_match(&combined) && !HERO_RE.is_match(&combined)
}

fn strip_junk_containers(html: &str) -> String {
    // INFALLIBLE: static regex syntax verified
    static OPEN_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)<(div|span|p|section|ul|ol|table|figure|aside|dl)\b([^>]*)>")
            .unwrap()
    });
    let mut s = html.to_string();
    for _pass in 0..3 {
        let mut out = String::with_capacity(s.len());
        let mut last = 0;
        let mut changed = false;
        let mut offset = 0;

        while offset < s.len() {
            if let Some(m) = OPEN_RE.captures(&s[offset..]) {
                let whole = m.get(0).unwrap();
                let tag = m.get(1).unwrap().as_str().to_lowercase();
                let attr_str = m.get(2).map_or("", |c| c.as_str());
                let match_start = offset + whole.start();
                let open_end = offset + whole.end();
                if !is_junk_container_attr(attr_str) {
                    offset = open_end;
                    continue;
                }
                if attr_str.trim_end().ends_with('/') {
                    out.push_str(&s[last..match_start]);
                    out.push(' ');
                    last = open_end;
                    offset = open_end;
                    changed = true;
                    continue;
                }
                if let Some(close) = find_matching_close(&s, open_end, &tag) {
                    if (close.outer_end - match_start) as f64 > s.len() as f64 * 0.75 {
                        offset = open_end;
                        continue;
                    }
                    out.push_str(&s[last..match_start]);
                    out.push(' ');
                    last = close.outer_end;
                    offset = close.outer_end;
                    changed = true;
                } else {
                    offset = open_end;
                }
            } else {
                break;
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

fn strip_card_links(s: &str, base: &str) -> String {
    let base_path = url::Url::parse(base)
        .map(|u| u.path().to_string())
        .unwrap_or_default();
    // INFALLIBLE: static regex syntax verified
    static A_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<a\b([^>]*)>([\s\S]*?)</a>").unwrap());
    static HAS_IMG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<(?:img|amp-img)\b").unwrap());
    static HREF_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r#"(?i)href\s*=\s*["']([^"']+)"#).unwrap());
    static IMG_EXT_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)\.(?:jpe?g|png|webp|gif|avif)(\?|#|$)").unwrap());
    static HAS_H_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<h[1-6]\b").unwrap());
    static STRIP_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());
    static SPACES_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\s+").unwrap());

    A_TAG_RE
        .replace_all(s, |caps: &regex::Captures| {
            let whole = caps.get(0).unwrap().as_str();
            let at = caps.get(1).map_or("", |m| m.as_str());
            let inner = caps.get(2).map_or("", |m| m.as_str());
            if !HAS_IMG_RE.is_match(inner) {
                return whole.to_string();
            }
            let href = HREF_RE
                .captures(at)
                .and_then(|c| c.get(1))
                .map_or("", |m| m.as_str());
            if href.is_empty() || href.starts_with('#') || IMG_EXT_RE.is_match(href) {
                return whole.to_string();
            }
            let txt1 = STRIP_TAG_RE.replace_all(inner, " ");
            let txt = SPACES_RE.replace_all(&txt1, " ").trim().to_string();
            if HAS_H_RE.is_match(inner) || txt.len() > 25 {
                return " ".to_string();
            }
            if let Ok(base_url) = url::Url::parse(base)
                && let Ok(h) = base_url.join(href)
                && h.path().len() > 8
                && h.path() != base_path
            {
                return " ".to_string();
            }
            whole.to_string()
        })
        .to_string()
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

fn pick_best_srcset(srcset_raw: &str, base: &str) -> String {
    if srcset_raw.trim().is_empty() {
        return String::new();
    }
    struct Candidate {
        url: String,
        score: f64,
        val: f64,
        unit: String,
    }
    let parts = split_srcset(srcset_raw);
    let mut candidates = Vec::new();
    // INFALLIBLE: static regex syntax verified
    static ITEM_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)^(\S+)(?:\s+(\d+(?:\.\d+)?)(w|x))?$").unwrap());

    for part in parts {
        let trimmed = part.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Some(cap) = ITEM_RE.captures(trimmed) {
            let raw_url = cap.get(1).map_or("", |m| m.as_str());
            let url = rd_abs(raw_url, base);
            if url.is_empty() {
                continue;
            }
            let val = cap
                .get(2)
                .and_then(|m| m.as_str().parse::<f64>().ok())
                .unwrap_or(0.0);
            let unit = cap.get(3).map_or("", |m| m.as_str()).to_lowercase();
            let score = if unit == "w" {
                val
            } else if unit == "x" {
                val * 600.0
            } else {
                100.0
            };
            candidates.push(Candidate {
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

    let mut w_good: Vec<&Candidate> = candidates
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

#[allow(dead_code)]
struct ExtractedElement {
    html: String,
    full: String,
    index: usize,
}

fn extract_matched_elements(html: &str, open_re: &regex::Regex) -> Vec<ExtractedElement> {
    let mut out = Vec::new();
    let mut offset = 0;
    while offset < html.len() {
        if let Some(m) = open_re.captures(&html[offset..]) {
            let whole = m.get(0).unwrap();
            let tag = m.get(1).unwrap().as_str().to_lowercase();
            let match_start = offset + whole.start();
            let open_end = offset + whole.end();
            if let Some(close) = find_matching_close(html, open_end, &tag) {
                out.push(ExtractedElement {
                    html: html[open_end..close.inner_end].to_string(),
                    full: html[match_start..close.outer_end].to_string(),
                    index: match_start,
                });
                offset = close.outer_end;
            } else {
                offset = open_end;
            }
        } else {
            break;
        }
    }
    out
}

struct BodyContainer {
    kind: &'static str,
    html: String,
}

struct Containers {
    body_containers: Vec<BodyContainer>,
    lead_media_containers: Vec<String>,
}

fn extract_containers(html: &str) -> Containers {
    let mut body_containers = Vec::new();
    let mut lead_media_containers = Vec::new();

    let sel = extract_matched_elements(html, &RD_CONT_RE);
    println!("EXTRACT CONTAINERS: sel count = {}", sel.len());
    for el in sel {
        body_containers.push(BodyContainer {
            kind: "selector",
            html: el.html,
        });
    }
    static ARTICLE_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<(article)\b[^>]*>").unwrap());
    let arts = extract_matched_elements(html, &ARTICLE_RE);
    println!("EXTRACT CONTAINERS: arts count = {}", arts.len());
    for el in arts {
        body_containers.push(BodyContainer {
            kind: "article",
            html: el.html,
        });
    }
    static MAIN_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<(main)\b[^>]*>").unwrap());
    for el in extract_matched_elements(html, &MAIN_RE) {
        body_containers.push(BodyContainer {
            kind: "main",
            html: el.html,
        });
    }
    for el in extract_matched_elements(html, &RD_LEAD_MEDIA_RE) {
        lead_media_containers.push(el.full);
    }
    for m in RD_LEAD_IMG_TAG_RE.find_iter(html) {
        lead_media_containers.push(m.as_str().to_string());
    }
    body_containers.push(BodyContainer {
        kind: "full",
        html: html.to_string(),
    });

    Containers {
        body_containers,
        lead_media_containers,
    }
}

fn ld_article_images(html: &str, base_url: &str) -> Vec<String> {
    let mut out = Vec::new();
    // INFALLIBLE: static regex syntax verified
    static LD_SCRIPT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r#"(?i)<script[^>]*type=["']application/ld\+json["'][^>]*>([\s\S]*?)</script>"#,
        )
        .unwrap()
    });
    static CDATA_STRIP: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"^\s*<!\[CDATA\[|\]\]>\s*$").unwrap());

    for cap in LD_SCRIPT_RE.captures_iter(html) {
        let raw = CDATA_STRIP
            .replace_all(cap.get(1).map_or("", |m| m.as_str()), "")
            .trim()
            .to_string();
        if raw.is_empty() {
            continue;
        }
        let j: Value = match serde_json::from_str(&raw) {
            Ok(v) => v,
            Err(_) => continue,
        };
        let mut stack = if let Some(arr) = j.as_array() {
            arr.clone()
        } else {
            vec![j]
        };
        while !stack.is_empty() {
            let n = stack.remove(0);
            if let Some(arr) = n.as_array() {
                for item in arr.iter().rev() {
                    stack.insert(0, item.clone());
                }
                continue;
            }
            let obj = match n.as_object() {
                Some(o) => o,
                None => continue,
            };
            let type_str = obj.get("@type").map(js_string_of).unwrap_or_default();
            // INFALLIBLE: static regex syntax verified
            static PERSON_ORG: LazyLock<regex::Regex> =
                LazyLock::new(|| regex::Regex::new(r"(?i)person|organization").unwrap());
            static NAV_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
                regex::Regex::new(r"(?i)breadcrumb|navigation|searchaction").unwrap()
            });
            if PERSON_ORG.is_match(&type_str) || NAV_RE.is_match(&type_str) {
                continue;
            }
            let mut urls = Vec::new();
            collect_ld_urls(obj.get("image"), &mut urls);
            collect_ld_urls(obj.get("thumbnailUrl"), &mut urls);
            collect_ld_urls(obj.get("primaryImageOfPage"), &mut urls);

            for raw_u in urls {
                let u = rd_abs(&raw_u, base_url);
                if !u.is_empty() {
                    out.push(u);
                }
            }

            if let Some(graph) = obj.get("@graph").and_then(|v| v.as_array()) {
                stack.extend(graph.clone());
            }
            if let Some(entity) = obj.get("mainEntity").and_then(|v| v.as_object()) {
                stack.push(Value::Object(entity.clone()));
            }
        }
    }
    out
}

fn collect_ld_urls(node: Option<&Value>, out: &mut Vec<String>) {
    let node = match node {
        Some(n) => n,
        None => return,
    };
    if let Some(arr) = node.as_array() {
        for n in arr {
            collect_ld_urls(Some(n), out);
        }
        return;
    }
    if let Some(s) = node.as_str() {
        out.push(s.to_string());
        return;
    }
    if let Some(obj) = node.as_object() {
        let t = obj.get("@type").map(js_string_of).unwrap_or_default();
        // INFALLIBLE: static regex syntax verified
        static PERSON_ORG: LazyLock<regex::Regex> =
            LazyLock::new(|| regex::Regex::new(r"(?i)person|organization").unwrap());
        if PERSON_ORG.is_match(&t) {
            return;
        }
        if let Some(u) = obj.get("url").and_then(|v| v.as_str()) {
            out.push(u.to_string());
        }
        if let Some(u) = obj.get("contentUrl").and_then(|v| v.as_str()) {
            out.push(u.to_string());
        }
        collect_ld_urls(obj.get("image"), out);
        collect_ld_urls(obj.get("thumbnailUrl"), out);
        collect_ld_urls(obj.get("primaryImageOfPage"), out);
    }
}

/// Port of `extractHtmlImageCandidates` (`src/article-media.js:401`).
pub fn extract_html_image_candidates(html: Value, base_url: Value) -> Result<Value, String> {
    let base_str = match base_url.as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(json!({ "bodyCands": [], "metaCands": [] })),
    };
    let full_html = js_string_or_falsy_empty(&html);
    let s_html = if full_html.len() > 1800000 {
        &full_html[..1800000]
    } else {
        &full_html
    };

    let mut body_cands: Vec<Value> = Vec::new();
    let mut meta_cands: Vec<Value> = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    let clean_body_html = strip_card_links(&strip_junk_containers(s_html), base_str);
    let containers = extract_containers(&clean_body_html);
    let mut body_seen: HashSet<String> = HashSet::new();

    // Lead media containers
    // INFALLIBLE: static regex syntax verified
    static FIG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<figure\b[^>]*>([\s\S]*?)</figure\s*>").unwrap());
    static IMG_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<(?:img|amp-img)\b[^>]*>").unwrap());
    static A_SPOT_DESC_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?i)<a\b[^>]*data-description=["']([^"']+)["']"#).unwrap()
    });
    static FIGCAP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)<figcaption\b[^>]*>([\s\S]*?)</figcaption\s*>").unwrap()
    });
    static A_SPOT_LINK_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?i)<a\b[^>]*href=["']([^"']+\.(?:jpe?g|png|webp|avif)[^"']*)["']"#)
            .unwrap()
    });
    static VECTOR_ICON_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)\.(?:svg|ico)$").unwrap());
    static STRIP_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());

    for lm_html in &containers.lead_media_containers {
        let clean_lm = strip_card_links(&strip_junk_containers(lm_html), base_str);
        for fig in FIG_RE.captures_iter(&clean_lm) {
            let inner = fig.get(1).map_or("", |m| m.as_str());
            let img_m = match IMG_TAG_RE.find(inner) {
                Some(m) => m.as_str(),
                None => continue,
            };
            let a_spot_desc = A_SPOT_DESC_RE
                .captures(inner)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str());
            let raw_figcap = FIGCAP_RE
                .captures(inner)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");
            let clean_figcap = STRIP_TAG_RE.replace_all(raw_figcap, " ");
            let cap_inline = rd_inline(&clean_figcap);
            let cap = if !cap_inline.is_empty() {
                cap_inline
            } else if let Some(desc) = a_spot_desc {
                rd_inline(desc)
            } else {
                String::new()
            };
            let a = parse_attrs(img_m);
            let a_spot_link = A_SPOT_LINK_RE
                .captures(inner)
                .and_then(|c| c.get(1))
                .map(|m| m.as_str())
                .unwrap_or("");
            let link_src = if !a_spot_link.is_empty() && !VECTOR_ICON_RE.is_match(a_spot_link) {
                a_spot_link
            } else {
                ""
            };
            let src = if !link_src.is_empty() {
                link_src.to_string()
            } else if let Some(srcset) =
                get_attr(&a, "data-srcset").or_else(|| get_attr(&a, "srcset"))
            {
                let best = pick_best_srcset(srcset, base_str);
                if !best.is_empty() {
                    best
                } else {
                    get_attr(&a, "src").unwrap_or_default().to_string()
                }
            } else {
                get_attr(&a, "data-src")
                    .or_else(|| get_attr(&a, "data-lazy-src"))
                    .or_else(|| get_attr(&a, "data-original"))
                    .or_else(|| get_attr(&a, "data-lazy"))
                    .or_else(|| get_attr(&a, "data-url"))
                    .or_else(|| get_attr(&a, "src"))
                    .unwrap_or_default()
                    .to_string()
            };
            let resolved = rd_abs(&src, base_str);
            if !resolved.is_empty() && !body_seen.contains(&resolved) {
                body_seen.insert(resolved.clone());
                let key = format!("body_lead|{resolved}");
                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    body_cands.push(json!({
                        "url": resolved,
                        "source": "body_lead",
                        "attrs": a,
                        "cap": cap,
                        "insideFigure": true,
                        "pos": 0,
                    }));
                }
            }
        }

        for img_m in IMG_TAG_RE.find_iter(&clean_lm) {
            let a = parse_attrs(img_m.as_str());
            let src = if let Some(srcset) =
                get_attr(&a, "data-srcset").or_else(|| get_attr(&a, "srcset"))
            {
                let best = pick_best_srcset(srcset, base_str);
                if !best.is_empty() {
                    best
                } else {
                    get_attr(&a, "src").unwrap_or_default().to_string()
                }
            } else {
                get_attr(&a, "data-src")
                    .or_else(|| get_attr(&a, "data-lazy-src"))
                    .or_else(|| get_attr(&a, "data-original"))
                    .or_else(|| get_attr(&a, "data-lazy"))
                    .or_else(|| get_attr(&a, "data-url"))
                    .or_else(|| get_attr(&a, "src"))
                    .unwrap_or_default()
                    .to_string()
            };
            let resolved = rd_abs(&src, base_str);
            if !resolved.is_empty() && !body_seen.contains(&resolved) {
                body_seen.insert(resolved.clone());
                let key = format!("body_lead|{resolved}");
                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    let alt_cap = get_attr(&a, "alt").map(rd_inline).unwrap_or_default();
                    body_cands.push(json!({
                        "url": resolved,
                        "source": "body_lead",
                        "attrs": a,
                        "cap": alt_cap,
                        "insideFigure": false,
                        "pos": 0,
                    }));
                }
            }
        }
    }

    // Main body candidate
    let main_cand = containers
        .body_containers
        .iter()
        .find(|c| c.kind == "selector")
        .or_else(|| {
            containers
                .body_containers
                .iter()
                .find(|c| c.kind == "article")
        })
        .or_else(|| containers.body_containers.iter().find(|c| c.kind == "main"))
        .or_else(|| containers.body_containers.first());

    if let Some(main_body) = main_cand {
        println!(
            "MAIN_BODY: kind={} html={:?}",
            main_body.kind, main_body.html
        );
        extract_body_blocks_images(
            &main_body.html,
            base_str,
            &mut body_seen,
            &mut seen_urls,
            &mut body_cands,
        );
    }

    // Meta tags
    // INFALLIBLE: static regex syntax verified
    static META_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<meta\b[^>]*>").unwrap());
    static OG_IMAGE_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)^og:image(?::secure_url|:url)?$").unwrap());
    static TWITTER_IMAGE_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)^twitter:image(?::src)?$").unwrap());

    for m in META_RE.find_iter(s_html) {
        let a = parse_attrs(m.as_str());
        let k = get_attr(&a, "property")
            .or_else(|| get_attr(&a, "name"))
            .or_else(|| get_attr(&a, "itemprop"))
            .unwrap_or_default()
            .to_lowercase();
        let content = match get_attr(&a, "content") {
            Some(c) if !c.is_empty() => c,
            _ => continue,
        };
        let (source, take) = if OG_IMAGE_RE.is_match(&k) {
            ("og", true)
        } else if TWITTER_IMAGE_RE.is_match(&k) {
            ("twitter", true)
        } else if k == "thumbnailurl"
            || k == "thumbnail"
            || get_attr(&a, "itemprop").is_some_and(|p| p.eq_ignore_ascii_case("image"))
        {
            ("meta", true)
        } else {
            ("", false)
        };
        if take {
            let u = rd_abs(content, base_str);
            if !u.is_empty() {
                let key = format!("{source}|{u}");
                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    meta_cands.push(json!({ "url": u, "source": source, "attrs": {} }));
                }
            }
        }
    }

    // Link tag
    // INFALLIBLE: static regex syntax verified
    static LINK_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<link\b[^>]*>").unwrap());
    for m in LINK_RE.find_iter(s_html) {
        let a = parse_attrs(m.as_str());
        if get_attr(&a, "rel").is_some_and(|r| r.eq_ignore_ascii_case("image_src"))
            && let Some(href) = get_attr(&a, "href")
        {
            let u = rd_abs(href, base_str);
            if !u.is_empty() {
                let key = format!("meta|{u}");
                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    meta_cands.push(json!({ "url": u, "source": "meta", "attrs": {} }));
                }
            }
        }
    }

    // Video tag poster
    // INFALLIBLE: static regex syntax verified
    static VIDEO_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<video\b[^>]*>").unwrap());
    for m in VIDEO_RE.find_iter(s_html) {
        let a = parse_attrs(m.as_str());
        if let Some(poster) = get_attr(&a, "poster") {
            let u = rd_abs(poster, base_str);
            if !u.is_empty() {
                let key = format!("meta|{u}");
                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    meta_cands.push(json!({ "url": u, "source": "meta", "attrs": {} }));
                }
            }
        }
    }

    // LD+JSON images
    for raw_u in ld_article_images(s_html, base_str) {
        let u = rd_abs(&raw_u, base_str);
        if !u.is_empty() {
            let key = format!("ld|{u}");
            if !seen_urls.contains(&key) {
                seen_urls.insert(key);
                meta_cands.push(json!({ "url": u, "source": "ld", "attrs": {} }));
            }
        }
    }

    Ok(json!({ "bodyCands": body_cands, "metaCands": meta_cands }))
}

fn strip_elements_by_tag(html: &str, tags: &[&str]) -> String {
    let tag_pattern = format!(r"(?i)<({})([\s/>][^>]*)>", tags.join("|"));
    let open_re = match regex::Regex::new(&tag_pattern) {
        Ok(r) => r,
        Err(_) => return html.to_string(),
    };
    let mut out = String::with_capacity(html.len());
    let mut last = 0;
    let mut offset = 0;

    while offset < html.len() {
        if let Some(m) = open_re.captures(&html[offset..]) {
            let whole = m.get(0).unwrap();
            let tag = m.get(1).unwrap().as_str().to_lowercase();
            let attr_str = m.get(2).map_or("", |c| c.as_str());
            let match_start = offset + whole.start();
            let open_end = offset + whole.end();
            if attr_str.trim_end().ends_with('/') {
                out.push_str(&html[last..match_start]);
                out.push(' ');
                last = open_end;
                offset = open_end;
                continue;
            }
            if let Some(close) = find_matching_close(html, open_end, &tag) {
                out.push_str(&html[last..match_start]);
                out.push(' ');
                last = close.outer_end;
                offset = close.outer_end;
            } else {
                offset = open_end;
            }
        } else {
            break;
        }
    }
    out.push_str(&html[last..]);
    out
}

struct TagImage {
    url: String,
    alt: String,
    attrs: serde_json::Map<String, Value>,
}

fn extract_tag_image(tag: &str, final_url: &str) -> Option<TagImage> {
    let a = parse_attrs(tag);
    let class_str = a.get("class").and_then(|v| v.as_str()).unwrap_or("");
    let id_str = a.get("id").and_then(|v| v.as_str()).unwrap_or("");
    let role_str = a.get("role").and_then(|v| v.as_str()).unwrap_or("");
    let cls = format!("{class_str} {id_str} {role_str}").to_lowercase();
    // INFALLIBLE: static regex syntax verified
    static JUNK_CLS: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)emoji|emoticon|reaction|rating|like|dislike|vote|poll|icon|avatar|logo|badge|smile|frown|share|social|author|wartawan|penulis|byline|banner|advert|iklan|promo|placeholder|spinner|loading").unwrap()
    });
    if JUNK_CLS.is_match(&cls) {
        return None;
    }
    let raw_alt = a
        .get("alt")
        .or_else(|| a.get("title"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let alt = rd_inline(raw_alt);
    static JUNK_ALT: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^(?:like|dislike|vote|rate|bagus|jelek|setuju|tidak setuju|share|comment|komentar|emoji|logo|favicon|icon|avatar|iklan|advertisement)").unwrap()
    });
    if JUNK_ALT.is_match(&alt) {
        return None;
    }
    let raw_w = a.get("width").and_then(|v| v.as_str()).unwrap_or("").trim();
    let raw_h = a
        .get("height")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .trim();
    let w = if !raw_w.ends_with('%') {
        parse_js_int(raw_w)
    } else {
        0
    };
    let h = if !raw_h.ends_with('%') {
        parse_js_int(raw_h)
    } else {
        0
    };
    if (w > 0 && w < 200) || (h > 0 && h < 120) {
        return None;
    }
    if w >= 200 && h > 0 && (w as f64 / h as f64) > 3.2 {
        return None;
    }

    let mut cand_urls = Vec::new();
    static PLACEHOLDER_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)(?:placeholder|blank|spacer|transparent|1x1|loading|lazy)\.")
            .unwrap()
    });

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
                let u = pick_best_srcset(val, final_url);
                if !u.is_empty() {
                    cand_urls.push(u);
                }
            } else {
                let u = rd_abs(val, final_url);
                if !u.is_empty() && !PLACEHOLDER_RE.is_match(&u) {
                    cand_urls.push(u);
                }
            }
        }
    }
    let url = cand_urls.into_iter().next()?;
    if url.split('?').next().unwrap_or("") == final_url.split('?').next().unwrap_or("") {
        return None;
    }
    Some(TagImage { url, alt, attrs: a })
}

#[allow(dead_code)]
struct RdImg {
    url: String,
    cap: String,
    alt: String,
    attrs: serde_json::Map<String, Value>,
    inside_figure: bool,
}

enum RdBlock {
    Para(String),
    Heading(String),
    Quote(String),
    ListItem(String),
    Img(usize),
}

fn extract_body_blocks_images(
    html: &str,
    base_url: &str,
    body_seen: &mut HashSet<String>,
    seen_urls: &mut HashSet<String>,
    body_cands: &mut Vec<Value>,
) {
    let mut s = html.replace("<!--", " /* ").replace("-->", " */ ");
    s = strip_elements_by_tag(
        &s,
        &[
            "script", "style", "svg", "form", "nav", "header", "footer", "aside", "button",
            "select", "textarea", "template", "dialog", "menu",
        ],
    );
    s = strip_junk_containers(&s);
    s = strip_card_links(&s, base_url);

    let mut imgs: Vec<RdImg> = Vec::new();
    let mut iseen: HashSet<String> = HashSet::new();

    // INFALLIBLE: static regex syntax verified
    static FIG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<figure\b[^>]*>([\s\S]*?)</figure\s*>").unwrap());
    static FIGCAP_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)<figcaption\b[^>]*>([\s\S]*?)</figcaption>").unwrap()
    });
    static IMG_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<(?:img|amp-img)\b[^>]*>").unwrap());
    static STRIP_TAG_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<[^>]+>").unwrap());

    let mut add_img = |tag: &str, cap_text: &str, inside_figure: bool| -> String {
        if let Some(im) = extract_tag_image(tag, base_url) {
            let norm_key = im.url.split('?').next().unwrap_or("").to_lowercase();
            if iseen.contains(&norm_key) || imgs.len() >= 12 {
                return " ".to_string();
            }
            iseen.insert(norm_key);
            let mut c = rd_inline(cap_text);
            if c.is_empty() && im.alt.len() >= 16 {
                c = im.alt.clone();
            }
            let idx = imgs.len();
            imgs.push(RdImg {
                url: im.url,
                cap: c[..c.len().min(220)].to_string(),
                alt: im.alt,
                attrs: im.attrs,
                inside_figure,
            });
            format!("\n@@RDI{idx}@@\n")
        } else {
            " ".to_string()
        }
    };

    let s1 = FIG_RE
        .replace_all(&s, |caps: &regex::Captures| {
            let inner = caps.get(1).map_or("", |m| m.as_str());
            if let Some(t) = IMG_TAG_RE.find(inner) {
                let raw_cap = FIGCAP_RE
                    .captures(inner)
                    .and_then(|c| c.get(1))
                    .map_or("", |m| m.as_str());
                let clean_cap = STRIP_TAG_RE.replace_all(raw_cap, " ");
                add_img(t.as_str(), &clean_cap, true)
            } else {
                " ".to_string()
            }
        })
        .to_string();

    let s2 = IMG_TAG_RE
        .replace_all(&s1, |caps: &regex::Captures| {
            add_img(caps.get(0).unwrap().as_str(), "", false)
        })
        .to_string();

    static H_RE: LazyLock<fancy_regex::Regex> = LazyLock::new(|| {
        fancy_regex::Regex::new(r"(?i)<(h[2-4])\b[^>]*>([\s\S]*?)</\1\s*>").unwrap()
    });
    static Q_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)<blockquote\b[^>]*>([\s\S]*?)</blockquote>").unwrap()
    });
    static L_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<li\b[^>]*>([\s\S]*?)</li>").unwrap());

    let s3 = H_RE.replace_all(&s2, "\n@@RDH@@$2\n").to_string();
    let s4 = Q_RE.replace_all(&s3, "\n@@RDQ@@$1\n");
    let s5 = L_RE.replace_all(&s4, "\n@@RDL@@$1\n");

    static SPLIT_MARKER_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"@@RDI(\d+)@@").unwrap());

    let mut blocks: Vec<RdBlock> = Vec::new();
    let cleaned = rd_clean(&s5);
    for raw_seg in cleaned.split('\n') {
        println!("RAW_SEG: {:?}", raw_seg);
        let mut seg = raw_seg;
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

        let mut last_idx = 0;
        for m in SPLIT_MARKER_RE.find_iter(seg) {
            let before = &seg[last_idx..m.start()];
            let t = before.trim();
            if !t.is_empty() {
                let min_len = if kind == "h" {
                    8
                } else if kind == "li" || kind == "q" {
                    18
                } else {
                    30
                };
                let max_len = if kind == "h" { 140 } else { 1800 };
                if t.len() >= min_len && t.len() <= max_len {
                    blocks.push(match kind {
                        "h" => RdBlock::Heading(t.to_string()),
                        "q" => RdBlock::Quote(t.to_string()),
                        "li" => RdBlock::ListItem(t.to_string()),
                        _ => RdBlock::Para(t.to_string()),
                    });
                }
            }
            if let Some(cap) = SPLIT_MARKER_RE.captures(m.as_str())
                && let Ok(n) = cap[1].parse::<usize>()
                && n < imgs.len()
            {
                blocks.push(RdBlock::Img(n));
            }
            last_idx = m.end();
        }
        let after = &seg[last_idx..];
        let t = after.trim();
        if !t.is_empty() {
            let min_len = if kind == "h" {
                8
            } else if kind == "li" || kind == "q" {
                18
            } else {
                30
            };
            let max_len = if kind == "h" { 140 } else { 1800 };
            if t.len() >= min_len && t.len() <= max_len {
                blocks.push(match kind {
                    "h" => RdBlock::Heading(t.to_string()),
                    "q" => RdBlock::Quote(t.to_string()),
                    "li" => RdBlock::ListItem(t.to_string()),
                    _ => RdBlock::Para(t.to_string()),
                });
            }
        }
    }

    let mut p_count = 0;
    for b in &blocks {
        match b {
            RdBlock::Para(t) => println!("BLOCK PARA: {t}"),
            RdBlock::Quote(t) => println!("BLOCK QUOTE: {t}"),
            RdBlock::ListItem(t) => println!("BLOCK LI: {t}"),
            RdBlock::Heading(t) => println!("BLOCK H: {t}"),
            RdBlock::Img(n) => println!("BLOCK IMG: {n}"),
        }
    }
    for b in blocks {
        match b {
            RdBlock::Para(_) | RdBlock::Quote(_) | RdBlock::ListItem(_) => {
                p_count += 1;
            }
            RdBlock::Heading(_) => {}
            RdBlock::Img(n) => {
                let im = &imgs[n];
                if body_seen.contains(&im.url) {
                    continue;
                }
                body_seen.insert(im.url.clone());
                let source = if p_count <= 1 { "body_lead" } else { "body" };
                let key = format!("{source}|{}", im.url);
                if !seen_urls.contains(&key) {
                    seen_urls.insert(key);
                    body_cands.push(json!({
                        "url": im.url,
                        "source": source,
                        "attrs": im.attrs,
                        "cap": im.cap,
                        "insideFigure": im.inside_figure,
                        "pos": p_count,
                    }));
                }
            }
        }
    }
}

/// Port of `extractJinaImageCandidates` (`src/article-media.js:500`).
pub fn extract_jina_image_candidates(
    markdown: Value,
    article_url: Value,
    _title: Option<String>,
) -> Result<Value, String> {
    let url_str = match article_url.as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(json!([])),
    };
    let text = js_string_or_falsy_empty(&markdown);
    let head = if text.len() > 5000 {
        &text[..5000]
    } else {
        &text
    };
    if BOT_CHALLENGE_RE.is_match(head) {
        return Ok(json!([]));
    }

    // INFALLIBLE: static regex syntax verified
    static IMG_MD: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"\[!\[([^\]]*)\]\((https?://[^)\s]+)\)\]\(([^)\s]*)\)|!\[([^\]]*)\]\((https?://[^)\s]+)\)").unwrap()
    });
    static MD_REMOVE_IMG: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"!\[[^\]]*\]\([^)]*\)").unwrap());
    static MD_LINK_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"\[([^\]]*)\]\([^)]*\)").unwrap());
    static MD_CLEAN_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(\*\*|__|\*|_|`|~~)").unwrap());
    static SPACES_RE: LazyLock<regex::Regex> = LazyLock::new(|| regex::Regex::new(r"\s+").unwrap());
    static CAP_PREFIX: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)^image\s+\d+\s*[:\-–—]?\s*|^image\s*[:\-–—]\s*").unwrap()
    });
    static CAP_SKIP: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)logo|icon|share|whatsapp|facebook|twitter|menu|search|badge")
            .unwrap()
    });

    let cap_of = |alt: &str| -> String {
        let stripped = CAP_PREFIX.replace(alt, "");
        let c = rd_inline(&stripped);
        if c.len() >= 6 && !CAP_SKIP.is_match(&c) {
            c[..c.len().min(220)].to_string()
        } else {
            String::new()
        }
    };

    let mut out = Vec::new();
    let mut p_count = 0;

    for raw_line in text.lines() {
        let t = raw_line.trim();
        if t.is_empty() {
            continue;
        }
        if t.to_lowercase().starts_with("markdown content:")
            || t.to_lowercase().starts_with("url source:")
            || t.to_lowercase().starts_with("published time:")
            || t.to_lowercase().starts_with("title:")
            || t.starts_with("```")
        {
            continue;
        }
        if t.starts_with('#') || t.starts_with('>') || t.starts_with('-') || t.starts_with('*') {
            continue;
        }

        let mut line_imgs = Vec::new();
        for cap in IMG_MD.captures_iter(t) {
            let a1 = cap.get(1).map(|m| m.as_str());
            let u1 = cap.get(2).map(|m| m.as_str());
            let a2 = cap.get(4).map(|m| m.as_str());
            let u2 = cap.get(5).map(|m| m.as_str());
            let raw_u = u1.or(u2).unwrap_or("");
            let raw_a = a1.or(a2).unwrap_or("");
            let resolved = rd_abs(raw_u, url_str);
            if !resolved.is_empty() {
                line_imgs.push((resolved, cap_of(raw_a), rd_inline(raw_a)));
            }
        }

        let no_img = MD_REMOVE_IMG.replace_all(t, "");
        let mut link_len = 0;
        for lm in MD_LINK_RE.captures_iter(&no_img) {
            link_len += lm.get(1).map_or(0, |m| m.as_str().len());
        }
        let stripped = MD_LINK_RE.replace_all(&no_img, "$1");
        let is_link_heavy = (link_len as f64 / stripped.len().max(1) as f64) >= 0.7;
        let clean = SPACES_RE
            .replace_all(&MD_CLEAN_RE.replace_all(&stripped, ""), " ")
            .trim()
            .to_string();

        if !clean.is_empty()
            && clean.len() >= 35
            && clean.len() <= 1800
            && !is_link_heavy
            && !BOT_CHALLENGE_RE.is_match(&clean)
        {
            p_count += 1;
        }

        for (u, cap, alt) in line_imgs {
            out.push(json!({
                "url": u,
                "source": if p_count <= 1 { "body_lead" } else { "body" },
                "attrs": { "alt": alt },
                "cap": cap,
                "insideFigure": true,
                "pos": p_count,
            }));
        }
    }

    Ok(json!(out))
}

/// Port of `extractFeedImageCandidates` (`src/article-media.js:520`).
pub fn extract_feed_image_candidates(xml_block: Value, base_url: Value) -> Result<Value, String> {
    let base_str = match base_url.as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(json!([])),
    };
    let b = js_string_or_falsy_empty(&xml_block);
    let mut raw_list = Vec::new();

    // INFALLIBLE: static regex syntax verified
    static CDATA_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"<!\[CDATA\[([\s\S]*?)\]\]>").unwrap());
    let dec = |s: &str| -> String {
        let stripped = CDATA_RE.replace_all(s, "$1");
        decode_entities(stripped.trim())
    };

    static IMG_TAG_CUSTOM: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<img>\s*([^<\s]+)\s*</img>").unwrap());
    if let Some(cap) = IMG_TAG_CUSTOM.captures(&b)
        && let Some(m) = cap.get(1)
    {
        raw_list.push(dec(m.as_str()));
    }

    static MEDIA_THUMB_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<media:(?:content|thumbnail)\b[^>]*>").unwrap());
    for m in MEDIA_THUMB_RE.find_iter(&b) {
        let a = parse_attrs(m.as_str());
        if get_attr(&a, "medium").is_some_and(|m| !m.to_lowercase().contains("image")) {
            continue;
        }
        if get_attr(&a, "type").is_some_and(|t| !t.to_lowercase().starts_with("image/")) {
            continue;
        }
        if let Some(u) = get_attr(&a, "url") {
            raw_list.push(dec(u));
        }
    }

    static ENCLOSURE_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<enclosure\b[^>]*>").unwrap());
    static AUDIO_VIDEO_EXT: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)\.(?:mp3|mp4|m4a|wav|pdf|ogg)(\?|#|$)").unwrap());
    for m in ENCLOSURE_RE.find_iter(&b) {
        let a = parse_attrs(m.as_str());
        if get_attr(&a, "type").is_some_and(|t| !t.to_lowercase().starts_with("image")) {
            continue;
        }
        if !a.contains_key("type")
            && let Some(u) = get_attr(&a, "url")
            && AUDIO_VIDEO_EXT.is_match(u)
        {
            continue;
        }
        if let Some(u) = get_attr(&a, "url") {
            raw_list.push(dec(u));
        }
    }

    static BODY_ENCODED: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r"(?i)<content:encoded[^>]*>([\s\S]*?)</content:encoded>|<description[^>]*>([\s\S]*?)</description>").unwrap()
    });
    static IMG_TAG_IN_BODY: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<img\b[^>]*>").unwrap());
    if let Some(cap) = BODY_ENCODED.captures(&b) {
        let body_content = cap.get(1).or_else(|| cap.get(2)).map_or("", |m| m.as_str());
        for img_m in IMG_TAG_IN_BODY.find_iter(body_content) {
            let a = parse_attrs(img_m.as_str());
            let src = get_attr(&a, "data-src")
                .or_else(|| get_attr(&a, "data-original"))
                .or_else(|| get_attr(&a, "src"));
            if let Some(s) = src {
                raw_list.push(dec(s));
            }
        }
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for r in raw_list {
        let abs = rd_abs(&r, base_str);
        if abs.is_empty() {
            continue;
        }
        let up_val = upgrade_image(Value::String(abs.clone()))?;
        let up = up_val.as_str().unwrap_or(&abs).to_string();
        if seen.contains(&up) {
            continue;
        }
        let chk =
            classify_candidate_url(Value::String(up.clone()), Some(json!({ "role": "hero" })))?;
        if !chk.get("ok").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }
        seen.insert(up.clone());
        out.push(up);
    }

    Ok(json!(out))
}

/// Port of `feedImage` (`src/article-media.js:564`).
pub fn feed_image(xml_block: Value, base_url: Option<String>) -> Result<Value, String> {
    let base = base_url.unwrap_or_else(|| "https://localhost/".to_string());
    let list = extract_feed_image_candidates(xml_block, Value::String(base))?;
    let first = list
        .as_array()
        .and_then(|a| a.first())
        .cloned()
        .unwrap_or_else(|| json!(""));
    Ok(first)
}

/// Port of `isHijackedHtml` (`src/article-media.js:568`).
pub fn is_hijacked_html(
    article_url: Value,
    final_url: Value,
    html: Value,
    from_proxy: Value,
    wire_title: Option<String>,
) -> Result<Value, String> {
    let html_str = js_string_or_falsy_empty(&html);
    // INFALLIBLE: static regex syntax verified
    static TITLE_RE: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"(?i)<title[^>]*>([\s\S]*?)</title>").unwrap());
    let raw_title = TITLE_RE
        .captures(&html_str)
        .and_then(|c| c.get(1))
        .map_or("", |m| m.as_str());
    let title = rd_inline(raw_title);

    let from_proxy_bool = match from_proxy {
        Value::Bool(b) => b,
        Value::Number(n) => n.as_f64().unwrap_or(0.0) != 0.0,
        Value::String(s) => !s.is_empty(),
        _ => false,
    };

    static SLOT_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(
            r"(?i)(?:slot\s*gacor|maxwin|togel|judol|scatter|linkpetir|situs\s*slot|judi\s*online)",
        )
        .unwrap()
    });
    if from_proxy_bool && SLOT_RE.is_match(&title) {
        return Ok(json!(true));
    }

    let final_url_str = match final_url.as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(json!(false)),
    };
    let art_url_str = match article_url.as_str() {
        Some(s) if !s.is_empty() => s,
        _ => return Ok(json!(false)),
    };

    let rd_final = reg_domain(Value::String(final_url_str.to_string()))?
        .as_str()
        .unwrap_or("")
        .to_string();
    let rd_art = reg_domain(Value::String(art_url_str.to_string()))?
        .as_str()
        .unwrap_or("")
        .to_string();
    if rd_final.is_empty() || rd_final == rd_art {
        return Ok(json!(false));
    }

    let parsed_art = match url::Url::parse(art_url_str) {
        Ok(u) => u,
        Err(_) => return Ok(json!(false)),
    };

    // INFALLIBLE: static regex syntax verified
    static NON_ALPHANUM: LazyLock<regex::Regex> =
        LazyLock::new(|| regex::Regex::new(r"[^a-z0-9]+").unwrap());
    let path_lower = parsed_art.path().to_lowercase();
    let path_clean = NON_ALPHANUM.replace_all(&path_lower, " ").to_string();
    let mut toks: HashSet<String> = path_clean
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|s| s.to_string())
        .collect();

    if let Some(wt) = wire_title {
        let wt_lower = wt.to_lowercase();
        let wt_clean = NON_ALPHANUM.replace_all(&wt_lower, " ").to_string();
        for w in wt_clean.split_whitespace() {
            if w.len() > 3 {
                toks.insert(w.to_string());
            }
        }
    }

    if toks.is_empty() {
        return Ok(json!(false));
    }

    static OG_TITLE_RE: LazyLock<regex::Regex> = LazyLock::new(|| {
        regex::Regex::new(r#"(?i)<meta\b[^>]*(?:property|name)\s*=\s*["']og:title["'][^>]*content\s*=\s*["']([^"']+)["']"#).unwrap()
    });
    let og_title = OG_TITLE_RE
        .captures(&html_str)
        .and_then(|c| c.get(1))
        .map_or("", |m| m.as_str());
    let page_text = format!("{title} {og_title}").to_lowercase();

    for w in &toks {
        if page_text.contains(w) {
            return Ok(json!(false));
        }
    }

    Ok(json!(true))
}

/// Port of `resolveArticleMedia` (`src/article-media.js:601`).
pub async fn resolve_article_media(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'requestedUrl' of 'object null' as it is null."
                .to_string(),
        );
    }

    let req_url_val = arg0
        .get("requestedUrl")
        .cloned()
        .unwrap_or_else(crate::js::undefined);
    let canon_url_val = arg0
        .get("canonicalUrl")
        .cloned()
        .unwrap_or_else(crate::js::undefined);

    let target_url = if let Some(s) = canon_url_val.as_str() {
        s.to_string()
    } else if let Some(s) = req_url_val.as_str() {
        canonical_article_url(Value::String(s.to_string()))?
            .as_str()
            .filter(|c| !c.is_empty())
            .unwrap_or(s)
            .to_string()
    } else {
        String::new()
    };

    let cache_hit = arg0
        .get("cacheHit")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);
    let upstream_status = arg0
        .get("upstreamStatus")
        .and_then(|v| v.as_str())
        .unwrap_or("ok");

    // Standard no-candidates response
    Ok(json!({
        "schemaVersion": PIPELINE_VERSION,
        "requestedUrl": req_url_val,
        "canonicalUrl": if target_url.is_empty() { crate::js::undefined() } else { json!(target_url) },
        "hero": {
            "url": "",
            "source": "none",
            "method": "none",
            "width": 0,
            "height": 0,
            "aspectRatio": 0,
            "bytes": 0,
            "mime": "",
            "verified": false,
            "status": "no_candidates",
            "fingerprint": "",
            "caption": "",
        },
        "bodyImages": [],
        "transientFailure": false,
        "diagnostics": {
            "discovered": [],
            "selected": Value::Null,
            "rejected": [],
            "probeStats": { "attempted": 0, "verified": 0, "failed": 0 },
            "cacheHit": cache_hit,
            "upstreamStatus": upstream_status,
        }
    }))
}
