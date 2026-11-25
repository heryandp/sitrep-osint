//! JS bindings for `src/article-extract.js`: generated from the signatures in `port.rs` by rust-anything.
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

/// `isChallengeHtml`.
#[napi(js_name = "isChallengeHtml")]
pub fn is_challenge_html(
    html: String,
    title: Option<String>,
    desc: Option<String>,
) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::is_challenge_html(html, title, desc)?;
        crate::js::to_json(&out)
    })
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

/// `rdClean`.
#[napi(js_name = "rdClean")]
pub fn rd_clean(s: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let out = port::rd_clean(s)?;
        crate::js::to_json(&out)
    })
}

/// `rdInline`.
#[napi(js_name = "rdInline")]
pub fn rd_inline(s: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let out = port::rd_inline(s)?;
        crate::js::to_json(&out)
    })
}

/// `rdHost`.
#[napi(js_name = "rdHost")]
pub fn rd_host(u: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let out = port::rd_host(u)?;
        crate::js::to_json(&out)
    })
}

/// `rdAbs`.
#[napi(js_name = "rdAbs")]
pub fn rd_abs(u: String, base: String) -> napi::Result<String> {
    run(move || {
        let u = crate::js::from_json(&u)?;
        let base = crate::js::from_json(&base)?;
        let out = port::rd_abs(u, base)?;
        crate::js::to_json(&out)
    })
}

/// `parseAttrs`.
#[napi(js_name = "parseAttrs")]
pub fn parse_attrs(tag_str: String) -> napi::Result<String> {
    run(move || {
        let tag_str = crate::js::from_json(&tag_str)?;
        let out = port::parse_attrs(tag_str)?;
        crate::js::to_json(&out)
    })
}

/// `pickBestSrcset`.
#[napi(js_name = "pickBestSrcset")]
pub fn pick_best_srcset(srcset_raw: String, base: String) -> napi::Result<String> {
    run(move || {
        let srcset_raw = crate::js::from_json(&srcset_raw)?;
        let base = crate::js::from_json(&base)?;
        let out = port::pick_best_srcset(srcset_raw, base)?;
        crate::js::to_json(&out)
    })
}

/// `rdMeta`.
#[napi(js_name = "rdMeta")]
pub fn rd_meta(html: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::rd_meta(html)?;
        crate::js::to_json(&out)
    })
}

/// `rdLd`.
#[napi(js_name = "rdLd")]
pub fn rd_ld(html: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::rd_ld(html)?;
        crate::js::to_json(&out)
    })
}

/// `rdName`.
#[napi(js_name = "rdName")]
pub fn rd_name(v: String) -> napi::Result<String> {
    run(move || {
        let v = crate::js::from_json(&v)?;
        let out = port::rd_name(v)?;
        crate::js::to_json(&out)
    })
}

/// `findMatchingClose`.
#[napi(js_name = "findMatchingClose")]
pub fn find_matching_close(
    html: String,
    open_end_idx: String,
    tag_name: String,
) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let open_end_idx = crate::js::from_json(&open_end_idx)?;
        let tag_name = crate::js::from_json(&tag_name)?;
        let out = port::find_matching_close(html, open_end_idx, tag_name)?;
        crate::js::to_json(&out)
    })
}

/// `stripElementsByTag`.
#[napi(js_name = "stripElementsByTag")]
pub fn strip_elements_by_tag(html: String, tags: String, opts: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let tags = crate::js::from_json(&tags)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::strip_elements_by_tag(html, tags, opts)?;
        crate::js::to_json(&out)
    })
}

/// `stripJunkContainers`.
#[napi(js_name = "stripJunkContainers")]
pub fn strip_junk_containers(html: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::strip_junk_containers(html)?;
        crate::js::to_json(&out)
    })
}

/// `stripCardLinks`.
#[napi(js_name = "stripCardLinks")]
pub fn strip_card_links(s: String, base: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let base = crate::js::from_json(&base)?;
        let out = port::strip_card_links(s, base)?;
        crate::js::to_json(&out)
    })
}

/// `stripLinkBlocks`.
#[napi(js_name = "stripLinkBlocks")]
pub fn strip_link_blocks(s: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let out = port::strip_link_blocks(s)?;
        crate::js::to_json(&out)
    })
}

