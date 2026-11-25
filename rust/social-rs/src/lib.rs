//! JS bindings for `src/social.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `socAssess`.
#[napi(js_name = "socAssess")]
pub fn soc_assess(text: String, extra: String, context: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let extra = crate::js::from_json(&extra)?;
        let context = crate::js::from_json(&context)?;
        let out = port::soc_assess(text, extra, context)?;
        crate::js::to_json(&out)
    })
}

/// `socTopics`.
#[napi(js_name = "socTopics")]
pub fn soc_topics(text: String, extra: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let extra = crate::js::from_json(&extra)?;
        let out = port::soc_topics(text, extra)?;
        crate::js::to_json(&out)
    })
}

/// `socAdhoc`.
#[napi(js_name = "socAdhoc")]
pub fn soc_adhoc(phrases: String) -> napi::Result<String> {
    run(move || {
        let phrases = crate::js::from_json(&phrases)?;
        let out = port::soc_adhoc(phrases)?;
        crate::js::to_json(&out)
    })
}

/// `socKeepX`.
#[napi(js_name = "socKeepX")]
pub fn soc_keep_x(p: String) -> napi::Result<String> {
    run(move || {
        let p = crate::js::from_json(&p)?;
        let out = port::soc_keep_x(p)?;
        crate::js::to_json(&out)
    })
}

/// `socKeepTh`.
#[napi(js_name = "socKeepTh")]
pub fn soc_keep_th(p: String) -> napi::Result<String> {
    run(move || {
        let p = crate::js::from_json(&p)?;
        let out = port::soc_keep_th(p)?;
        crate::js::to_json(&out)
    })
}

/// `socBalance`.
#[napi(js_name = "socBalance")]
pub fn soc_balance(
    arr: String,
    max: String,
    user_of: String,
    per_topic: Option<f64>,
    per_user: Option<f64>,
    user_cap: Option<f64>,
) -> napi::Result<String> {
    run(move || {
        let arr = crate::js::from_json(&arr)?;
        let max = crate::js::from_json(&max)?;
        let user_of = crate::js::from_json(&user_of)?;
        let out = port::soc_balance(arr, max, user_of, per_topic, per_user, user_cap)?;
        crate::js::to_json(&out)
    })
}

/// `socQueries`.
#[napi(js_name = "socQueries")]
pub fn soc_queries(kind: String, extra: String) -> napi::Result<String> {
    run(move || {
        let kind = crate::js::from_json(&kind)?;
        let extra = crate::js::from_json(&extra)?;
        let out = port::soc_queries(kind, extra)?;
        crate::js::to_json(&out)
    })
}

/// `socHot`.
#[napi(js_name = "socHot")]
pub fn soc_hot(posts: String, now: String) -> napi::Result<String> {
    run(move || {
        let posts = crate::js::from_json(&posts)?;
        let now = crate::js::from_json(&now)?;
        let out = port::soc_hot(posts, now)?;
        crate::js::to_json(&out)
    })
}

/// `socNonLatin`.
#[napi(js_name = "socNonLatin")]
pub fn soc_non_latin(t: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::soc_non_latin(t)?;
        crate::js::to_json(&out)
    })
}

/// `socVi`.
#[napi(js_name = "socVi")]
pub fn soc_vi(t: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::soc_vi(t)?;
        crate::js::to_json(&out)
    })
}

/// `socHashJunk`.
#[napi(js_name = "socHashJunk")]
pub fn soc_hash_junk(t: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::soc_hash_junk(t)?;
        crate::js::to_json(&out)
    })
}

/// `astroturfSig`.
#[napi(js_name = "astroturfSig")]
pub fn astroturf_sig(t: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::astroturf_sig(t)?;
        crate::js::to_json(&out)
    })
}

/// `socForeignLang`.
#[napi(js_name = "socForeignLang")]
pub fn soc_foreign_lang(t: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::soc_foreign_lang(t)?;
        crate::js::to_json(&out)
    })
}

/// `socKeywordMine`.
#[napi(js_name = "socKeywordMine")]
pub fn soc_keyword_mine(posts: String, topic_id: String, arg2: String) -> napi::Result<String> {
    run(move || {
        let posts = crate::js::from_json(&posts)?;
        let topic_id = crate::js::from_json(&topic_id)?;
        let arg2 = crate::js::from_json(&arg2)?;
        let out = port::soc_keyword_mine(posts, topic_id, arg2)?;
        crate::js::to_json(&out)
    })
}

