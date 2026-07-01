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
mod tests {
    use super::*;

    fn rarity(total: i64, where_: &str) -> RarityStat {
        RarityStat {
            flagged_count: 0,
            attested_total: total,
            noun: "callees".into(),
            where_: where_.into(),
        }
    }

    #[test]
    fn bpe_line_matches_golden_shape() {
        let ev = Evidence::Bpe(BpeEvidence {
            surprising_identifiers: vec![
                CommonEntry {
                    name: "sessionmaker".into(),
                    count: 0,
                },
                CommonEntry {
                    name: "connect".into(),
                    count: 0,
                },
                CommonEntry {
                    name: "url".into(),
                    count: 0,
                },
                CommonEntry {
                    name: "engine".into(),
                    count: 0,
                },
                CommonEntry {
                    name: "bind".into(),
                    count: 0,
                },
            ],
        });
        let lines = format_evidence(&ev, false, 1);
        assert_eq!(
            lines,
            vec!["     ↳ sessionmaker (0×), connect (0×), url (0×) (+2 more)".to_string()]
        );
    }

    #[test]
    fn call_receiver_lines_with_denominator() {
        // Exercises the "0 of N" rarity branch and the common-here line that
        // neither committed golden hits.
        let ev = Evidence::CallReceiver(CallReceiverEvidence {
            unfamiliar_callees: vec!["mongoose".into(), "connect".into()],
            rarity: rarity(1_247, "this cluster"),
            common_here: vec![
                CommonEntry {
                    name: "select".into(),
                    count: 40,
                },
                CommonEntry {
                    name: "insert".into(),
                    count: 12,
                },
            ],
        });
        let lines = format_evidence(&ev, false, 1);
        assert_eq!(
            lines,
            vec![
                "     ↳ mongoose, connect — 0 of 1,247 callees in this cluster".to_string(),
                "       common here: select (40×), insert (12×)".to_string(),
            ]
        );
    }

    #[test]
    fn import_annotates_line_and_colors_when_asked() {
        let ev = Evidence::Import(ImportEvidence {
            foreign_specifiers: vec!["msgspec".into()],
            rarity: RarityStat {
                flagged_count: 0,
                attested_total: 7,
                noun: "module specifiers".into(),
                where_: "repo".into(),
            },
            common_here: vec![CommonEntry {
                name: "react".into(),
                count: 5,
            }],
            foreign_specifier_spans: vec![(
                "msgspec".into(),
                SourceSpan {
                    line: 3,
                    col_start: 7,
                    col_end: 14,
                },
            )],
        });
        // hunk_start_line=5 → file line 5 + 3 - 1 = 7.
        let plain = format_evidence(&ev, false, 5);
        assert_eq!(
            plain[0],
            "     ↳ msgspec (L7) — never seen in repo".to_string()
        );
        assert_eq!(plain[1], "       common here: react (5×)".to_string());
        // Colored path wraps each line in dim ANSI.
        let colored = format_evidence(&ev, true, 5);
        assert!(colored[0].starts_with("     \x1b[2m↳ msgspec"));
        assert!(colored[0].ends_with("\x1b[0m"));
    }

    #[test]
    fn lines_of_interest_and_carets() {
        let ev = Evidence::Import(ImportEvidence {
            foreign_specifiers: vec!["msgspec".into()],
            rarity: RarityStat {
                flagged_count: 0,
                attested_total: 7,
                noun: "module specifiers".into(),
                where_: "repo".into(),
            },
            common_here: vec![],
            foreign_specifier_spans: vec![(
                "msgspec".into(),
                SourceSpan {
                    line: 2,
                    col_start: 7,
                    col_end: 17,
                },
            )],
        });
        let loi = evidence_lines_of_interest(Some(&ev));
        assert!(loi.contains(&2) && loi.len() == 1);
        let carets = evidence_caret_spans(Some(&ev));
        assert_eq!(carets.get(&2).map(|v| v.len()), Some(1));
        // BPE / None carry no carets or lines-of-interest.
        assert!(evidence_caret_spans(None).is_empty());
        assert!(evidence_lines_of_interest(None).is_empty());
    }
}
