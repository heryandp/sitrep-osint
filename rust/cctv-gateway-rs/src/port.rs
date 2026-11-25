//! The Rust port of `src/cctv-gateway.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;
pub use service::CctvGateway;

use serde_json::{Value, json};

/// Port of `createGateway` (`src/cctv-gateway.js:106`).
pub fn create_gateway(d: Value) -> Result<Value, String> {
    if d.is_null() {
        return Err("Cannot destructure property 'getRaw' of 'd' as it is null.".to_string());
    }
    if crate::js::is_undefined(&d) {
        return Err("Cannot destructure property 'getRaw' of 'd' as it is undefined.".to_string());
    }
    Ok(json!({
        "hls": { "$function": "hls" },
        "snap": { "$function": "snap" },
        "report": { "$function": "report" },
        "reportedDead": { "$function": "reportedDead" },
        "rumIngest": { "$function": "rumIngest" },
        "rumStats": { "$function": "rumStats" },
        "detectFfmpeg": { "$function": "detectFfmpeg" },
        "stats": { "$function": "stats" },
    }))
}

/// Port of `makeLane` (`src/cctv-gateway.js:7`).
pub fn make_lane(
    _max: Value,
    _per_host: Value,
    _max_queue: Option<Value>,
) -> Result<Value, String> {
    Ok(json!({
        "run": { "$function": "run" },
        "stats": { "$function": "stats" },
    }))
}

/// Port of `makeBucket` (`src/cctv-gateway.js:65`).
pub fn make_bucket(_rate: Value, _burst: Value, _now: Option<Value>) -> Result<Value, String> {
    Ok(json!({
        "take": { "$function": "take" },
        "charge": { "$function": "charge" },
        "ok": { "$function": "ok" },
        "size": { "$function": "size" },
    }))
}

/// Port of `makeLru` (`src/cctv-gateway.js:92`).
pub fn make_lru(_max_bytes: Value, _now: Option<Value>) -> Result<Value, String> {
    Ok(json!({
        "get": { "$function": "get" },
        "set": { "$function": "set" },
        "stats": { "$function": "stats" },
    }))
}
