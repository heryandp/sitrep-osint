#![forbid(unsafe_code)]

use crate::port::frames::{
    EvalClause, FRAME_TOPICS, detect_frame, extract_affected_entity_str, extract_event_time_str,
    extract_impact_str,
};
use crate::port::geo_service;
use crate::port::tables::*;
use fancy_regex::Regex as FancyRegex;
use regex::Regex;
use serde_json::{Map, Value, json};
use sha1::{Digest, Sha1};
use std::collections::HashSet;
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization;

macro_rules! lazy_re {
    ($name:ident, $pat:literal) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| re($pat));
    };
}

// Helper for ordered unique Vec<String> matching JS Set iteration order
struct OrderedSet {
    items: Vec<String>,
    seen: HashSet<String>,
}

impl OrderedSet {
    fn new() -> Self {
        Self {
            items: Vec::new(),
            seen: HashSet::new(),
        }
    }
    fn insert(&mut self, s: String) {
        if self.seen.insert(s.clone()) {
            self.items.push(s);
        }
    }
    fn contains(&self, s: &str) -> bool {
        self.seen.contains(s)
    }
    fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

pub fn decode_basic_entities_str(s: &str) -> String {
    let step1 = NUM_ENTITY_RE.replace_all(s, |caps: &regex::Captures<'_>| {
        let n = caps.get(1).map_or("", |m| m.as_str());
        let cp = if let Some(hex) = n.strip_prefix('x').or_else(|| n.strip_prefix('X')) {
            u32::from_str_radix(hex, 16).ok()
        } else {
            n.parse::<u32>().ok()
        };
        match cp {
            Some(c) if c > 0 && c <= 0x10_FFFF && !(0xD800..=0xDFFF).contains(&c) => {
                char::from_u32(c)
                    .map(|ch| ch.to_string())
                    .unwrap_or_default()
            }
            _ => String::new(),
        }
    });
    let step2 = NAMED_ENTITY_RE.replace_all(&step1, |caps: &regex::Captures<'_>| {
        let k = caps.get(1).map_or("", |m| m.as_str()).to_ascii_lowercase();
        match k.as_str() {
            "lt" => "<",
            "gt" => ">",
            "quot" => "\"",
            "apos" => "'",
            "nbsp" => " ",
            "mdash" => "—",
            "ndash" => "–",
            "hellip" => "…",
            _ => caps.get(0).map_or("", |m| m.as_str()),
        }
        .to_string()
    });
    AMP_ENTITY_RE.replace_all(&step2, "&").into_owned()
}

pub fn segment_hashtag_str(tag: &str) -> String {
    let clean = LEADING_HASH_RE.replace(tag, "");
    let clean = clean.trim();
    if clean.is_empty() {
        return String::new();
    }
    let s1 = UNDERSCORE_RE.replace_all(clean, " ");
    let s2 = CAMEL_RE.replace_all(&s1, "$1 $2");
    let s3 = LETTER_DIGIT_RE.replace_all(&s2, "$1 $2");
    let s4 = DIGIT_LETTER_RE.replace_all(&s3, "$1 $2");
    let s5 = s4.to_lowercase();
    let spaced = WS_RE.replace_all(&s5, " ");
    let spaced = spaced.trim();

    let mut parts: Vec<String> = Vec::new();
    for seg in spaced.split(' ') {
        if seg.encode_utf16().count() <= 6 {
            parts.push(seg.to_string());
            continue;
        }
        let mut rest = seg;
        let mut out: Vec<String> = Vec::new();
        let mut guard = 0usize;
        while !rest.is_empty() && guard < 12 {
            guard += 1;
            if let Some(&hit) = HASHTAG_LEXICON.iter().find(|&&w| rest.starts_with(w)) {
                out.push(hit.to_string());
                rest = &rest[hit.len()..];
            } else {
                let indices: Vec<usize> = rest.char_indices().map(|(i, _)| i).collect();
                let mut split_at: Option<usize> = None;
                if indices.len() > 5 {
                    for &byte_idx in &indices[3..indices.len() - 2] {
                        let tail = &rest[byte_idx..];
                        if HASHTAG_LEXICON
                            .iter()
                            .any(|&w| tail == w || tail.starts_with(w))
                        {
                            split_at = Some(byte_idx);
                            break;
                        }
                    }
                }
                if let Some(idx) = split_at {
                    out.push(rest[..idx].to_string());
                    rest = &rest[idx..];
                } else {
                    out.push(rest.to_string());
                    rest = "";
                }
            }
        }
        parts.extend(out);
    }
    WS_RE.replace_all(&parts.join(" "), " ").trim().to_string()
}

fn is_unicode_letter(ch: char) -> bool {
    if ch.is_ascii() {
        ch.is_ascii_alphabetic()
    } else {
        let mut buf = [0u8; 4];
        LETTER_RE.is_match(ch.encode_utf8(&mut buf))
    }
}

fn collapse_elongation(tok: &str) -> String {
    let mut out = String::with_capacity(tok.len());
    let mut chars = tok.chars().peekable();
    while let Some(c) = chars.next() {
        let mut count = 1usize;
        while chars.peek() == Some(&c) {
            chars.next();
            count += 1;
        }
        if count >= 3 && is_unicode_letter(c) {
            out.push(c);
        } else {
            for _ in 0..count {
                out.push(c);
            }
        }
    }
    out
}

pub fn normalize_phrase_text_str(text: &str, expand_geo_abbr: bool) -> String {
    let lower_nfkc: String = text.to_lowercase().nfkc().collect();
    let no_url = URL_RE.replace_all(&lower_nfkc, " ");
    let no_mention = MENTION_RE.replace_all(&no_url, " ");
    let safe_leisure = JALAN_JALAN_RE.replace_all(&no_mention, "jalan_jalan_leisure");
    let replaced = WORD_TOKEN_RE.replace_all(&safe_leisure, |caps: &regex::Captures<'_>| {
        let word = caps.get(0).map_or("", |m| m.as_str());
        if word == "jalan_jalan_leisure" {
            return "wisata jalan_jalan".to_string();
        }
        if word.contains('-') && !word.starts_with("cve-") {
            let subs: Vec<&str> = word.split('-').filter(|s| !s.is_empty()).collect();
            if subs.len() == 2 && subs[0] == subs[1] {
                let base = collapse_elongation(subs[0]);
                let mapped = SLANG_MAP
                    .get(base.as_str())
                    .copied()
                    .unwrap_or(base.as_str());
                return format!("{mapped} {mapped}");
            }
        }
        let collapsed = collapse_elongation(word);
        if expand_geo_abbr && let Some(&geo) = GEO_ABBR_MAP.get(collapsed.as_str()) {
            return geo.to_string();
        }
        if let Some(&slang) = SLANG_MAP.get(collapsed.as_str()) {
            return slang.to_string();
        }
        collapsed
    });
    WS_RE.replace_all(&replaced, " ").trim().to_string()
}

fn fancy_split<'a>(re: &FancyRegex, text: &'a str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut last = 0usize;
    for m in re.find_iter(text).flatten() {
        parts.push(&text[last..m.start()]);
        last = m.end();
    }
    parts.push(&text[last..]);
    parts
}

