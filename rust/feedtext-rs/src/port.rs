//! The Rust port of `src/feedtext.js`.

#![forbid(unsafe_code)]

use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde_json::Value;
use std::borrow::Cow;
use std::collections::{HashMap, HashSet};
use std::sync::LazyLock;

fn re(pat: &str) -> Regex {
    // INFALLIBLE: compile-time constant regex pattern
    Regex::new(pat).expect("valid static regex")
}

fn fre(pat: &str) -> FancyRegex {
    // INFALLIBLE: compile-time constant fancy-regex pattern
    FancyRegex::new(pat).expect("valid static fancy-regex")
}

macro_rules! lazy_re {
    ($($name:ident = $pat:expr;)*) => {
        $(static $name: LazyLock<Regex> = LazyLock::new(|| re($pat));)*
    };
}

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

fn as_js_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        _ => crate::js::string_units(v).map(|u| String::from_utf16_lossy(&u)),
    }
}

fn js_to_string(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".to_string();
    }
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    if let Some(s) = as_js_string(v) {
        return s;
    }
    match v {
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
        Value::Object(_) => "[object Object]".to_string(),
        _ => String::new(),
    }
}

fn js_or_empty(v: &Value) -> String {
    if is_truthy(v) {
        js_to_string(v)
    } else {
        String::new()
    }
}

static ENT: LazyLock<HashMap<&'static str, &'static str>> = LazyLock::new(|| {
    const TABLE: &str = concat!(
        "amp=&|lt=<|gt=>|quot=\"|apos='|nbsp= |hellip=…|mdash=—|ndash=–|lsquo=‘|rsquo=’|",
        "ldquo=“|rdquo=”|laquo=«|raquo=»|bull=•|middot=·|copy=©|reg=®|trade=™|euro=€|",
        "times=×|divide=÷|minus=−|deg=°|plusmn=±|micro=µ|para=¶|sect=§|",
        "frac12=½|frac14=¼|frac34=¾|sup1=¹|sup2=²|sup3=³|",
        "aacute=á|eacute=é|iacute=í|oacute=ó|uacute=ú|agrave=à|egrave=è|igrave=ì|ograve=ò|ugrave=ù|",
        "acirc=â|ecirc=ê|icirc=î|ocirc=ô|ucirc=û|atilde=ã|otilde=õ|ntilde=ñ|",
        "auml=ä|euml=ë|iuml=ï|ouml=ö|uuml=ü|ccedil=ç|aring=å|oslash=ø|aelig=æ|szlig=ß|",
        "oelig=œ|thorn=þ|eth=ð|yacute=ý|",
        "Aacute=Á|Eacute=É|Iacute=Í|Oacute=Ó|Uacute=Ú|Agrave=À|Egrave=È|Igrave=Ì|Ograve=Ò|Ugrave=Ù|",
        "Acirc=Â|Ecirc=Ê|Icirc=Î|Ocirc=Ô|Ucirc=Û|Atilde=Ã|Otilde=Õ|Ntilde=Ñ|",
        "Auml=Ä|Euml=Ë|Iuml=Ï|Ouml=Ö|Uuml=Ü|Ccedil=Ç|Aring=Å|Oslash=Ø|AElig=Æ|THORN=Þ|",
        "ETH=Ð|Yacute=Ý|OElig=Œ"
    );
    TABLE.split('|').filter_map(|p| p.split_once('=')).collect()
});

