//! The terminal card — readable in 5 seconds, honest, fits 80 columns.
//!
//! Layout: headline (window · attribution share · finding count), per-group
//! counts in plain language, ONE worst offender as a concrete mini-story,
//! the rest below the fold, framing + "what `argot init` would do" close.
//! `color = false` (NO_COLOR / non-TTY) renders the same layout unstyled —
//! pipe-safe, no ANSI.

use super::attribution::Attribution;
use super::report::{AuditReport, Finding, GroupStatus};

const WIDTH: usize = 74;

fn paint(text: &str, code: &str, color: bool) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

/// Plain-language one-liner per rule group.
fn group_blurb(group: &str) -> &'static str {
    match group {
        "voice" => "code foreign to how this repo writes",
        "semantic" => "functions you already had, or code filed oddly",
        "architecture" => "imports that break the repo's layering",
        "integrity" => "tests deleted, disabled, or weakened",
        _ => "",
    }
}

/// Middle-ellipsize a path to `max` chars: keep the leading directories AND
/// the filename end. Repos with parallel trees (guava's `guava/` +
/// `android/guava/`) have findings whose only distinguishing part is the
/// prefix — a left-ellipsis would erase exactly that and render two distinct
/// files as identical rows.
fn clip_path(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let head_len = (max - 1) / 3;
    let tail_len = max - 1 - head_len;
    let head: String = s.chars().take(head_len).collect();
    let tail: String = s.chars().skip(n - tail_len).collect();
    format!("{head}…{tail}")
}

/// Right-clip to `max` chars with a trailing ellipsis.
fn clip_right(s: &str, max: usize) -> String {
    let n = s.chars().count();
    if n <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max.saturating_sub(1)).collect();
    format!("{head}…")
}

fn glyph(confidence: &str, color: bool) -> String {
    match confidence {
        "foreign" => paint("!", "31", color),
        "suspicious" => paint("?", "33", color),
        _ => paint("·", "34", color),
    }
}

fn attribution_tag(a: Attribution, color: bool) -> String {
    match a {
        Attribution::AiAssisted => paint("ai-assisted", "36", color),
        Attribution::Human => paint("human", "2", color),
        Attribution::Unknown => paint("unknown", "33", color),
    }
}

/// A marker fitted for card display: when the raw trailer is too long, drop
/// the `<email>` part first (the human-readable half identifies the tool),
/// then clip.
fn display_marker(marker: &str, max: usize) -> String {
    if marker.chars().count() <= max {
        return marker.to_string();
    }
    let without_email = marker
        .split_once('<')
        .map(|(head, _)| head.trim_end().to_string())
        .unwrap_or_else(|| marker.to_string());
    clip_right(&without_email, max)
}

fn span(f: &Finding) -> String {
    if f.line_start == f.line_end {
        format!(":L{}", f.line_start)
    } else {
        format!(":L{}-{}", f.line_start, f.line_end)
    }
}

fn rule_head(title: &str) -> String {
    let title = format!("━━ {title} ");
    let fill = WIDTH.saturating_sub(title.chars().count());
    format!("{title}{}", "━".repeat(fill))
}

