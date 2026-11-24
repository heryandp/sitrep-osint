//! JS bindings for `src/indonesian-nlp.js`: generated from the signatures in `port.rs` by rust-anything.
//! Regenerate with `rust-anything refresh` after changing them.

use napi_derive::napi;

pub mod js;
pub mod port;
pub mod wire;

#[allow(unused_imports)]
use serde_json::Value;
pub mod utf16;

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|s| (*s).to_string())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "unknown panic".into())
}

#[allow(dead_code)]
fn run<T>(f: impl FnOnce() -> Result<T, String>) -> napi::Result<T> {
    match std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)) {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `decodeBasicEntities`.
#[napi(js_name = "decodeBasicEntities")]
pub fn decode_basic_entities(s: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let out = port::decode_basic_entities(s)?;
        crate::js::to_json(&out)
    })
}

/// `normalizeIndonesian`.
#[napi(js_name = "normalizeIndonesian")]
pub fn normalize_indonesian(raw_text: String) -> napi::Result<String> {
    run(move || {
        let raw_text = crate::js::from_json(&raw_text)?;
        let out = port::normalize_indonesian(raw_text)?;
        crate::js::to_json(&out)
    })
}

/// `normalizePhraseText`.
#[napi(js_name = "normalizePhraseText")]
pub fn normalize_phrase_text(text: String, expand_geo_abbr: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let expand_geo_abbr = crate::js::from_json(&expand_geo_abbr)?;
        let out = port::normalize_phrase_text(text, expand_geo_abbr)?;
        crate::js::to_json(&out)
    })
}

/// `segmentHashtag`.
#[napi(js_name = "segmentHashtag")]
pub fn segment_hashtag(tag: String) -> napi::Result<String> {
    run(move || {
        let tag = crate::js::from_json(&tag)?;
        let out = port::segment_hashtag(tag)?;
        crate::js::to_json(&out)
    })
}

/// `isHistoricalClause`.
#[napi(js_name = "isHistoricalClause")]
pub fn is_historical_clause(text: String, year: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let year = crate::js::from_json(&year)?;
        let out = port::is_historical_clause(text, year)?;
        crate::js::to_json(&out)
    })
}

/// `isGenericLocationPhrase`.
#[napi(js_name = "isGenericLocationPhrase")]
pub fn is_generic_location_phrase(phrase: String) -> napi::Result<String> {
    run(move || {
        let phrase = crate::js::from_json(&phrase)?;
        let out = port::is_generic_location_phrase(phrase)?;
        crate::js::to_json(&out)
    })
}

/// `analyzePost`.
#[napi(js_name = "analyzePost")]
pub fn analyze_post(raw_text: String, options: String) -> napi::Result<String> {
    run(move || {
        let raw_text = crate::js::from_json(&raw_text)?;
        let options = crate::js::from_json(&options)?;
        let out = port::analyze_post(raw_text, options)?;
        crate::js::to_json(&out)
    })
}

/// `extractImpact`.
#[napi(js_name = "extractImpact")]
pub fn extract_impact(raw_or_norm: String) -> napi::Result<String> {
    run(move || {
        let raw_or_norm = crate::js::from_json(&raw_or_norm)?;
        let out = port::extract_impact(raw_or_norm)?;
        crate::js::to_json(&out)
    })
}

/// `extractAffectedEntity`.
#[napi(js_name = "extractAffectedEntity")]
pub fn extract_affected_entity(text: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::extract_affected_entity(text)?;
        crate::js::to_json(&out)
    })
}

/// `extractEventTime`.
#[napi(js_name = "extractEventTime")]
pub fn extract_event_time(text: String, is_historical: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let is_historical = crate::js::from_json(&is_historical)?;
        let out = port::extract_event_time(text, is_historical)?;
        crate::js::to_json(&out)
    })
}

/// `metaphorBlocksTopic`.
#[napi(js_name = "metaphorBlocksTopic")]
pub fn metaphor_blocks_topic(metaphor_ids: String, topic_id: String) -> napi::Result<String> {
    run(move || {
        let metaphor_ids = crate::js::from_json(&metaphor_ids)?;
        let topic_id = crate::js::from_json(&topic_id)?;
        let out = port::metaphor_blocks_topic(metaphor_ids, topic_id)?;
        crate::js::to_json(&out)
    })
}

// ---- the binary wire ------------------------------------------------------------------------

type WireBuffers = (
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint8Array,
    napi::bindgen_prelude::Uint32Array,
);