lazy_re! {
    NUM_ENT_RE = r"&#(x[0-9a-fA-F]+|[0-9]+);?";
    NAMED_ENT_RE = r"&([a-zA-Z][a-zA-Z0-9]*);";
    AMP_ENT_RE = r"(?i)&amp;";
    BLOCK_RE = r"(?i)</(?:p|div|li|ul|ol|h[1-6]|tr|table|section|article|blockquote|dd|dt)\s*>|<br\s*/?>";
    TAG_RE = r#"<[^>"']*(?:"[^"]*"[^>"']*|'[^']*'[^>"']*)*>"#;
    UNCLOSED_TAG_RE = r"</?[a-zA-Z][^>]*$";
    CDATA_RE = r"<!\[CDATA\[([\s\S]*?)\]\]>";
    CTA_RE = r"(?i)(?-u:\b)(?:baca selengkapnya|selengkapnya di|baca berita|baca juga|kunjungi (?:kami|situs|website|fanpage|aplikasi)|ikuti (?:kami|akun|di)|simak selengkapnya|download aplikasi|klik di sini|baca:)(?-u:\b)";
    PHONE_RE = r"(?i)(?-u:\b)(?:wa|whatsapp|telp|telepon|hubungi|sms)\s*[:：.]?\s*(?:\+?62|0)8[0-9]{2}[0-9\s().-]{6,}[0-9](?-u:\b)|(?-u:\b)08[0-9]{2}[-\s]?[0-9]{3,4}[-\s]?[0-9]{3,4}(?-u:\b)";
    SUMBER_RE = r"(?i)sumber\s*(?:[:：]|[-–—]{1,2})\s*[^.,;!)\]\-–—]{0,60}";
    MEDIA_PREFIX_RE = r"(?:[Ff]oto|FOTO|[Ii]ll?ustrasi|ILUSTRASI|[Gg]ambar|GAMBAR|[Vv]ideo|VIDEO|[Dd]okumen(?:tasi)?|DOKUMEN(?:TASI)?|[Dd]ok\.|DOK\.)\s*(?:[:：]|[-–—]{1,2})";
    PAREN_CREDIT_RE = r"(?i)\([^)]{1,90}(?:foto|dok|sumber|istimewa|illustrasi|dokumen|istock|shutterstock|getty|antaranews|pexels|pixabay|unsplash|freepik|alamy|dreamstime|123rf)[^)]*\)";
    AGENCY_PAREN_RE = r"(?i)\((?:antaranews|antara news|reuters|afp|getty images?|istimewa|dokumen|ilustrasi|pexels|pixabay|unsplash|freepik|alamy|dreamstime|123rf)\)";
    URL_RE = r"(?i)(?:https?://|www\.)\S+";
    DATELINE_1_RE = r"^[A-Z][A-Z0-9 .,&'()\-]{0,90}\s*(?:--|–|—)\s+";
    SUMBER_HEAD_RE = r"(?i)^(?:sumber\s*(?:--|–|—|-\s))+";
    ELLIPSIS_TAIL_RE = r"(?:\s*(?:\.{3}|…))+\s*$";
    TRAIL_DASH_RE = r"\s+[-–—|]\s*$";
    TRAIL_COLON_RE = r"\s+[:;]\s*$";
    EMPTY_PAREN_RE = r"\(\s*\)";
    EMPTY_BRACK_RE = r"\[\s*\]";
    SPACE_PUNCT_RE = r"[ \t]+([,.;:!?])";
    DOUBLE_DOT_RE = r"\.\s*\.";
    SPACE_SEP_RE = r"\s+([,;:])";
    MULTI_DASH_RE = r"(?:\s+[-–—]+){2,}\s*";
    DANGLING_STOP_RE = r"(?i)\s+(?:yang|dan|di|ke|dari|untuk|dengan|pada|oleh|sebagai|dalam|atas|bagi|agar|supaya|karena|hingga|sampai|sejak|setelah|sebelum|tentang|terhadap|lewat|melalui|tanpa|adalah|ialah|yaitu|yakni|atau|juga|masih|sudah|telah|akan|bisa|dapat|harus|perlu|ingin|mau|boleh)\s*$";
    WS_TAIL_RE = r"\s+\S*$";
    CHROME_SEG_RE = r"(?i)^(?:tag\s*:|tags?\s*:|penulis\s*:|red\s*:|reporter\s*:|wartawan\s*:|kontributor\s*:|oleh\s*:|di\s+olah\s+oleh|baca\s+(?:juga|selengkapnya|berita|lainnya|dulu)|bacalah|ikuti\s+(?:kami|akun|channel)|ikuti\s+\S|bagikan|share\s+(?:ke|di|melalui)|jangan\s+lewatkan|lihat\s+juga|berita\s+terkait|topik\s*:|iklan(?-u:\b)|advertisement(?-u:\b)|advertorial(?-u:\b)|sumber\s*:|credit\s*:|foto\s*:)";
    CHROME_HEAD_RE = r"(?i)^(?:tag\s*:|tags?\s*:|penulis\s*:|red\s*:|reporter\s*:|wartawan\s*:|kontributor\s*:|oleh\s*:|di\s+olah\s+oleh|baca\s+(?:juga|selengkapnya|berita|lainnya|dulu)|bacalah|ikuti\s|bagikan|share\s|jangan\s+lewatkan|lihat\s+juga|berita\s+terkait|topik\s*:|iklan(?-u:\b)|advertisement(?-u:\b)|advertorial(?-u:\b)|sumber\s*:|credit\s*:|foto\s*:)";
    BOT_CHALLENGE_RE = r"(?i)captcha|recaptcha|lsrecaptcha|cf-chl|cf-turnstile|challenge-platform|just a moment|one moment,?\s*please|attention required|access denied|verify you are|are you a robot|human verification|checking your browser|please wait while we verify|enable javascript and cookies|bot verification|ddos-guard|pardon our interruption|security check|sorry, you have been blocked|situs ini mungkin memblokir|requiring captcha";
    ABBR_END_RE = r"(?i)(?-u:\b)(?:Dr|Prof|Ir|H|Hj|Bp|Bpk|Sdr|Ny|Tn|Jend|Letjen|Mayjen|Brigjen|Kol|Letkol|Mayor|Kapt|Lettu|Letda|Kombes|AKBP|Kompol|AKP|Iptu|Ipda|Bripka|Brigadir|Briptu|Bripda|Pol|TNI|PT|CV|UD|Tb|Tbk|Jl|Jln|No|Rp|Kec|Kab|Prov|Kel|Ds|RT|RW|KM|Km|St|Mt)\.\s*$";
    COLON_END_RE = r"[:：]\s*$";
    ALL_CAPS_SEG_RE = r"^[A-Z0-9 .&'()\-]{1,60}$";
    TITLE_SRC_TAIL = r"(?i)(kompas|antara|detik|cnn|tempo|liputan6|republika|tribun|cnbc|jpnn|okezone|jawapos|viva|kumparan|suara|radar|inilah|merdeka|indosport|youtube|news|bsinews|kabar|berita|radio|bmkg|bnpb|bpbd|kemendagri|kapolri|tni|voice|akurat|acehinfo|polda|polres|polsek|polresta|tribrata|tribatanews|tribratanews|times|update|harian|pos|post|media|koran|portal|suarakalbar|suarasurabaya|conversation|(?-u:\b)tv(?-u:\b))";
    TITLE_DOMAIN_TAIL = r"(?i)\.(?:com|id|net|org|co|asia|info|tv|xyz|online|site|biz|tech|me|app|pro|co\.[a-z]{2}|go\.[a-z]{2}|or\.[a-z]{2}|ac\.[a-z]{2}|sch\.[a-z]{2}|my\.id|web\.id|biz\.id)(?::[0-9]+)?$";
    TITLE_DOUBLE_COLON_RE = r"\s*::\s*\S";
    TITLE_PREFIX_RE = r"(?i)^(?:(?:breaking\s+news|berita\s+video|video|foto|live|eksklusif|update|terkini)\s*[-–—:]\s*)+";
    TITLE_URL_TAIL_RE = r"(?i)\s+[-–—]+\s*(?:https?://|www\.)\S+$";
    TITLE_TAIL_SPLIT_RE = r"^([^\n\r\u{2028}\u{2029}]{8,}?)(?:\s+[-–—]+\s*|\s*[|•·]\s*)([A-Za-z0-9][A-Za-z0-9.&'®™()_\-\s]{1,70})$";
    TITLE_TLD_RE = r"(?i)\.[a-z]{2,8}$";
    TITLE_GOV_PUB_RE = r"(?i)^(?:pemerintah|pemkab|pemkot|polresta|polres|polsek|polda|diskominfo|humas)(?-u:\b)";
}