fn split_clauses_str(raw_text: &str) -> Vec<String> {
    let decoded: String = decode_basic_entities_str(raw_text).nfkc().collect();
    let no_url = URL_RE.replace_all(&decoded, " ");
    let protected = ABBR_DOT_RE.replace_all(&no_url, "$1\u{0000}");
    fancy_split(&SPLIT_CLAUSE_RE, &protected)
        .into_iter()
        .map(|s| s.replace('\u{0000}', ".").trim().to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

fn has_question_mark(rc_no_tags: &str) -> bool {
    let stripped = DEFANG_URL_RE.replace_all(rc_no_tags, " ");
    let bytes = stripped.as_bytes();
    for (i, &b) in bytes.iter().enumerate() {
        if b == b'?' {
            // Check negative lookahead (?![a-z0-9_]+=) case-insensitively
            let mut j = i + 1;
            while j < bytes.len() && (bytes[j].is_ascii_alphanumeric() || bytes[j] == b'_') {
                j += 1;
            }
            if !(j > i + 1 && j < bytes.len() && bytes[j] == b'=') {
                return true;
            }
        }
    }
    false
}

#[derive(Clone)]
pub struct ClauseItem {
    pub index: usize,
    pub raw: String,
    pub raw_no_tags: String,
    pub norm_prose: String,
    pub norm_geo: String,
    pub norm_full: String,
    pub hashtags: Vec<String>,
    pub has_question_mark: bool,
    pub doc_raw: String,
}

impl ClauseItem {
    fn to_json(&self) -> Value {
        json!({
            "index": self.index,
            "raw": self.raw,
            "rawNoTags": self.raw_no_tags,
            "normProse": self.norm_prose,
            "normGeo": self.norm_geo,
            "normFull": self.norm_full,
            "hashtags": self.hashtags,
            "hasQuestionMark": self.has_question_mark,
            "docRaw": self.doc_raw,
        })
    }
}

pub struct NormDoc {
    pub raw: String,
    pub decoded: String,
    pub normalized: String,
    pub normalized_prose: String,
    pub normalized_geo: String,
    pub urls: Vec<String>,
    pub mentions: Vec<String>,
    pub hashtags: Vec<String>,
    pub hashtags_segmented: Vec<String>,
    pub emoji_signals: Map<String, Value>,
    pub collective_redup: Vec<String>,
    pub leisure_redup: Vec<String>,
    pub content_hash: String,
    pub clauses: Vec<ClauseItem>,
}

impl NormDoc {
    pub fn to_json(&self) -> Value {
        json!({
            "raw": self.raw,
            "decoded": self.decoded,
            "normalized": self.normalized,
            "normalizedProse": self.normalized_prose,
            "normalizedGeo": self.normalized_geo,
            "urls": self.urls,
            "mentions": self.mentions,
            "hashtags": self.hashtags,
            "hashtagsSegmented": self.hashtags_segmented,
            "emojiSignals": Value::Object(self.emoji_signals.clone()),
            "reduplication": {
                "collective": self.collective_redup,
                "leisure": self.leisure_redup,
            },
            "contentHash": self.content_hash,
            "clauses": self.clauses.iter().map(ClauseItem::to_json).collect::<Vec<_>>(),
        })
    }
}

pub fn normalize_indonesian_doc(raw: &str) -> NormDoc {
    let decoded: String = decode_basic_entities_str(raw).nfkc().collect();
    let urls: Vec<String> = URL_RE
        .find_iter(&decoded)
        .map(|m| m.as_str().to_string())
        .collect();
    let mentions: Vec<String> = MENTION_RE
        .find_iter(&decoded)
        .map(|m| m.as_str().to_string())
        .collect();
    let raw_hashtags: Vec<String> = HASHTAG_RE
        .find_iter(&decoded)
        .map(|m| m.as_str().to_string())
        .collect();
    let hashtags_segmented: Vec<String> = raw_hashtags
        .iter()
        .map(|t| segment_hashtag_str(t))
        .filter(|s| !s.is_empty())
        .collect();

    let mut emoji_signals = Map::new();
    for &(k, ref r) in EMOJI_CUES.iter() {
        if r.is_match(&decoded) {
            emoji_signals.insert(k.to_string(), Value::Bool(true));
        }
    }

    let prose_without_tags = HASHTAG_RE.replace_all(&decoded, " ");
    let normalized_prose = normalize_phrase_text_str(&prose_without_tags, false);
    let normalized_geo_prose = normalize_phrase_text_str(&prose_without_tags, true);

    let join_with_tags = |base: &str| -> String {
        let mut parts: Vec<&str> = Vec::new();
        if !base.is_empty() {
            parts.push(base);
        }
        for t in &hashtags_segmented {
            if !t.is_empty() {
                parts.push(t.as_str());
            }
        }
        WS_RE.replace_all(&parts.join(" "), " ").trim().to_string()
    };

    let normalized = join_with_tags(&normalized_prose);
    let normalized_geo = join_with_tags(&normalized_geo_prose);

    let collective_redup: Vec<String> = REDUPL_COLLECTIVE_RE
        .find_iter(&prose_without_tags)
        .map(|m| m.as_str().to_lowercase())
        .collect();
    let leisure_redup: Vec<String> = REDUPL_LEISURE_RE
        .find_iter(&prose_without_tags)
        .map(|m| m.as_str().to_lowercase())
        .collect();

    let hash_input = if normalized_prose.is_empty() {
        &normalized
    } else {
        &normalized_prose
    };
    let digest = Sha1::digest(hash_input.as_bytes());
    let mut content_hash = String::with_capacity(16);
    for b in &digest[..8] {
        use std::fmt::Write;
        let _ = write!(content_hash, "{b:02x}");
    }

    let raw_clauses = split_clauses_str(raw);
    let clauses: Vec<ClauseItem> = raw_clauses
        .into_iter()
        .enumerate()
        .map(|(idx, rc)| {
            let rc_no_tags = HASHTAG_RE.replace_all(&rc, " ");
            let rc_tags: Vec<String> = HASHTAG_RE
                .find_iter(&rc)
                .map(|m| segment_hashtag_str(m.as_str()))
                .filter(|s| !s.is_empty())
                .collect();
            let norm_prose = normalize_phrase_text_str(&rc_no_tags, false);
            let norm_geo = normalize_phrase_text_str(&rc_no_tags, true);
            let mut full_parts: Vec<&str> = Vec::new();
            if !norm_prose.is_empty() {
                full_parts.push(&norm_prose);
            }
            for t in &rc_tags {
                if !t.is_empty() {
                    full_parts.push(t.as_str());
                }
            }
            let norm_full = full_parts.join(" ").trim().to_string();
            let qm = has_question_mark(&rc_no_tags);
            let raw_no_tags = rc_no_tags.trim().to_string();
            ClauseItem {
                index: idx,
                raw: rc,
                raw_no_tags,
                norm_prose,
                norm_geo,
                norm_full,
                hashtags: rc_tags,
                has_question_mark: qm,
                doc_raw: raw.to_string(),
            }
        })
        .collect();

    NormDoc {
        raw: raw.to_string(),
        decoded,
        normalized,
        normalized_prose,
        normalized_geo,
        urls,
        mentions,
        hashtags: raw_hashtags,
        hashtags_segmented,
        emoji_signals,
        collective_redup,
        leisure_redup,
        content_hash,
        clauses,
    }
}

pub fn is_historical_clause_str(text: &str, text_or_empty: &str, year: f64) -> bool {
    let has_current_time = HAS_CURRENT_TIME_RE.is_match(text);
    let c1 = HIST_CLEAN_1_RE.replace_all(text_or_empty, " ");
    let cleaned = HIST_CLEAN_2_RE.replace_all(&c1, " ");
    if has_current_time && !HIST_NOSTALGIA_RE.is_match(text) {
        return false;
    }
    if HISTORICAL_RE.is_match(&cleaned) {
        return true;
    }
    if let Some(caps) = HISTORICAL_YEAR_RE.captures(&cleaned)
        && let Some(m) = caps.get(1)
        && let Ok(y) = m.as_str().parse::<f64>()
    {
        return y < year;
    }
    false
}

lazy_re!(NON_ALNUM_SPACE_RE, r"[^a-z0-9\s]+");
lazy_re!(
    LOC_PREFIX_RE,
    r"(?i)^(?:kab|kota|kecamatan|desa|kelurahan)\s+"
);
lazy_re!(
    CLEAN_PREFIX_RE,
    r"(?i)^(?:kab|kabupaten|kota|kecamatan|kec|provinsi|prov|desa|kelurahan|wilayah|daerah)\s+"
);
lazy_re!(
    CITY_DIR_RE,
    r"(?i)^(?:jakarta|bandung|surabaya|semarang|medan|makassar|tangerang|bekasi|bogor|depok|yogyakarta)\s+(?:utara|selatan|timur|barat|pusat|kota|raya)$"
);

fn title_case_ascii(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut prev_word = false;
    for ch in s.chars() {
        let is_word = ch.is_ascii_alphanumeric() || ch == '_';
        if !prev_word && ch.is_ascii_lowercase() {
            out.push(ch.to_ascii_uppercase());
        } else {
            out.push(ch);
        }
        prev_word = is_word;
    }
    out
}

pub fn is_generic_location_phrase_str(phrase: &str) -> bool {
    let lower_nfkc: String = phrase.to_lowercase().nfkc().collect();
    let clean = NON_ALNUM_SPACE_RE.replace_all(&lower_nfkc, " ");
    let clean = WS_RE.replace_all(&clean, " ");
    let clean = clean.trim();
    if clean.is_empty() || GENERIC_GEO_TOKENS.contains(clean) {
        return true;
    }
    let expanded = GEO_ABBR_MAP.get(clean).copied().unwrap_or(clean);
    let titled = title_case_ascii(expanded);
    let loc = geo_service::locate_text(expanded)
        .or_else(|| geo_service::locate_text(&titled))
        .or_else(|| geo_service::locate_text(&format!("di {titled}")))
        .or_else(|| geo_service::locate_text(&format!("Kota {titled}")));
    let Some(loc) = loc else {
        return false;
    };
    let loc_name = loc.get("name").and_then(Value::as_str).unwrap_or("");
    let loc_prov = loc.get("prov").and_then(Value::as_str).unwrap_or("");
    let loc_norm_raw = geo_service::norm_str(loc_name);
    let loc_norm = LOC_PREFIX_RE.replace(&loc_norm_raw, "");
    let prov_norm = geo_service::norm_str(loc_prov);
    let clean_norm_raw = geo_service::norm_str(expanded);
    let clean_norm = CLEAN_PREFIX_RE.replace(&clean_norm_raw, "");
    if clean_norm.is_empty() {
        return true;
    }
    if clean_norm == loc_norm || (!prov_norm.is_empty() && clean_norm == prov_norm) {
        return true;
    }
    if loc_norm.contains(clean_norm.as_ref()) && clean_norm.len() >= 4 {
        return true;
    }
    CITY_DIR_RE.is_match(&clean_norm)
}

lazy_re!(CLEAN_LANG_RE, r"https?://\S+|[@#]\S+");
lazy_re!(WEAK_PREC_RE, r"^(?:desa|kelurahan|poi)$");

struct LangProfile {
    is_foreign: bool,
    lang: &'static str,
}

fn detect_whatlang(clean: &str) -> &'static str {
    if clean.chars().count() < 10 {
        return "und";
    }
    match whatlang::detect(clean) {
        Some(info) if info.is_reliable() => match info.lang() {
            whatlang::Lang::Nob => "nor",
            other => other.code(),
        },
        _ => "und",
    }
}

fn detect_language_profile(raw: &str) -> LangProfile {
    let clean = CLEAN_LANG_RE.replace_all(raw, " ");
    let clean = clean.trim();
    let id_hits = ID_UNAMBIGUOUS_CORE_RE.find_iter(clean).count();
    let euro_hits = EUROPEAN_WORDS_RE.find_iter(clean).count();
    if id_hits >= 2 {
        return LangProfile {
            is_foreign: false,
            lang: "ind",
        };
    }
    let lang = detect_whatlang(clean);
    let foreign_lang_detected = matches!(
        lang,
        "nld" | "deu" | "fra" | "spa" | "por" | "ita" | "rus" | "pol" | "dan" | "swe" | "nor"
    );
    let candidate_foreign = (euro_hits >= 2 && id_hits == 0)
        || (foreign_lang_detected && id_hits < 2)
        || (euro_hits >= 4 && id_hits < 2);
    if !candidate_foreign {
        return LangProfile {
            is_foreign: false,
            lang,
        };
    }
    let mut has_local = LOCAL_ORG_RE.is_match(clean) || LOCAL_MACRO_RE.is_match(clean);
    if !has_local && let Some(loc) = geo_service::locate_text(clean) {
        let prec = loc.get("prec").and_then(Value::as_str).unwrap_or("");
        has_local = !WEAK_PREC_RE.is_match(prec);
    }
    LangProfile {
        is_foreign: !has_local,
        lang,
    }
}

fn format_loc(loc: Option<Value>) -> Option<Map<String, Value>> {
    let loc = loc?;
    let obj = loc.as_object()?;
    let mut out = Map::new();
    out.insert(
        "name".into(),
        obj.get("name").cloned().unwrap_or(Value::Null),
    );
    out.insert(
        "prov".into(),
        obj.get("prov").cloned().unwrap_or(Value::Null),
    );
    out.insert(
        "prec".into(),
        obj.get("prec").cloned().unwrap_or(Value::Null),
    );
    out.insert("la".into(), obj.get("la").cloned().unwrap_or(Value::Null));
    out.insert("lo".into(), obj.get("lo").cloned().unwrap_or(Value::Null));
    for key in ["provCode", "kabCode", "kecCode", "code"] {
        if let Some(v) = obj.get(key) {
            let truthy = match v {
                Value::Null => false,
                Value::Bool(b) => *b,
                Value::String(s) => !s.is_empty(),
                Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0),
                _ => true,
            };
            if truthy {
                out.insert(key.into(), v.clone());
            }
        }
    }
    Some(out)
}

