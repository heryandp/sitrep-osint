//! Pure-Rust helpers for `rust/app` callers, kept outside `port.rs` top-level `pub fn`s
//! so `rust-anything refresh` does not try to bridge generic functions over N-API.

#![forbid(unsafe_code)]

use std::collections::HashSet;

/// Jaccard similarity between two token slices.
pub fn jaccard_similarity<S: AsRef<str>>(tokens_a: &[S], tokens_b: &[S]) -> f64 {
    let set_a: HashSet<&str> = tokens_a.iter().map(AsRef::as_ref).collect();
    let set_b: HashSet<&str> = tokens_b.iter().map(AsRef::as_ref).collect();
    if set_a.is_empty() || set_b.is_empty() {
        return 0.0;
    }
    let inter = set_a.iter().filter(|t| set_b.contains(*t)).count();
    let union = set_a.len() + set_b.len() - inter;
    if union > 0 {
        inter as f64 / union as f64
    } else {
        0.0
    }
}