pub fn render(report: &AuditReport, color: bool) -> String {
    let bold = |s: &str| paint(s, "1", color);
    let dim = |s: &str| paint(s, "2", color);
    let mut out = String::new();
    let push = |out: &mut String, line: &str| {
        out.push_str(line);
        out.push('\n');
    };

    push(&mut out, &bold(&rule_head("argot audit")));

    // Headline: window · dates · commits, then AI share · findings verdict.
    let w = &report.window;
    let window_label = match &w.requested {
        super::report::RequestedWindow::Commits(_) => {
            format!("last {} commits", w.effective_commits)
        }
        super::report::RequestedWindow::Since(s) => {
            format!("since {s} ({} commits)", w.effective_commits)
        }
    };
    let total = report.commits.total;
    push(
        &mut out,
        &format!(
            "  {window_label} · {} → {} · {total} commit{} audited",
            w.base_date,
            w.head_date,
            if total == 1 { "" } else { "s" }
        ),
    );
    let c = &report.commits;
    let share = format!("{}%", super::report::ai_share_pct(c.ai_assisted, c.total));
    let n = report.findings.len();
    let verdict = if report.hunks_scanned == 0 {
        "no supported source changed".to_string()
    } else if n == 0 {
        format!("0 findings in {} hunks", report.hunks_scanned)
    } else {
        format!(
            "{n} finding{} would have met review",
            if n == 1 { "" } else { "s" }
        )
    };
    push(
        &mut out,
        &bold(&format!(
            "  {share} carry AI markers ({} of {}) · {verdict}",
            c.ai_assisted, c.total
        )),
    );
    if let Some(note) = &w.clamp_note {
        push(&mut out, &dim(&format!("  note: {note}")));
    }
    push(&mut out, "");

    // Per-group counts: groups with findings, plus every skipped group —
    // a skipped group is marked, never rendered as a silent zero.
    let mut group_lines = Vec::new();
    for g in &report.groups {
        match g.status {
            GroupStatus::Scored if g.findings > 0 => {
                group_lines.push((
                    g.group,
                    format!("{:>3}", g.findings),
                    group_blurb(g.group).to_string(),
                ));
            }
            GroupStatus::Skipped => {
                let reason = g.skip_reason.clone().unwrap_or_default();
                group_lines.push((g.group, "  —".to_string(), format!("skipped: {reason}")));
            }
            _ => {}
        }
    }
    if !group_lines.is_empty() {
        let name_w = group_lines.iter().map(|(g, ..)| g.len()).max().unwrap_or(0);
        for (group, count, blurb) in &group_lines {
            push(
                &mut out,
                &format!(
                    "  {}  {count}  {}",
                    bold(&format!("{group:<name_w$}")),
                    dim(blurb)
                ),
            );
        }
        push(&mut out, "");
    }

    if report.hunks_scanned == 0 {
        push(
            &mut out,
            &format!(
                "  These {} commit(s) touched no supported source files (docs-only?).",
                w.effective_commits
            ),
        );
        if w.clamp.is_none() {
            push(
                &mut out,
                &dim("  Try a wider window: argot audit --commits 200"),
            );
        }
    } else if report.findings.is_empty() {
        push(
            &mut out,
            &format!(
                "  Nothing argot would have raised — {} hunks audited, all in voice.",
                report.hunks_scanned
            ),
        );
        push(
            &mut out,
            &dim("  A quiet audit is a good sign: your recent history speaks the"),
        );
        push(&mut out, &dim("  repo's language."));
    } else {
        render_findings(&mut out, report, color);
    }

    // Close: honest framing + what argot init does going forward.
    push(&mut out, "");
    push(
        &mut out,
        &dim("  Merged code is accepted code — read each finding as \"would have"),
    );
    push(
        &mut out,
        &dim("  prompted review before merge\", not a bug list. \"human\" means no AI"),
    );
    push(
        &mut out,
        &dim("  markers were found; the AI share is a floor, not a census."),
    );
    push(
        &mut out,
        &format!(
            "  Next: {} fits today's voice so {} raises these",
            bold("argot init"),
            bold("argot check")
        ),
    );
    push(&mut out, "  before they merge.");
    push(&mut out, &bold(&"━".repeat(WIDTH)));
    out
}

