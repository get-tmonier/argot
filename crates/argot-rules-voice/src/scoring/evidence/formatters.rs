//! Per-reason renderers that turn an [`Evidence`] payload into lines.
//!
//! Port of `engine/argot/scoring/evidence/formatters.py`. Each reason gets its
//! own render path; all shared layout decisions live in [`super::layout`]. The
//! formatters own only the per-reason "where to read names from" choice and the
//! no-color / color rendering toggle.

use super::layout::{
    format_common_here_line, format_frequency, format_rarity, should_show_common_here,
    truncate_with_overflow, TOP_K_COMMON_HERE, TOP_K_NAMES,
};
use super::types::{
    BpeEvidence, CallReceiverEvidence, CommonEntry, Evidence, ImportEvidence, RarityStat,
    SourceSpan,
};
use std::collections::{HashMap, HashSet};

// Two ANSI codes — evidence rendering is dim by design (secondary info sitting
// under each headline hit).
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

// Indents align the `↳` glyph under the score column of the headline (5
// spaces) and the `common here:` body two characters deeper (7 spaces).
const NAMES_INDENT: &str = "     ";
const COMMON_INDENT: &str = "       ";

const GLYPH: &str = "↳";
const COMMON_PREFIX: &str = "common here:";

/// Wrap `text` in dim ANSI when colour is enabled, else return as-is.
fn dim(text: &str, use_color: bool) -> String {
    if use_color {
        format!("{DIM}{text}{RESET}")
    } else {
        text.to_string()
    }
}

/// Render the `↳` line body or return `None` to suppress. No names → no line.
fn names_line(names: &[String], rarity: &RarityStat, use_color: bool) -> Option<String> {
    if names.is_empty() {
        return None;
    }
    let body = format!(
        "{} — {}",
        truncate_with_overflow(names, TOP_K_NAMES),
        format_rarity(rarity)
    );
    Some(format!(
        "{NAMES_INDENT}{}",
        dim(&format!("{GLYPH} {body}"), use_color)
    ))
}

/// Return `"name (Lnn)"` when a file line is known, else just `name`.
fn annotate_with_line(name: &str, file_line: Option<usize>) -> String {
    match file_line {
        Some(l) => format!("{name} (L{l})"),
        None => name.to_string(),
    }
}

/// Render the `common here: ...` line body. Caller pre-checks the floor.
fn common_here_line(entries: &[CommonEntry], use_color: bool) -> String {
    let body = format_common_here_line(entries, TOP_K_COMMON_HERE);
    format!(
        "{COMMON_INDENT}{}",
        dim(&format!("{COMMON_PREFIX} {body}"), use_color)
    )
}

/// Render [`BpeEvidence`] to a single `↳` line of per-token counts.
fn render_bpe(evidence: &BpeEvidence, use_color: bool) -> Vec<String> {
    if evidence.surprising_identifiers.is_empty() {
        return Vec::new();
    }
    let head_len = TOP_K_NAMES.min(evidence.surprising_identifiers.len());
    let head = &evidence.surprising_identifiers[..head_len];
    let mut body = head
        .iter()
        .map(|e| format!("{} ({})", e.name, format_frequency(e.count)))
        .collect::<Vec<_>>()
        .join(", ");
    let overflow = evidence.surprising_identifiers.len() - head_len;
    if overflow > 0 {
        body = format!("{body} (+{overflow} more)");
    }
    vec![format!(
        "{NAMES_INDENT}{}",
        dim(&format!("{GLYPH} {body}"), use_color)
    )]
}

/// Convert a hunk-relative line for `name` to a 1-indexed file line. `None`
/// when no span was captured — the formatter then renders the bare name.
fn file_line_for(name: &str, evidence: &ImportEvidence, hunk_start_line: usize) -> Option<usize> {
    let span = evidence.span_for(name)?;
    Some(hunk_start_line + span.line - 1)
}

/// Render [`ImportEvidence`] to at most two lines under an import-fired hit.
fn render_import(
    evidence: &ImportEvidence,
    use_color: bool,
    hunk_start_line: usize,
) -> Vec<String> {
    let mut out = Vec::new();
    let annotated: Vec<String> = evidence
        .foreign_specifiers
        .iter()
        .map(|name| annotate_with_line(name, file_line_for(name, evidence, hunk_start_line)))
        .collect();
    if let Some(line) = names_line(&annotated, &evidence.rarity, use_color) {
        out.push(line);
    }
    if should_show_common_here(&evidence.common_here) {
        out.push(common_here_line(&evidence.common_here, use_color));
    }
    out
}

/// Render [`CallReceiverEvidence`] to at most two lines under a CR-fired hit.
fn render_call_receiver(evidence: &CallReceiverEvidence, use_color: bool) -> Vec<String> {
    let mut out = Vec::new();
    if let Some(line) = names_line(&evidence.unfamiliar_callees, &evidence.rarity, use_color) {
        out.push(line);
    }
    if should_show_common_here(&evidence.common_here) {
        out.push(common_here_line(&evidence.common_here, use_color));
    }
    out
}

/// Single-entrypoint dispatcher for the renderer. Routes the variant to the
/// matching per-reason render path. `hunk_start_line` is forwarded so import
/// evidence can convert hunk-relative line annotations into file lines.
pub fn format_evidence(
    evidence: &Evidence,
    use_color: bool,
    hunk_start_line: usize,
) -> Vec<String> {
    match evidence {
        Evidence::Bpe(e) => render_bpe(e, use_color),
        Evidence::Import(e) => render_import(e, use_color, hunk_start_line),
        Evidence::CallReceiver(e) => render_call_receiver(e, use_color),
    }
}

/// Hunk-relative line numbers the renderer should keep visible past the
/// truncation cap (the flagged import lines). Empty for non-import evidence.
pub fn evidence_lines_of_interest(evidence: Option<&Evidence>) -> HashSet<usize> {
    match evidence {
        Some(Evidence::Import(e)) => e
            .foreign_specifier_spans
            .iter()
            .map(|(_, span)| span.line)
            .filter(|&l| l >= 1)
            .collect(),
        _ => HashSet::new(),
    }
}

/// `{hunk_line: [SourceSpan, ...]}` for the eslint-style carets. Empty for
/// non-import evidence (BPE / call-receiver hits, evidence without spans).
pub fn evidence_caret_spans(evidence: Option<&Evidence>) -> HashMap<usize, Vec<SourceSpan>> {
    let mut out: HashMap<usize, Vec<SourceSpan>> = HashMap::new();
    if let Some(Evidence::Import(e)) = evidence {
        for (_, span) in &e.foreign_specifier_spans {
            out.entry(span.line).or_default().push(span.clone());
        }
    }
    out
}

#[cfg(test)]
mod tests;
