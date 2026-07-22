//! Markdown card — pasteable into a PR or issue. No ANSI, no box chars;
//! same content order as the terminal card: headline, groups, worst
//! offender, the rest under `<details>`, honest framing.

use super::report::{AuditReport, Finding, GroupStatus, RequestedWindow, METHOD_NOTE};

fn span(f: &Finding) -> String {
    if f.line_start == f.line_end {
        format!("L{}", f.line_start)
    } else {
        format!("L{}-{}", f.line_start, f.line_end)
    }
}

fn commit_cell(f: &Finding) -> String {
    match &f.commit.short {
        Some(short) => format!("`{short}` ({})", f.commit.attribution.as_str()),
        None => "unknown".to_string(),
    }
}

pub fn render(report: &AuditReport) -> String {
    let mut out = String::new();
    let w = &report.window;
    let c = &report.commits;

    out.push_str("## argot audit\n\n");
    let window_label = match &w.requested {
        RequestedWindow::Commits(_) => format!("last {} commits", w.effective_commits),
        RequestedWindow::Since(s) => format!("since {s} ({} commits)", w.effective_commits),
    };
    let share = super::report::ai_share_pct(c.ai_assisted, c.total);
    let n = report.findings.len();
    let verdict = if report.hunks_scanned == 0 {
        "no supported source changed".to_string()
    } else if n == 0 {
        format!("**0 findings** in {} hunks", report.hunks_scanned)
    } else {
        format!(
            "**{n} finding{}** argot would have raised before merge",
            if n == 1 { "" } else { "s" }
        )
    };
    out.push_str(&format!(
        "**{window_label}** ({} → {}) · {} commit{} audited · \
         **{share}%** carry AI markers ({} of {}) · {verdict}\n\n",
        w.base_date,
        w.head_date,
        c.total,
        if c.total == 1 { "" } else { "s" },
        c.ai_assisted,
        c.total
    ));
    if let Some(note) = &w.clamp_note {
        out.push_str(&format!("> note: {note}\n\n"));
    }

    // Groups: findings + skips; all-quiet groups collapse into the verdict.
    let rows: Vec<String> = report
        .groups
        .iter()
        .filter_map(|g| match g.status {
            GroupStatus::Scored if g.findings > 0 => {
                Some(format!("| {} | {} | |", g.group, g.findings))
            }
            GroupStatus::Skipped => Some(format!(
                "| {} | — | skipped: {} |",
                g.group,
                g.skip_reason.as_deref().unwrap_or("")
            )),
            _ => None,
        })
        .collect();
    if !rows.is_empty() {
        out.push_str("| group | findings | note |\n|---|---|---|\n");
        for r in rows {
            out.push_str(&r);
            out.push('\n');
        }
        out.push('\n');
    }

    if report.hunks_scanned == 0 {
        out.push_str(&format!(
            "These {} commit(s) touched no supported source files (docs-only?).\n\n",
            w.effective_commits
        ));
    } else if report.findings.is_empty() {
        out.push_str(
            "A quiet audit is a bounded result, not proof that every change is ready to accept.\n\n",
        );
    } else {
        let worst = &report.findings[0];
        out.push_str("**Worst offender** — ");
        out.push_str(&format!(
            "`{}:{}` · {} · {}",
            worst.path,
            span(worst),
            worst.rule,
            commit_cell(worst)
        ));
        if let Some(subject) = &worst.commit.subject {
            out.push_str(&format!("\n> \"{subject}\""));
            if let Some(marker) = worst.commit.markers.first() {
                out.push_str(&format!(" — `{marker}`"));
            }
        }
        if let Some(ev) = &worst.evidence {
            out.push_str(&format!("\n> {ev}"));
        }
        out.push_str("\n\n");

        if report.findings.len() > 1 {
            out.push_str(&format!(
                "<details>\n<summary>All {} findings</summary>\n\n",
                report.findings.len()
            ));
            out.push_str(
                "| file | rule | confidence | commit | evidence |\n|---|---|---|---|---|\n",
            );
            for f in &report.findings {
                out.push_str(&format!(
                    "| `{}:{}` | {} | {} | {} | {} |\n",
                    f.path,
                    span(f),
                    f.rule,
                    f.confidence,
                    commit_cell(f),
                    f.evidence.as_deref().unwrap_or("").replace('|', "\\|")
                ));
            }
            out.push_str("\n</details>\n\n");
        }
    }

    out.push_str(&format!(
        "Merged code is accepted code — read each finding as \"would have prompted \
         review before merge\", not a bug list.\n\n\
         {METHOD_NOTE}\n\n\
         Next: `argot init` fits today's voice so `argot check` raises these before they merge.\n",
    ));
    out
}

#[cfg(test)]
mod tests {
    use super::super::report::*;
    use super::*;

    fn report(findings: Vec<Finding>) -> AuditReport {
        AuditReport {
            schema_version: SCHEMA_VERSION,
            generated_by: "argot vtest".into(),
            window: WindowReport {
                requested: RequestedWindow::Since("6m".into()),
                effective_commits: 120,
                clamp: None,
                clamp_note: None,
                base: "b".repeat(40),
                head: "h".repeat(40),
                base_date: "2026-01-10".into(),
                head_date: "2026-07-12".into(),
            },
            commits: CommitsReport {
                total: 300,
                ai_assisted: 100,
                human: 200,
                unknown: 0,
            },
            hunks_scanned: 500,
            groups: vec![GroupReport {
                group: "voice",
                status: GroupStatus::Scored,
                findings: findings.len(),
                skip_reason: None,
            }],
            findings,
        }
    }

    #[test]
    fn markdown_has_headline_table_and_details() {
        let f = Finding {
            rule: "foreign-import".into(),
            rule_label: "unfamiliar import".into(),
            group: "voice",
            confidence: "foreign".into(),
            severity: "error".into(),
            path: "src/x.py".into(),
            line_start: 1,
            line_end: 2,
            evidence: Some("↳ requests — 0 of 74".into()),
            commit: FindingCommit {
                sha: Some("c".repeat(40)),
                short: Some("ccccccc".into()),
                subject: Some("add client".into()),
                attribution: super::super::attribution::Attribution::AiAssisted,
                markers: vec!["Co-Authored-By: Claude <noreply@anthropic.com>".into()],
            },
        };
        let md = render(&report(vec![f.clone(), f]));
        assert!(md.contains("## argot audit"));
        assert!(md.contains("since 6m (120 commits)"));
        assert!(md.contains("**33%** carry AI markers (100 of 300)"));
        assert!(md.contains("| voice | 2 |"));
        assert!(md.contains("**Worst offender**"));
        assert!(md.contains("<details>"));
        assert!(md.contains("`ccccccc` (ai-assisted)"));
        assert!(md.contains("argot init"));
        assert!(!md.contains('\x1b'));
    }

    #[test]
    fn quiet_markdown_is_positive() {
        let md = render(&report(vec![]));
        assert!(md.contains("**0 findings** in 500 hunks"));
        assert!(md.contains("quiet audit is a bounded result"));
        assert!(md.contains(METHOD_NOTE));
    }
}