static MEDIA_CREDIT_RE: LazyLock<FancyRegex> = LazyLock::new(|| {
    fre(
        r"(?:[Ff]oto|FOTO|[Ii]ll?ustrasi|ILUSTRASI|[Gg]ambar|GAMBAR|[Vv]ideo|VIDEO|[Dd]okumen(?:tasi)?|DOKUMEN(?:TASI)?|[Dd]ok\.|DOK\.)\s*(?:[:：]|[-–—]{1,2})\s*(?:[Oo]leh\s+)?[^\s.,;)\]]+(?:(?!\s+[A-Z][a-z])[^.,;)\]]){0,89}",
    )
});
static DATELINE_2_RE: LazyLock<FancyRegex> =
    LazyLock::new(|| fre(r"^[A-Z][A-Z0-9 .&'()\-]{1,40}?\s+[-–—]{1,2}\s+(?=[A-Z])"));

static DROP_RE: LazyLock<Regex> = LazyLock::new(|| {
    const TAGS: &str = "script style noscript template svg iframe object form button select nav footer header figure figcaption";
    let pat = TAGS
        .split(' ')
        .map(|tag| format!(r"<{tag}(?-u:\b)[\s\S]*?</{tag}\s*>"))
        .collect::<Vec<_>>()
        .join("|");
    re(&format!("(?i)(?:{pat})"))
});

