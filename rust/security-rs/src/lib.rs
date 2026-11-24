//! JS bindings for `src/security.js`: generated from the signatures in `port.rs` by rust-anything.
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

#[allow(dead_code)]
async fn run_async<T>(f: impl std::future::Future<Output = Result<T, String>>) -> napi::Result<T> {
    use futures::FutureExt as _;
    match std::panic::AssertUnwindSafe(f).catch_unwind().await {
        Ok(result) => result.map_err(napi::Error::from_reason),
        Err(payload) => Err(napi::Error::from_reason(format!(
            "Rust panic: {}",
            panic_message(&*payload)
        ))),
    }
}

/// `rdIPv4Bytes`.
#[napi(js_name = "rdIPv4Bytes")]
pub fn rd_i_pv4_bytes(ip: String) -> napi::Result<String> {
    run(move || {
        let ip = crate::js::from_json(&ip)?;
        let out = port::rd_i_pv4_bytes(ip)?;
        crate::js::to_json(&out)
    })
}

/// `rdPrivateBytes`.
#[napi(js_name = "rdPrivateBytes")]
pub fn rd_private_bytes(b: String) -> napi::Result<String> {
    run(move || {
        let b = crate::js::from_json(&b)?;
        let out = port::rd_private_bytes(b)?;
        crate::js::to_json(&out)
    })
}

/// `rdPrivateIP`.
#[napi(js_name = "rdPrivateIP")]
pub fn rd_private_ip(ip: String) -> napi::Result<String> {
    run(move || {
        let ip = crate::js::from_json(&ip)?;
        let out = port::rd_private_ip(ip)?;
        crate::js::to_json(&out)
    })
}

/// `publicURL`.
#[napi(js_name = "publicURL")]
pub fn public_url(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::public_url(raw)?;
        crate::js::to_json(&out)
    })
}

/// `verifyPassword`.
#[napi(js_name = "verifyPassword")]
pub async fn verify_password(password: String, stored: String) -> napi::Result<String> {
    run_async(async move {
        let password = crate::js::from_json(&password)?;
        let stored = crate::js::from_json(&stored)?;
        let out = port::verify_password(password, stored).await?;
        crate::js::to_json(&out)
    })
    .await
}

/// `parseUsersSpec`.
#[napi(js_name = "parseUsersSpec")]
pub fn parse_users_spec(spec: String) -> napi::Result<String> {
    run(move || {
        let spec = crate::js::from_json(&spec)?;
        let out = port::parse_users_spec(spec)?;
        crate::js::to_json(&out)
    })
}

/// `hasRole`.
#[napi(js_name = "hasRole")]
pub fn has_role(principal: String, min_role: String) -> napi::Result<String> {
    run(move || {
        let principal = crate::js::from_json(&principal)?;
        let min_role = crate::js::from_json(&min_role)?;
        let out = port::has_role(principal, min_role)?;
        crate::js::to_json(&out)
    })
}

/// `rateKey`.
#[napi(js_name = "rateKey")]
pub fn rate_key(ip: String) -> napi::Result<String> {
    run(move || {
        let ip = crate::js::from_json(&ip)?;
        let out = port::rate_key(ip)?;
        crate::js::to_json(&out)
    })
}

/// `getClientIp`.
#[napi(js_name = "getClientIp")]
pub fn get_client_ip(
    req: String,
    trust_proxy: Option<bool>,
    trusted_hops: String,
) -> napi::Result<String> {
    run(move || {
        let req = crate::js::from_json(&req)?;
        let trusted_hops = crate::js::from_json(&trusted_hops)?;
        let out = port::get_client_ip(req, trust_proxy, trusted_hops)?;
        crate::js::to_json(&out)
    })
}

/// `isHttpsReq`.
#[napi(js_name = "isHttpsReq")]
pub fn is_https_req(req: String, trust_proxy: Option<bool>) -> napi::Result<String> {
    run(move || {
        let req = crate::js::from_json(&req)?;
        let out = port::is_https_req(req, trust_proxy)?;
        crate::js::to_json(&out)
    })
}

/// `isSameOriginPost`.
#[napi(js_name = "isSameOriginPost")]
pub fn is_same_origin_post(req: String, trust_proxy: Option<bool>) -> napi::Result<String> {
    run(move || {
        let req = crate::js::from_json(&req)?;
        let out = port::is_same_origin_post(req, trust_proxy)?;
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

static S_RD_I_PV4_BYTES_0: wire::Schema = wire::Schema::Dyn;
static S_RD_I_PV4_BYTES_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdIPv4Bytes` over the binary wire.
#[napi(js_name = "rdIPv4BytesWire")]
pub fn rd_i_pv4_bytes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ip: serde_json::Value = wire::read_value(r, &S_RD_I_PV4_BYTES_0)?;
        let out = &port::rd_i_pv4_bytes(ip)?;
        wire::write_value(w, out, &S_RD_I_PV4_BYTES_OUT)?;
        Ok(())
    })
}

static S_RD_PRIVATE_BYTES_0: wire::Schema = wire::Schema::Dyn;
static S_RD_PRIVATE_BYTES_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdPrivateBytes` over the binary wire.
#[napi(js_name = "rdPrivateBytesWire")]
pub fn rd_private_bytes_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let b: serde_json::Value = wire::read_value(r, &S_RD_PRIVATE_BYTES_0)?;
        let out = &port::rd_private_bytes(b)?;
        wire::write_value(w, out, &S_RD_PRIVATE_BYTES_OUT)?;
        Ok(())
    })
}

