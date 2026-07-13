//! Human and machine rendering of `check` findings, plus `--add-ignores`.

use super::{ext_to_lang, extension, CheckArgs, CheckOutcome};
use crate::finding::Finding;
use crate::output::{
    render_github, render_json, render_sarif, FileScan, HitRecord, OutputFormat, ReportMeta,
};
use crate::rules::{self, RuleSettings};
use crate::scoring::adapters::LanguageAdapter;
use crate::scoring::evidence::types::SourceSpan;
use crate::text::splitlines;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fs;
use std::path::Path;

// ANSI color codes for the human `check` render. Every colored write goes
// through `paint`, which is a no-op when `use_color` is false — so the
// `NO_COLOR` / non-tty path stays byte-identical to the parity fixtures.
const C_RED: &str = "\x1b[31m";
const C_YELLOW: &str = "\x1b[33m";
const C_BLUE: &str = "\x1b[34m";
const C_BOLD: &str = "\x1b[1m";
pub(super) const C_DIM: &str = "\x1b[2m";
const C_RESET: &str = "\x1b[0m";
/// The accent color for a confidence tier: red (foreign), yellow (suspicious),
/// blue (unusual).
fn confidence_color(tier: &str) -> &'static str {
    match tier {
        "foreign" => C_RED,
        "suspicious" => C_YELLOW,
        _ => C_BLUE,
    }
}
/// Wrap `text` in an ANSI code when `use_color`, else return it unchanged.
pub(super) fn paint(text: &str, color: &str, use_color: bool) -> String {
    if use_color {
        format!("{color}{text}{C_RESET}")
    } else {
        text.to_string()
    }
}
/// Classify a hit into a confidence tier.
///
/// Confidence expresses the *strength of the evidence that a hunk is foreign*,
/// derived per signal-kind — not one margin rule for every reason:
///
/// * **Categorical foreign signals** are `foreign` by nature. A foreign import
///   is a dependency the repo has never used (0-usage at the fit SHA) — the
///   top-tier signal, and the one argot catches most reliably. Its score is a
///   *count* of never-before-seen modules against a threshold of 1.0, so the
///   additive margins below (calibrated for the BPE nat scale) would misfile a
///   lone foreign import as `unusual` — the weakest tier — even though it *is*
///   the definition of `foreign`.
/// * **Distributional signals** (BPE surprise, convention rarity, unfamiliar
///   callee) grade by margin above the calibrated threshold: the margin there
///   genuinely measures how far outside the repo's voice the hunk sits.
/// * **Structural findings** (`redundant` / `misplaced` / `layering`) pin to
///   `unusual` — they surface real, linter-invisible structure (a duplicate, a
///   misplacement, a crossed boundary) for the author to judge; their scores
///   are not on the foreignness scale the margins above grade.
/// * **Integrity findings** (`test-deleted` / `test-disabled` /
///   `test-weakened`) pin to `suspicious`: each is a discrete, evidenced
///   event (a marker added, assertions excised) that survived the FP
///   refinements and the repo's own calibrated gates — stronger than
///   `unusual`, but not the categorical certainty of a 0-usage import.
///
/// Whether a finding fails the check is its rule's configured severity
/// (`error` / `warn` / `off`), not this tier.
pub(super) fn confidence(reason: &str, score: f64, threshold: f64) -> &'static str {
    match reason {
        "import" => "foreign",
        "redundant" | "misplaced" | "layering" => "unusual",
        "test_deleted" | "test_disabled" | "test_weakened" => "suspicious",
        _ => {
            if score >= threshold + 1.5 {
                "foreign"
            } else if score >= threshold + 0.5 {
                "suspicious"
            } else {
                "unusual"
            }
        }
    }
}
/// Build the eslint-style `^^^^^` underline for one source line
/// (`_render_caret_line`, `use_color=false`). Column ranges are byte offsets;
/// overlapping spans merge; returns `None` when no caret ends up printable.
fn render_caret_line(
    raw_line: &str,
    spans: &[SourceSpan],
    visible_prefix_width: usize,
) -> Option<String> {
    let line_len = raw_line.len(); // byte length, matching the byte-offset spans
    let mut covered = vec![false; line_len];
    for sp in spans {
        let end = sp.col_end.min(line_len);
        for c in covered.iter_mut().take(end).skip(sp.col_start) {
            *c = true;
        }
    }
    if !covered.iter().any(|&c| c) {
        return None;
    }
    let underline: String = covered.iter().map(|&c| if c { '^' } else { ' ' }).collect();
    let underline = underline.trim_end();
    if underline.is_empty() {
        return None;
    }
    Some(format!("{}{}", " ".repeat(visible_prefix_width), underline))
}
/// Format the hunk body as a numbered code block (`_render_hunk_body`,
/// `use_color=false`). `max_lines = None` in verbose mode. `must_show_hunk_lines`
/// grows the truncation budget to keep flagged lines in-frame;
/// `caret_spans_by_line` draws `^^^^` underlines below flagged source lines.
/// Returns `(lines, overflow)`.
fn render_hunk_body(
    content: &str,
    start_line: usize,
    max_lines: Option<usize>,
    must_show_hunk_lines: &HashSet<usize>,
    caret_spans_by_line: &HashMap<usize, Vec<SourceSpan>>,
    use_color: bool,
    caret_color: &str,
) -> (Vec<String>, usize) {
    if let Some(n) = max_lines {
        if n == 0 {
            return (Vec::new(), splitlines(content).len());
        }
    }
    let raw_lines = splitlines(content);
    if raw_lines.is_empty() {
        return (Vec::new(), 0);
    }
    let shown = match max_lines {
        None => raw_lines.len(),
        Some(n) => {
            let mut shown = n.min(raw_lines.len());
            // Smart-peek: grow the budget so any flagged hunk-relative line is
            // in-frame, bounded by the actual hunk length.
            let max_in_range = must_show_hunk_lines
                .iter()
                .copied()
                .filter(|&ln| 1 <= ln && ln <= raw_lines.len())
                .max();
            if let Some(m) = max_in_range {
                shown = raw_lines.len().min(shown.max(m));
            }
            shown
        }
    };
    let overflow = raw_lines.len() - shown;
    let width = (start_line + shown - 1).to_string().len();
    // Visible-prefix width for caret alignment: "  " + ln digits + " " + "|" + " ".
    let caret_pad = 2 + width + 1 + 1 + 1;
    let mut out: Vec<String> = Vec::new();
    for (i, line) in raw_lines.iter().take(shown).enumerate() {
        let ln = start_line + i;
        out.push(format!("  {:>width$} | {}", ln, line, width = width));
        // The i-th rendered line is hunk-line (i + 1) regardless of start_line.
        if let Some(spans) = caret_spans_by_line.get(&(i + 1)) {
            if let Some(caret) = render_caret_line(line, spans, caret_pad) {
                out.push(paint(&caret, caret_color, use_color));
            }
        }
    }
    if overflow > 0 {
        let plural = if overflow != 1 { "s" } else { "" };
        out.push(paint(
            &format!(
                "  {}   (+{} more line{})",
                " ".repeat(width),
                overflow,
                plural
            ),
            C_DIM,
            use_color,
        ));
    }
    (out, overflow)
}
/// Render grouped results (`_render_results`). Colored per-severity when
/// `use_color`; otherwise byte-identical to the parity fixtures. Returns whether
/// any hunk body was truncated.
pub(super) fn render_results(
    hits: &[&Finding],
    hunk_lines: Option<usize>,
    use_color: bool,
    out: &mut String,
) -> bool {
    // Banner tier counts use the per-hit calibrated threshold.
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for h in hits {
        *counts
            .entry(confidence(&h.reason, h.score, h.threshold))
            .or_insert(0) += 1;
    }
    let total = hits.len();
    let mut tier_parts: Vec<String> = Vec::new();
    for tier in ["foreign", "suspicious", "unusual"] {
        let c = *counts.get(tier).unwrap_or(&0);
        if c > 0 {
            tier_parts.push(format!(
                "{c} {}",
                paint(tier, confidence_color(tier), use_color)
            ));
        }
    }
    let mut banner = format!(
        "argot check · {} hunk{} above threshold",
        total,
        if total != 1 { "s" } else { "" }
    );
    if !tier_parts.is_empty() {
        banner.push_str(&format!(" ({})", tier_parts.join(" · ")));
    }
    out.push_str(&banner);
    out.push('\n');
    out.push_str("note: argot is a probabilistic style linter — verify before action.\n");
    out.push('\n');

    // Group by file; file_max starts at 0.0 (defaultdict(float)) so all-negative
    // scores tie at 0.0 and files fall back to first-appearance (walk) order.
    let mut order: Vec<String> = Vec::new();
    let mut file_max: HashMap<String, f64> = HashMap::new();
    let mut file_hits: HashMap<String, Vec<&Finding>> = HashMap::new();
    for h in hits {
        if !file_hits.contains_key(&h.file_path) {
            order.push(h.file_path.clone());
        }
        let m = file_max.entry(h.file_path.clone()).or_insert(0.0);
        if h.score > *m {
            *m = h.score;
        }
        file_hits.entry(h.file_path.clone()).or_default().push(h);
    }
    let mut sorted_files = order;
    // Stable descending sort by file_max (ties keep insertion order).
    sorted_files.sort_by(|a, b| {
        file_max[b]
            .partial_cmp(&file_max[a])
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    let mut any_truncated = false;
    let n_files = sorted_files.len();
    for (i, fp) in sorted_files.iter().enumerate() {
        out.push_str(&paint(fp, C_BOLD, use_color));
        out.push('\n');

        let mut fhits: Vec<&Finding> = file_hits[fp].clone();
        fhits.sort_by_key(|h| h.line); // stable by line asc

        for h in &fhits {
            let sev = confidence(&h.reason, h.score, h.threshold);
            let color = confidence_color(sev);
            let line_str = if h.line == h.line_end {
                format!("L{}", h.line)
            } else {
                format!("L{}-L{}", h.line, h.line_end)
            };
            // The meta line names the rule (`foreign-import`, `redundant`, …);
            // internal reasons without a rule (`none` under --threshold) print raw.
            let meta = format!("· {} · {}", h.source, rules::code_for_reason(&h.reason));
            let glyph = match sev {
                "foreign" => "!",
                "suspicious" => "?",
                _ => ".",
            };
            // ANSI codes are zero-width, so the `{:<13}`/`{:>6.2}` columns still
            // align; only the glyph, severity word, and hash carry escapes.
            out.push_str(&format!(
                "  {}  {:<13} {:>6.2}  {}  {} {}\n",
                paint(glyph, color, use_color),
                line_str,
                h.score,
                paint(sev, color, use_color),
                meta,
                paint(&format!("[{}]", h.hash), C_DIM, use_color),
            ));

            // Rule-owned evidence sits between the headline and the hunk body.
            // `hunk_start_line = h.line` lets import evidence render `(L7)`
            // file-line annotations.
            if let Some(ev) = &h.evidence {
                for line in ev.human(use_color, h.line) {
                    out.push_str(&line);
                    out.push('\n');
                }
            }

            // Smart-peek keeps flagged lines in-frame; caret spans drive the
            // eslint-style `^^^^` underlines under the offending bytes.
            let must_show = h
                .evidence
                .as_ref()
                .map(|e| e.lines_of_interest())
                .unwrap_or_default();
            let caret_spans = h
                .evidence
                .as_ref()
                .map(|e| e.caret_spans())
                .unwrap_or_default();
            let (body, overflow) = render_hunk_body(
                &h.hunk_content,
                h.line,
                hunk_lines,
                &must_show,
                &caret_spans,
                use_color,
                color,
            );
            for line in body {
                out.push_str(&line);
                out.push('\n');
            }
            if overflow > 0 {
                any_truncated = true;
            }
        }

        if i < n_files - 1 {
            out.push('\n');
        }
    }

    any_truncated
}
/// Insert inline `argot: ignore-next-line` comments above the given 1-indexed
/// lines of `source`, bottom-up so earlier insertions never shift later
/// targets. Each comment copies the target line's indentation. Pure — the
/// caller does the I/O.
pub(super) fn insert_ignore_comments(source: &str, comments: &[(usize, String)]) -> String {
    let mut lines: Vec<String> = source.split('\n').map(str::to_string).collect();
    let mut sorted: Vec<&(usize, String)> = comments.iter().collect();
    sorted.sort_by_key(|(line, _)| std::cmp::Reverse(*line));
    for (line, text) in sorted {
        let idx = line.saturating_sub(1).min(lines.len());
        let indent: String = lines
            .get(idx)
            .map(|l| l.chars().take_while(|c| c.is_whitespace()).collect())
            .unwrap_or_default();
        lines.insert(idx, format!("{indent}{text}"));
    }
    lines.join("\n")
}
/// `--add-ignores`: write one inline suppression above every visible finding
/// (deduped per line; a line carrying several rules gets one unscoped
/// comment). Adoption tooling — a wall of existing findings becomes a set of
/// reviewable, greppable comments instead of a red first run.
pub(super) fn add_ignore_comments(
    args: &CheckArgs,
    visible: &[&Finding],
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    stderr: String,
) -> CheckOutcome {
    // Only the working-tree modes: editing files based on a historical ref's
    // line numbers would write comments into the wrong places.
    if !args.reference.is_empty() || args.commit.as_deref().is_some_and(|c| !c.is_empty()) {
        return CheckOutcome::err(
            "error: --add-ignores edits the working tree — run it without a ref/--commit\n"
                .to_string(),
            2,
        );
    }
    if visible.is_empty() {
        return CheckOutcome {
            stdout: "No findings — nothing to ignore.\n".to_string(),
            stderr,
            exit_code: 0,
        };
    }

    // file → line → rules found there.
    let mut by_file: BTreeMap<&str, BTreeMap<usize, Vec<&str>>> = BTreeMap::new();
    for h in visible {
        by_file
            .entry(h.file_path.as_str())
            .or_default()
            .entry(h.line)
            .or_default()
            .push(rules::code_for_reason(&h.reason));
    }

    let mut files_written = 0usize;
    let mut comments_written = 0usize;
    let mut stderr = stderr;
    for (file, lines) in &by_file {
        let Some(prefix) = ext_to_lang(&extension(file))
            .and_then(|l| filter_adapters.get(l))
            .map(|a| a.line_comment_prefix())
        else {
            stderr.push_str(&format!("[argot] {file}: unknown language — skipped\n"));
            continue;
        };
        let path = Path::new(&args.repo_path).join(file);
        let Ok(source) = fs::read_to_string(&path) else {
            stderr.push_str(&format!("[argot] {file}: unreadable — skipped\n"));
            continue;
        };
        let comments: Vec<(usize, String)> = lines
            .iter()
            .map(|(line, rule_names)| {
                let mut names: Vec<&str> = rule_names.clone();
                names.sort_unstable();
                names.dedup();
                let scope = if names.len() == 1 {
                    format!(" rule={}", names[0])
                } else {
                    String::new()
                };
                (
                    *line,
                    format!(
                        "{prefix} argot: ignore-next-line{scope} — baselined by --add-ignores; review"
                    ),
                )
            })
            .collect();
        let updated = insert_ignore_comments(&source, &comments);
        if let Err(e) = fs::write(&path, updated) {
            stderr.push_str(&format!("[argot] {file}: write failed ({e}) — skipped\n"));
            continue;
        }
        files_written += 1;
        comments_written += comments.len();
    }

    CheckOutcome {
        stdout: format!(
            "Added {comments_written} ignore comment(s) across {files_written} file(s) — \
             review them, then commit (each carries a greppable reason).\n"
        ),
        stderr,
        exit_code: 0,
    }
}
/// Flatten visible hits into serializable [`HitRecord`]s for the machine
/// formats. Confidence is measured against the per-hit calibrated threshold,
/// matching the human rendering; severity is the rule's configured level;
/// evidence lines are the same per-reason lines the human path prints, with
/// layout indentation stripped.
pub(super) fn hit_records(
    hits: &[&Finding],
    settings: &RuleSettings,
    registry: &rules::Registry,
) -> Vec<HitRecord> {
    hits.iter()
        .map(|h| HitRecord {
            path: h.file_path.clone(),
            line_start: h.line,
            line_end: h.line_end,
            score: h.score,
            threshold: h.threshold,
            confidence: confidence(&h.reason, h.score, h.threshold).to_string(),
            severity: settings.severity_of_reason(&h.reason).as_str().to_string(),
            rule: rules::code_for_reason(&h.reason).to_string(),
            rule_label: registry.label_for_reason(&h.reason).to_string(),
            source: h.source.clone(),
            hash: h.hash.clone(),
            evidence: h
                .evidence
                .as_ref()
                .map(|e| e.machine(h.line))
                .unwrap_or_default(),
            symbol: h.evidence.as_ref().and_then(|e| e.symbol()),
            // Verbatim, untruncated flagged specifiers for import findings —
            // machine consumers (e.g. `argot audit`) classify these without
            // re-parsing the rendered evidence, which caps the list at TOP_K.
            foreign_specifiers: h
                .evidence
                .as_ref()
                .map(|e| e.foreign_specifiers())
                .unwrap_or_default(),
            similarity: h.evidence.as_ref().and_then(|e| e.similarity()),
        })
        .collect()
}
pub(super) fn report_meta(
    args: &CheckArgs,
    scanned: String,
    hunks_scanned: usize,
    files_scanned: Vec<FileScan>,
    model: &str,
) -> ReportMeta {
    ReportMeta {
        // The workspace shares one version across crates, so this matches the
        // CLI binary's version.
        tool_version: env!("CARGO_PKG_VERSION").to_string(),
        repo: args.repo_path.clone(),
        scanned,
        hunks_scanned,
        files_scanned,
        model: model.to_string(),
    }
}
/// Render the complete machine-format document (json/sarif) for stdout.
pub(super) fn render_machine(
    format: OutputFormat,
    meta: &ReportMeta,
    records: &[HitRecord],
) -> String {
    match format {
        OutputFormat::Sarif => render_sarif(meta, records),
        OutputFormat::Github => render_github(records),
        _ => render_json(meta, records),
    }
}