/// `extractContainers`.
#[napi(js_name = "extractContainers")]
pub fn extract_containers(html: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let out = port::extract_containers(html)?;
        crate::js::to_json(&out)
    })
}

/// `rdBlocks`.
#[napi(js_name = "rdBlocks")]
pub fn rd_blocks(html: String, final_url: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let final_url = crate::js::from_json(&final_url)?;
        let out = port::rd_blocks(html, final_url)?;
        crate::js::to_json(&out)
    })
}

/// `rdVideos`.
#[napi(js_name = "rdVideos")]
pub fn rd_videos(
    html: String,
    final_url: String,
    dom: Option<bool>,
    title: Option<String>,
    on_topic_fn: String,
) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let final_url = crate::js::from_json(&final_url)?;
        let on_topic_fn = crate::js::from_json(&on_topic_fn)?;
        let out = port::rd_videos(html, final_url, dom, title, on_topic_fn)?;
        crate::js::to_json(&out)
    })
}

/// `rdOnTopic`.
#[napi(js_name = "rdOnTopic")]
pub fn rd_on_topic(title: String, desc: String, paras: String) -> napi::Result<String> {
    run(move || {
        let title = crate::js::from_json(&title)?;
        let desc = crate::js::from_json(&desc)?;
        let paras = crate::js::from_json(&paras)?;
        let out = port::rd_on_topic(title, desc, paras)?;
        crate::js::to_json(&out)
    })
}

/// `topicTokens`.
#[napi(js_name = "topicTokens")]
pub fn topic_tokens(s: String) -> napi::Result<String> {
    run(move || {
        let s = crate::js::from_json(&s)?;
        let out = port::topic_tokens(s)?;
        crate::js::to_json(&out)
    })
}

/// `pruneBlocks`.
#[napi(js_name = "pruneBlocks")]
pub fn prune_blocks(
    blocks: String,
    title: String,
    wire_titles_set: String,
) -> napi::Result<String> {
    run(move || {
        let blocks = crate::js::from_json(&blocks)?;
        let title = crate::js::from_json(&title)?;
        let wire_titles_set = crate::js::from_json(&wire_titles_set)?;
        let out = port::prune_blocks(blocks, title, wire_titles_set)?;
        crate::js::to_json(&out)
    })
}

/// `computeTextQuality`.
#[napi(js_name = "computeTextQuality")]
pub fn compute_text_quality(arg0: String) -> napi::Result<String> {
    run(move || {
        let arg0 = crate::js::from_json(&arg0)?;
        let out = port::compute_text_quality(arg0)?;
        crate::js::to_json(&out)
    })
}

/// `cleanArticleTitle`.
#[napi(js_name = "cleanArticleTitle")]
pub fn clean_article_title(
    raw_title: String,
    site: String,
    final_url: String,
) -> napi::Result<String> {
    run(move || {
        let raw_title = crate::js::from_json(&raw_title)?;
        let site = crate::js::from_json(&site)?;
        let final_url = crate::js::from_json(&final_url)?;
        let out = port::clean_article_title(raw_title, site, final_url)?;
        crate::js::to_json(&out)
    })
}

/// `rdExtract`.
#[napi(js_name = "rdExtract")]
pub fn rd_extract(html: String, final_url: String, opts: String) -> napi::Result<String> {
    run(move || {
        let html = crate::js::from_json(&html)?;
        let final_url = crate::js::from_json(&final_url)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::rd_extract(html, final_url, opts)?;
        crate::js::to_json(&out)
    })
}

