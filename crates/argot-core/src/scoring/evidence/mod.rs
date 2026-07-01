//! Evidence layer — port of `engine/argot/scoring/evidence`.
//!
//! Per-reason `Evidence` payloads carried on `ScoredHunk`, the shared layout
//! primitives, the per-reason renderers (`format_evidence`), and the
//! per-reason collectors that build a payload once a hunk fires. The public
//! entrypoints used by `check` are [`formatters::format_evidence`],
//! [`formatters::evidence_lines_of_interest`], and
//! [`formatters::evidence_caret_spans`]; the scorer builds payloads via the
//! `collect_*_evidence` functions.

pub mod bpe;
pub mod bpe_reconstruction;
pub mod call_receiver;
pub mod formatters;
pub mod imports;
pub mod layout;
pub mod types;

pub use bpe::collect_bpe_evidence;
pub use call_receiver::collect_call_receiver_evidence;
pub use formatters::{evidence_caret_spans, evidence_lines_of_interest, format_evidence};
pub use imports::collect_import_evidence;
pub use types::{
    BpeEvidence, CallReceiverEvidence, CommonEntry, Evidence, EvidenceCorpus, EvidenceCorpusTotals,
    ImportEvidence, RarityStat, SourceSpan,
};
