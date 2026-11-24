//! JS bindings for `src/tomtom.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `haversine`.
#[napi(js_name = "haversine")]
pub fn haversine(a: String, b: String) -> napi::Result<String> {
    run(move || {
        let a = crate::js::from_json(&a)?;
        let b = crate::js::from_json(&b)?;
        let out = port::haversine(a, b)?;
        crate::js::to_json(&out)
    })
}

/// `roundGrid`.
#[napi(js_name = "roundGrid")]
pub fn round_grid(v: String, step: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let step = crate::js::from_json(&step)?;
        let out = port::round_grid(v, step)?;
        crate::js::to_json(&out)
    })
}

/// `roundGridStr`.
#[napi(js_name = "roundGridStr")]
pub fn round_grid_str(v: String, step: String, d: Option<f64>) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let step = crate::js::from_json(&step)?;
        let out = port::round_grid_str(v, step, d)?;
        crate::js::to_json(&out)
    })
}

/// `ttHostOK`.
#[napi(js_name = "ttHostOK")]
pub fn tt_host_ok(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::tt_host_ok(u)?;
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

static S_HAVERSINE_0: wire::Schema = wire::Schema::Dyn;
static S_HAVERSINE_1: wire::Schema = wire::Schema::Dyn;
static S_HAVERSINE_OUT: wire::Schema = wire::Schema::Dyn;

/// `haversine` over the binary wire.
#[napi(js_name = "haversineWire")]
pub fn haversine_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let a: serde_json::Value = wire::read_value(r, &S_HAVERSINE_0)?;
        let b: serde_json::Value = wire::read_value(r, &S_HAVERSINE_1)?;
        let out = &port::haversine(a, b)?;
        wire::write_value(w, out, &S_HAVERSINE_OUT)?;
        Ok(())
    })
}

static S_ROUND_GRID_0: wire::Schema = wire::Schema::Dyn;
static S_ROUND_GRID_1: wire::Schema = wire::Schema::Dyn;
static S_ROUND_GRID_OUT: wire::Schema = wire::Schema::Dyn;

/// `roundGrid` over the binary wire.
#[napi(js_name = "roundGridWire")]
pub fn round_grid_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: serde_json::Value = wire::read_value(r, &S_ROUND_GRID_0)?;
        let step: serde_json::Value = wire::read_value(r, &S_ROUND_GRID_1)?;
        let out = &port::round_grid(v, step)?;
        wire::write_value(w, out, &S_ROUND_GRID_OUT)?;
        Ok(())
    })
}

static S_ROUND_GRID_STR_0: wire::Schema = wire::Schema::Dyn;
static S_ROUND_GRID_STR_1: wire::Schema = wire::Schema::Dyn;
static S_ROUND_GRID_STR_OUT: wire::Schema = wire::Schema::Dyn;

/// `roundGridStr` over the binary wire.
#[napi(js_name = "roundGridStrWire")]
pub fn round_grid_str_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: serde_json::Value = wire::read_value(r, &S_ROUND_GRID_STR_0)?;
        let step: serde_json::Value = wire::read_value(r, &S_ROUND_GRID_STR_1)?;
        let d: Option<f64> = r.opt_js(|r| r.f64())?;
        let out = &port::round_grid_str(v, step, d)?;
        wire::write_value(w, out, &S_ROUND_GRID_STR_OUT)?;
        Ok(())
    })
}

static S_TT_HOST_OK_0: wire::Schema = wire::Schema::Dyn;
static S_TT_HOST_OK_OUT: wire::Schema = wire::Schema::Dyn;

/// `ttHostOK` over the binary wire.
#[napi(js_name = "ttHostOKWire")]
pub fn tt_host_ok_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: serde_json::Value = wire::read_value(r, &S_TT_HOST_OK_0)?;
        let out = &port::tt_host_ok(u)?;
        wire::write_value(w, out, &S_TT_HOST_OK_OUT)?;
        Ok(())
    })
}