fn prec_rank(prec: &str) -> u8 {
    match prec {
        "poi" | "desa" | "kelurahan" => 4,
        "kec" => 3,
        "kab" => 2,
        "region" | "prov" => 1,
        _ => 0,
    }
}

fn locate_best(clause_raw: &str, clause_norm_geo: &str) -> Option<Value> {
    let l1 = geo_service::locate_text(clause_raw);
    let l2 = if !clause_norm_geo.is_empty() && clause_norm_geo != clause_raw {
        geo_service::locate_text(clause_norm_geo)
    } else {
        None
    };
    match (l1, l2) {
        (Some(a), Some(b)) => {
            let p1 = a.get("prec").and_then(Value::as_str).unwrap_or("");
            let p2 = b.get("prec").and_then(Value::as_str).unwrap_or("");
            if prec_rank(p2) > prec_rank(p1) {
                Some(b)
            } else {
                Some(a)
            }
        }
        (a, b) => a.or(b),
    }
}

lazy_re!(
    AUTHOR_DISASTER_RE,
    r"(?i)(?-u:\b)(?:kebanjiran|terbakar|banjir|gempa|longsor|ambruk|meledak|terendam)(?-u:\b)"
);

struct ClauseIntentHints {
    is_historical: bool,
    has_event_evidence: bool,
    is_warning: bool,
    has_hazard_cue: bool,
    has_hazard_question: bool,
    is_non_event_chatter: bool,
}

#[derive(Clone)]
struct GeoResult {
    scope: &'static str,
    location: Option<Map<String, Value>>,
    primary_role: &'static str,
    confidence: Option<f64>,
    mentions: Vec<Map<String, Value>>,
}

impl GeoResult {
    fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("scope".into(), Value::String(self.scope.into()));
        m.insert(
            "location".into(),
            self.location
                .clone()
                .map(Value::Object)
                .unwrap_or(Value::Null),
        );
        m.insert(
            "primaryRole".into(),
            Value::String(self.primary_role.into()),
        );
        if let Some(c) = self.confidence {
            m.insert("confidence".into(), crate::js::number(c));
        }
        m.insert(
            "mentions".into(),
            Value::Array(self.mentions.iter().cloned().map(Value::Object).collect()),
        );
        Value::Object(m)
    }
}

fn push_mention(
    mentions: &mut Vec<Map<String, Value>>,
    loc: &Map<String, Value>,
    role: &'static str,
    confidence: f64,
    span: Option<&str>,
    front: bool,
) {
    let mut entry = loc.clone();
    entry.insert("role".into(), Value::String(role.into()));
    entry.insert("confidence".into(), crate::js::number(confidence));
    if let Some(sp) = span {
        entry.insert("span".into(), Value::String(sp.to_string()));
    }
    if front {
        mentions.insert(0, entry);
    } else {
        mentions.push(entry);
    }
}

fn resolve_clause_geo(clause: &ClauseItem, hints: &ClauseIntentHints) -> GeoResult {
    let raw = if !clause.raw_no_tags.is_empty() {
        &clause.raw_no_tags
    } else {
        &clause.raw
    };
    let norm = &clause.norm_geo;
    let nfd: String = raw
        .nfd()
        .filter(|&c| !('\u{0300}'..='\u{036F}').contains(&c))
        .collect();
    let foreign_hit = FOREIGN_MARKER_RE.is_match(&nfd);

    let mut mentions: Vec<Map<String, Value>> = Vec::new();
    let mut stripped_raw = raw.to_string();
    let mut stripped_norm = norm.clone();

    if let Some(rm) = READER_SEG_RE
        .captures(norm)
        .ok()
        .flatten()
        .or_else(|| READER_SEG_RE.captures(raw).ok().flatten())
    {
        let loc_txt = rm
            .get(1)
            .or_else(|| rm.get(2))
            .map_or("", |m| m.as_str())
            .trim();
        let full = rm.get(0).map_or("", |m| m.as_str());
        if !loc_txt.is_empty()
            && let Some(r_loc) = format_loc(locate_best(loc_txt, loc_txt))
        {
            push_mention(
                &mut mentions,
                &r_loc,
                GEO_READER_LOCATION,
                0.88,
                Some(full.trim()),
                false,
            );
            stripped_norm = stripped_norm.replacen(full, " ", 1);
            if let Ok(re_loc) = Regex::new(&format!("(?i){}", regex::escape(loc_txt))) {
                stripped_raw = re_loc.replace(&stripped_raw, " ").into_owned();
            }
        }
    }

    if let Some(am) = AUTHOR_CUR_SEG_RE
        .captures(norm)
        .ok()
        .flatten()
        .or_else(|| AUTHOR_CUR_SEG_RE.captures(raw).ok().flatten())
    {
        let loc_txt = am.get(1).map_or("", |m| m.as_str()).trim();
        let full = am.get(0).map_or("", |m| m.as_str());
        if !loc_txt.is_empty()
            && let Some(a_loc) = format_loc(locate_best(loc_txt, loc_txt))
            && !AUTHOR_DISASTER_RE.is_match(norm)
        {
            push_mention(
                &mut mentions,
                &a_loc,
                GEO_AUTHOR_CURRENT,
                0.85,
                Some(full.trim()),
                false,
            );
        }
    }

    if let Some(hm) = AUTHOR_HOME_SEG_RE
        .captures(norm)
        .ok()
        .flatten()
        .or_else(|| AUTHOR_HOME_SEG_RE.captures(raw).ok().flatten())
    {
        let loc_txt = hm.get(1).map_or("", |m| m.as_str()).trim();
        let full = hm.get(0).map_or("", |m| m.as_str());
        if !loc_txt.is_empty()
            && let Some(h_loc) = format_loc(locate_best(loc_txt, loc_txt))
        {
            push_mention(
                &mut mentions,
                &h_loc,
                GEO_AUTHOR_HOMETOWN,
                0.82,
                Some(full.trim()),
                false,
            );
        }
    }

    let mut dateline_loc: Option<Value> = None;
    if let Some(dm) = DATELINE_SEG_RE.find(raw) {
        let dl = geo_service::locate_text(dm.as_str());
        if let Some(ref d) = dl
            && let Some(f) = format_loc(Some(d.clone()))
        {
            push_mention(
                &mut mentions,
                &f,
                GEO_NEWS_SOURCE,
                0.78,
                Some(dm.as_str().trim()),
                false,
            );
            if stripped_raw.len() >= dm.as_str().len() {
                stripped_raw = stripped_raw[dm.as_str().len()..].to_string();
            }
        }
        dateline_loc = dl;
    }

    let loc = locate_best(&stripped_raw, &stripped_norm);
    if foreign_hit {
        let weak = loc
            .as_ref()
            .and_then(|l| l.get("prec").and_then(Value::as_str))
            .is_some_and(|p| WEAK_PREC_RE.is_match(p));
        let has_local_override = (loc.is_some() && !weak) || LOCAL_ORG_RE.is_match(raw);
        return GeoResult {
            scope: if has_local_override {
                "unknown"
            } else {
                "foreign"
            },
            location: None,
            primary_role: if !has_local_override {
                GEO_AMBIGUOUS
            } else {
                GEO_UNKNOWN
            },
            confidence: None,
            mentions,
        };
    }

    let local_org_or_macro = LOCAL_ORG_RE.is_match(raw) || LOCAL_MACRO_RE.is_match(raw);
    let formatted = format_loc(loc);

    let mut role = GEO_UNKNOWN;
    let mut confidence = 0.0f64;

    if let Some(ref fmt) = formatted {
        let fmt_name = fmt.get("name").and_then(Value::as_str).unwrap_or("");
        let fmt_prec = fmt.get("prec").and_then(Value::as_str).unwrap_or("");
        let matched_author = mentions
            .iter()
            .find(|m| {
                m.get("name").and_then(Value::as_str) == Some(fmt_name)
                    && matches!(
                        m.get("role").and_then(Value::as_str),
                        Some(GEO_AUTHOR_CURRENT | GEO_AUTHOR_HOMETOWN | GEO_READER_LOCATION)
                    )
            })
            .cloned();

        if hints.is_historical {
            role = GEO_HISTORICAL;
            confidence = 0.84;
        } else if let Some(ma) = matched_author
            .as_ref()
            .filter(|_| !hints.has_event_evidence && !hints.has_hazard_cue)
        {
            role = match ma
                .get("role")
                .and_then(Value::as_str)
                .unwrap_or(GEO_UNKNOWN)
            {
                GEO_AUTHOR_CURRENT => GEO_AUTHOR_CURRENT,
                GEO_AUTHOR_HOMETOWN => GEO_AUTHOR_HOMETOWN,
                GEO_READER_LOCATION => GEO_READER_LOCATION,
                _ => GEO_UNKNOWN,
            };
            confidence = ma.get("confidence").and_then(Value::as_f64).unwrap_or(0.0);
        } else if hints.is_non_event_chatter {
            role = matched_author
                .as_ref()
                .and_then(|m| m.get("role").and_then(Value::as_str))
                .map(|r| match r {
                    GEO_AUTHOR_CURRENT => GEO_AUTHOR_CURRENT,
                    GEO_AUTHOR_HOMETOWN => GEO_AUTHOR_HOMETOWN,
                    GEO_READER_LOCATION => GEO_READER_LOCATION,
                    _ => GEO_MENTIONED_IN_PASSING,
                })
                .unwrap_or(GEO_MENTIONED_IN_PASSING);
            confidence = 0.75;
        } else if hints.has_event_evidence
            || hints.is_warning
            || hints.has_hazard_question
            || hints.has_hazard_cue
        {
            let base_name = fmt_name.split(',').next().unwrap_or("");
            let pat = format!(
                r"(?i)(?-u:\b)(?:di|ke|wilayah|daerah|kawasan|sekitar|untuk|area|kota|kabupaten|provinsi)\s+{}",
                regex::escape(base_name)
            );
            let has_prep = Regex::new(&pat).is_ok_and(|r| r.is_match(norm));
            role = GEO_EVENT_LOCATION;
            confidence = match fmt_prec {
                "poi" | "kec" | "desa" | "kelurahan" => {
                    if has_prep {
                        0.94
                    } else {
                        0.88
                    }
                }
                "kab" => {
                    if has_prep {
                        0.92
                    } else {
                        0.84
                    }
                }
                _ => {
                    if has_prep {
                        0.80
                    } else {
                        0.72
                    }
                }
            };
        } else if dateline_loc
            .as_ref()
            .and_then(|d| d.get("name").and_then(Value::as_str))
            == Some(fmt_name)
        {
            role = GEO_NEWS_SOURCE;
            confidence = 0.76;
        } else {
            role = matched_author
                .as_ref()
                .and_then(|m| m.get("role").and_then(Value::as_str))
                .map(|r| match r {
                    GEO_AUTHOR_CURRENT => GEO_AUTHOR_CURRENT,
                    GEO_AUTHOR_HOMETOWN => GEO_AUTHOR_HOMETOWN,
                    GEO_READER_LOCATION => GEO_READER_LOCATION,
                    _ => GEO_MENTIONED_IN_PASSING,
                })
                .unwrap_or(GEO_MENTIONED_IN_PASSING);
            confidence = 0.65;
        }

        let already = mentions.iter().any(|m| {
            m.get("name").and_then(Value::as_str) == Some(fmt_name)
                && m.get("role").and_then(Value::as_str) == Some(role)
        });
        if !already {
            push_mention(&mut mentions, fmt, role, confidence, None, true);
        }
    }

    GeoResult {
        scope: if formatted.is_some() || local_org_or_macro {
            "Indonesia"
        } else {
            "unknown"
        },
        location: formatted,
        primary_role: role,
        confidence: Some(confidence),
        mentions,
    }
}

