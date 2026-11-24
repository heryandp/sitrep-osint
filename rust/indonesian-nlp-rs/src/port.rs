//! The Rust port of `src/indonesian-nlp.js`.

#![forbid(unsafe_code)]

#[path = "analysis.rs"]
pub mod analysis;
#[path = "frames.rs"]
pub mod frames;
#[path = "../../geocode-rs/src/service.rs"]
#[allow(dead_code)]
pub mod geo_service;
mod pinned;
#[path = "tables.rs"]
pub mod tables;

use serde_json::Value;

fn js_is_truthy(v: &Value) -> bool {
    if crate::js::is_undefined(v) {
        return false;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n != 0.0 && !n.is_nan();
    }
    if let Some(units) = crate::js::string_units(v) {
        return !units.is_empty();
    }
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n.as_f64().is_some_and(|x| x != 0.0 && !x.is_nan()),
        Value::String(s) => !s.is_empty(),
        Value::Array(_) | Value::Object(_) => true,
    }
}

fn js_to_string(v: &Value) -> String {
    if crate::js::is_undefined(v) {
        return "undefined".into();
    }
    if let Some(n) = crate::js::as_number(v) {
        return crate::js::number_to_string(n);
    }
    if let Some(units) = crate::js::string_units(v) {
        return String::from_utf16_lossy(&units);
    }
    match v {
        Value::Null => "null".into(),
        Value::Bool(b) => if *b { "true" } else { "false" }.into(),
        Value::Number(n) => crate::js::number_to_string(n.as_f64().unwrap_or(f64::NAN)),
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            let mut out = String::new();
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                if !item.is_null() && !crate::js::is_undefined(item) {
                    out.push_str(&js_to_string(item));
                }
            }
            out
        }
        Value::Object(_) => "[object Object]".into(),
    }
}

fn js_or_empty_string(v: &Value) -> String {
    if !js_is_truthy(v) {
        String::new()
    } else {
        js_to_string(v)
    }
}

fn parse_js_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    if t == "Infinity" || t == "+Infinity" {
        return f64::INFINITY;
    }
    if t == "-Infinity" {
        return f64::NEG_INFINITY;
    }
    if t.eq_ignore_ascii_case("nan")
        || t.eq_ignore_ascii_case("inf")
        || t.eq_ignore_ascii_case("-inf")
        || t.eq_ignore_ascii_case("+inf")
        || t.eq_ignore_ascii_case("infinity")
        || t.eq_ignore_ascii_case("-infinity")
        || t.eq_ignore_ascii_case("+infinity")
    {
        return f64::NAN;
    }
    if let Some(hex) = t.strip_prefix("0x").or_else(|| t.strip_prefix("0X")) {
        return u64::from_str_radix(hex, 16)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(oct) = t.strip_prefix("0o").or_else(|| t.strip_prefix("0O")) {
        return u64::from_str_radix(oct, 8)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    if let Some(bin) = t.strip_prefix("0b").or_else(|| t.strip_prefix("0B")) {
        return u64::from_str_radix(bin, 2)
            .map(|v| v as f64)
            .unwrap_or(f64::NAN);
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

fn js_to_number(v: &Value) -> f64 {
    if crate::js::is_undefined(v) {
        return f64::NAN;
    }
    if let Some(n) = crate::js::as_number(v) {
        return n;
    }
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
        Value::String(s) => parse_js_number(s),
        Value::Array(arr) => match arr.as_slice() {
            [] => 0.0,
            [only] if only.is_null() || crate::js::is_undefined(only) => 0.0,
            [only] => js_to_number(only),
            _ => f64::NAN,
        },
        Value::Object(_) => f64::NAN,
    }
}

fn current_utc_year() -> f64 {
    let secs = (pinned::now_ms() / 1000.0).floor() as i64;
    let z = secs.div_euclid(86_400) + 719_468;
    let era = z.div_euclid(146_097);
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = (yoe as i64) + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (y + i64::from(m <= 2)) as f64
}

/// Port of `decodeBasicEntities` (`src/indonesian-nlp.js:73`).
pub fn decode_basic_entities(s: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&s);
    Ok(Value::String(analysis::decode_basic_entities_str(&input)))
}

/// Port of `normalizeIndonesian` (`src/indonesian-nlp.js:305`).
pub fn normalize_indonesian(raw_text: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&raw_text);
    Ok(analysis::normalize_indonesian_doc(&input).to_json())
}