fn render_findings(out: &mut String, report: &AuditReport, color: bool) {
    let bold = |s: &str| paint(s, "1", color);
    let dim = |s: &str| paint(s, "2", color);
    let push = |out: &mut String, line: &str| {
        out.push_str(line);
        out.push('\n');
    };

    // The worst offender — findings are sorted worst-first.
    let worst = &report.findings[0];
    let commit_bit = match (&worst.commit.short, worst.commit.attribution) {
        (Some(short), a) => format!("commit {short} · {}", attribution_tag(a, color)),
        (None, a) => attribution_tag(a, color).to_string(),
    };
    push(out, &format!("  {} — {commit_bit}", bold("Worst offender")));
    push(
        out,
        &format!(
            "  {} {}{} · {}",
            glyph(&worst.confidence, color),
            clip_path(&worst.path, 50),
            span(worst),
            bold(&worst.rule),
        ),
    );
    let story = match (&worst.commit.subject, worst.commit.attribution) {
        (Some(subject), Attribution::AiAssisted) => {
            let marker = worst
                .commit
                .markers
                .first()
                .map(|m| display_marker(m, 36))
                .unwrap_or_default();
            format!("\"{}\" — {marker}", clip_right(subject, 28))
        }
        (Some(subject), Attribution::Human) => {
            format!("\"{}\" — no AI markers", clip_right(subject, 48))
        }
        _ => "introducing commit could not be resolved".to_string(),
    };
    push(out, &dim(&format!("      {story}")));
    if let Some(ev) = &worst.evidence {
        push(out, &dim(&format!("      {}", clip_right(ev, WIDTH - 6))));
    }

    // The rest, below the fold: aligned columns, at most 8 rows.
    if report.findings.len() > 1 {
        push(out, "");
        push(
            out,
            &dim(&format!("  ── the rest {}", "─".repeat(WIDTH - 14))),
        );
        let rest = &report.findings[1..];
        let shown = &rest[..rest.len().min(8)];
        let rule_w = shown.iter().map(|f| f.rule.len()).max().unwrap_or(0);
        for f in shown {
            let loc = clip_path(&format!("{}{}", f.path, span(f)), 38);
            let short = f.commit.short.as_deref().unwrap_or("—");
            push(
                out,
                &format!(
                    "  {} {loc:<38}  {:<rule_w$}  {short:<7}  {}",
                    glyph(&f.confidence, color),
                    f.rule,
                    attribution_tag(f.commit.attribution, color)
                ),
            );
        }
        if rest.len() > shown.len() {
            push(
                out,
                &dim(&format!(
                    "    … and {} more — argot audit --format json for the full list",
                    rest.len() - shown.len()
                )),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::super::attribution::Attribution;
    use super::super::report::*;
    use super::*;

    fn base_report(findings: Vec<Finding>, hunks: u64) -> AuditReport {
        let mut findings = findings;
        AuditReport::sort_findings(&mut findings);
        let groups = vec![
            GroupReport {
                group: "voice",
                status: GroupStatus::Scored,
                findings: findings.iter().filter(|f| f.group == "voice").count(),
                skip_reason: None,
            },
            GroupReport {
                group: "semantic",
                status: GroupStatus::Skipped,
                findings: 0,
                skip_reason: Some("embedding model not available (offline?)".into()),
            },
        ];
        AuditReport {
            schema_version: SCHEMA_VERSION,
            generated_by: "argot vtest".into(),
            window: WindowReport {
                requested: RequestedWindow::Commits(50),
                effective_commits: 50,
                clamp: None,
                clamp_note: None,
                base: "b".repeat(40),
                head: "h".repeat(40),
                base_date: "2026-05-28".into(),
                head_date: "2026-07-12".into(),
            },
            commits: CommitsReport {
                total: 200,
                ai_assisted: 66,
                human: 134,
                unknown: 0,
            },
            hunks_scanned: hunks,
            groups,
            findings,
        }
    }

    fn finding(rule: &str, confidence: &str, path: &str, attribution: Attribution) -> Finding {
        Finding {
            rule: rule.into(),
            rule_label: rule.into(),
            group: "voice",
            confidence: confidence.into(),
            severity: "error".into(),
            path: path.into(),
            line_start: 3,
            line_end: 9,
            evidence: Some("↳ requests — 0 of 74 files import it".into()),
            commit: FindingCommit {
                sha: Some("c".repeat(40)),
                short: Some("ccccccc".into()),
                subject: Some("add http client".into()),
                attribution,
                markers: match attribution {
                    Attribution::AiAssisted => {
                        vec!["Co-Authored-By: Claude <noreply@anthropic.com>".into()]
                    }
                    _ => vec![],
                },
            },
        }
    }

    #[test]
    fn card_leads_with_headline_and_worst_offender() {
        let report = base_report(
            vec![
                finding(
                    "foreign-import",
                    "foreign",
                    "src/api.py",
                    Attribution::AiAssisted,
                ),
                finding("rare-tokens", "unusual", "src/util.py", Attribution::Human),
            ],
            900,
        );
        let card = render(&report, false);
        assert!(card.contains("33% carry AI markers (66 of 200)"), "{card}");
        assert!(card.contains("2 findings would have met review"));
        assert!(card.contains("Worst offender"));
        // Worst = the foreign-confidence one, with its marker story.
        let worst_idx = card.find("Worst offender").unwrap();
        let after = &card[worst_idx..];
        assert!(after.contains("foreign-import"), "{after}");
        assert!(after.contains("Co-Authored-By: Claude"), "{after}");
        assert!(card.contains("skipped: embedding model not available"));
        assert!(card.contains("would have\n  prompted review before merge"));
        assert!(card.contains("argot init"));
    }

    #[test]
    fn quiet_card_is_a_positive_verdict() {
        let card = render(&base_report(vec![], 1200), false);
        assert!(card.contains("all in voice"));
        assert!(card.contains("quiet audit is a good sign"));
        assert!(card.contains("0 findings in 1200 hunks"));
        assert!(!card.contains("Worst offender"));
    }

    #[test]
    fn docs_only_window_suggests_widening_only_when_not_clamped() {
        let card = render(&base_report(vec![], 0), false);
        assert!(card.contains("no supported source files"));
        assert!(card.contains("--commits 200"));
        let mut report = base_report(vec![], 0);
        report.window.clamp = Some("history");
        report.window.clamp_note = Some("history is shorter than the request".into());
        let card = render(&report, false);
        assert!(!card.contains("--commits 200"));
        assert!(card.contains("note: history is shorter"));
    }

    #[test]
    fn parallel_tree_paths_stay_distinguishable_when_clipped() {
        // guava ships identical files under guava/ and android/guava/ — the
        // only distinguishing part is the PREFIX, so clipping must keep both
        // ends of the path.
        let a = "guava/src/com/google/thirdparty/publicsuffix/PublicSuffixPatterns.java";
        let b = "android/guava/src/com/google/thirdparty/publicsuffix/PublicSuffixPatterns.java";
        let ca = clip_path(a, 38);
        let cb = clip_path(b, 38);
        assert_ne!(ca, cb, "{ca} vs {cb}");
        assert!(ca.chars().count() <= 38);
        assert!(
            ca.starts_with("guava/") && cb.starts_with("android/"),
            "{ca} vs {cb}"
        );
        assert!(ca.ends_with(".java") && cb.ends_with(".java"));
    }

    #[test]
    fn no_color_render_has_no_ansi_and_fits_80_cols() {
        let long_path = "packages/deeply/nested/module/with/long/name/implementation_file.ts";
        let report = base_report(
            (0..12)
                .map(|i| {
                    finding(
                        "foreign-import",
                        if i % 2 == 0 { "foreign" } else { "unusual" },
                        long_path,
                        Attribution::AiAssisted,
                    )
                })
                .collect(),
            300,
        );
        let card = render(&report, false);
        assert!(!card.contains('\x1b'));
        for line in card.lines() {
            assert!(
                line.chars().count() <= 80,
                "line over 80 cols ({}): {line}",
                line.chars().count()
            );
        }
        assert!(card.contains("… and 3 more"));
    }
}
