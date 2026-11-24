//! JS bindings for `src/social-cluster.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `classifySourceProvenance`.
#[napi(js_name = "classifySourceProvenance")]
pub fn classify_source_provenance(post: String, assessment: String) -> napi::Result<String> {
    run(move || {
        let post = crate::js::from_json(&post)?;
        let assessment = crate::js::from_json(&assessment)?;
        let out = port::classify_source_provenance(post, assessment)?;
        crate::js::to_json(&out)
    })
}

/// `stripBoilerplateForSimilarity`.
#[napi(js_name = "stripBoilerplateForSimilarity")]
pub fn strip_boilerplate_for_similarity(text: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::strip_boilerplate_for_similarity(text)?;
        crate::js::to_json(&out)
    })
}

/// `similarityTokens`.
#[napi(js_name = "similarityTokens")]
pub fn similarity_tokens(text: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::similarity_tokens(text)?;
        crate::js::to_json(&out)
    })
}

/// `isNearDuplicate`.
#[napi(js_name = "isNearDuplicate")]
pub fn is_near_duplicate(
    text_a: String,
    text_b: String,
    threshold: String,
) -> napi::Result<String> {
    run(move || {
        let text_a = crate::js::from_json(&text_a)?;
        let text_b = crate::js::from_json(&text_b)?;
        let threshold = crate::js::from_json(&threshold)?;
        let out = port::is_near_duplicate(text_a, text_b, threshold)?;
        crate::js::to_json(&out)
    })
}

/// `deduplicatePosts`.
#[napi(js_name = "deduplicatePosts")]
pub fn deduplicate_posts(posts: String, opts: String) -> napi::Result<String> {
    run(move || {
        let posts = crate::js::from_json(&posts)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::deduplicate_posts(posts, opts)?;
        crate::js::to_json(&out)
    })
}

/// `eventSpecificTokens`.
#[napi(js_name = "eventSpecificTokens")]
pub fn event_specific_tokens(ev_text: String, place_name: String) -> napi::Result<String> {
    run(move || {
        let ev_text = crate::js::from_json(&ev_text)?;
        let place_name = crate::js::from_json(&place_name)?;
        let out = port::event_specific_tokens(ev_text, place_name)?;
        crate::js::to_json(&out)
    })
}

/// `clusterIncidents`.
#[napi(js_name = "clusterIncidents")]
pub fn cluster_incidents(posts: String, opts: String) -> napi::Result<String> {
    run(move || {
        let posts = crate::js::from_json(&posts)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::cluster_incidents(posts, opts)?;
        crate::js::to_json(&out)
    })
}

/// `matchPostToNews`.
#[napi(js_name = "matchPostToNews")]
pub fn match_post_to_news(post: String, news_events: String, now: String) -> napi::Result<String> {
    run(move || {
        let post = crate::js::from_json(&post)?;
        let news_events = crate::js::from_json(&news_events)?;
        let now = crate::js::from_json(&now)?;
        let out = port::match_post_to_news(post, news_events, now)?;
        crate::js::to_json(&out)
    })
}