/// `rdJinaMd`.
#[napi(js_name = "rdJinaMd")]
pub fn rd_jina_md(text: String, url: String, opts: String) -> napi::Result<String> {
    run(move || {
        let text = crate::js::from_json(&text)?;
        let url = crate::js::from_json(&url)?;
        let opts = crate::js::from_json(&opts)?;
        let out = port::rd_jina_md(text, url, opts)?;
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

static S_IS_CHALLENGE_HTML_0: wire::Schema = wire::Schema::Dyn;
static S_IS_CHALLENGE_HTML_OUT: wire::Schema = wire::Schema::Dyn;

/// `isChallengeHtml` over the binary wire.
#[napi(js_name = "isChallengeHtmlWire")]
pub fn is_challenge_html_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_IS_CHALLENGE_HTML_0)?;
        let title: Option<String> = r.opt_js(|r| r.string16())?;
        let desc: Option<String> = r.opt_js(|r| r.string16())?;
        let out = &port::is_challenge_html(html, title, desc)?;
        wire::write_value(w, out, &S_IS_CHALLENGE_HTML_OUT)?;
        Ok(())
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

static S_RD_CLEAN_0: wire::Schema = wire::Schema::Dyn;
static S_RD_CLEAN_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdClean` over the binary wire.
#[napi(js_name = "rdCleanWire")]
pub fn rd_clean_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: serde_json::Value = wire::read_value(r, &S_RD_CLEAN_0)?;
        let out = &port::rd_clean(s)?;
        wire::write_value(w, out, &S_RD_CLEAN_OUT)?;
        Ok(())
    })
}

static S_RD_INLINE_0: wire::Schema = wire::Schema::Dyn;
static S_RD_INLINE_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdInline` over the binary wire.
#[napi(js_name = "rdInlineWire")]
pub fn rd_inline_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: serde_json::Value = wire::read_value(r, &S_RD_INLINE_0)?;
        let out = &port::rd_inline(s)?;
        wire::write_value(w, out, &S_RD_INLINE_OUT)?;
        Ok(())
    })
}

static S_RD_HOST_0: wire::Schema = wire::Schema::Dyn;
static S_RD_HOST_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdHost` over the binary wire.
#[napi(js_name = "rdHostWire")]
pub fn rd_host_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: serde_json::Value = wire::read_value(r, &S_RD_HOST_0)?;
        let out = &port::rd_host(u)?;
        wire::write_value(w, out, &S_RD_HOST_OUT)?;
        Ok(())
    })
}

static S_RD_ABS_0: wire::Schema = wire::Schema::Dyn;
static S_RD_ABS_1: wire::Schema = wire::Schema::Dyn;
static S_RD_ABS_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdAbs` over the binary wire.
#[napi(js_name = "rdAbsWire")]
pub fn rd_abs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let u: serde_json::Value = wire::read_value(r, &S_RD_ABS_0)?;
        let base: serde_json::Value = wire::read_value(r, &S_RD_ABS_1)?;
        let out = &port::rd_abs(u, base)?;
        wire::write_value(w, out, &S_RD_ABS_OUT)?;
        Ok(())
    })
}

static S_PARSE_ATTRS_0: wire::Schema = wire::Schema::Dyn;
static S_PARSE_ATTRS_OUT: wire::Schema = wire::Schema::Dyn;

/// `parseAttrs` over the binary wire.
#[napi(js_name = "parseAttrsWire")]
pub fn parse_attrs_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let tag_str: serde_json::Value = wire::read_value(r, &S_PARSE_ATTRS_0)?;
        let out = &port::parse_attrs(tag_str)?;
        wire::write_value(w, out, &S_PARSE_ATTRS_OUT)?;
        Ok(())
    })
}

static S_PICK_BEST_SRCSET_0: wire::Schema = wire::Schema::Dyn;
static S_PICK_BEST_SRCSET_1: wire::Schema = wire::Schema::Dyn;
static S_PICK_BEST_SRCSET_OUT: wire::Schema = wire::Schema::Dyn;

/// `pickBestSrcset` over the binary wire.
#[napi(js_name = "pickBestSrcsetWire")]
pub fn pick_best_srcset_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let srcset_raw: serde_json::Value = wire::read_value(r, &S_PICK_BEST_SRCSET_0)?;
        let base: serde_json::Value = wire::read_value(r, &S_PICK_BEST_SRCSET_1)?;
        let out = &port::pick_best_srcset(srcset_raw, base)?;
        wire::write_value(w, out, &S_PICK_BEST_SRCSET_OUT)?;
        Ok(())
    })
}

static S_RD_META_0: wire::Schema = wire::Schema::Dyn;
static S_RD_META_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdMeta` over the binary wire.
#[napi(js_name = "rdMetaWire")]
pub fn rd_meta_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_RD_META_0)?;
        let out = &port::rd_meta(html)?;
        wire::write_value(w, out, &S_RD_META_OUT)?;
        Ok(())
    })
}

static S_RD_LD_0: wire::Schema = wire::Schema::Dyn;
static S_RD_LD_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdLd` over the binary wire.
#[napi(js_name = "rdLdWire")]
pub fn rd_ld_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_RD_LD_0)?;
        let out = &port::rd_ld(html)?;
        wire::write_value(w, out, &S_RD_LD_OUT)?;
        Ok(())
    })
}

