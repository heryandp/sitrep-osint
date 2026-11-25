//! The Rust port of `src/reader-service.js`.

#![forbid(unsafe_code)]
#![allow(
    dead_code,
    unused_imports,
    clippy::duplicate_mod,
    clippy::collapsible_if,
    clippy::manual_map,
    clippy::too_many_arguments
)]

#[path = "../../article-extract-rs/src/port.rs"]
pub mod article_extract;

#[path = "../../article-media-rs/src/port.rs"]
pub mod article_media;

#[path = "../../feedtext-rs/src/port.rs"]
pub mod feedtext;

#[path = "service.rs"]
pub mod service;
pub use service::ReaderService;

use serde_json::{Value, json};

/// Port of `createReaderService` (`src/reader-service.js:3`).
pub fn create_reader_service(arg0: Value) -> Result<Value, String> {
    if arg0.is_null() {
        return Err(
            "Cannot destructure property 'CFG' of 'object null' as it is null.".to_string(),
        );
    }
    if crate::js::is_undefined(&arg0) {
        return Err(
            "Cannot destructure property 'CFG' of 'undefined' as it is undefined.".to_string(),
        );
    }

    // In JS: `const { publicURL, rdAssertPublic } = SEC;`
    // If arg0 is not an object, or doesn't have `SEC` property, or SEC is not an object:
    let sec = match &arg0 {
        Value::Object(map) => map.get("SEC"),
        _ => None,
    };

    match sec {
        Some(Value::Object(_)) => {
            // Valid initialization
            Ok(json!({
                "status": "initialized",
                "service": "ReaderService"
            }))
        }
        _ => {
            Err("Cannot destructure property 'publicURL' of 'SEC' as it is undefined.".to_string())
        }
    }
}
