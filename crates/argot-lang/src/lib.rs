//! argot-lang — the language substrate: per-language tree-sitter adapters,
//! tokenisation (tree-sitter leaf tokens + BPE), CPython-exact text utilities,
//! and the dataset JSONL wire format.
//!
//! A leaf crate: language- and corpus-agnostic, no cargo features, and no
//! dependency on argot-core (the scoring engine depends on this crate, never
//! the other way around). argot-core re-exports these modules at their
//! original paths (`bpe`, `dataset`, `text`, `tokenize`, `scoring::adapters`,
//! `scoring::filters`, `scoring::ts_parse`, and `check`'s extension-routing
//! functions) so existing callers are unaffected by the split.

pub mod adapters;
pub mod bpe;
pub mod dataset;
pub mod ext;
pub mod filters;
pub mod text;
pub mod tokenize;
pub mod ts_parse;

#[cfg(test)]
mod test_support;
