//! Test-only support shared across this crate's test modules.
//!
//! argot-lang is a leaf crate (no dependency on argot-core), so its tests
//! can't reach argot-core's `config::default_generated_markers()`. This is a
//! small, representative stand-in — not exhaustive, just enough to exercise
//! the caller-supplied generated-marker scan the way each test needs: a
//! "do not edit" phrasing, an `@generated` tag, and an `auto-generated` tag.

#[cfg(test)]
pub(crate) fn generated_markers() -> Vec<String> {
    ["do not edit", "@generated", "auto-generated"]
        .iter()
        .map(|s| s.to_string())
        .collect()
}
