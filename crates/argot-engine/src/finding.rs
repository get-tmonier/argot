//! The finding contract — what every detection pass produces.
//!
//! A [`Finding`] is rule-agnostic: it carries the winning reason code, the
//! location, and an opaque, rule-owned [`RenderEvidence`] payload. The shared
//! render paths (human / JSON / SARIF / GitHub) speak only to this contract,
//! so a rule group can be added or removed without touching them.

use std::collections::{HashMap, HashSet};

/// A 1-indexed line + 0-indexed column range inside a hunk's text.
///
/// `col_start` is inclusive, `col_end` exclusive — the half-open convention
/// tree-sitter uses. Both columns are **byte** offsets in the raw source line;
/// callers must not align carets against ANSI-highlighted output.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceSpan {
    pub line: usize,
    pub col_start: usize,
    pub col_end: usize,
}

/// Which suppression surface muted a finding (`None` = reported normally).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SuppressedBy {
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
pub trait RenderEvidence: Send + Sync {
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

/// One above-threshold finding plus everything needed to explain it.
pub struct Finding {
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