/// `collapseSocialFeed`.
#[napi(js_name = "collapseSocialFeed")]
pub fn collapse_social_feed(posts: String, opts: String) -> napi::Result<String> {
    run(move || {
        let posts = crate::js::from_json(&posts)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::collapse_social_feed(posts, opts)?;
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

static S_CLASSIFY_SOURCE_PROVENANCE_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_SOURCE_PROVENANCE_1: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_SOURCE_PROVENANCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifySourceProvenance` over the binary wire.
#[napi(js_name = "classifySourceProvenanceWire")]
pub fn classify_source_provenance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let post: Value = wire::read_value(r, &S_CLASSIFY_SOURCE_PROVENANCE_0)?;
        let assessment: Value = wire::read_value(r, &S_CLASSIFY_SOURCE_PROVENANCE_1)?;
        let out = &port::classify_source_provenance(post, assessment)?;
        wire::write_value(w, out, &S_CLASSIFY_SOURCE_PROVENANCE_OUT)?;
        Ok(())
    })
}

static S_STRIP_BOILERPLATE_FOR_SIMILARITY_0: wire::Schema = wire::Schema::Dyn;
static S_STRIP_BOILERPLATE_FOR_SIMILARITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `stripBoilerplateForSimilarity` over the binary wire.
#[napi(js_name = "stripBoilerplateForSimilarityWire")]
pub fn strip_boilerplate_for_similarity_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_STRIP_BOILERPLATE_FOR_SIMILARITY_0)?;
        let out = &port::strip_boilerplate_for_similarity(text)?;
        wire::write_value(w, out, &S_STRIP_BOILERPLATE_FOR_SIMILARITY_OUT)?;
        Ok(())
    })
}

static S_SIMILARITY_TOKENS_0: wire::Schema = wire::Schema::Dyn;
static S_SIMILARITY_TOKENS_OUT: wire::Schema = wire::Schema::Dyn;

/// `similarityTokens` over the binary wire.
#[napi(js_name = "similarityTokensWire")]
pub fn similarity_tokens_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: Value = wire::read_value(r, &S_SIMILARITY_TOKENS_0)?;
        let out = &port::similarity_tokens(text)?;
        wire::write_value(w, out, &S_SIMILARITY_TOKENS_OUT)?;
        Ok(())
    })
}

static S_IS_NEAR_DUPLICATE_0: wire::Schema = wire::Schema::Dyn;
static S_IS_NEAR_DUPLICATE_1: wire::Schema = wire::Schema::Dyn;
static S_IS_NEAR_DUPLICATE_2: wire::Schema = wire::Schema::Dyn;
static S_IS_NEAR_DUPLICATE_OUT: wire::Schema = wire::Schema::Dyn;

/// `isNearDuplicate` over the binary wire.
#[napi(js_name = "isNearDuplicateWire")]
pub fn is_near_duplicate_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text_a: Value = wire::read_value(r, &S_IS_NEAR_DUPLICATE_0)?;
        let text_b: Value = wire::read_value(r, &S_IS_NEAR_DUPLICATE_1)?;
        let threshold: Value = wire::read_value(r, &S_IS_NEAR_DUPLICATE_2)?;
        let out = &port::is_near_duplicate(text_a, text_b, threshold)?;
        wire::write_value(w, out, &S_IS_NEAR_DUPLICATE_OUT)?;
        Ok(())
    })
}

static S_DEDUPLICATE_POSTS_0: wire::Schema = wire::Schema::Dyn;
static S_DEDUPLICATE_POSTS_1: wire::Schema = wire::Schema::Dyn;
static S_DEDUPLICATE_POSTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `deduplicatePosts` over the binary wire.
#[napi(js_name = "deduplicatePostsWire")]
pub fn deduplicate_posts_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let posts: Value = wire::read_value(r, &S_DEDUPLICATE_POSTS_0)?;
        let opts: Value = wire::read_value(r, &S_DEDUPLICATE_POSTS_1)?;
        let out = &port::deduplicate_posts(posts, opts)?;
        wire::write_value(w, out, &S_DEDUPLICATE_POSTS_OUT)?;
        Ok(())
    })
}

static S_EVENT_SPECIFIC_TOKENS_0: wire::Schema = wire::Schema::Dyn;
static S_EVENT_SPECIFIC_TOKENS_1: wire::Schema = wire::Schema::Dyn;
static S_EVENT_SPECIFIC_TOKENS_OUT: wire::Schema = wire::Schema::Dyn;

/// `eventSpecificTokens` over the binary wire.
#[napi(js_name = "eventSpecificTokensWire")]
pub fn event_specific_tokens_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ev_text: Value = wire::read_value(r, &S_EVENT_SPECIFIC_TOKENS_0)?;
        let place_name: Value = wire::read_value(r, &S_EVENT_SPECIFIC_TOKENS_1)?;
        let out = &port::event_specific_tokens(ev_text, place_name)?;
        wire::write_value(w, out, &S_EVENT_SPECIFIC_TOKENS_OUT)?;
        Ok(())
    })
}

static S_CLUSTER_INCIDENTS_0: wire::Schema = wire::Schema::Dyn;
static S_CLUSTER_INCIDENTS_1: wire::Schema = wire::Schema::Dyn;
static S_CLUSTER_INCIDENTS_OUT: wire::Schema = wire::Schema::Dyn;

/// `clusterIncidents` over the binary wire.
#[napi(js_name = "clusterIncidentsWire")]
pub fn cluster_incidents_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let posts: Value = wire::read_value(r, &S_CLUSTER_INCIDENTS_0)?;
        let opts: Value = wire::read_value(r, &S_CLUSTER_INCIDENTS_1)?;
        let out = &port::cluster_incidents(posts, opts)?;
        wire::write_value(w, out, &S_CLUSTER_INCIDENTS_OUT)?;
        Ok(())
    })
}

static S_MATCH_POST_TO_NEWS_0: wire::Schema = wire::Schema::Dyn;
static S_MATCH_POST_TO_NEWS_1: wire::Schema = wire::Schema::Dyn;
static S_MATCH_POST_TO_NEWS_2: wire::Schema = wire::Schema::Dyn;
static S_MATCH_POST_TO_NEWS_OUT: wire::Schema = wire::Schema::Dyn;

/// `matchPostToNews` over the binary wire.
#[napi(js_name = "matchPostToNewsWire")]
pub fn match_post_to_news_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let post: Value = wire::read_value(r, &S_MATCH_POST_TO_NEWS_0)?;
        let news_events: Value = wire::read_value(r, &S_MATCH_POST_TO_NEWS_1)?;
        let now: Value = wire::read_value(r, &S_MATCH_POST_TO_NEWS_2)?;
        let out = &port::match_post_to_news(post, news_events, now)?;
        wire::write_value(w, out, &S_MATCH_POST_TO_NEWS_OUT)?;
        Ok(())
    })
}

static S_COLLAPSE_SOCIAL_FEED_0: wire::Schema = wire::Schema::Dyn;
static S_COLLAPSE_SOCIAL_FEED_1: wire::Schema = wire::Schema::Dyn;
static S_COLLAPSE_SOCIAL_FEED_OUT: wire::Schema = wire::Schema::Dyn;

/// `collapseSocialFeed` over the binary wire.
#[napi(js_name = "collapseSocialFeedWire")]
pub fn collapse_social_feed_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let posts: Value = wire::read_value(r, &S_COLLAPSE_SOCIAL_FEED_0)?;
        let opts: Value = wire::read_value(r, &S_COLLAPSE_SOCIAL_FEED_1)?;
        let out = &port::collapse_social_feed(posts, opts)?;
        wire::write_value(w, out, &S_COLLAPSE_SOCIAL_FEED_OUT)?;
        Ok(())
    })
}
