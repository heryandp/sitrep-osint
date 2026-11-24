//! JS bindings for `src/feedtext.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `decodeEntities`.
#[napi(js_name = "decodeEntities")]
pub fn decode_entities(s: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let out = port::decode_entities(s)?;
        crate::js::to_json(&out)
    })
}

/// `htmlToText`.
#[napi(js_name = "htmlToText")]
pub fn html_to_text(html: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::html_to_text(html)?;
        crate::js::to_json(&out)
    })
}

/// `descFrom`.
#[napi(js_name = "descFrom")]
pub fn desc_from(fields: String, title: String) -> napi::Result<String> {
    run(move || {
        let fields = crate::js::from_json(&fields)?;
        let title = crate::js::from_json(&title)?;
        let out = port::desc_from(fields, title)?;
        crate::js::to_json(&out)
    })
}

/// `newsEnglish`.
#[napi(js_name = "newsEnglish")]
pub fn news_english(title: String, desc: String) -> napi::Result<String> {
    run(move || {
        let title = crate::js::from_json(&title)?;
        let desc = crate::js::from_json(&desc)?;
        let out = port::news_english(title, desc)?;
        crate::js::to_json(&out)
    })
}

/// `cleanTitle`.
#[napi(js_name = "cleanTitle")]
pub fn clean_title(t: String, r#pub: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let r#pub = crate::js::from_json(&r#pub)?;
        let out = port::clean_title(t, r#pub)?;
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

static S_DECODE_ENTITIES_0: wire::Schema = wire::Schema::Dyn;
static S_DECODE_ENTITIES_OUT: wire::Schema = wire::Schema::Dyn;

/// `decodeEntities` over the binary wire.
#[napi(js_name = "decodeEntitiesWire")]
pub fn decode_entities_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: serde_json::Value = wire::read_value(r, &S_DECODE_ENTITIES_0)?;
        let out = &port::decode_entities(s)?;
        wire::write_value(w, out, &S_DECODE_ENTITIES_OUT)?;
        Ok(())
    })
}

static S_HTML_TO_TEXT_0: wire::Schema = wire::Schema::Dyn;
static S_HTML_TO_TEXT_OUT: wire::Schema = wire::Schema::Dyn;

/// `htmlToText` over the binary wire.
#[napi(js_name = "htmlToTextWire")]
pub fn html_to_text_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_HTML_TO_TEXT_0)?;
        let out = &port::html_to_text(html)?;
        wire::write_value(w, out, &S_HTML_TO_TEXT_OUT)?;
        Ok(())
    })
}

static S_DESC_FROM_0: wire::Schema = wire::Schema::Dyn;
static S_DESC_FROM_1: wire::Schema = wire::Schema::Dyn;
static S_DESC_FROM_OUT: wire::Schema = wire::Schema::Dyn;

/// `descFrom` over the binary wire.
#[napi(js_name = "descFromWire")]
pub fn desc_from_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let fields: serde_json::Value = wire::read_value(r, &S_DESC_FROM_0)?;
        let title: serde_json::Value = wire::read_value(r, &S_DESC_FROM_1)?;
        let out = &port::desc_from(fields, title)?;
        wire::write_value(w, out, &S_DESC_FROM_OUT)?;
        Ok(())
    })
}

static S_NEWS_ENGLISH_0: wire::Schema = wire::Schema::Dyn;
static S_NEWS_ENGLISH_1: wire::Schema = wire::Schema::Dyn;
static S_NEWS_ENGLISH_OUT: wire::Schema = wire::Schema::Dyn;

/// `newsEnglish` over the binary wire.
#[napi(js_name = "newsEnglishWire")]
pub fn news_english_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let title: serde_json::Value = wire::read_value(r, &S_NEWS_ENGLISH_0)?;
        let desc: serde_json::Value = wire::read_value(r, &S_NEWS_ENGLISH_1)?;
        let out = &port::news_english(title, desc)?;
        wire::write_value(w, out, &S_NEWS_ENGLISH_OUT)?;
        Ok(())
    })
}

static S_CLEAN_TITLE_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_TITLE_1: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_TITLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `cleanTitle` over the binary wire.
#[napi(js_name = "cleanTitleWire")]
pub fn clean_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: serde_json::Value = wire::read_value(r, &S_CLEAN_TITLE_0)?;
        let r#pub: serde_json::Value = wire::read_value(r, &S_CLEAN_TITLE_1)?;
        let out = &port::clean_title(t, r#pub)?;
        wire::write_value(w, out, &S_CLEAN_TITLE_OUT)?;
        Ok(())
    })
}