// analyzePost helper regexes
lazy_re!(
    MAIN_CLAUSE_COMMA_RE,
    r"(?i)^[^,]{8,},\s+(?:ruko|rumah|gedung|bangunan|toko|pabrik|kantor|lahan|mobil)\s+(?:itu|tersebut|ini)(?-u:\b)"
);
lazy_re!(
    STRONG_LISTING_DEAL_RE,
    r"(?i)(?-u:\b)(?:over\s*kredit|for\s*sale|for\s*rent|open\s*rent|looking\s*to\s*rent|(?-u:\b)wts(?-u:\b)|(?-u:\b)wtb(?-u:\b)|(?-u:\b)wtt(?-u:\b)|loker|lowongan\s+kerja|kirim\s+cv|kpr|dp\s*(?:0|nol)|cicilan|booking\s*fee|nego|siap\s+huni|lt\s*[0-9]+|lb\s*[0-9]+|shm|hgb|tahun\s+pembelian|transmisi|stnk|(?-u:\b)km\s*:?\s*[0-9]|showroom|ready\s*stock|open\s*order|pre[- ]?order|jastip|war\s+tiket|sewa\s+(?:raket|kamera|iphone|samsung|kost|kosan|kontrakan|apartemen|ruko|armybomb|army\s*bomb|lightstick)|rekomendasi\s+(?:kost|kosan|kontrakan|cafe|kafe|kuliner|makanan|cakes?|salon|dokter|klinik|tempat|bengkel|laundry|hotel|villa)|recommendation)(?-u:\b)"
);
lazy_re!(
    CYBER_INCIDENT_CUE_RE,
    r"(?i)(?-u:\b)(?:database|data\s+pasien|kebocoran\s+data|data\s+breach|diretas|dibobol|hacker|ransomware|malware)(?-u:\b)"
);
lazy_re!(
    PHYS_DISASTER_CUE_RE,
    r"(?i)(?-u:\b)(?:kebakaran|terbakar|banjir\s+merendam|banjir\s+bandang|gempa|erupsi|longsor\s+menutup|puting\s+beliung|kecelakaan|tabrakan|tertembak|pembunuhan)(?-u:\b)"
);
lazy_re!(
    SAFETY_EMERG_CUE_RE,
    r"(?i)(?-u:\b)(?:evakuasi|mengungsi|dilarikan\s+ke\s+rs|ditangkap|diamankan\s+polisi|tewas|meninggal\s+dunia|luka[- ]luka|korban\s+jiwa)(?-u:\b)"
);
lazy_re!(
    COMMENTARY_META_RE,
    r"(?i)(?-u:\b)(?:ramai\s+dibahas|jadi\s+perbincangan|netizen\s+(?:bahas|soroti)|dampak\s+(?:kebijakan|ekonomi)\s+terhadap|menuai\s+(?:pro\s+dan\s+kontra|kritik|kecaman|pujian)|polemik\s+soal|pengamat\s+menilai)(?-u:\b)"
);
lazy_re!(
    MOCKERY_HAZARD_RE,
    r"(?-u:\b)(?:banjir|gempa|kebakaran|macet|demo|tenggelam)(?-u:\b)"
);
lazy_re!(
    DBD_GAME_1_RE,
    r"(?i)(?-u:\b)(?:dead\s+by\s+daylight|main\s+dbd|play\s+dbd|jogar\s+dbd)(?-u:\b)"
);
lazy_re!(DBD_TOKEN_RE, r"(?i)(?-u:\b)dbd(?-u:\b)");
lazy_re!(
    DBD_GAME_2_RE,
    r"(?i)(?-u:\b)(?:game|gaming|crashed|tome|grabs?|pick\s*ups?|crawled|dcing|perks?|killer|survivor|hooked|gen|pallets?|lobby|steam|fps)(?-u:\b)"
);
lazy_re!(
    TRADING_HALT_RE,
    r"(?i)(?-u:\b)(?:trading\s+halt|bursa\s+efek)(?-u:\b)"
);
lazy_re!(
    PRICE_WORD_RE,
    r"(?-u:\b)(?:harga|tarif|biaya|ongkos|tiket|penduduk|order|penjualan|omzet)(?-u:\b)"
);
lazy_re!(
    HAZARD_CUE_1_RE,
    r"(?-u:\b)(?:banjir|kebanjiran|gempa|tsunami|erupsi|kebakaran|terbakar|longsor|kecelakaan|tabrakan|penembakan|begal|(?:ber|men|di)?demo|unjuk\s+rasa|aksi\s+massa|aksi\s+kamisan|kamisan|revolusi|kampus\s+dikepung|kawan\s+diculik|gas\s+air\s+mata|water\s+cannon|bentrok|ricuh|(?:ga|gak|tidak|kurang)\s+kondusif|keracunan|listrik\s+padam|mati\s+lampu|puting\s+beliung|angin\s+kencang|phk|pemutusan\s+hubungan\s+kerja|pabrik\s+tutup|trading\s+halt)(?-u:\b)|(?-u:\b)tanggul(?-u:\b)[^.!?\n]{0,20}(?-u:\b)jebol(?-u:\b)|(?-u:\b)(?:gunung|g\.)\s+[a-z]+(?-u:\b)[^.!?\n]{0,30}(?-u:\b)status(?-u:\b)[^.!?\n]{0,20}(?-u:\b)(?:awas|siaga)(?-u:\b)|(?-u:\b)(?:gelar|menggelar|melakukan)\s+aksi(?-u:\b)|(?-u:\b)aksi\s+(?:tolak|menolak|mahasiswa|buruh|massa)(?-u:\b)|(?-u:\b)(?:komdigi|kominfo)(?-u:\b)[^.!?\n]{0,50}(?-u:\b)(?:takedown|sensor|blokir|konten\s+kritis|anggaran)(?-u:\b)|(?-u:\b)takedown\s+konten(?-u:\b)|(?-u:\b)(?:air(?:\s+(?:hujan|kali|sungai|selokan|got))?)\s+(?:(?:sudah|mulai|lagi|terus)\s+){0,3}(?:naik|masuk)"
);
lazy_re!(HAZARD_CUE_LED_RE, r"(?-u:\b)(?:ledakan|meledak)(?-u:\b)");
lazy_re!(
    SUPPLY_CUE_RE,
    r"(?-u:\b)(?:antrean\s+(?:bbm|spbu|pertalite|pertamax|solar)|kelangkaan\s+(?:bbm|elpiji|beras)|harga\s+(?:beras|telur|minyak\s+goreng|cabai|gula|sembako|bbm)[^.!?\n]{0,55}(?-u:\b)(?:naik|melonjak|merangkak|mahal|turun|melambung)|operasi\s+pasar|antrean\s+panjang\s+di\s+spbu|(?:bansos|bantuan\s+(?:sosial|langsung\s+tunai)|pkh|bpnt|blt)(?-u:\b)[^.!?\n]{0,40}(?-u:\b)(?:cair|pencairan|disalurkan|penerima|kemensos))(?-u:\b)"
);
lazy_re!(REKOMENDASI_RE, r"(?-u:\b)rekomendasi(?-u:\b)");
lazy_re!(
    WARN_EXTRA_RE,
    r"(?-u:\b)(?:hujan\s+ekstrem|hujan\s+lebat|cuaca\s+ekstrem|gelombang\s+tinggi|angin\s+kencang|evakuasi)(?-u:\b)"
);
lazy_re!(WARN_TSUNAMI_RE, r"(?-u:\b)tsunami(?-u:\b)");
lazy_re!(
    WARN_FLOOD_RE,
    r"(?-u:\b)(?:banjir|rob|longsor|luapan)(?-u:\b)"
);
lazy_re!(
    WARN_WEATHER_RE,
    r"(?-u:\b)(?:hujan\s+ekstrem|hujan\s+lebat|cuaca\s+ekstrem|angin\s+kencang|gelombang|badai|siklon|puting\s+beliung)(?-u:\b)"
);
lazy_re!(
    DIG_POL_CUE_RE,
    r"(?i)(?-u:\b)(?:komdigi|kominfo)(?-u:\b)[^.!?\n]{0,50}(?-u:\b)(?:takedown|sensor|blokir|konten\s+kritis|anggaran)(?-u:\b)|(?-u:\b)takedown\s+konten(?-u:\b)|(?-u:\b)indonesian\s+politics(?-u:\b)|(?-u:\b)resignation\s+of\s+(?:indonesia['’]?s\s+)?(?:president|vice[- ]?president)(?-u:\b)"
);
lazy_re!(
    NEG_GENANGAN_RE,
    r"(?i)(?-u:\b)(?:tidak|bukan|belum|tanpa|nihil|nol|anti|bebas)\s+(?:ada\s+)?genangan(?-u:\b)"
);