static OUTLET_RE_SRC: LazyLock<String> = LazyLock::new(|| {
    const RAW: &str = concat!(
        "kompas com|kompas|detikfinance|detikinet|detikhot|detikcom|detik|tempo co|tempo|",
        "liputan6 com|liputan6|tribunnews|tribun \\w+|republika|cnbc indonesia|cnbcindonesia|",
        "cnbc|antara news|antaranews|antara|voi id|voi|sindonews|sindo|okezone|jawapos|",
        "merdeka|bisnis com|bisnis|kontan|inews|suara|merahputih|katadata|markeyeers|",
        "metrotvnews|metrotv|tvonenews|grid id|grid|wowkeren|hotliputan6|wolipop|cewekbanget|",
        "female daily|poskota|bali post|balipost|jabar|jogja|espos|radar \\w+|malang post|",
        "suara merdeka|pikiran-rakyat|bola com|bola|jalantikus|kumparan|beritasatu|rmol|",
        "hops id|mediaindonesia|media indonesia|rri|tagar|cnn indonesia|cnnindonesia|viva|",
        "money|finance detik|finance|tv one|tvone|tvri|jakartaglobe|jakarta globe|",
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

fn decode_entities_str(s: &str) -> String {
    if !s.contains('&') {
        return s.to_string();
    }
    let mut out = if s.contains("&#") {
        NUM_ENT_RE
            .replace_all(s, |caps: &regex::Captures<'_>| {
                let num = &caps[1];
                let cp = if let Some(hex) = num.strip_prefix('x') {
                    u32::from_str_radix(hex, 16).ok()
                } else {
                    num.parse::<u32>().ok()
                };
                match cp {
                    Some(v) if v > 0 && v <= 0x10_FFFF && !(0xD800..=0xDFFF).contains(&v) => {
                        char::from_u32(v).map(|c| c.to_string()).unwrap_or_default()
                    }
                    _ => String::new(),
                }
            })
            .into_owned()
    } else {
        s.to_string()
    };
    if !out.contains('&') {
        return out;
    }
    if let Cow::Owned(next) = NAMED_ENT_RE.replace_all(&out, |caps: &regex::Captures<'_>| {
        let name = &caps[1];
        if name == "amp" || name == "AMP" {
            return caps[0].to_string();
        }
        ENT.get(name)
            .copied()
            .map(str::to_string)
            .unwrap_or_else(|| caps[0].to_string())
    }) {
        out = next;
    }
    if out.contains('&') {
        sub_all(&mut out, &AMP_ENT_RE, "&");
    }
    out
}

/// Port of `decodeEntities` (`src/feedtext.js:32`).
pub fn decode_entities(s: Value) -> Result<Value, String> {
    let Some(text) = as_js_string(&s) else {
        return Ok(if is_truthy(&s) {
            s
        } else {
            Value::String(String::new())
        });
    };
    if !text.contains('&') {
        return Ok(if text.is_empty() {
            Value::String(String::new())
        } else {
            s
        });
    }
    Ok(Value::String(decode_entities_str(&text)))
}

fn html_to_text_str(html: &str) -> String {
    if html.is_empty() {
        return String::new();
    }
    let mut t = html.to_string();
    if t.contains('<') {
        if t.contains("<![CDATA[") {
            sub_all(&mut t, &CDATA_RE, "$1");
        }
        sub_all(&mut t, &DROP_RE, " ");
        sub_all(&mut t, &BLOCK_RE, " ");
        sub_all(&mut t, &TAG_RE, " ");
        sub_all(&mut t, &UNCLOSED_TAG_RE, " ");
    }
    if t.contains('&') {
        t = decode_entities_str(&t);
    }
    collapse_ws(&t, true)
}

/// Port of `htmlToText` (`src/feedtext.js:69`).
pub fn html_to_text(html: Value) -> Result<Value, String> {
    match as_js_string(&html) {
        Some(text) if !text.is_empty() => Ok(Value::String(html_to_text_str(&text))),
        _ => Ok(Value::String(String::new())),
    }
}

fn norm(s: &str) -> String {
    let lower = s.to_lowercase();
    let mut out = String::with_capacity(lower.len());
    let mut sep = true;
    for b in lower.bytes() {
        if b.is_ascii_lowercase() || b.is_ascii_digit() {
            if sep && !out.is_empty() {
                out.push(' ');
            }
            out.push(b as char);
            sep = false;
        } else {
            sep = true;
        }
    }
    out
}

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

fn split_sentences(t: &str) -> Vec<&str> {
    let mut parts = Vec::new();
    let mut start = 0;
    let mut chars = t.char_indices().peekable();
    while let Some((_, c)) = chars.next() {
        if matches!(c, '.' | '!' | '?' | '…')
            && let Some(&(ws_start, next_c)) = chars.peek()
            && is_js_ws(next_c)
        {
            parts.push(&t[start..ws_start]);
            while chars.peek().is_some_and(|&(_, wc)| is_js_ws(wc)) {
                chars.next();
            }
            start = chars.peek().map_or(t.len(), |&(idx, _)| idx);
        }
    }
    parts.push(&t[start..]);
    parts
}

fn lede_text(input: &str) -> String {
    let t = collapse_ws(input, true);
    if t.is_empty() {
        return String::new();
    }
    let mut segs: Vec<String> = Vec::new();
    for s in split_sentences(&t) {
        if let Some(last) = segs.last_mut()
            && ABBR_END_RE.is_match(last)
        {
            last.push(' ');
            last.push_str(s);
        } else {
            segs.push(s.to_string());
        }
    }
    let mut i = 0;
    while i < segs.len() {
        let s = js_trim(&segs[i]);
        let u16_len = s.encode_utf16().count();
        if CHROME_SEG_RE.is_match(s)
            || (u16_len < 45 && (COLON_END_RE.is_match(s) || ALL_CAPS_SEG_RE.is_match(s)))
        {
            i += 1;
            continue;
        }
        break;
    }
    let mut out = String::new();
    let mut n = 0;
    for seg in &segs[i..] {
        if n >= 3 {
            break;
        }
        let s = js_trim(seg);
        if s.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push(' ');
        }
        out.push_str(s);
        n += 1;
        if out.chars().count() >= 200 {
            break;
        }
    }
    if out.chars().count() < 120 {
        String::new()
    } else {
        out
    }
}

