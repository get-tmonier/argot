//! The finding contract — what every detection pass produces.
//!
//! A [`Finding`] is rule-agnostic: it carries the winning reason code, the
//! location, and an opaque, rule-owned [`RenderEvidence`] payload. The shared
//! render paths (human / JSON / SARIF / GitHub) speak only to this contract,
//! so a rule group can be added or removed without touching them.

use std::collections::{HashMap, HashSet};

use crate::scoring::evidence::types::{Evidence, SourceSpan};
use crate::scoring::evidence::{evidence_caret_spans, evidence_lines_of_interest, format_evidence};

/// Which suppression surface muted a finding (`None` = reported normally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SuppressedBy {
    /// An `argot.toml` `[exclude].paths` pattern.
    Exclude,
    /// An inline `# argot: ignore` comment.
    Inline,
    /// An `argot.toml` `[[mute]]` entry.
    Mute,
}

/// Rule-owned evidence rendering — how a finding explains itself on each
/// output surface. Implemented once per rule group; the shared render paths
/// call the trait and never match on the group.
pub(crate) trait RenderEvidence: Send + Sync {
    /// Human-format lines, exactly as printed under the finding headline
    /// (including layout indentation and optional ANSI color).
    fn human(&self, use_color: bool, hunk_start_line: usize) -> Vec<String>;
    /// Machine-format lines (JSON/SARIF `evidence`) — layout indentation
    /// stripped.
    fn machine(&self, hunk_start_line: usize) -> Vec<String>;
    /// Similarity score for reinvention findings (`HitRecord.similarity`).
    fn similarity(&self) -> Option<f32> {
        None
    }
    /// Verbatim, untruncated flagged import specifiers
    /// (`HitRecord.foreign_specifiers`).
    fn foreign_specifiers(&self) -> Vec<String> {
        Vec::new()
    }
    /// The named symbol the finding is about (`HitRecord.symbol`).
    fn symbol(&self) -> Option<String> {
        None
    }
    /// 1-indexed source lines smart-peek must keep in frame.
    fn lines_of_interest(&self) -> HashSet<usize> {
        HashSet::new()
    }
    /// Caret (`^^^^`) spans keyed by 1-indexed source line.
    fn caret_spans(&self) -> HashMap<usize, Vec<SourceSpan>> {
        HashMap::new()
    }
}

/// The statistical voice evidence renders through its existing formatters —
/// the exact strings the pre-contract render paths produced.
impl RenderEvidence for Evidence {
    fn human(&self, use_color: bool, hunk_start_line: usize) -> Vec<String> {
        format_evidence(self, use_color, hunk_start_line)
    }

    fn machine(&self, hunk_start_line: usize) -> Vec<String> {
        format_evidence(self, false, hunk_start_line)
            .into_iter()
            .map(|l| l.trim().to_string())
            .collect()
    }

    fn foreign_specifiers(&self) -> Vec<String> {
        match self {
            Evidence::Import(imp) => imp.foreign_specifiers.clone(),
            _ => Vec::new(),
        }
    }

    fn lines_of_interest(&self) -> HashSet<usize> {
        evidence_lines_of_interest(Some(self))
    }

    fn caret_spans(&self) -> HashMap<usize, Vec<SourceSpan>> {
        evidence_caret_spans(Some(self))
    }
}

/// One above-threshold finding plus everything needed to explain it.
pub(crate) struct Finding {
    /// The winning candidate's score (adjusted for contributions), measured
    /// against the winning candidate's threshold — so severity tiers mean
    /// the same thing for every reason. A call-receiver hit that crossed on
    /// a +5 contribution reads as the strong signal it is, not as its raw
    /// BPE component.
    pub score: f64,
    pub file_path: String,
    pub line: usize,
    pub line_end: usize,
    pub source: String,
    pub reason: String,
    pub flagged: bool,
    pub threshold: f64,
    pub hunk_content: String,
    /// Rule-owned evidence for the winning reason (`None` when the pass had
    /// nothing to explain beyond the location).
    pub evidence: Option<Box<dyn RenderEvidence>>,
    /// Content-based hit hash (path + winning reason + normalized hunk).
    pub hash: String,
    /// Set when a suppression surface muted this finding.
    pub suppressed_by: Option<SuppressedBy>,
}