fn round2(x: f64) -> f64 {
    // Match JS `+(x).toFixed(2)`
    let s = format!("{x:.2}");
    s.parse::<f64>().unwrap_or(x)
}

#[derive(Clone)]
struct ClauseFlags {
    is_denial: bool,
    is_hist: bool,
    is_quest: bool,
    is_hearsay: bool,
    is_cond: bool,
    is_edu: bool,
    is_ceremony: bool,
    is_religious: bool,
    is_official_agency: bool,
    is_warning: bool,
    is_official_update: bool,
    is_eyewitness: bool,
    is_commercial: bool,
    is_ad: bool,
    is_vent: bool,
    is_opinion: bool,
    is_spam: bool,
    is_reader: bool,
    is_satire: Value,
    is_commentary_meta: bool,
    has_hazard_cue: bool,
    has_situational_supply_cue: bool,
    has_hazard_question: bool,
    has_event_evidence: bool,
    metaphors: Vec<String>,
    negated_tokens: Vec<String>,
}

impl ClauseFlags {
    fn to_json(&self) -> Value {
        json!({
            "isDenial": self.is_denial,
            "isHist": self.is_hist,
            "isQuest": self.is_quest,
            "isHearsay": self.is_hearsay,
            "isCond": self.is_cond,
            "isEdu": self.is_edu,
            "isCeremony": self.is_ceremony,
            "isReligious": self.is_religious,
            "isOfficialAgency": self.is_official_agency,
            "isWarning": self.is_warning,
            "isOfficialUpdate": self.is_official_update,
            "isEyewitness": self.is_eyewitness,
            "isCommercial": self.is_commercial,
            "isAd": self.is_ad,
            "isVent": self.is_vent,
            "isOpinion": self.is_opinion,
            "isSpam": self.is_spam,
            "isReader": self.is_reader,
            "isSatire": self.is_satire,
            "isCommentaryMeta": self.is_commentary_meta,
            "hasHazardCue": self.has_hazard_cue,
            "hasSituationalSupplyCue": self.has_situational_supply_cue,
            "hasHazardQuestion": self.has_hazard_question,
            "hasEventEvidence": self.has_event_evidence,
            "metaphors": self.metaphors,
            "negatedTokens": self.negated_tokens,
        })
    }
}

struct ClauseEvent {
    action: &'static str,
    strong: bool,
    event_type: &'static str,
    eyewitness_cue: bool,
    minor_waterlogging: bool,
    topic: &'static str,
    category: &'static str,
}

impl ClauseEvent {
    fn to_json(&self) -> Value {
        let mut m = Map::new();
        m.insert("action".into(), Value::String(self.action.into()));
        m.insert("strong".into(), Value::Bool(self.strong));
        m.insert("type".into(), Value::String(self.event_type.into()));
        if self.eyewitness_cue {
            m.insert("eyewitnessCue".into(), Value::Bool(true));
        }
        if self.minor_waterlogging {
            m.insert("minorWaterlogging".into(), Value::Bool(true));
        }
        m.insert("topic".into(), Value::String(self.topic.into()));
        m.insert("category".into(), Value::String(self.category.into()));
        Value::Object(m)
    }
}

struct ClauseResult {
    clause: ClauseItem,
    geo: GeoResult,
    events: Vec<ClauseEvent>,
    flags: ClauseFlags,
}