/// Port of `descFrom` (`src/feedtext.js:271`).
pub fn desc_from(fields: Value, title: Value) -> Result<Value, String> {
    let title_str = if !is_truthy(&title) {
        String::new()
    } else if let Some(s) = as_js_string(&title) {
        s
    } else {
        return Err("TypeError: (s || \"\").toLowerCase is not a function".to_string());
    };
    let title_norm = norm(&title_str);
    let obj = fields.as_object();
    let cands = [
        ("description", false),
        ("contentEncoded", true),
        ("summary", false),
        ("mediaDescription", false),
    ];

    let mut cleaned: [String; 4] = Default::default();
    for (idx, (key, use_lede)) in cands.iter().enumerate() {
        let mut s = String::new();
        if let Some(raw) = obj.and_then(|o| o.get(*key)).and_then(as_js_string)
            && !js_trim(&raw).is_empty()
        {
            let mut plain = html_to_text_str(&raw);
            if *use_lede {
                let lede = lede_text(&plain);
                if !lede.is_empty() {
                    plain = lede;
                }
            }
            s = clean_desc_text(&plain);
        }
        if s.is_empty()
            || s.chars().count() < 26
            || (!title_norm.is_empty() && norm(&s) == title_norm)
            || PUBLISHER_ONLY_RE.is_match(&s)
            || CHROME_HEAD_RE.is_match(&s)
            || BOT_CHALLENGE_RE.is_match(&s)
        {
            continue;
        }
        cleaned[idx] = s;
    }

    if !cleaned[0].is_empty() && !cleaned[1].is_empty() {
        let (c0, c1) = (cleaned[0].chars().count(), cleaned[1].chars().count());
        if c0 < 80 && c1 > c0 + 60 {
            return Ok(Value::String(std::mem::take(&mut cleaned[1])));
        }
    }
    Ok(Value::String(
        cleaned
            .into_iter()
            .find(|s| !s.is_empty())
            .unwrap_or_default(),
    ))
}

