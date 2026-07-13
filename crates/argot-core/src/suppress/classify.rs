//! Per-file suppression classification — the one place a finding is tested
//! against the three suppression surfaces, in precedence order:
//! `[exclude].paths` → inline `# argot:` comment → `argot.toml [[mute]]`.
//!
//! Every detection pass builds a [`FileSuppressions`] once per file and
//! consults it per finding, so no pass hand-rolls the ordering (and none can
//! drift from it).

use crate::finding::SuppressedBy;
use crate::suppress::{parse_inline, InlineSuppressions, InlineWarning, SuppressionRule};

/// One file's resolved suppression surfaces.
pub(crate) struct FileSuppressions<'a> {
    file_path: &'a str,
    ignored_by_pattern: bool,
    inline: InlineSuppressions,
    mute_rules: &'a [SuppressionRule],
}

impl<'a> FileSuppressions<'a> {
    /// Parse the file's inline suppression comments (skipped when the language
    /// has no known comment prefix) and bind the run's mute rules.
    /// `ignored_by_pattern` marks a file matched by `[exclude].paths` — still
    /// scored, but every finding classifies as excluded.
    pub fn parse(
        file_path: &'a str,
        source: &str,
        comment_prefix: Option<&str>,
        mute_rules: &'a [SuppressionRule],
        ignored_by_pattern: bool,
    ) -> Self {
        let inline = comment_prefix
            .map(|p| parse_inline(source, p))
            .unwrap_or_default();
        FileSuppressions {
            file_path,
            ignored_by_pattern,
            inline,
            mute_rules,
        }
    }

    /// Which surface (if any) mutes a finding at `line..line_end` with this
    /// reason code and hit hash.
    pub fn classify(
        &self,
        reason: &str,
        hash: &str,
        line: usize,
        line_end: usize,
    ) -> Option<SuppressedBy> {
        if self.ignored_by_pattern {
            Some(SuppressedBy::Exclude)
        } else if self.inline.suppresses(line, line_end, reason) {
            Some(SuppressedBy::Inline)
        } else if self
            .mute_rules
            .iter()
            .any(|r| r.matches(self.file_path, reason, hash))
        {
            Some(SuppressedBy::Mute)
        } else {
            None
        }
    }

    /// Malformed-inline-comment warnings from parsing this file. Only the
    /// voice pass reports these (once per file, deduped); the additive passes
    /// stay silent — reporting is the caller's decision, not this type's.
    pub fn warnings(&self) -> &[InlineWarning] {
        &self.inline.warnings
    }
}

#[cfg(test)]
mod tests;
