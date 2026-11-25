//! JS bindings for `src/article-media.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `canonicalArticleUrl`.
#[napi(js_name = "canonicalArticleUrl")]
pub fn canonical_article_url(raw: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let out = port::canonical_article_url(raw)?;
        crate::js::to_json(&out)
    })
}

/// `regDomain`.
#[napi(js_name = "regDomain")]
pub fn reg_domain(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::reg_domain(u)?;
        crate::js::to_json(&out)
    })
}

/// `resolveUrl`.
#[napi(js_name = "resolveUrl")]
pub fn resolve_url(raw: String, base: String) -> napi::Result<String> {
    run(move || {
        let raw = crate::js::from_json(&raw)?;
        let base = crate::js::from_json(&base)?;
        let out = port::resolve_url(raw, base)?;
        crate::js::to_json(&out)
    })
}

/// `upgradeImage`.
#[napi(js_name = "upgradeImage")]
pub fn upgrade_image(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::upgrade_image(u)?;
        crate::js::to_json(&out)
    })
}

/// `imageKey`.
#[napi(js_name = "imageKey")]
pub fn image_key(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::image_key(u)?;
        crate::js::to_json(&out)
    })
}

/// `dirKey`.
#[napi(js_name = "dirKey")]
pub fn dir_key(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::dir_key(u)?;
        crate::js::to_json(&out)
    })
}

/// `rdSmallHint`.
#[napi(js_name = "rdSmallHint")]
pub fn rd_small_hint(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::rd_small_hint(u)?;
        crate::js::to_json(&out)
    })
}

/// `rdJunkUrl`.
#[napi(js_name = "rdJunkUrl")]
pub fn rd_junk_url(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::rd_junk_url(u)?;
        crate::js::to_json(&out)
    })
}

/// `classifyCandidateUrl`.
#[napi(js_name = "classifyCandidateUrl")]
pub fn classify_candidate_url(url_val: String, ctx: String) -> napi::Result<String> {
    run(move || {
        let url_val = crate::js::from_json(&url_val)?;
        let ctx = crate::js::from_json(&ctx)?;
        let out = port::classify_candidate_url(url_val, ctx)?;
        crate::js::to_json(&out)
    })
}

/// `classifyCandidateAttrs`.
#[napi(js_name = "classifyCandidateAttrs")]
pub fn classify_candidate_attrs(attrs_val: String) -> napi::Result<String> {
    run(move || {
        let attrs_val = crate::js::from_json(&attrs_val)?;
        let out = port::classify_candidate_attrs(attrs_val)?;
        crate::js::to_json(&out)
    })
}

/// `imageSize`.
#[napi(js_name = "imageSize")]
pub fn image_size(buf: String) -> napi::Result<String> {
    run(move || {
        let buf = crate::js::from_json(&buf)?;
        let out = port::image_size(buf)?;
        crate::js::to_json(&out)
    })
}

/// `inspectImageBuffer`.
#[napi(js_name = "inspectImageBuffer")]
pub fn inspect_image_buffer(buf: String, content_type: Option<String>) -> napi::Result<String> {
    run(move || {
        let buf = crate::js::from_json(&buf)?;
        let out = port::inspect_image_buffer(buf, content_type)?;
        crate::js::to_json(&out)
    })
}

/// `validateImageMetrics`.
#[napi(js_name = "validateImageMetrics")]
pub fn validate_image_metrics(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::validate_image_metrics(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `extractHtmlImageCandidates`.
#[napi(js_name = "extractHtmlImageCandidates")]
pub fn extract_html_image_candidates(html: String, base_url: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let base_url = crate::js::from_json(&base_url)?;
        let out = port::extract_html_image_candidates(html, base_url)?;
        crate::js::to_json(&out)
    })
}

/// `extractJinaImageCandidates`.
#[napi(js_name = "extractJinaImageCandidates")]
pub fn extract_jina_image_candidates(
    markdown: String,
    article_url: String,
    _title: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let markdown = crate::js::from_json(&markdown)?;
        let article_url = crate::js::from_json(&article_url)?;
        let out = port::extract_jina_image_candidates(markdown, article_url, _title)?;
        crate::js::to_json(&out)
    })
}

