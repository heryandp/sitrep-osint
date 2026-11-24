//! JS bindings for `src/news-service.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `cleanPub`.
#[napi(js_name = "cleanPub")]
pub fn clean_pub(v: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::clean_pub(v)?;
        crate::js::to_json(&out)
    })
}

/// `isBannedHostUrl`.
#[napi(js_name = "isBannedHostUrl")]
pub fn is_banned_host_url(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::is_banned_host_url(u)?;
        crate::js::to_json(&out)
    })
}

/// `isBannedNewsItem`.
#[napi(js_name = "isBannedNewsItem")]
pub fn is_banned_news_item(it: String) -> napi::Result<String> {
    run(move || {
        let it = crate::js::from_json(&it)?;
        let out = port::is_banned_news_item(it)?;
        crate::js::to_json(&out)
    })
}

/// `breakNorm`.
#[napi(js_name = "breakNorm")]
pub fn break_norm(t: String) -> napi::Result<String> {
    run(move || {
        let t = crate::js::from_json(&t)?;
        let out = port::break_norm(t)?;
        crate::js::to_json(&out)
    })
}

/// `wibDayStartMs`.
#[napi(js_name = "wibDayStartMs")]
pub fn wib_day_start_ms(now: String) -> napi::Result<String> {
    run(move || {
        let now = crate::js::from_json(&now)?;
        let out = port::wib_day_start_ms(now)?;
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

static S_CLEAN_PUB_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_PUB_OUT: wire::Schema = wire::Schema::Dyn;

/// `cleanPub` over the binary wire.
#[napi(js_name = "cleanPubWire")]
pub fn clean_pub_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: Value = wire::read_value(r, &S_CLEAN_PUB_0)?;
        let out = &port::clean_pub(v)?;
        wire::write_value(w, out, &S_CLEAN_PUB_OUT)?;
        Ok(())
    })
}

static S_IS_BANNED_HOST_URL_0: wire::Schema = wire::Schema::Dyn;
static S_IS_BANNED_HOST_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `isBannedHostUrl` over the binary wire.
#[napi(js_name = "isBannedHostUrlWire")]
pub fn is_banned_host_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_IS_BANNED_HOST_URL_0)?;
        let out = &port::is_banned_host_url(u)?;
        wire::write_value(w, out, &S_IS_BANNED_HOST_URL_OUT)?;
        Ok(())
    })
}

static S_IS_BANNED_NEWS_ITEM_0: wire::Schema = wire::Schema::Dyn;
static S_IS_BANNED_NEWS_ITEM_OUT: wire::Schema = wire::Schema::Dyn;

/// `isBannedNewsItem` over the binary wire.
#[napi(js_name = "isBannedNewsItemWire")]
pub fn is_banned_news_item_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let it: Value = wire::read_value(r, &S_IS_BANNED_NEWS_ITEM_0)?;
        let out = &port::is_banned_news_item(it)?;
        wire::write_value(w, out, &S_IS_BANNED_NEWS_ITEM_OUT)?;
        Ok(())
    })
}

static S_BREAK_NORM_0: wire::Schema = wire::Schema::Dyn;
static S_BREAK_NORM_OUT: wire::Schema = wire::Schema::Dyn;

/// `breakNorm` over the binary wire.
#[napi(js_name = "breakNormWire")]
pub fn break_norm_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let t: Value = wire::read_value(r, &S_BREAK_NORM_0)?;
        let out = &port::break_norm(t)?;
        wire::write_value(w, out, &S_BREAK_NORM_OUT)?;
        Ok(())
    })
}

static S_WIB_DAY_START_MS_0: wire::Schema = wire::Schema::Dyn;
static S_WIB_DAY_START_MS_OUT: wire::Schema = wire::Schema::Dyn;

/// `wibDayStartMs` over the binary wire.
#[napi(js_name = "wibDayStartMsWire")]
pub fn wib_day_start_ms_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let now: Value = wire::read_value(r, &S_WIB_DAY_START_MS_0)?;
        let out = &port::wib_day_start_ms(now)?;
        wire::write_value(w, out, &S_WIB_DAY_START_MS_OUT)?;
        Ok(())
    })
}