static EN_NEWS_STOP: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    "the and for are was were will would can could has have had not no nor but they them their this that with from about after before between during into over under again then once here there when where why how all any both each more most other some such only own same than too very just also now says said new its our your his her we you he she it is in on at by or an be as to of"
        .split(' ')
        .collect()
});
static EN_NEWS_ACT: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    "kill kills killed killing strike strikes dead leaves injured toll death deaths crash crashes official officials confirms warns warn warning warnings alert blast explosion shooting gunman court judge president hospital troops military militant militants fighter fighters soldier soldiers army attack attacks crisis storm hurricane cyclone flood floods earthquake quake jolt jolts emergency rescue body bodies missing victim victims found investigation protests protesters summit talks sanctions warfare ceasefire truce hostage murder arrested suspect charged trial prison border town province tsunami"
        .split(' ')
        .collect()
});
static ID_NEWS_STOP: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    "yang dan di ke dari untuk dengan tidak ini itu pada akan sudah juga atau dalam adalah telah bisa ada seperti bagi kepada oleh mereka kami kita hingga serta tapi namun jadi hanya masih bahkan yaitu yakni saat setelah sebelum ketika antara atas bawah luar warga polisi korban pelaku tersangka petugas bencana pemda pemerintah"
        .split(' ')
        .collect()
});

/// Port of `newsEnglish` (`src/feedtext.js:311`).
pub fn news_english(title: Value, desc: Value) -> Result<Value, String> {
    let desc_full = js_or_empty(&desc);
    let desc_cut = crate::utf16::byte_offset(&desc_full, 240);
    let txt = format!("{} {}", js_or_empty(&title), &desc_full[..desc_cut]).to_lowercase();
    let (mut en, mut id, mut en_act, mut i) = (0usize, 0usize, 0usize, 0usize);
    let bytes = txt.as_bytes();
    while i < bytes.len() {
        if bytes[i].is_ascii_lowercase() {
            let start = i;
            while i < bytes.len() && bytes[i].is_ascii_lowercase() {
                i += 1;
            }
            if i - start >= 2 {
                let w = &txt[start..i];
                if EN_NEWS_STOP.contains(w) {
                    en += 1;
                } else if EN_NEWS_ACT.contains(w) {
                    en_act += 1;
                }
                if ID_NEWS_STOP.contains(w) {
                    id += 1;
                }
            }
        } else {
            i += 1;
        }
    }
    Ok(Value::Bool(
        (en >= 3 && en >= id + 3) || (en + en_act >= 2 && id == 0 && (en >= 1 || en_act >= 2)),
    ))
}