/// `extractFeedImageCandidates`.
#[napi(js_name = "extractFeedImageCandidates")]
pub fn extract_feed_image_candidates(xml_block: String, base_url: String) -> napi::Result<String> {
    run(move || {
        let xml_block = crate::js::from_json(&xml_block)?;
        let base_url = crate::js::from_json(&base_url)?;
        let out = port::extract_feed_image_candidates(xml_block, base_url)?;
        crate::js::to_json(&out)
    })
}

/// `feedImage`.
#[napi(js_name = "feedImage")]
pub fn feed_image(xml_block: String, base_url: Option<String>) -> napi::Result<String> {
    run(move || {
        let xml_block = crate::js::from_json(&xml_block)?;
        let out = port::feed_image(xml_block, base_url)?;
        crate::js::to_json(&out)
    })
}

/// `isHijackedHtml`.
#[napi(js_name = "isHijackedHtml")]
pub fn is_hijacked_html(
    article_url: String,
    final_url: String,
    html: String,
    from_proxy: String,
    wire_title: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let article_url = crate::js::from_json(&article_url)?;
        let final_url = crate::js::from_json(&final_url)?;
        let html = crate::js::from_json(&html)?;
        let from_proxy = crate::js::from_json(&from_proxy)?;
        let out = port::is_hijacked_html(article_url, final_url, html, from_proxy, wire_title)?;
        crate::js::to_json(&out)
    })
}

/// `resolveArticleMedia`.
#[napi(js_name = "resolveArticleMedia")]
pub async fn resolve_article_media(arg0: String) -> napi::Result<String> {
    run_async(async move {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::resolve_article_media(arg0).await?;
        crate::js::to_json(&out)
    })
    .await
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

static S_CANONICAL_ARTICLE_URL_0: wire::Schema = wire::Schema::Dyn;
static S_CANONICAL_ARTICLE_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `canonicalArticleUrl` over the binary wire.
#[napi(js_name = "canonicalArticleUrlWire")]
pub fn canonical_article_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_CANONICAL_ARTICLE_URL_0)?;
        let out = &port::canonical_article_url(raw)?;
        wire::write_value(w, out, &S_CANONICAL_ARTICLE_URL_OUT)?;
        Ok(())
    })
}

static S_REG_DOMAIN_0: wire::Schema = wire::Schema::Dyn;
static S_REG_DOMAIN_OUT: wire::Schema = wire::Schema::Dyn;

/// `regDomain` over the binary wire.
#[napi(js_name = "regDomainWire")]
pub fn reg_domain_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_REG_DOMAIN_0)?;
        let out = &port::reg_domain(u)?;
        wire::write_value(w, out, &S_REG_DOMAIN_OUT)?;
        Ok(())
    })
}

static S_RESOLVE_URL_0: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_URL_1: wire::Schema = wire::Schema::Dyn;
static S_RESOLVE_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `resolveUrl` over the binary wire.
#[napi(js_name = "resolveUrlWire")]
pub fn resolve_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw: Value = wire::read_value(r, &S_RESOLVE_URL_0)?;
        let base: Value = wire::read_value(r, &S_RESOLVE_URL_1)?;
        let out = &port::resolve_url(raw, base)?;
        wire::write_value(w, out, &S_RESOLVE_URL_OUT)?;
        Ok(())
    })
}

static S_UPGRADE_IMAGE_0: wire::Schema = wire::Schema::Dyn;
static S_UPGRADE_IMAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `upgradeImage` over the binary wire.
#[napi(js_name = "upgradeImageWire")]
pub fn upgrade_image_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_UPGRADE_IMAGE_0)?;
        let out = &port::upgrade_image(u)?;
        wire::write_value(w, out, &S_UPGRADE_IMAGE_OUT)?;
        Ok(())
    })
}

static S_IMAGE_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_IMAGE_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `imageKey` over the binary wire.
#[napi(js_name = "imageKeyWire")]
pub fn image_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_IMAGE_KEY_0)?;
        let out = &port::image_key(u)?;
        wire::write_value(w, out, &S_IMAGE_KEY_OUT)?;
        Ok(())
    })
}

static S_DIR_KEY_0: wire::Schema = wire::Schema::Dyn;
static S_DIR_KEY_OUT: wire::Schema = wire::Schema::Dyn;