/// `explainPost`.
#[napi(js_name = "explainPost")]
pub fn explain_post(text: String, extra: String, context: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let extra = crate::js::from_json(&extra)?;
        let context = crate::js::from_json(&context)?;
        let out = port::explain_post(text, extra, context)?;
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

static S_SOC_ASSESS_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_ASSESS_1: wire::Schema = wire::Schema::Dyn;
static S_SOC_ASSESS_2: wire::Schema = wire::Schema::Dyn;
static S_SOC_ASSESS_OUT: wire::Schema = wire::Schema::Dyn;

/// `socAssess` over the binary wire.
#[napi(js_name = "socAssessWire")]
pub fn soc_assess_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: serde_json::Value = wire::read_value(r, &S_SOC_ASSESS_0)?;
        let extra: serde_json::Value = wire::read_value(r, &S_SOC_ASSESS_1)?;
        let context: serde_json::Value = wire::read_value(r, &S_SOC_ASSESS_2)?;
        let out = &port::soc_assess(text, extra, context)?;
        wire::write_value(w, out, &S_SOC_ASSESS_OUT)?;
        Ok(())
    })
}

static S_SOC_TOPICS_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_TOPICS_1: wire::Schema = wire::Schema::Dyn;
static S_SOC_TOPICS_OUT: wire::Schema = wire::Schema::Dyn;

/// `socTopics` over the binary wire.
#[napi(js_name = "socTopicsWire")]
pub fn soc_topics_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: serde_json::Value = wire::read_value(r, &S_SOC_TOPICS_0)?;
        let extra: serde_json::Value = wire::read_value(r, &S_SOC_TOPICS_1)?;
        let out = &port::soc_topics(text, extra)?;
        wire::write_value(w, out, &S_SOC_TOPICS_OUT)?;
        Ok(())
    })
}

static S_SOC_ADHOC_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_ADHOC_OUT: wire::Schema = wire::Schema::Dyn;

/// `socAdhoc` over the binary wire.
#[napi(js_name = "socAdhocWire")]
pub fn soc_adhoc_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let phrases: serde_json::Value = wire::read_value(r, &S_SOC_ADHOC_0)?;
        let out = &port::soc_adhoc(phrases)?;
        wire::write_value(w, out, &S_SOC_ADHOC_OUT)?;
        Ok(())
    })
}

static S_SOC_KEEP_X_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_KEEP_X_OUT: wire::Schema = wire::Schema::Dyn;

/// `socKeepX` over the binary wire.
#[napi(js_name = "socKeepXWire")]
pub fn soc_keep_x_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let p: serde_json::Value = wire::read_value(r, &S_SOC_KEEP_X_0)?;
        let out = &port::soc_keep_x(p)?;
        wire::write_value(w, out, &S_SOC_KEEP_X_OUT)?;
        Ok(())
    })
}

static S_SOC_KEEP_TH_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_KEEP_TH_OUT: wire::Schema = wire::Schema::Dyn;

/// `socKeepTh` over the binary wire.
#[napi(js_name = "socKeepThWire")]
pub fn soc_keep_th_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let p: serde_json::Value = wire::read_value(r, &S_SOC_KEEP_TH_0)?;
        let out = &port::soc_keep_th(p)?;
        wire::write_value(w, out, &S_SOC_KEEP_TH_OUT)?;
        Ok(())
    })
}

static S_SOC_BALANCE_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_BALANCE_1: wire::Schema = wire::Schema::Dyn;
static S_SOC_BALANCE_2: wire::Schema = wire::Schema::Dyn;
static S_SOC_BALANCE_OUT: wire::Schema = wire::Schema::Dyn;

/// `socBalance` over the binary wire.
#[napi(js_name = "socBalanceWire")]
pub fn soc_balance_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arr: serde_json::Value = wire::read_value(r, &S_SOC_BALANCE_0)?;
        let max: serde_json::Value = wire::read_value(r, &S_SOC_BALANCE_1)?;
        let user_of: serde_json::Value = wire::read_value(r, &S_SOC_BALANCE_2)?;
        let per_topic: Option<f64> = r.opt_js(|r| r.f64())?;
        let per_user: Option<f64> = r.opt_js(|r| r.f64())?;
        let user_cap: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::soc_balance(arr, max, user_of, per_topic, per_user, user_cap)?;
        wire::write_value(w, out, &S_SOC_BALANCE_OUT)?;
        Ok(())
    })
}

static S_SOC_QUERIES_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_QUERIES_1: wire::Schema = wire::Schema::Dyn;
static S_SOC_QUERIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `socQueries` over the binary wire.
#[napi(js_name = "socQueriesWire")]
pub fn soc_queries_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let kind: serde_json::Value = wire::read_value(r, &S_SOC_QUERIES_0)?;
        let extra: serde_json::Value = wire::read_value(r, &S_SOC_QUERIES_1)?;
        let out = &port::soc_queries(kind, extra)?;
        wire::write_value(w, out, &S_SOC_QUERIES_OUT)?;
        Ok(())
    })
}

static S_SOC_HOT_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_HOT_OUT: wire::Schema = wire::Schema::Dyn;