/// Port of `normalizePhraseText` (`src/indonesian-nlp.js:258`).
pub fn normalize_phrase_text(text: Value, expand_geo_abbr: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&text);
    let expand = js_is_truthy(&expand_geo_abbr);
    Ok(Value::String(analysis::normalize_phrase_text_str(
        &input, expand,
    )))
}

/// Port of `segmentHashtag` (`src/indonesian-nlp.js:178`).
pub fn segment_hashtag(tag: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&tag);
    Ok(Value::String(analysis::segment_hashtag_str(&input)))
}

/// Port of `isHistoricalClause` (`src/indonesian-nlp.js:388`).
pub fn is_historical_clause(text: Value, year: Value) -> Result<Value, String> {
    let text_str = js_to_string(&text);
    let text_or_empty = js_or_empty_string(&text);
    let y = if crate::js::is_undefined(&year) {
        current_utc_year()
    } else {
        js_to_number(&year)
    };
    Ok(Value::Bool(analysis::is_historical_clause_str(
        &text_str,
        &text_or_empty,
        y,
    )))
}

/// Port of `isGenericLocationPhrase` (`src/indonesian-nlp.js:962`).
pub fn is_generic_location_phrase(phrase: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&phrase);
    Ok(Value::Bool(analysis::is_generic_location_phrase_str(
        &input,
    )))
}

/// Port of `analyzePost` (`src/indonesian-nlp.js:1013`).
pub fn analyze_post(raw_text: Value, options: Value) -> Result<Value, String> {
    if options.is_null() {
        return Err("TypeError: Cannot read properties of null (reading 'year')".into());
    }
    let year = if crate::js::is_undefined(&options) {
        current_utc_year()
    } else {
        options
            .as_object()
            .and_then(|o| o.get("year"))
            .and_then(crate::js::as_number)
            .filter(|n| n.is_finite())
            .unwrap_or_else(current_utc_year)
    };
    let input = js_or_empty_string(&raw_text);
    Ok(analysis::analyze_post_str(&input, year))
}

/// Port of `extractImpact` (`src/indonesian-nlp.js:754`).
pub fn extract_impact(raw_or_norm: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&raw_or_norm);
    Ok(frames::extract_impact_str(&input))
}

/// Port of `extractAffectedEntity` (`src/indonesian-nlp.js:778`).
pub fn extract_affected_entity(text: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&text);
    Ok(frames::extract_affected_entity_str(&input)
        .map(|s| Value::String(s.into()))
        .unwrap_or(Value::Null))
}

/// Port of `extractEventTime` (`src/indonesian-nlp.js:793`).
pub fn extract_event_time(text: Value, is_historical: Value) -> Result<Value, String> {
    let input = js_or_empty_string(&text);
    let hist = js_is_truthy(&is_historical);
    Ok(frames::extract_event_time_str(&input, hist))
}