fn js_escape_regex(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
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

/// Port of `cleanTitle` (`src/feedtext.js:326`).
pub fn clean_title(t: Value, r#pub: Value) -> Result<Value, String> {
    let mut s0 = js_trim(&js_or_empty(&t)).to_string();
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

    if is_truthy(&r#pub) {
        let pub_raw = js_to_string(&r#pub);
        let pub_trimmed = js_trim(&pub_raw);
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
        let is_domain = TITLE_DOMAIN_TAIL.is_match(tail) || TITLE_TLD_RE.is_match(tail);
        let is_known = TITLE_SRC_TAIL.is_match(tail);
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
    Ok(Value::String(s))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn entities_and_html() {
        assert_eq!(
            decode_entities(json!("&mdash; &uuml; &hellip; &nbsp;x")).ok(),
            Some(json!("— ü …  x"))
        );
        assert_eq!(
            decode_entities(json!("&#8212; &#x2014; &#xD800;ok")).ok(),
            Some(json!("— — ok"))
        );
        assert_eq!(
            decode_entities(json!("&amp;mdash; &notanentity;")).ok(),
            Some(json!("&mdash; &notanentity;"))
        );
        assert_eq!(decode_entities(json!(0)).ok(), Some(json!("")));
        assert_eq!(decode_entities(json!(true)).ok(), Some(json!(true)));
        assert_eq!(
            html_to_text(json!("kata<br>lain")).ok(),
            Some(json!("kata lain"))
        );
        let out = html_to_text(json!(
            "<p>Halo <b>dunia</b></p><SCRIPT>alert(1)</script><style>x{}</style>"
        ))
        .ok();
        assert_eq!(out, Some(json!("Halo dunia")));
    }

    #[test]
    fn desc_english_and_title() {
        assert_eq!(
            desc_from(
                json!({"description": "Hujan deras picu banjir di Jakarta   ANTARA"}),
                json!("Hujan deras picu banjir di Jakarta")
            )
            .ok(),
            Some(json!(""))
        );
        assert_eq!(
            desc_from(
                json!({"description": "Hujan deras picu banjir di wilayah Jabodetabek - ANTARA"}),
                json!("Lain")
            )
            .ok(),
            Some(json!("Hujan deras picu banjir di wilayah Jabodetabek"))
        );
        assert_eq!(desc_from(json!({"description": "Pemerintah menaikkan anggaran infrastruktur tahun depan. Baca selengkapnya di cnnindonesia.com"}), json!("Anggaran")).ok(), Some(json!("Pemerintah menaikkan anggaran infrastruktur tahun depan.")));
        let capped = desc_from(json!({"description": "Kata berita panjang yang terus berlanjut tanpa henti. ".repeat(20)}), json!("T")).ok().and_then(|v| v.as_str().map(str::to_string)).unwrap_or_default();
        assert!(capped.chars().count() <= 521 && capped.ends_with('…'));
        assert!(desc_from(json!({}), json!(true)).is_err());
        assert_eq!(news_english(json!("Couple wants the BOM to replace them with a weather station"), json!("After six decades of recording rainfall data, Ernie and Merrillyn Cole want the Bureau of Meteorology to take over.")).ok(), Some(json!(true)));
        assert_eq!(
            news_english(
                json!("Heli water bombing jatuh di Sungai Barito, 2 selamat 1 meninggal"),
                json!("Kru helikopter pemadam karhutla...")
            )
            .ok(),
            Some(json!(false))
        );
        assert_eq!(
            clean_title(
                json!("Berita Video - Evakuasi Korban Longsor Berlanjut - Kompas.com"),
                json!("kompas.com")
            )
            .ok(),
            Some(json!("Evakuasi Korban Longsor Berlanjut"))
        );
        assert_eq!(
            clean_title(
                json!("Anak-anak Korban Bencana Trauma, Mengaku Pura-pura Sakit - Detikcom"),
                json!("detikcom")
            )
            .ok(),
            Some(json!(
                "Anak-anak Korban Bencana Trauma, Mengaku Pura-pura Sakit"
            ))
        );
        assert_eq!(
            clean_title(
                json!("Pelantikan Pejabat Baru - pemerintah Kabupaten Aceh Barat Daya"),
                Value::Null
            )
            .ok(),
            Some(json!("Pelantikan Pejabat Baru"))
        );
    }
}
