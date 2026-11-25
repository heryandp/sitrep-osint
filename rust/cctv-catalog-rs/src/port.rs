//! The Rust port of `src/cctv-catalog.js`.

#![forbid(unsafe_code)]

#[path = "service.rs"]
pub mod service;
pub use service::CctvCatalog;

use serde_json::json;

/// Port of `createCctvCatalog` (`src/cctv-catalog.js:6`).
pub fn create_cctv_catalog(arg0: serde_json::Value) -> Result<serde_json::Value, String> {
    if arg0.is_null() {
        return Err("Cannot destructure property 'fs' of 'object null' as it is null.".to_string());
    }
    if crate::js::is_undefined(&arg0) {
        return Err(
            "Cannot destructure property 'fs' of 'undefined' as it is undefined.".to_string(),
        );
    }

    Ok(json!({
        "moleFresh": { "$function": "moleFresh" },
        "cctvCatalog": { "$function": "cctvCatalog" },
        "warmCctv": { "$function": "warmCctv" },
        "hostCors": { "$map": [] },
        "saveCctvHist": { "$function": "saveCctvHist" }
    }))
}