/// Port of `metaphorBlocksTopic` (`src/indonesian-nlp.js:460`).
pub fn metaphor_blocks_topic(metaphor_ids: Value, topic_id: Value) -> Result<Value, String> {
    if !js_is_truthy(&metaphor_ids) {
        return Ok(Value::Bool(false));
    }
    let ids: Vec<String> = if let Some(arr) = metaphor_ids.as_array() {
        arr.iter().map(js_to_string).collect()
    } else if metaphor_ids.is_string() || crate::js::string_units(&metaphor_ids).is_some() {
        return Ok(Value::Bool(false));
    } else {
        return Err("TypeError: (metaphorIds || []) is not iterable".into());
    };
    let topic_str = topic_id.as_str().unwrap_or("");
    let matched = if topic_id.is_string() {
        tables::metaphor_blocks_topic_str(&ids, topic_str)
    } else {
        ids.iter().any(|id| {
            matches!(
                id.as_str(),
                "metaphor_ledakan"
                    | "metaphor_affective_mismatch"
                    | "nonincident_sport_entertainment"
            )
        })
    };
    Ok(Value::Bool(matched))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_normalization_and_entities() {
        assert_eq!(
            analysis::decode_basic_entities_str("A &amp; B &lt;C&gt; &#65; &#x42;"),
            "A & B <C> A B"
        );
        assert_eq!(
            analysis::normalize_phrase_text_str(
                "Gue gak bisa jalan-jalan di Jaksel krn ujan bgt",
                true
            ),
            "saya tidak bisa wisata jalan_jalan di jakarta selatan karena hujan banget"
        );
        assert_eq!(
            analysis::normalize_phrase_text_str("Rumah-rumah bangeeeet", false),
            "rumah rumah banget"
        );
    }

    #[test]
    fn test_hashtag_segmentation() {
        assert_eq!(
            analysis::segment_hashtag_str("#BanjirJakartaSelatan"),
            "banjir jakarta selatan"
        );
        assert_eq!(
            analysis::segment_hashtag_str("#banjirbandanggarut"),
            "banjirbandang garut"
        );
        assert_eq!(
            analysis::segment_hashtag_str("#CoretaxError"),
            "coretax error"
        );
        assert_eq!(analysis::segment_hashtag_str("###"), "");
    }

    #[test]
    fn test_foreign_language_filtering() {
        let nl = analysis::analyze_post_str(
            "De regering in Den Haag heeft vandaag een nieuwe coalitie gevormd na de verkiezingen.",
            2026.0,
        );
        assert_eq!(nl["intent"], json!("unrelated"));
        assert_eq!(nl["flags"]["anyForeign"], json!(true));
        assert_eq!(nl["layerTrace"]["isForeignLanguage"], json!(true));

        let id = analysis::analyze_post_str(
            "Banjir bandang merendam ratusan rumah warga di Garut pagi ini, BPBD mengevakuasi korban.",
            2026.0,
        );
        assert_eq!(id["flags"]["anyForeign"], json!(false));
        assert_eq!(id["flags"]["hasActionableEvent"], json!(true));
        assert_eq!(id["event"]["eventType"], json!("flood"));
    }

    #[test]
    fn test_metaphor_blocking() {
        assert!(tables::metaphor_blocks_topic_str(
            &["metaphor_banjir".into()],
            "banjir"
        ));
        assert!(!tables::metaphor_blocks_topic_str(
            &["metaphor_banjir".into()],
            "gempa"
        ));
        assert!(tables::metaphor_blocks_topic_str(
            &["metaphor_ledakan".into()],
            "gempa"
        ));

        let meta_post = analysis::analyze_post_str(
            "Banjir orderan dan banjir cuan di toko online hari ini!",
            2026.0,
        );
        assert_eq!(meta_post["flags"]["anyMetaphor"], json!(true));
        assert_eq!(meta_post["flags"]["hasActionableEvent"], json!(false));
    }

    #[test]
    fn test_analyze_post_events_and_errors() {
        let err = analyze_post(json!("test"), Value::Null);
        assert!(err.is_err());

        let quake = analysis::analyze_post_str(
            "Gempa M5.6 mengguncang Cianjur pukul 13.21 WIB, terasa kencang sampai rumah saya bergetar!",
            2026.0,
        );
        assert_eq!(quake["intent"], json!("eyewitness_report"));
        assert_eq!(quake["event"]["eventType"], json!("earthquake"));
        assert_eq!(quake["event"]["sourceType"], json!("eyewitness"));
    }
}