/// `socHot` over the binary wire.
#[napi(js_name = "socHotWire")]
pub fn soc_hot_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let posts: serde_json::Value = wire::read_value(r, &S_SOC_HOT_0)?;
        let now: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::soc_hot(posts, now)?;
        wire::write_value(w, out, &S_SOC_HOT_OUT)?;
        Ok(())
    })
}

static S_SOC_NON_LATIN_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_NON_LATIN_OUT: wire::Schema = wire::Schema::Dyn;

/// `socNonLatin` over the binary wire.
#[napi(js_name = "socNonLatinWire")]
pub fn soc_non_latin_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_SOC_NON_LATIN_0)?;
        let out = &port::soc_non_latin(t)?;
        wire::write_value(w, out, &S_SOC_NON_LATIN_OUT)?;
        Ok(())
    })
}

static S_SOC_VI_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_VI_OUT: wire::Schema = wire::Schema::Dyn;

/// `socVi` over the binary wire.
#[napi(js_name = "socViWire")]
pub fn soc_vi_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_SOC_VI_0)?;
        let out = &port::soc_vi(t)?;
        wire::write_value(w, out, &S_SOC_VI_OUT)?;
        Ok(())
    })
}

static S_SOC_HASH_JUNK_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_HASH_JUNK_OUT: wire::Schema = wire::Schema::Dyn;

/// `socHashJunk` over the binary wire.
#[napi(js_name = "socHashJunkWire")]
pub fn soc_hash_junk_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_SOC_HASH_JUNK_0)?;
        let out = &port::soc_hash_junk(t)?;
        wire::write_value(w, out, &S_SOC_HASH_JUNK_OUT)?;
        Ok(())
    })
}

static S_ASTROTURF_SIG_0: wire::Schema = wire::Schema::Dyn;
static S_ASTROTURF_SIG_OUT: wire::Schema = wire::Schema::Dyn;

/// `astroturfSig` over the binary wire.
#[napi(js_name = "astroturfSigWire")]
pub fn astroturf_sig_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_ASTROTURF_SIG_0)?;
        let out = &port::astroturf_sig(t)?;
        wire::write_value(w, out, &S_ASTROTURF_SIG_OUT)?;
        Ok(())
    })
}

static S_SOC_FOREIGN_LANG_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_FOREIGN_LANG_OUT: wire::Schema = wire::Schema::Dyn;

/// `socForeignLang` over the binary wire.
#[napi(js_name = "socForeignLangWire")]
pub fn soc_foreign_lang_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_SOC_FOREIGN_LANG_0)?;
        let out = &port::soc_foreign_lang(t)?;
        wire::write_value(w, out, &S_SOC_FOREIGN_LANG_OUT)?;
        Ok(())
    })
}

static S_SOC_KEYWORD_MINE_0: wire::Schema = wire::Schema::Dyn;
static S_SOC_KEYWORD_MINE_1: wire::Schema = wire::Schema::Dyn;
static S_SOC_KEYWORD_MINE_2: wire::Schema = wire::Schema::Dyn;
static S_SOC_KEYWORD_MINE_OUT: wire::Schema = wire::Schema::Dyn;

/// `socKeywordMine` over the binary wire.
#[napi(js_name = "socKeywordMineWire")]
pub fn soc_keyword_mine_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let posts: serde_json::Value = wire::read_value(r, &S_SOC_KEYWORD_MINE_0)?;
        let topic_id: serde_json::Value = wire::read_value(r, &S_SOC_KEYWORD_MINE_1)?;
        let arg2: serde_json::Value = wire::read_value(r, &S_SOC_KEYWORD_MINE_2)?;
        let out = &port::soc_keyword_mine(posts, topic_id, arg2)?;
        wire::write_value(w, out, &S_SOC_KEYWORD_MINE_OUT)?;
        Ok(())
    })
}

static S_EXPLAIN_POST_0: wire::Schema = wire::Schema::Dyn;
static S_EXPLAIN_POST_1: wire::Schema = wire::Schema::Dyn;
static S_EXPLAIN_POST_2: wire::Schema = wire::Schema::Dyn;
static S_EXPLAIN_POST_OUT: wire::Schema = wire::Schema::Dyn;

/// `explainPost` over the binary wire.
#[napi(js_name = "explainPostWire")]
pub fn explain_post_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: serde_json::Value = wire::read_value(r, &S_EXPLAIN_POST_0)?;
        let extra: serde_json::Value = wire::read_value(r, &S_EXPLAIN_POST_1)?;
        let context: serde_json::Value = wire::read_value(r, &S_EXPLAIN_POST_2)?;
        let out = &port::explain_post(text, extra, context)?;
        wire::write_value(w, out, &S_EXPLAIN_POST_OUT)?;
        Ok(())
    })
}