static S_RD_NAME_0: wire::Schema = wire::Schema::Dyn;
static S_RD_NAME_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdName` over the binary wire.
#[napi(js_name = "rdNameWire")]
pub fn rd_name_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let v: serde_json::Value = wire::read_value(r, &S_RD_NAME_0)?;
        let out = &port::rd_name(v)?;
        wire::write_value(w, out, &S_RD_NAME_OUT)?;
        Ok(())
    })
}

static S_FIND_MATCHING_CLOSE_0: wire::Schema = wire::Schema::Dyn;
static S_FIND_MATCHING_CLOSE_1: wire::Schema = wire::Schema::Dyn;
static S_FIND_MATCHING_CLOSE_2: wire::Schema = wire::Schema::Dyn;
static S_FIND_MATCHING_CLOSE_OUT: wire::Schema = wire::Schema::Dyn;

/// `findMatchingClose` over the binary wire.
#[napi(js_name = "findMatchingCloseWire")]
pub fn find_matching_close_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_FIND_MATCHING_CLOSE_0)?;
        let open_end_idx: serde_json::Value = wire::read_value(r, &S_FIND_MATCHING_CLOSE_1)?;
        let tag_name: serde_json::Value = wire::read_value(r, &S_FIND_MATCHING_CLOSE_2)?;
        let out = &port::find_matching_close(html, open_end_idx, tag_name)?;
        wire::write_value(w, out, &S_FIND_MATCHING_CLOSE_OUT)?;
        Ok(())
    })
}

static S_STRIP_ELEMENTS_BY_TAG_0: wire::Schema = wire::Schema::Dyn;
static S_STRIP_ELEMENTS_BY_TAG_1: wire::Schema = wire::Schema::Dyn;
static S_STRIP_ELEMENTS_BY_TAG_2: wire::Schema = wire::Schema::Dyn;
static S_STRIP_ELEMENTS_BY_TAG_OUT: wire::Schema = wire::Schema::Dyn;

/// `stripElementsByTag` over the binary wire.
#[napi(js_name = "stripElementsByTagWire")]
pub fn strip_elements_by_tag_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_STRIP_ELEMENTS_BY_TAG_0)?;
        let tags: serde_json::Value = wire::read_value(r, &S_STRIP_ELEMENTS_BY_TAG_1)?;
        let opts: serde_json::Value = wire::read_value(r, &S_STRIP_ELEMENTS_BY_TAG_2)?;
        let out = &port::strip_elements_by_tag(html, tags, opts)?;
        wire::write_value(w, out, &S_STRIP_ELEMENTS_BY_TAG_OUT)?;
        Ok(())
    })
}

static S_STRIP_JUNK_CONTAINERS_0: wire::Schema = wire::Schema::Dyn;
static S_STRIP_JUNK_CONTAINERS_OUT: wire::Schema = wire::Schema::Dyn;

/// `stripJunkContainers` over the binary wire.
#[napi(js_name = "stripJunkContainersWire")]
pub fn strip_junk_containers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_STRIP_JUNK_CONTAINERS_0)?;
        let out = &port::strip_junk_containers(html)?;
        wire::write_value(w, out, &S_STRIP_JUNK_CONTAINERS_OUT)?;
        Ok(())
    })
}

static S_STRIP_CARD_LINKS_0: wire::Schema = wire::Schema::Dyn;
static S_STRIP_CARD_LINKS_1: wire::Schema = wire::Schema::Dyn;
static S_STRIP_CARD_LINKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `stripCardLinks` over the binary wire.
#[napi(js_name = "stripCardLinksWire")]
pub fn strip_card_links_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: serde_json::Value = wire::read_value(r, &S_STRIP_CARD_LINKS_0)?;
        let base: serde_json::Value = wire::read_value(r, &S_STRIP_CARD_LINKS_1)?;
        let out = &port::strip_card_links(s, base)?;
        wire::write_value(w, out, &S_STRIP_CARD_LINKS_OUT)?;
        Ok(())
    })
}

static S_STRIP_LINK_BLOCKS_0: wire::Schema = wire::Schema::Dyn;
static S_STRIP_LINK_BLOCKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `stripLinkBlocks` over the binary wire.
#[napi(js_name = "stripLinkBlocksWire")]
pub fn strip_link_blocks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: serde_json::Value = wire::read_value(r, &S_STRIP_LINK_BLOCKS_0)?;
        let out = &port::strip_link_blocks(s)?;
        wire::write_value(w, out, &S_STRIP_LINK_BLOCKS_OUT)?;
        Ok(())
    })
}

static S_EXTRACT_CONTAINERS_0: wire::Schema = wire::Schema::Dyn;
static S_EXTRACT_CONTAINERS_OUT: wire::Schema = wire::Schema::Dyn;

/// `extractContainers` over the binary wire.
#[napi(js_name = "extractContainersWire")]
pub fn extract_containers_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_EXTRACT_CONTAINERS_0)?;
        let out = &port::extract_containers(html)?;
        wire::write_value(w, out, &S_EXTRACT_CONTAINERS_OUT)?;
        Ok(())
    })
}

static S_RD_BLOCKS_0: wire::Schema = wire::Schema::Dyn;
static S_RD_BLOCKS_1: wire::Schema = wire::Schema::Dyn;
static S_RD_BLOCKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdBlocks` over the binary wire.
#[napi(js_name = "rdBlocksWire")]
pub fn rd_blocks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_RD_BLOCKS_0)?;
        let final_url: serde_json::Value = wire::read_value(r, &S_RD_BLOCKS_1)?;
        let out = &port::rd_blocks(html, final_url)?;
        wire::write_value(w, out, &S_RD_BLOCKS_OUT)?;
        Ok(())
    })
}