pub fn analyze_post_str(raw_text: &str, year: f64) -> Value {
    let norm_doc = normalize_indonesian_doc(raw_text);
    let prose_no_tags = HASHTAG_RE.replace_all(&norm_doc.decoded, " ");
    let prose_norm = normalize_phrase_text_str(&prose_no_tags, true);

    let lang_profile = detect_language_profile(raw_text);
    if lang_profile.is_foreign {
        let mut scores = Map::new();
        for &intent in INTENTS {
            scores.insert(
                intent.to_string(),
                crate::js::number(if intent == "unrelated" { 0.95 } else { 0.0 }),
            );
        }
        let foreign_clauses: Vec<Value> = norm_doc
            .clauses
            .iter()
            .map(|c| {
                json!({
                    "clause": c.to_json(),
                    "geo": { "scope": "foreign", "location": null, "primaryRole": GEO_UNKNOWN, "confidence": 0, "mentions": [] },
                    "events": [],
                    "flags": {
                        "isDenial": false, "isHist": false, "isQuest": false, "isHearsay": false, "isCond": false,
                        "isEdu": false, "isCeremony": false, "isOfficialAgency": false, "isWarning": false,
                        "isOfficialUpdate": false, "isEyewitness": false, "isCommercial": false, "isAd": false,
                        "isVent": false, "isOpinion": false, "isSpam": false, "isReader": false, "isCommentaryMeta": false,
                        "hasHazardCue": false, "hasSituationalSupplyCue": false, "hasHazardQuestion": false,
                        "hasEventEvidence": false, "metaphors": [], "negatedTokens": [],
                    }
                })
            })
            .collect();
        return json!({
            "normDoc": norm_doc.to_json(),
            "clauses": foreign_clauses,
            "intent": "unrelated",
            "intentScores": Value::Object(scores),
            "confidence": 0.95,
            "geographicContext": { "name": null, "prov": null, "prec": null, "role": GEO_UNKNOWN, "confidence": 0, "mentions": [] },
            "event": null,
            "extractedEvents": [],
            "negatedEvents": [],
            "metaphorHits": [],
            "flags": {
                "anyForeign": true,
                "anyOfficialWarning": false,
                "anyOfficialUpdate": false,
                "anyEyewitness": false,
                "anyDenial": false,
                "anyHistorical": false,
                "anyQuestion": false,
                "anyHearsay": false,
                "anyEducational": false,
                "anyReligious": false,
                "anyCommercial": false,
                "anyAd": false,
                "anyVent": false,
                "anyOpinion": false,
                "anySpam": false,
                "anyMetaphor": false,
                "anyReaderAction": false,
                "anyMinorWaterlogging": false,
                "hasActionableEvent": false,
            },
            "layerTrace": {
                "isForeignLanguage": true,
                "detectedLang": lang_profile.lang,
                "normalization": {
                    "clauses": norm_doc.clauses.len(),
                    "hashtagsSegmented": norm_doc.hashtags_segmented,
                    "reduplication": {
                        "collective": norm_doc.collective_redup,
                        "leisure": norm_doc.leisure_redup,
                    },
                },
                "intentScoresTop3": [["unrelated", 0.95]],
                "geoRole": GEO_UNKNOWN,
                "negatedEvents": [],
                "metaphors": [],
            }
        });
    }

    let mut clause_results: Vec<ClauseResult> = Vec::with_capacity(norm_doc.clauses.len());
    let mut all_mentions: Vec<Map<String, Value>> = Vec::new();
    let mut negated_events = OrderedSet::new();
    let mut any_foreign = false;
    let mut any_official_warning = false;
    let mut any_official_update = false;
    let mut any_eyewitness = false;
    let mut any_denial = false;
    let mut any_historical = false;
    let mut any_question = false;
    let mut any_hearsay = false;
    let mut any_satire = false;
    let mut any_satire_val = Value::Bool(false);
    let mut any_educational = false;
    let mut any_religious = false;
    let mut any_commercial = false;
    let mut any_ad = false;
    let mut any_vent = false;
    let mut any_opinion = false;
    let mut any_spam = false;
    let mut any_metaphor = false;
    let mut any_reader_action = false;
    let mut any_minor_waterlogging = false;
    let mut metaphor_hits = OrderedSet::new();

    let mockery_emoji = norm_doc.emoji_signals.contains_key("mockery");

    for c in &norm_doc.clauses {
        let t = c.norm_geo.as_str();
        let raw_c = if !c.raw_no_tags.is_empty() {
            c.raw_no_tags.as_str()
        } else {
            c.raw.as_str()
        };

        let mut negated_tokens = OrderedSet::new();
        for caps in NEG_PREFIX_RE.captures_iter(t) {
            if let Some(m) = caps.get(1) {
                negated_tokens.insert(m.as_str().trim().to_string());
            }
        }
        for caps in NEG_POST_RE.captures_iter(t) {
            if let Some(m) = caps.get(1) {
                negated_tokens.insert(m.as_str().to_lowercase());
            }
        }
        for m in FREE_OF_DISASTER_RE.find_iter(t) {
            negated_tokens.insert(m.as_str().to_lowercase());
        }

        let mut active_text_for_event = t.to_string();
        if let Some(corr) = CORRECTION_RE.captures(t) {
            let neg_part = corr.get(1).map_or("", |m| m.as_str());
            active_text_for_event = corr.get(2).map_or("", |m| m.as_str()).to_string();
            for kw in [
                "banjir",
                "gempa",
                "kebakaran",
                "longsor",
                "kecelakaan",
                "ledakan",
                "tsunami",
            ] {
                if neg_part.contains(kw) {
                    negated_tokens.insert(kw.to_string());
                    negated_events.insert(kw.to_string());
                }
            }
        }

        let is_denial =
            DENIAL_ANY_RE.is_match(raw_c) || DENIAL_ANY_RE.is_match(t) || NEG_POST_RE.is_match(t);
        let is_hist =
            is_historical_clause_str(raw_c, raw_c, year) || is_historical_clause_str(t, t, year);
        let is_quest =
            c.has_question_mark || QUESTION_RE.is_match(t) || QUESTION_RE.is_match(raw_c);
        let has_main_clause_before_comma = MAIN_CLAUSE_COMMA_RE.is_match(raw_c);
        let is_hearsay = !has_main_clause_before_comma && HEARSAY_RE.is_match(t);
        let is_cond = CONDITIONAL_RE.is_match(t);
        let is_edu = EDUCATION_RE.is_match(t) || EDUCATION_RE.is_match(raw_c);
        let is_ceremony = CEREMONY_RE.is_match(t) || CEREMONY_RE.is_match(raw_c);
        let is_official_agency = OFFICIAL_AGENCY_RE.is_match(raw_c)
            || OFFICIAL_AGENCY_RE.is_match(t)
            || OFFICIAL_AGENCY_RE.is_match(&norm_doc.decoded);
        let is_warning = !is_ceremony
            && (OFFICIAL_WARNING_RE.is_match(t) || OFFICIAL_WARNING_RE.is_match(raw_c));
        let is_official_update =
            is_official_agency && OFFICIAL_UPDATE_RE.is_match(t) && !is_warning && !is_ceremony;
        let is_eyewitness =
            EYEWITNESS_FRAMING_RE.is_match(t) || EYEWITNESS_FRAMING_RE.is_match(raw_c);
        let is_commercial =
            COMMERCIAL_LISTING_RE.is_match(raw_c) || COMMERCIAL_LISTING_RE.is_match(t);
        let strong_listing_deal = STRONG_LISTING_DEAL_RE.is_match(raw_c);
        let has_cyber_incident_cue = CYBER_INCIDENT_CUE_RE.is_match(t);
        let has_physical_disaster_cue = PHYS_DISASTER_CUE_RE.is_match(t);
        let has_safety_emergency_cue =
            has_physical_disaster_cue || has_cyber_incident_cue || SAFETY_EMERG_CUE_RE.is_match(t);
        let religious_practice_count = RELIGIOUS_PRACTICE_ALL_RE
            .find_iter(t)
            .map(|m| m.as_str().to_lowercase())
            .collect::<HashSet<_>>()
            .len();
        let is_religious = !has_safety_emergency_cue
            && (RELIGIOUS_FATWA_STRONG_RE.is_match(t)
                || RELIGIOUS_FATWA_STRONG_RE.is_match(raw_c)
                || religious_practice_count >= 2);
        let is_ad = ADVERTISEMENT_RE.is_match(raw_c) || ADVERTISEMENT_RE.is_match(t);
        let is_vent = PERSONAL_VENT_RE.is_match(raw_c) || PERSONAL_VENT_RE.is_match(t);
        let is_opinion = OPINION_PRAISE_RE.is_match(raw_c) || OPINION_PRAISE_RE.is_match(t);
        let is_spam = SPAM_SCAM_RE.is_match(raw_c) || SPAM_SCAM_RE.is_match(t);
        let is_reader = READER_ACTION_RE.is_match(t);
        let is_commentary_meta = COMMENTARY_META_RE.is_match(t);

        let is_satire_bool = SATIRE_IRONY_RE.is_match(t)
            || (FREE_OF_DISASTER_RE.is_match(t) && (is_hearsay || mockery_emoji))
            || (mockery_emoji && MOCKERY_HAZARD_RE.is_match(t));
        let is_satire_val = if is_satire_bool {
            Value::Bool(true)
        } else if mockery_emoji {
            Value::Bool(false)
        } else {
            crate::js::undefined()
        };
        let is_satire = is_satire_bool;

        let mut clause_metaphors: Vec<String> = Vec::new();
        let is_doc_gaming_dbd = DBD_GAME_1_RE.is_match(&c.doc_raw)
            || (DBD_TOKEN_RE.is_match(&c.doc_raw) && DBD_GAME_2_RE.is_match(&c.doc_raw));
        if is_doc_gaming_dbd {
            clause_metaphors.push("nonincident_sport_entertainment".to_string());
        }
        let has_trading_halt = TRADING_HALT_RE.is_match(t);
        for &(mf_id, ref mf_re) in METAPHOR_FRAMES.iter() {
            if mf_id == "metaphor_jatuh" && has_trading_halt {
                continue;
            }
            if mf_re.is_match(t) || mf_re.is_match(raw_c) {
                clause_metaphors.push(mf_id.to_string());
            }
        }
        for m in &clause_metaphors {
            metaphor_hits.insert(m.clone());
        }

        let eval_clause = EvalClause {
            raw: &c.raw,
            norm_geo: &active_text_for_event,
            doc_raw: &c.doc_raw,
        };
        let mut clause_events: Vec<ClauseEvent> = Vec::new();
        let effective_commercial = is_commercial
            && !has_cyber_incident_cue
            && (strong_listing_deal || !has_physical_disaster_cue);
        let skip_for_events = is_edu
            || is_ceremony
            || is_religious
            || effective_commercial
            || is_ad
            || is_vent
            || is_spam
            || is_satire
            || is_cond;

        if !skip_for_events {
            for &topic in FRAME_TOPICS {
                if metaphor_blocks_topic_str(&clause_metaphors, topic) {
                    continue;
                }
                let Some(det) = detect_frame(topic, &eval_clause) else {
                    continue;
                };
                let is_topic_negated = if det.minor_waterlogging {
                    negated_tokens.contains("genangan")
                        || negated_tokens.contains("waterlogging")
                        || NEG_GENANGAN_RE.is_match(&active_text_for_event)
                } else {
                    let pat = format!(
                        r"(?i)(?-u:\b){NEG_WORDS}\s+(?:ada\s+|terjadi\s+|mengalami\s+)?(?:{}|{})(?-u:\b)",
                        regex::escape(topic),
                        regex::escape(det.event_type)
                    );
                    negated_tokens.contains(topic)
                        || negated_tokens.contains(det.event_type)
                        || Regex::new(&pat).is_ok_and(|r| r.is_match(&active_text_for_event))
                        || FREE_OF_DISASTER_RE.is_match(&active_text_for_event)
                };
                if is_topic_negated {
                    negated_events.insert(
                        if det.minor_waterlogging {
                            "genangan"
                        } else {
                            topic
                        }
                        .to_string(),
                    );
                    continue;
                }
                clause_events.push(ClauseEvent {
                    action: det.action,
                    strong: det.strong,
                    event_type: det.event_type,
                    eyewitness_cue: det.eyewitness_cue,
                    minor_waterlogging: det.minor_waterlogging,
                    topic: det.topic,
                    category: det.category,
                });
            }
        }

        let has_price_word = PRICE_WORD_RE.is_match(t);
        let has_hazard_cue = (HAZARD_CUE_1_RE.is_match(t)
            || (!has_price_word && HAZARD_CUE_LED_RE.is_match(t)))
            && !effective_commercial
            && !is_ad
            && !is_vent
            && clause_metaphors.is_empty();
        let has_situational_supply_cue = SUPPLY_CUE_RE.is_match(t)
            && !effective_commercial
            && !is_ad
            && !is_vent
            && !is_spam
            && clause_metaphors.is_empty();
        let has_hazard_question = is_quest && has_hazard_cue && !REKOMENDASI_RE.is_match(t);
        let has_event_evidence = clause_events.iter().any(|e| e.strong)
            || (is_warning && (has_hazard_cue || WARN_EXTRA_RE.is_match(t)));
        if clause_events.iter().any(|e| e.minor_waterlogging) {
            any_minor_waterlogging = true;
        }

        let geo = resolve_clause_geo(
            c,
            &ClauseIntentHints {
                is_historical: is_hist,
                has_event_evidence,
                is_warning,
                has_hazard_cue: has_hazard_cue && !is_denial,
                has_hazard_question,
                is_non_event_chatter: is_denial
                    || is_commercial
                    || is_ad
                    || is_vent
                    || is_opinion
                    || is_spam
                    || is_edu
                    || is_religious
                    || !clause_metaphors.is_empty(),
            },
        );

        if geo.scope == "foreign" {
            any_foreign = true;
        }
        all_mentions.extend(geo.mentions.iter().cloned());

        any_official_warning |= is_warning;
        any_official_update |= is_official_update;
        any_eyewitness |= is_eyewitness || clause_events.iter().any(|e| e.eyewitness_cue);
        any_denial |= is_denial || (!negated_events.is_empty() && clause_events.is_empty());
        any_historical |= is_hist;
        any_question |= is_quest;
        any_hearsay |= is_hearsay;
        if !any_satire {
            any_satire_val = is_satire_val.clone();
        }
        any_satire |= is_satire;
        any_educational |= is_edu;
        any_religious |= is_religious;
        any_commercial |= effective_commercial;
        any_ad |= is_ad;
        any_vent |= is_vent;
        any_opinion |= is_opinion;
        any_spam |= is_spam;
        any_metaphor |= !clause_metaphors.is_empty();
        any_reader_action |= is_reader;

        clause_results.push(ClauseResult {
            clause: c.clone(),
            geo,
            events: clause_events,
            flags: ClauseFlags {
                is_denial,
                is_hist,
                is_quest,
                is_hearsay,
                is_cond,
                is_edu,
                is_ceremony,
                is_religious,
                is_official_agency,
                is_warning,
                is_official_update,
                is_eyewitness,
                is_commercial,
                is_ad,
                is_vent,
                is_opinion,
                is_spam,
                is_reader,
                is_satire: is_satire_val,
                is_commentary_meta,
                has_hazard_cue,
                has_situational_supply_cue,
                has_hazard_question,
                has_event_evidence,
                metaphors: clause_metaphors,
                negated_tokens: negated_tokens.items,
            },
        });
    }

    // Inherit geo from adjacent clause
    let n_clauses = clause_results.len();
    for i in 0..n_clauses {
        let needs_geo = clause_results[i].geo.location.is_none()
            && clause_results[i].geo.scope == "unknown"
            && !any_foreign
            && (!clause_results[i].events.is_empty() || clause_results[i].flags.is_warning);
        if !needs_geo {
            continue;
        }
        let adj_indices: [Option<usize>; 2] =
            [i.checked_sub(1), (i + 1 < n_clauses).then_some(i + 1)];
        for adj in adj_indices.into_iter().flatten() {
            let nb = &clause_results[adj];
            let bad_role = matches!(
                nb.geo.primary_role,
                GEO_READER_LOCATION | GEO_AUTHOR_CURRENT | GEO_AUTHOR_HOMETOWN | GEO_HISTORICAL
            );
            if nb.geo.scope == "Indonesia"
                && nb.geo.location.is_some()
                && !bad_role
                && !nb.flags.is_denial
                && !nb.flags.is_hist
                && !nb.flags.is_reader
                && !nb.flags.is_hearsay
                && !nb.flags.is_commercial
                && !nb.flags.is_ad
                && !nb.flags.is_vent
                && !nb.flags.is_opinion
                && !nb.flags.is_spam
            {
                let nb_loc = nb.geo.location.clone();
                let nb_conf = nb.geo.confidence.filter(|&c| c != 0.0).unwrap_or(0.8);
                let new_conf = (nb_conf - 0.06).max(0.76);
                clause_results[i].geo.scope = "Indonesia";
                clause_results[i].geo.location = nb_loc.clone();
                clause_results[i].geo.primary_role = GEO_EVENT_LOCATION;
                clause_results[i].geo.confidence = Some(new_conf);
                if let Some(ref loc_map) = nb_loc {
                    push_mention(
                        &mut all_mentions,
                        loc_map,
                        GEO_EVENT_LOCATION,
                        new_conf,
                        None,
                        true,
                    );
                }
                break;
            }
        }
    }

    let impact = extract_impact_str(&prose_norm);
    let affected_entity = extract_affected_entity_str(&prose_norm);
    let global_is_hist = any_historical
        && !clause_results
            .iter()
            .any(|c| !c.events.is_empty() && !c.flags.is_hist);
    let global_event_time = extract_event_time_str(&prose_norm, global_is_hist);

    struct DetectedEvent {
        topic: &'static str,
        category: &'static str,
        event_type: &'static str,
        event_action: &'static str,
        affected_entity: Option<&'static str>,
        location: Option<Value>,
        event_time: Value,
        impact: Value,
        source_type: &'static str,
        evidence: Vec<String>,
        strong: bool,
        minor_waterlogging: bool,
        clause_flags: ClauseFlags,
        scope: &'static str,
    }

    impl DetectedEvent {
        fn to_json(&self) -> Value {
            json!({
                "topic": self.topic,
                "category": self.category,
                "eventType": self.event_type,
                "eventAction": self.event_action,
                "affectedEntity": self.affected_entity,
                "location": self.location,
                "eventTime": self.event_time,
                "impact": self.impact,
                "sourceType": self.source_type,
                "evidence": self.evidence,
                "strong": self.strong,
                "minorWaterlogging": self.minor_waterlogging,
                "clauseFlags": self.clause_flags.to_json(),
                "scope": self.scope,
            })
        }
    }

    let mut detected_events: Vec<DetectedEvent> = Vec::new();
    for cr in &clause_results {
        for ev in &cr.events {
            if cr.flags.is_denial || cr.flags.is_hist {
                continue;
            }
            let source_type = if cr.flags.is_official_agency || cr.flags.is_warning {
                "official_agency"
            } else if cr.flags.is_eyewitness || ev.eyewitness_cue {
                "eyewitness"
            } else {
                "anonymous"
            };
            let loc_val = cr.geo.location.as_ref().map(|loc| {
                let conf = cr.geo.confidence.filter(|&c| c != 0.0).unwrap_or(0.85);
                json!({
                    "name": loc.get("name").cloned().unwrap_or(Value::Null),
                    "prov": loc.get("prov").cloned().unwrap_or(Value::Null),
                    "prec": loc.get("prec").cloned().unwrap_or(Value::Null),
                    "role": if cr.geo.primary_role == GEO_UNKNOWN { GEO_EVENT_LOCATION } else { cr.geo.primary_role },
                    "confidence": crate::js::number(round2(conf)),
                })
            });
            let ev_text = if !cr.clause.raw_no_tags.is_empty() {
                &cr.clause.raw_no_tags
            } else {
                &cr.clause.raw
            };
            detected_events.push(DetectedEvent {
                topic: ev.topic,
                category: ev.category,
                event_type: ev.event_type,
                event_action: ev.action,
                affected_entity: extract_affected_entity_str(&cr.clause.norm_prose)
                    .or(affected_entity),
                location: loc_val,
                event_time: extract_event_time_str(&cr.clause.norm_prose, cr.flags.is_hist),
                impact: impact.clone(),
                source_type,
                evidence: vec![ev_text.to_string()],
                strong: ev.strong,
                minor_waterlogging: ev.minor_waterlogging,
                clause_flags: cr.flags.clone(),
                scope: cr.geo.scope,
            });
        }
    }

    if any_official_warning
        && detected_events.is_empty()
        && let Some(wc) = clause_results
            .iter()
            .find(|cr| cr.flags.is_warning && cr.geo.scope != "foreign")
    {
        let t = &wc.clause.norm_geo;
        let is_tsunami = WARN_TSUNAMI_RE.is_match(t);
        let is_flood_warn = WARN_FLOOD_RE.is_match(t);
        let is_weather_warn = WARN_WEATHER_RE.is_match(t);
        let topic = if is_tsunami {
            "gempa"
        } else if is_flood_warn {
            "banjir"
        } else if is_weather_warn {
            "badai"
        } else {
            "insiden"
        };
        let ev_type = if is_tsunami {
            "tsunami"
        } else if is_flood_warn {
            "flood"
        } else if is_weather_warn {
            "extreme_weather"
        } else {
            "warning"
        };
        let cat = if is_tsunami || is_weather_warn {
            "disaster"
        } else if is_flood_warn {
            "flood"
        } else {
            "insiden"
        };
        let loc_val = wc.geo.location.as_ref().map(|loc| {
            let conf = wc.geo.confidence.filter(|&c| c != 0.0).unwrap_or(0.9);
            json!({
                "name": loc.get("name").cloned().unwrap_or(Value::Null),
                "prov": loc.get("prov").cloned().unwrap_or(Value::Null),
                "prec": loc.get("prec").cloned().unwrap_or(Value::Null),
                "role": GEO_EVENT_LOCATION,
                "confidence": crate::js::number(round2(conf)),
            })
        });
        let ev_text = if !wc.clause.raw_no_tags.is_empty() {
            &wc.clause.raw_no_tags
        } else {
            &wc.clause.raw
        };
        detected_events.push(DetectedEvent {
            topic,
            category: cat,
            event_type: ev_type,
            event_action: "warning_issued",
            affected_entity: extract_affected_entity_str(t).or(Some("public_area")),
            location: loc_val,
            event_time: extract_event_time_str(t, false),
            impact: impact.clone(),
            source_type: "official_agency",
            evidence: vec![ev_text.to_string()],
            strong: true,
            minor_waterlogging: false,
            clause_flags: wc.flags.clone(),
            scope: wc.geo.scope,
        });
    }

    // geographicContext
    let primary_ev_loc_name: Option<String> = detected_events
        .iter()
        .find(|e| e.strong && e.scope == "Indonesia" && e.location.is_some())
        .and_then(|e| e.location.as_ref()?.get("name")?.as_str().map(String::from));

    let best_event_mention = primary_ev_loc_name
        .as_deref()
        .and_then(|name| {
            all_mentions.iter().find(|m| {
                m.get("name").and_then(Value::as_str) == Some(name)
                    && m.get("role").and_then(Value::as_str) == Some(GEO_EVENT_LOCATION)
            })
        })
        .or_else(|| {
            all_mentions
                .iter()
                .find(|m| m.get("role").and_then(Value::as_str) == Some(GEO_EVENT_LOCATION))
        });
    let best_any_mention = best_event_mention.or_else(|| all_mentions.first());
    let mentions_json = Value::Array(all_mentions.iter().cloned().map(Value::Object).collect());

    let geographic_context = if let Some(m) = best_any_mention {
        let conf = m
            .get("confidence")
            .and_then(Value::as_f64)
            .filter(|&c| c != 0.0)
            .unwrap_or(0.7);
        let prov = m
            .get("prov")
            .cloned()
            .filter(|v| !matches!(v, Value::Null) && v.as_str() != Some(""))
            .unwrap_or(Value::Null);
        let prec = m
            .get("prec")
            .cloned()
            .filter(|v| !matches!(v, Value::Null) && v.as_str() != Some(""))
            .unwrap_or(Value::Null);
        json!({
            "name": m.get("name").cloned().unwrap_or(Value::Null),
            "prov": prov,
            "prec": prec,
            "la": m.get("la").cloned().unwrap_or(Value::Null),
            "lo": m.get("lo").cloned().unwrap_or(Value::Null),
            "role": m.get("role").cloned().unwrap_or(Value::Null),
            "confidence": crate::js::number(round2(conf)),
            "mentions": mentions_json,
        })
    } else {
        json!({
            "name": null,
            "prov": null,
            "prec": null,
            "role": GEO_UNKNOWN,
            "confidence": 0,
            "mentions": mentions_json,
        })
    };

    // Intent scoring
    let mut scores_arr = [0.0f64; 19];
    let idx_of = |name: &str| -> usize { INTENTS.iter().position(|&i| i == name).unwrap_or(18) };
    let has_actionable_event = detected_events.iter().any(|e| {
        e.strong
            && e.scope != "foreign"
            && !e.minor_waterlogging
            && !e.clause_flags.is_quest
            && !e.clause_flags.is_hearsay
            && !e.clause_flags.is_reader
            && !e.clause_flags.is_cond
            && !e.clause_flags.is_commentary_meta
    });
    let has_any_event = !detected_events.is_empty();
    let has_active_indonesia_scope = clause_results
        .iter()
        .any(|cr| cr.geo.scope == "Indonesia" && !cr.flags.is_reader && !cr.flags.is_hist);
    let any_commentary_meta = clause_results.iter().any(|cr| cr.flags.is_commentary_meta);

    if any_spam {
        scores_arr[idx_of("spam")] += 0.92;
    }
    if any_commercial && !has_actionable_event {
        scores_arr[idx_of("commercial_listing")] += 0.91;
    }
    if any_ad && !has_actionable_event {
        scores_arr[idx_of("advertisement")] += 0.88;
    }
    if any_vent && !has_actionable_event {
        scores_arr[idx_of("personal_vent")] += 0.88;
    }
    if any_educational && !has_actionable_event {
        scores_arr[idx_of("educational")] += 0.86;
    }
    if any_historical && !has_actionable_event {
        scores_arr[idx_of("historical")] += 0.86;
    }
    if any_satire && !has_actionable_event {
        scores_arr[idx_of("satire")] += 0.82;
    }
    if (any_denial || (!negated_events.is_empty() && !has_any_event)) && !has_actionable_event {
        scores_arr[idx_of("denial_or_debunk")] +=
            if DENIAL_ANY_RE.is_match(&norm_doc.decoded) || !negated_events.is_empty() {
                0.89
            } else {
                0.75
            };
    }
    if any_question && !any_commercial && !any_spam {
        scores_arr[idx_of("question")] +=
            if clause_results.iter().any(|c| c.flags.has_hazard_question) {
                0.84
            } else {
                0.78
            };
    }
    if any_hearsay && !any_official_warning && !any_eyewitness && !any_satire {
        scores_arr[idx_of("rumor")] += 0.78;
    }
    if any_official_warning && !any_denial && !any_satire {
        scores_arr[idx_of("official_warning")] += 0.94;
    }
    if any_official_update && has_actionable_event {
        scores_arr[idx_of("official_update")] += 0.85;
    }
    if has_actionable_event && any_eyewitness && !any_reader_action && !any_question && !any_hearsay
    {
        scores_arr[idx_of("eyewitness_report")] += 0.90;
    }
    if has_actionable_event
        && !any_reader_action
        && !any_question
        && !any_hearsay
        && !any_official_warning
    {
        scores_arr[idx_of("incident_report")] += 0.89;
    }
    if any_reader_action && has_any_event {
        scores_arr[idx_of("personal_experience")] += 0.78;
        scores_arr[idx_of("discussion")] += 0.74;
    }
    if any_commentary_meta && !has_actionable_event {
        scores_arr[idx_of("discussion")] += 0.78;
    }
    if any_minor_waterlogging && !has_actionable_event {
        let i_eye = idx_of("eyewitness_report");
        scores_arr[i_eye] = scores_arr[i_eye].max(0.78);
        scores_arr[idx_of("denial_or_debunk")] = 0.0;
    }
    if any_opinion && !has_actionable_event {
        scores_arr[idx_of("opinion")] += 0.84;
    }
    if any_metaphor && !has_actionable_event {
        scores_arr[idx_of("unrelated")] += 0.80;
    }
    if any_religious && !has_actionable_event {
        let i_unrel = idx_of("unrelated");
        scores_arr[i_unrel] = scores_arr[i_unrel].max(0.92);
    }

    let max_score = scores_arr.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if max_score < 0.45 {
        let has_supply_cue = clause_results
            .iter()
            .any(|c| c.flags.has_situational_supply_cue);
        let has_dig_pol_cue = DIG_POL_CUE_RE.is_match(&norm_doc.decoded);
        if (clause_results.iter().any(|c| c.flags.has_hazard_cue)
            || has_supply_cue
            || has_dig_pol_cue
            || has_any_event)
            && (has_active_indonesia_scope
                || LOCAL_ORG_RE.is_match(&norm_doc.decoded)
                || LOCAL_MACRO_RE.is_match(&norm_doc.decoded))
            && !any_commercial
            && !any_ad
            && !any_vent
            && !any_opinion
            && !any_spam
            && !any_metaphor
            && !any_denial
            && !any_historical
            && !any_educational
        {
            scores_arr[idx_of("discussion")] = if has_supply_cue { 0.64 } else { 0.65 };
        } else if norm_doc.normalized_prose.encode_utf16().count() < 12 {
            scores_arr[idx_of("unknown")] = 0.55;
        } else {
            scores_arr[idx_of("unrelated")] = 0.68;
        }
    }

    let mut best_intent = "unknown";
    let mut best_score = -1.0f64;
    for &intent in PRIORITY_ORDER {
        let sc = scores_arr[idx_of(intent)];
        if sc > best_score + 0.01 {
            best_score = sc;
            best_intent = intent;
        }
    }

    let mut scores_map = Map::new();
    for (i, &intent) in INTENTS.iter().enumerate() {
        scores_map.insert(intent.to_string(), crate::js::number(scores_arr[i]));
    }

    let event = detected_events.first().map(|pe| {
        let mut m = Map::new();
        m.insert("eventType".into(), Value::String(pe.event_type.into()));
        m.insert("eventAction".into(), Value::String(pe.event_action.into()));
        m.insert(
            "affectedEntity".into(),
            pe.affected_entity
                .map(|s| Value::String(s.into()))
                .unwrap_or(Value::Null),
        );
        m.insert(
            "location".into(),
            pe.location.clone().unwrap_or(Value::Null),
        );
        let ev_time = if pe.event_time.is_null() {
            global_event_time.clone()
        } else {
            pe.event_time.clone()
        };
        m.insert("eventTime".into(), ev_time);
        m.insert("impact".into(), pe.impact.clone());
        m.insert("sourceType".into(), Value::String(pe.source_type.into()));
        m.insert("evidence".into(), json!(pe.evidence));
        if !negated_events.is_empty() {
            m.insert("negatedEvents".into(), json!(negated_events.items));
        }
        Value::Object(m)
    });

    let clauses_json: Vec<Value> = clause_results
        .iter()
        .map(|cr| {
            json!({
                "clause": cr.clause.to_json(),
                "geo": cr.geo.to_json(),
                "events": cr.events.iter().map(ClauseEvent::to_json).collect::<Vec<_>>(),
                "flags": cr.flags.to_json(),
            })
        })
        .collect();

    let confidence = round2(best_score.clamp(0.15, 0.99));

    json!({
        "normDoc": norm_doc.to_json(),
        "clauses": clauses_json,
        "intent": best_intent,
        "intentScores": Value::Object(scores_map),
        "confidence": crate::js::number(confidence),
        "geographicContext": geographic_context,
        "event": event,
        "extractedEvents": detected_events.iter().map(DetectedEvent::to_json).collect::<Vec<_>>(),
        "negatedEvents": negated_events.items,
        "metaphorHits": metaphor_hits.items,
        "flags": {
            "anyForeign": any_foreign,
            "anyOfficialWarning": any_official_warning,
            "anyOfficialUpdate": any_official_update,
            "anyEyewitness": any_eyewitness,
            "anyDenial": any_denial,
            "anyHistorical": any_historical,
            "anyQuestion": any_question,
            "anyHearsay": any_hearsay,
            "anySatire": any_satire_val,
            "anyEducational": any_educational,
            "anyReligious": any_religious,
            "anyCommercial": any_commercial,
            "anyAd": any_ad,
            "anyVent": any_vent,
            "anyOpinion": any_opinion,
            "anySpam": any_spam,
            "anyMetaphor": any_metaphor,
            "anyReaderAction": any_reader_action,
            "anyMinorWaterlogging": any_minor_waterlogging,
            "hasActionableEvent": has_actionable_event,
        }
    })
}