static S_RD_PRIVATE_IP_0: wire::Schema = wire::Schema::Dyn;
static S_RD_PRIVATE_IP_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdPrivateIP` over the binary wire.
#[napi(js_name = "rdPrivateIPWire")]
pub fn rd_private_ip_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ip: serde_json::Value = wire::read_value(r, &S_RD_PRIVATE_IP_0)?;
        let out = &port::rd_private_ip(ip)?;
        wire::write_value(w, out, &S_RD_PRIVATE_IP_OUT)?;
        Ok(())
    })
}

static S_PUBLIC_URL_0: wire::Schema = wire::Schema::Dyn;
static S_PUBLIC_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `publicURL` over the binary wire.
#[napi(js_name = "publicURLWire")]
pub fn public_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: serde_json::Value = wire::read_value(r, &S_PUBLIC_URL_0)?;
        let out = &port::public_url(raw)?;
        wire::write_value(w, out, &S_PUBLIC_URL_OUT)?;
        Ok(())
    })
}

static S_PARSE_USERS_SPEC_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_USERS_SPEC_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseUsersSpec` over the binary wire.
#[napi(js_name = "parseUsersSpecWire")]
pub fn parse_users_spec_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let spec: serde_json::Value = wire::read_value(r, &S_PARSE_USERS_SPEC_0)?;
        let out = &port::parse_users_spec(spec)?;
        wire::write_value(w, out, &S_PARSE_USERS_SPEC_OUT)?;
        Ok(())
    })
}

static S_HAS_ROLE_0: wire::Schema = wire::Schema::Dyn;
static S_HAS_ROLE_1: wire::Schema = wire::Schema::Dyn;
static S_HAS_ROLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `hasRole` over the binary wire.
#[napi(js_name = "hasRoleWire")]
pub fn has_role_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let principal: serde_json::Value = wire::read_value(r, &S_HAS_ROLE_0)?;
        let min_role: serde_json::Value = wire::read_value(r, &S_HAS_ROLE_1)?;
        let out = &port::has_role(principal, min_role)?;
        wire::write_value(w, out, &S_HAS_ROLE_OUT)?;
        Ok(())
    })
}

static S_RATE_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_RATE_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `rateKey` over the binary wire.
#[napi(js_name = "rateKeyWire")]
pub fn rate_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let ip: serde_json::Value = wire::read_value(r, &S_RATE_KEY_0)?;
        let out = &port::rate_key(ip)?;
        wire::write_value(w, out, &S_RATE_KEY_OUT)?;
        Ok(())
    })
}

static S_GET_CLIENT_IP_0: wire::Schema = wire::Schema::Dyn;
static S_GET_CLIENT_IP_OUT: wire::Schema = wire::Schema::Dyn;

/// `getClientIp` over the binary wire.
#[napi(js_name = "getClientIpWire")]
pub fn get_client_ip_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let req: serde_json::Value = wire::read_value(r, &S_GET_CLIENT_IP_0)?;
        let trust_proxy: Option<bool> = r.opt_js(|r| r.bool())?;
        let trusted_hops: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::get_client_ip(req, trust_proxy, trusted_hops)?;
        wire::write_value(w, out, &S_GET_CLIENT_IP_OUT)?;
        Ok(())
    })
}

static S_IS_HTTPS_REQ_0: wire::Schema = wire::Schema::Dyn;
static S_IS_HTTPS_REQ_OUT: wire::Schema = wire::Schema::Dyn;

/// `isHttpsReq` over the binary wire.
#[napi(js_name = "isHttpsReqWire")]
pub fn is_https_req_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let req: serde_json::Value = wire::read_value(r, &S_IS_HTTPS_REQ_0)?;
        let trust_proxy: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::is_https_req(req, trust_proxy)?;
        wire::write_value(w, out, &S_IS_HTTPS_REQ_OUT)?;
        Ok(())
    })
}

static S_IS_SAME_ORIGIN_POST_0: wire::Schema = wire::Schema::Dyn;
static S_IS_SAME_ORIGIN_POST_OUT: wire::Schema = wire::Schema::Dyn;

/// `isSameOriginPost` over the binary wire.
#[napi(js_name = "isSameOriginPostWire")]
pub fn is_same_origin_post_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let req: serde_json::Value = wire::read_value(r, &S_IS_SAME_ORIGIN_POST_0)?;
        let trust_proxy: Option<bool> = r.opt_js(|r| r.bool())?;
        let out = &port::is_same_origin_post(req, trust_proxy)?;
        wire::write_value(w, out, &S_IS_SAME_ORIGIN_POST_OUT)?;
        Ok(())
    })
}
