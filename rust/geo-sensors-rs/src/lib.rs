//! JS bindings for `src/geo-sensors.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `createGeoSensors`.
#[napi(js_name = "createGeoSensors")]
pub fn create_geo_sensors(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::create_geo_sensors(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `parseMagma`.
#[napi(js_name = "parseMagma")]
pub fn parse_magma(html: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::parse_magma(html)?;
        crate::js::to_json(&out)
    })
}

/// `csvLine`.
#[napi(js_name = "csvLine")]
pub fn csv_line(line: String) -> napi::Result<String> {
    run(move || {
        let line = crate::js::from_json(&line)?;
        let out = port::csv_line(line)?;
        crate::js::to_json(&out)
    })
}

/// `parseCsv`.
#[napi(js_name = "parseCsv")]
pub fn parse_csv(text: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::parse_csv(text)?;
        crate::js::to_json(&out)
    })
}

/// `parseCapField`.
#[napi(js_name = "parseCapField")]
pub fn parse_cap_field(xml: String, tag: String) -> napi::Result<String> {
    run(move || {
        let xml = crate::js::from_json(&xml)?;
        let tag = crate::js::from_json(&tag)?;
        let out = port::parse_cap_field(xml, tag)?;
        crate::js::to_json(&out)
    })
}

/// `parsePolygon`.
#[napi(js_name = "parsePolygon")]
pub fn parse_polygon(text: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let out = port::parse_polygon(text)?;
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

static S_CREATE_GEO_SENSORS_0: wire::Schema = wire::Schema::Dyn;
static S_CREATE_GEO_SENSORS_OUT: wire::Schema = wire::Schema::Dyn;

/// `createGeoSensors` over the binary wire.
#[napi(js_name = "createGeoSensorsWire")]
pub fn create_geo_sensors_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_CREATE_GEO_SENSORS_0)?;
        let out = &port::create_geo_sensors(arg0)?;
        wire::write_value(w, out, &S_CREATE_GEO_SENSORS_OUT)?;
        Ok(())
    })
}

static S_PARSE_MAGMA_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_MAGMA_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseMagma` over the binary wire.
#[napi(js_name = "parseMagmaWire")]
pub fn parse_magma_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_PARSE_MAGMA_0)?;
        let out = &port::parse_magma(html)?;
        wire::write_value(w, out, &S_PARSE_MAGMA_OUT)?;
        Ok(())
    })
}

static S_CSV_LINE_0: wire::Schema = wire::Schema::Dyn;
static S_CSV_LINE_OUT: wire::Schema = wire::Schema::Dyn;

/// `csvLine` over the binary wire.
#[napi(js_name = "csvLineWire")]
pub fn csv_line_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let line: serde_json::Value = wire::read_value(r, &S_CSV_LINE_0)?;
        let out = &port::csv_line(line)?;
        wire::write_value(w, out, &S_CSV_LINE_OUT)?;
        Ok(())
    })
}

static S_PARSE_CSV_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_CSV_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseCsv` over the binary wire.
#[napi(js_name = "parseCsvWire")]
pub fn parse_csv_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: serde_json::Value = wire::read_value(r, &S_PARSE_CSV_0)?;
        let out = &port::parse_csv(text)?;
        wire::write_value(w, out, &S_PARSE_CSV_OUT)?;
        Ok(())
    })
}

static S_PARSE_CAP_FIELD_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_CAP_FIELD_1: wire::Schema = wire::Schema::Dyn;
static S_PARSE_CAP_FIELD_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseCapField` over the binary wire.
#[napi(js_name = "parseCapFieldWire")]
pub fn parse_cap_field_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let xml: serde_json::Value = wire::read_value(r, &S_PARSE_CAP_FIELD_0)?;
        let tag: serde_json::Value = wire::read_value(r, &S_PARSE_CAP_FIELD_1)?;
        let out = &port::parse_cap_field(xml, tag)?;
        wire::write_value(w, out, &S_PARSE_CAP_FIELD_OUT)?;
        Ok(())
    })
}

static S_PARSE_POLYGON_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_POLYGON_OUT: wire::Schema = wire::Schema::Dyn;

/// `parsePolygon` over the binary wire.
#[napi(js_name = "parsePolygonWire")]
pub fn parse_polygon_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: serde_json::Value = wire::read_value(r, &S_PARSE_POLYGON_0)?;
        let out = &port::parse_polygon(text)?;
        wire::write_value(w, out, &S_PARSE_POLYGON_OUT)?;
        Ok(())
    })
}