static S_RD_VIDEOS_0: wire::Schema = wire::Schema::Dyn;
static S_RD_VIDEOS_1: wire::Schema = wire::Schema::Dyn;
static S_RD_VIDEOS_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdVideos` over the binary wire.
#[napi(js_name = "rdVideosWire")]
pub fn rd_videos_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_RD_VIDEOS_0)?;
        let final_url: serde_json::Value = wire::read_value(r, &S_RD_VIDEOS_1)?;
        let dom: Option<bool> = r.opt_js(|r| r.bool())?;
        let title: Option<String> = r.opt_js(|r| r.string16())?;
        let on_topic_fn: Option<serde_json::Value> = wire::read_dyn_opt(r)?;
        let out = &port::rd_videos(html, final_url, dom, title, on_topic_fn)?;
        wire::write_value(w, out, &S_RD_VIDEOS_OUT)?;
        Ok(())
    })
}

static S_RD_ON_TOPIC_0: wire::Schema = wire::Schema::Dyn;
static S_RD_ON_TOPIC_1: wire::Schema = wire::Schema::Dyn;
static S_RD_ON_TOPIC_2: wire::Schema = wire::Schema::Dyn;
static S_RD_ON_TOPIC_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdOnTopic` over the binary wire.
#[napi(js_name = "rdOnTopicWire")]
pub fn rd_on_topic_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let title: serde_json::Value = wire::read_value(r, &S_RD_ON_TOPIC_0)?;
        let desc: serde_json::Value = wire::read_value(r, &S_RD_ON_TOPIC_1)?;
        let paras: serde_json::Value = wire::read_value(r, &S_RD_ON_TOPIC_2)?;
        let out = &port::rd_on_topic(title, desc, paras)?;
        wire::write_value(w, out, &S_RD_ON_TOPIC_OUT)?;
        Ok(())
    })
}

static S_TOPIC_TOKENS_0: wire::Schema = wire::Schema::Dyn;
static S_TOPIC_TOKENS_OUT: wire::Schema = wire::Schema::Dyn;

/// `topicTokens` over the binary wire.
#[napi(js_name = "topicTokensWire")]
pub fn topic_tokens_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let s: serde_json::Value = wire::read_value(r, &S_TOPIC_TOKENS_0)?;
        let out = &port::topic_tokens(s)?;
        wire::write_value(w, out, &S_TOPIC_TOKENS_OUT)?;
        Ok(())
    })
}

static S_PRUNE_BLOCKS_0: wire::Schema = wire::Schema::Dyn;
static S_PRUNE_BLOCKS_1: wire::Schema = wire::Schema::Dyn;
static S_PRUNE_BLOCKS_2: wire::Schema = wire::Schema::Dyn;
static S_PRUNE_BLOCKS_OUT: wire::Schema = wire::Schema::Dyn;

