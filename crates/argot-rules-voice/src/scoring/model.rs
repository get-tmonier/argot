//! The fit-time voice model — the corpus-derived scorer state persisted in
//! `scorer-config.json` (v3) at calibrate time and loaded back at check time.
//!
//! Why a snapshot: `check` must score new code against what the repo's voice
//! *was when the model was fitted*. Rebuilding scorer state from the files on
//! disk at check time lets brand-new code attest its own callees and token
//! frequencies — the unattested-callee branches then never fire on exactly
//! the code `check` exists to judge (issue #79). The import stage always had
//! this property via the `import_modules` snapshot; the model block extends
//! it to the BPE corpus statistics and the call-receiver attestation.
//!
//! Serialization is deterministic (BTreeMap ordering + sorted lists), so the
//! same corpus and config produce byte-identical artifacts, and the model
//! hash is a stable fingerprint of the learned state.

use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Repo-corpus BPE token statistics (`BpeScorer`'s fitted state). Keys are
/// tokenizer ids rendered as strings, matching the generic baseline's shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BpeStats {
    pub token_counts: BTreeMap<String, u64>,
    pub total_tokens: u64,
}

/// One callee-bag cluster: its member files (repo-root-relative, sorted) and
/// per-callee file-presence counts within the cluster.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ClusterModel {
    pub files: Vec<String>,
    pub callee_counts: BTreeMap<String, usize>,
}

/// The call-receiver's fitted state: the global attested-callee set plus the
/// cluster partition. Document frequencies (rarity-weighting substrate) are
/// derivable as the per-callee sum across clusters — each corpus file belongs
/// to exactly one cluster — so they are not stored separately.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CallReceiverModel {
    pub attested: Vec<String>,
    pub n_corpus_files: usize,
    pub clusters: BTreeMap<String, ClusterModel>,
    /// Symbols the repo *declares* anywhere in its corpus — functions, methods,
    /// classes, structs, namespaces, enums (via the adapter's
    /// `callable_definitions`). A bare call to one, or a `Type::method` /
    /// `ns::func` whose leading segment is one, is the repo's own code reached
    /// across translation units — not foreign — even when it was never *called*
    /// at fit (so absent from `attested`). Separates rocksdb's own
    /// `ResetState` / `AlignedBuffer::` from foreign `event_base_new` / `absl::`.
    /// Absent in artifacts fitted before this field existed (then no symbol is
    /// treated as repo-declared — the prior behaviour).
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub defined_symbols: Vec<String>,
}

/// The convention-rarity model: corpus frequencies of AST node kinds and of
/// abstract identifier shapes (character-class morphology only — camelCase /
/// snake_case / PascalCase / SCREAMING / flat / other), plus the calibrated
/// firing bars. A hunk whose rarest present convention clears a bar is
/// phrased in a construct or naming style the corpus does not use — a voice
/// signal that token surprise cannot see (the tokens themselves are common;
/// their arrangement is not).
///
/// Bars are the max convention surprisal over the calibration sample, so the
/// stage essentially never fires on in-voice code; per the calibration
/// contract the stage is suppressed on the calibration side (asymmetric by
/// construction — see docs/agents/calibration-contract.md).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ConventionModel {
    pub node_kinds: BTreeMap<String, u64>,
    pub total_nodes: u64,
    pub ident_shapes: BTreeMap<String, u64>,
    pub total_idents: u64,
    pub syntax_bar: f64,
    /// Per-morphology firing bar (shape class → max windowed surprisal over the
    /// calibration sample). A single scalar bar conflated shapes: an in-voice
    /// window concentrated in one shape (a `SCREAMING_SNAKE` constants block)
    /// would raise the bar against a genuinely-foreign shape (camelCase in a
    /// snake_case repo). Per-shape bars judge each morphology against the repo's
    /// own concentration of *that* shape. A shape absent here was never
    /// concentrated in-voice, so it fires whenever it dominates a hunk.
    pub ident_bars: BTreeMap<String, f64>,
}

/// The complete per-language model block.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct LanguageModel {
    pub bpe: BpeStats,
    pub call_receiver: CallReceiverModel,
    /// Absent in artifacts fitted before the convention stage existed — the
    /// stage is then simply off.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conventions: Option<ConventionModel>,
}

impl LanguageModel {
    /// Stable fingerprint of the learned state: MD5 over the canonical JSON
    /// serialization (deterministic by construction). A fingerprint for
    /// reproducibility checks ("is my model identical to yours?"), not a
    /// security boundary.
    pub fn hash(&self) -> String {
        let canonical = serde_json::to_string(self).expect("model serializes");
        let mut hasher = Md5::new();
        hasher.update(canonical.as_bytes());
        let digest = hasher.finalize();
        digest.iter().map(|b| format!("{b:02x}")).collect()
    }
}

#[cfg(test)]
mod tests;