thread_local! {
    static WIRE: std::cell::RefCell<Option<WireBuffers>> = const { std::cell::RefCell::new(None) };
    static PENDING: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
    static SCRATCH: std::cell::RefCell<Vec<u8>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// Registers the argument, result and argument-length buffers `index.cjs` uses.
#[napi(js_name = "rustAnythingWireInit")]
pub fn rust_anything_wire_init(
    input: napi::bindgen_prelude::Uint8Array,
    output: napi::bindgen_prelude::Uint8Array,
    len: napi::bindgen_prelude::Uint32Array,
) {
    WIRE.with(|w| *w.borrow_mut() = Some((input, output, len)));
}

/// Copies a result that did not fit into the (now larger) result buffer.
#[napi(js_name = "rustAnythingWireFlush")]
pub fn rust_anything_wire_flush() -> f64 {
    let bytes = PENDING.with(|p| std::mem::take(&mut *p.borrow_mut()));
    WIRE.with(|w| {
        let mut w = w.borrow_mut();
        let Some((_, output, _)) = w.as_mut() else {
            return -1.0;
        };
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = bytes.len().min(out.len());
        out[..n].copy_from_slice(&bytes[..n]);
        n as f64
    })
}

/// Reads the argument bytes (`wLen[0]` of them), runs `f`, and writes its result; a negative
/// return is the size of a result that did not fit, kept for `rustAnythingWireFlush`.
fn wire_io(
    f: impl FnOnce(&mut wire::Reader, &mut wire::Writer) -> Result<(), String>,
) -> napi::Result<f64> {
    WIRE.with(|cell| {
        let mut guard = cell.borrow_mut();
        let Some((input, output, len)) = guard.as_mut() else {
            return Err(napi::Error::from_reason(
                "rust-anything: the wire buffers are not registered",
            ));
        };
        let len = (len.first().copied().unwrap_or(0) as usize).min(input.len());
        let mut w = wire::Writer(SCRATCH.with(|s| std::mem::take(&mut *s.borrow_mut())));
        w.0.clear();
        let result = run(|| f(&mut wire::Reader::new(&input[..len]), &mut w));
        let keep = |w: wire::Writer| SCRATCH.with(|s| *s.borrow_mut() = w.0);
        match result {
            // A result of another shape than the TS type: JS runs the call through JSON.
            Err(e) if e.reason.starts_with(wire::MISMATCH) => {
                keep(w);
                return Ok(f64::NAN);
            }
            Err(e) => {
                keep(w);
                let mut m = wire::Writer::default();
                m.string16(&e.reason);
                // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
                let out = unsafe { output.as_mut() };
                if m.0.len() > out.len() {
                    return Err(e);
                }
                out[..m.0.len()].copy_from_slice(&m.0);
                return Ok(f64::INFINITY);
            }
            Ok(()) => {}
        }
        // SAFETY: the call is synchronous; JS does not touch `output` until it returns.
        let out = unsafe { output.as_mut() };
        let n = w.0.len();
        if n > out.len() {
            PENDING.with(|p| *p.borrow_mut() = w.0);
            return Ok(-(n as f64));
        }
        out[..n].copy_from_slice(&w.0);
        keep(w);
        Ok(n as f64)
    })
}

static S_DECODE_BASIC_ENTITIES_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_BASIC_ENTITIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `decodeBasicEntities` over the binary wire.
#[napi(js_name = "decodeBasicEntitiesWire")]
pub fn decode_basic_entities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: Value = wire::read_value(r, &S_DECODE_BASIC_ENTITIES_0)?;
        let out = &port::decode_basic_entities(s)?;
        wire::write_value(w, out, &S_DECODE_BASIC_ENTITIES_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_INDONESIAN_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_INDONESIAN_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizeIndonesian` over the binary wire.
#[napi(js_name = "normalizeIndonesianWire")]
pub fn normalize_indonesian_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_text: Value = wire::read_value(r, &S_NORMALIZE_INDONESIAN_0)?;
        let out = &port::normalize_indonesian(raw_text)?;
        wire::write_value(w, out, &S_NORMALIZE_INDONESIAN_OUT)?;
        Ok(())
    })
}

static S_NORMALIZE_PHRASE_TEXT_0: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PHRASE_TEXT_1: wire::Schema = wire::Schema::Dyn;
static S_NORMALIZE_PHRASE_TEXT_OUT: wire::Schema = wire::Schema::Dyn;

/// `normalizePhraseText` over the binary wire.
#[napi(js_name = "normalizePhraseTextWire")]
pub fn normalize_phrase_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_NORMALIZE_PHRASE_TEXT_0)?;
        let expand_geo_abbr: Value = wire::read_value(r, &S_NORMALIZE_PHRASE_TEXT_1)?;
        let out = &port::normalize_phrase_text(text, expand_geo_abbr)?;
        wire::write_value(w, out, &S_NORMALIZE_PHRASE_TEXT_OUT)?;
        Ok(())
    })
}

static S_SEGMENT_HASHTAG_0: wire::Schema = wire::Schema::Dyn;
static S_SEGMENT_HASHTAG_OUT: wire::Schema = wire::Schema::Dyn;

/// `segmentHashtag` over the binary wire.
#[napi(js_name = "segmentHashtagWire")]
pub fn segment_hashtag_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tag: Value = wire::read_value(r, &S_SEGMENT_HASHTAG_0)?;
        let out = &port::segment_hashtag(tag)?;
        wire::write_value(w, out, &S_SEGMENT_HASHTAG_OUT)?;
        Ok(())
    })
}

static S_IS_HISTORICAL_CLAUSE_0: wire::Schema = wire::Schema::Dyn;
static S_IS_HISTORICAL_CLAUSE_1: wire::Schema = wire::Schema::Dyn;
static S_IS_HISTORICAL_CLAUSE_OUT: wire::Schema = wire::Schema::Dyn;

/// `isHistoricalClause` over the binary wire.
#[napi(js_name = "isHistoricalClauseWire")]
pub fn is_historical_clause_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_IS_HISTORICAL_CLAUSE_0)?;
        let year: Value = wire::read_value(r, &S_IS_HISTORICAL_CLAUSE_1)?;
        let out = &port::is_historical_clause(text, year)?;
        wire::write_value(w, out, &S_IS_HISTORICAL_CLAUSE_OUT)?;
        Ok(())
    })
}

static S_IS_GENERIC_LOCATION_PHRASE_0: wire::Schema = wire::Schema::Dyn;
static S_IS_GENERIC_LOCATION_PHRASE_OUT: wire::Schema = wire::Schema::Dyn;

/// `isGenericLocationPhrase` over the binary wire.
#[napi(js_name = "isGenericLocationPhraseWire")]
pub fn is_generic_location_phrase_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let phrase: Value = wire::read_value(r, &S_IS_GENERIC_LOCATION_PHRASE_0)?;
        let out = &port::is_generic_location_phrase(phrase)?;
        wire::write_value(w, out, &S_IS_GENERIC_LOCATION_PHRASE_OUT)?;
        Ok(())
    })
}

static S_ANALYZE_POST_0: wire::Schema = wire::Schema::Dyn;
static S_ANALYZE_POST_1: wire::Schema = wire::Schema::Dyn;
static S_ANALYZE_POST_OUT: wire::Schema = wire::Schema::Dyn;

/// `analyzePost` over the binary wire.
#[napi(js_name = "analyzePostWire")]
pub fn analyze_post_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_text: Value = wire::read_value(r, &S_ANALYZE_POST_0)?;
        let options: Value = wire::read_value(r, &S_ANALYZE_POST_1)?;
        let out = &port::analyze_post(raw_text, options)?;
        wire::write_value(w, out, &S_ANALYZE_POST_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_IMPACT_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_IMPACT_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractImpact` over the binary wire.
#[napi(js_name = "extractImpactWire")]
pub fn extract_impact_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_or_norm: Value = wire::read_value(r, &S_EXTRACT_IMPACT_0)?;
        let out = &port::extract_impact(raw_or_norm)?;
        wire::write_value(w, out, &S_EXTRACT_IMPACT_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_AFFECTED_ENTITY_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_AFFECTED_ENTITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractAffectedEntity` over the binary wire.
#[napi(js_name = "extractAffectedEntityWire")]
pub fn extract_affected_entity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_EXTRACT_AFFECTED_ENTITY_0)?;
        let out = &port::extract_affected_entity(text)?;
        wire::write_value(w, out, &S_EXTRACT_AFFECTED_ENTITY_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_EVENT_TIME_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_EVENT_TIME_1: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_EVENT_TIME_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractEventTime` over the binary wire.
#[napi(js_name = "extractEventTimeWire")]
pub fn extract_event_time_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_EXTRACT_EVENT_TIME_0)?;
        let is_historical: Value = wire::read_value(r, &S_EXTRACT_EVENT_TIME_1)?;
        let out = &port::extract_event_time(text, is_historical)?;
        wire::write_value(w, out, &S_EXTRACT_EVENT_TIME_OUT)?;
        Ok(())
    })
}

static S_METAPHOR_BLOCKS_TOPIC_0: wire::Schema = wire::Schema::Dyn;
static S_METAPHOR_BLOCKS_TOPIC_1: wire::Schema = wire::Schema::Dyn;
static S_METAPHOR_BLOCKS_TOPIC_OUT: wire::Schema = wire::Schema::Dyn;

/// `metaphorBlocksTopic` over the binary wire.
#[napi(js_name = "metaphorBlocksTopicWire")]
pub fn metaphor_blocks_topic_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let metaphor_ids: Value = wire::read_value(r, &S_METAPHOR_BLOCKS_TOPIC_0)?;
        let topic_id: Value = wire::read_value(r, &S_METAPHOR_BLOCKS_TOPIC_1)?;
        let out = &port::metaphor_blocks_topic(metaphor_ids, topic_id)?;
        wire::write_value(w, out, &S_METAPHOR_BLOCKS_TOPIC_OUT)?;
        Ok(())
    })
}