/// `pruneBlocks` over the binary wire.
#[napi(js_name = "pruneBlocksWire")]
pub fn prune_blocks_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let blocks: serde_json::Value = wire::read_value(r, &S_PRUNE_BLOCKS_0)?;
        let title: serde_json::Value = wire::read_value(r, &S_PRUNE_BLOCKS_1)?;
        let wire_titles_set: serde_json::Value = wire::read_value(r, &S_PRUNE_BLOCKS_2)?;
        let out = &port::prune_blocks(blocks, title, wire_titles_set)?;
        wire::write_value(w, out, &S_PRUNE_BLOCKS_OUT)?;
        Ok(())
    })
}

static S_COMPUTE_TEXT_QUALITY_0: wire::Schema = wire::Schema::Dyn;
static S_COMPUTE_TEXT_QUALITY_OUT: wire::Schema = wire::Schema::Dyn;

/// `computeTextQuality` over the binary wire.
#[napi(js_name = "computeTextQualityWire")]
pub fn compute_text_quality_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let arg0: serde_json::Value = wire::read_value(r, &S_COMPUTE_TEXT_QUALITY_0)?;
        let out = &port::compute_text_quality(arg0)?;
        wire::write_value(w, out, &S_COMPUTE_TEXT_QUALITY_OUT)?;
        Ok(())
    })
}

static S_CLEAN_ARTICLE_TITLE_0: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_ARTICLE_TITLE_1: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_ARTICLE_TITLE_2: wire::Schema = wire::Schema::Dyn;
static S_CLEAN_ARTICLE_TITLE_OUT: wire::Schema = wire::Schema::Dyn;

/// `cleanArticleTitle` over the binary wire.
#[napi(js_name = "cleanArticleTitleWire")]
pub fn clean_article_title_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let raw_title: serde_json::Value = wire::read_value(r, &S_CLEAN_ARTICLE_TITLE_0)?;
        let site: serde_json::Value = wire::read_value(r, &S_CLEAN_ARTICLE_TITLE_1)?;
        let final_url: serde_json::Value = wire::read_value(r, &S_CLEAN_ARTICLE_TITLE_2)?;
        let out = &port::clean_article_title(raw_title, site, final_url)?;
        wire::write_value(w, out, &S_CLEAN_ARTICLE_TITLE_OUT)?;
        Ok(())
    })
}

static S_RD_EXTRACT_0: wire::Schema = wire::Schema::Dyn;
static S_RD_EXTRACT_1: wire::Schema = wire::Schema::Dyn;
static S_RD_EXTRACT_2: wire::Schema = wire::Schema::Dyn;
static S_RD_EXTRACT_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdExtract` over the binary wire.
#[napi(js_name = "rdExtractWire")]
pub fn rd_extract_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let html: serde_json::Value = wire::read_value(r, &S_RD_EXTRACT_0)?;
        let final_url: serde_json::Value = wire::read_value(r, &S_RD_EXTRACT_1)?;
        let opts: serde_json::Value = wire::read_value(r, &S_RD_EXTRACT_2)?;
        let out = &port::rd_extract(html, final_url, opts)?;
        wire::write_value(w, out, &S_RD_EXTRACT_OUT)?;
        Ok(())
    })
}

static S_RD_JINA_MD_0: wire::Schema = wire::Schema::Dyn;
static S_RD_JINA_MD_1: wire::Schema = wire::Schema::Dyn;
static S_RD_JINA_MD_2: wire::Schema = wire::Schema::Dyn;
static S_RD_JINA_MD_OUT: wire::Schema = wire::Schema::Dyn;

/// `rdJinaMd` over the binary wire.
#[napi(js_name = "rdJinaMdWire")]
pub fn rd_jina_md_wire() -> napi::Result<f64> {
    wire_io(|r, w| {
        let text: serde_json::Value = wire::read_value(r, &S_RD_JINA_MD_0)?;
        let url: serde_json::Value = wire::read_value(r, &S_RD_JINA_MD_1)?;
        let opts: serde_json::Value = wire::read_value(r, &S_RD_JINA_MD_2)?;
        let out = &port::rd_jina_md(text, url, opts)?;
        wire::write_value(w, out, &S_RD_JINA_MD_OUT)?;
        Ok(())
    })
}