/// `dirKey` over the binary wire.
#[napi(js_name = "dirKeyWire")]
pub fn dir_key_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_DIR_KEY_0)?;
        let out = &port::dir_key(u)?;
        wire::write_value(w, out, &S_DIR_KEY_OUT)?;
        Ok(())
    })
}

static S_RD_SMALL_HINT_0: wire::Schema = wire::Schema::Dyn;
static S_RD_SMALL_HINT_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdSmallHint` over the binary wire.
#[napi(js_name = "rdSmallHintWire")]
pub fn rd_small_hint_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_RD_SMALL_HINT_0)?;
        let out = &port::rd_small_hint(u)?;
        wire::write_value(w, out, &S_RD_SMALL_HINT_OUT)?;
        Ok(())
    })
}

static S_RD_JUNK_URL_0: wire::Schema = wire::Schema::Dyn;
static S_RD_JUNK_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdJunkUrl` over the binary wire.
#[napi(js_name = "rdJunkUrlWire")]
pub fn rd_junk_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: Value = wire::read_value(r, &S_RD_JUNK_URL_0)?;
        let out = &port::rd_junk_url(u)?;
        wire::write_value(w, out, &S_RD_JUNK_URL_OUT)?;
        Ok(())
    })
}

static S_CLASSIFY_CANDIDATE_URL_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_CANDIDATE_URL_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyCandidateUrl` over the binary wire.
#[napi(js_name = "classifyCandidateUrlWire")]
pub fn classify_candidate_url_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let url_val: Value = wire::read_value(r, &S_CLASSIFY_CANDIDATE_URL_0)?;
        let ctx: Option<Value> = wire::read_dyn_opt(r)?;
        let out = &port::classify_candidate_url(url_val, ctx)?;
        wire::write_value(w, out, &S_CLASSIFY_CANDIDATE_URL_OUT)?;
        Ok(())
    })
}

static S_CLASSIFY_CANDIDATE_ATTRS_0: wire::Schema = wire::Schema::Dyn;
static S_CLASSIFY_CANDIDATE_ATTRS_OUT: wire::Schema = wire::Schema::Dyn;

/// `classifyCandidateAttrs` over the binary wire.
#[napi(js_name = "classifyCandidateAttrsWire")]
pub fn classify_candidate_attrs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let attrs_val: Value = wire::read_value(r, &S_CLASSIFY_CANDIDATE_ATTRS_0)?;
        let out = &port::classify_candidate_attrs(attrs_val)?;
        wire::write_value(w, out, &S_CLASSIFY_CANDIDATE_ATTRS_OUT)?;
        Ok(())
    })
}

static S_IMAGE_SIZE_0: wire::Schema = wire::Schema::Dyn;
static S_IMAGE_SIZE_OUT: wire::Schema = wire::Schema::Dyn;

/// `imageSize` over the binary wire.
#[napi(js_name = "imageSizeWire")]
pub fn image_size_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let buf: Value = wire::read_value(r, &S_IMAGE_SIZE_0)?;
        let out = &port::image_size(buf)?;
        wire::write_value(w, out, &S_IMAGE_SIZE_OUT)?;
        Ok(())
    })
}

static S_INSPECT_IMAGE_BUFFER_0: wire::Schema = wire::Schema::Dyn;
static S_INSPECT_IMAGE_BUFFER_OUT: wire::Schema = wire::Schema::Dyn;

/// `inspectImageBuffer` over the binary wire.
#[napi(js_name = "inspectImageBufferWire")]
pub fn inspect_image_buffer_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let buf: Value = wire::read_value(r, &S_INSPECT_IMAGE_BUFFER_0)?;
        let content_type: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::inspect_image_buffer(buf, content_type)?;
        wire::write_value(w, out, &S_INSPECT_IMAGE_BUFFER_OUT)?;
        Ok(())
    })
}

static S_VALIDATE_IMAGE_METRICS_0: wire::Schema = wire::Schema::Dyn;
static S_VALIDATE_IMAGE_METRICS_OUT: wire::Schema = wire::Schema::Dyn;

/// `validateImageMetrics` over the binary wire.
#[napi(js_name = "validateImageMetricsWire")]
pub fn validate_image_metrics_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: Value = wire::read_value(r, &S_VALIDATE_IMAGE_METRICS_0)?;
        let out = &port::validate_image_metrics(arg0)?;
        wire::write_value(w, out, &S_VALIDATE_IMAGE_METRICS_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_HTML_IMAGE_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_HTML_IMAGE_CANDIDATES_1: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_HTML_IMAGE_CANDIDATES_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractHtmlImageCandidates` over the binary wire.
#[napi(js_name = "extractHtmlImageCandidatesWire")]
pub fn extract_html_image_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: Value = wire::read_value(r, &S_EXTRACT_HTML_IMAGE_CANDIDATES_0)?;
        let base_url: Value = wire::read_value(r, &S_EXTRACT_HTML_IMAGE_CANDIDATES_1)?;
        let out = &port::extract_html_image_candidates(html, base_url)?;
        wire::write_value(w, out, &S_EXTRACT_HTML_IMAGE_CANDIDATES_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_JINA_IMAGE_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_JINA_IMAGE_CANDIDATES_1: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_JINA_IMAGE_CANDIDATES_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractJinaImageCandidates` over the binary wire.
#[napi(js_name = "extractJinaImageCandidatesWire")]
pub fn extract_jina_image_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let markdown: Value = wire::read_value(r, &S_EXTRACT_JINA_IMAGE_CANDIDATES_0)?;
        let article_url: Value = wire::read_value(r, &S_EXTRACT_JINA_IMAGE_CANDIDATES_1)?;
        let _title: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::extract_jina_image_candidates(markdown, article_url, _title)?;
        wire::write_value(w, out, &S_EXTRACT_JINA_IMAGE_CANDIDATES_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_FEED_IMAGE_CANDIDATES_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_FEED_IMAGE_CANDIDATES_1: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_FEED_IMAGE_CANDIDATES_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractFeedImageCandidates` over the binary wire.
#[napi(js_name = "extractFeedImageCandidatesWire")]
pub fn extract_feed_image_candidates_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let xml_block: Value = wire::read_value(r, &S_EXTRACT_FEED_IMAGE_CANDIDATES_0)?;
        let base_url: Value = wire::read_value(r, &S_EXTRACT_FEED_IMAGE_CANDIDATES_1)?;
        let out = &port::extract_feed_image_candidates(xml_block, base_url)?;
        wire::write_value(w, out, &S_EXTRACT_FEED_IMAGE_CANDIDATES_OUT)?;
        Ok(())
    })
}

static S_FEED_IMAGE_0: wire::Schema = wire::Schema::Dyn;
static S_FEED_IMAGE_OUT: wire::Schema = wire::Schema::Dyn;

/// `feedImage` over the binary wire.
#[napi(js_name = "feedImageWire")]
pub fn feed_image_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let xml_block: Value = wire::read_value(r, &S_FEED_IMAGE_0)?;
        let base_url: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::feed_image(xml_block, base_url)?;
        wire::write_value(w, out, &S_FEED_IMAGE_OUT)?;
        Ok(())
    })
}

static S_IS_HIJACKED_HTML_0: wire::Schema = wire::Schema::Dyn;
static S_IS_HIJACKED_HTML_1: wire::Schema = wire::Schema::Dyn;
static S_IS_HIJACKED_HTML_2: wire::Schema = wire::Schema::Dyn;
static S_IS_HIJACKED_HTML_3: wire::Schema = wire::Schema::Dyn;
static S_IS_HIJACKED_HTML_OUT: wire::Schema = wire::Schema::Dyn;

/// `isHijackedHtml` over the binary wire.
#[napi(js_name = "isHijackedHtmlWire")]
pub fn is_hijacked_html_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let article_url: Value = wire::read_value(r, &S_IS_HIJACKED_HTML_0)?;
        let final_url: Value = wire::read_value(r, &S_IS_HIJACKED_HTML_1)?;
        let html: Value = wire::read_value(r, &S_IS_HIJACKED_HTML_2)?;
        let from_proxy: Value = wire::read_value(r, &S_IS_HIJACKED_HTML_3)?;
        let wire_title: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_hijacked_html(article_url, final_url, html, from_proxy, wire_title)?;
        wire::write_value(w, out, &S_IS_HIJACKED_HTML_OUT)?;
        Ok(())
    })
}
