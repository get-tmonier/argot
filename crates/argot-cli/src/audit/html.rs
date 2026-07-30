//! HTML card — one self-contained file (inline CSS, zero external
//! requests, no JS), light + dark via `prefers-color-scheme`,
//! screenshot-ready: `argot audit --format html > audit.html`.

use super::attribution::Attribution;
use super::report::{
    AuditReport, Finding, GroupStatus, RequestedWindow, CI_GUIDE_URL, METHOD_NOTE,
};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn span(f: &Finding) -> String {
    if f.line_start == f.line_end {
        format!("L{}", f.line_start)
    } else {
        format!("L{}-{}", f.line_start, f.line_end)
    }
}

fn attribution_chip(a: Attribution) -> String {
    let class = match a {
        Attribution::AiAssisted => "ai",
        Attribution::Human => "hum",
        Attribution::Unknown => "unk",
    };
    format!("<span class=\"chip {class}\">{}</span>", a.as_str())
}

const STYLE: &str = r#"
  :root {
    --bg: #f5f5f4; --card: #ffffff; --ink: #1c1917; --muted: #78716c;
    --line: #e7e5e4; --accent: #0f766e; --bad: #b91c1c; --warn: #b45309;
    --chip-ai: #ecfeff; --chip-ai-ink: #155e75;
    --chip-hum: #f5f5f4; --chip-hum-ink: #57534e;
    --chip-unk: #fef3c7; --chip-unk-ink: #92400e;
    --mono: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  }
  @media (prefers-color-scheme: dark) {
    :root {
      --bg: #171412; --card: #211d1a; --ink: #e7e5e4; --muted: #a8a29e;
      --line: #37322e; --accent: #2dd4bf; --bad: #f87171; --warn: #fbbf24;
      --chip-ai: #164e63; --chip-ai-ink: #cffafe;
      --chip-hum: #292524; --chip-hum-ink: #d6d3d1;
      --chip-unk: #451a03; --chip-unk-ink: #fde68a;
    }
  }
  * { box-sizing: border-box; margin: 0; }
  body {
    background: var(--bg); color: var(--ink);
    font: 15px/1.55 ui-sans-serif, system-ui, -apple-system, "Segoe UI", sans-serif;
    padding: 40px 16px; display: flex; justify-content: center;
  }
  .card {
    background: var(--card); border: 1px solid var(--line); border-radius: 12px;
    max-width: 720px; width: 100%; padding: 28px 32px;
  }
  h1 { font-size: 18px; letter-spacing: .01em; }
  h1 span { color: var(--muted); font-weight: 400; }
  .meta { color: var(--muted); font-size: 13px; margin-top: 2px; }
  .stats { display: flex; gap: 28px; margin: 20px 0 4px; flex-wrap: wrap; }
  .stat b { display: block; font-size: 26px; line-height: 1.2; }
  .stat span { color: var(--muted); font-size: 12.5px; }
  .note { color: var(--muted); font-size: 13px; margin-top: 10px; font-style: italic; }
  .groups { margin: 18px 0 0; border-top: 1px solid var(--line); padding-top: 14px; }
  .group { display: flex; gap: 10px; font-size: 14px; padding: 3px 0; }
  .group b { min-width: 110px; font-weight: 600; }
  .group .n { min-width: 26px; text-align: right; font-variant-numeric: tabular-nums; }
  .group .why { color: var(--muted); }
  .worst { margin-top: 20px; border: 1px solid var(--line); border-left: 3px solid var(--bad);
           border-radius: 8px; padding: 12px 16px; }
  .worst h2, .rest h2 { font-size: 12px; text-transform: uppercase; letter-spacing: .08em;
                        color: var(--muted); margin-bottom: 8px; }
  .loc { font-family: var(--mono); font-size: 13.5px; }
  .rule { color: var(--accent); font-weight: 600; }
  .subject { color: var(--muted); font-size: 13.5px; margin-top: 4px; }
  .evidence { font-family: var(--mono); font-size: 12.5px; color: var(--muted); margin-top: 6px;
              white-space: pre-wrap; overflow-wrap: anywhere; }
  .chip { border-radius: 999px; padding: 1px 9px; font-size: 11.5px; font-weight: 600;
          vertical-align: 1px; white-space: nowrap; }
  .chip.ai { background: var(--chip-ai); color: var(--chip-ai-ink); }
  .chip.hum { background: var(--chip-hum); color: var(--chip-hum-ink); }
  .chip.unk { background: var(--chip-unk); color: var(--chip-unk-ink); }
  .rest { margin-top: 20px; }
  .rest .tablewrap { overflow-x: auto; }
  table { border-collapse: collapse; width: 100%; font-size: 13px; }
  td, th { text-align: left; padding: 5px 10px 5px 0; border-top: 1px solid var(--line);
           vertical-align: top; }
  th { border-top: none; color: var(--muted); font-weight: 500; font-size: 12px; }
  td.mono { font-family: var(--mono); font-size: 12.5px; }
  .quiet { margin-top: 20px; border: 1px solid var(--line); border-left: 3px solid var(--accent);
           border-radius: 8px; padding: 14px 16px; font-size: 14px; }
  .frame { color: var(--muted); font-size: 12.5px; margin-top: 22px; border-top: 1px solid var(--line);
           padding-top: 12px; }
  .frame code { font-family: var(--mono); font-size: 12px; }
  .brand { color: var(--muted); font-size: 12px; margin-top: 16px; text-align: center;
           letter-spacing: .02em; }
  .brand b { color: var(--accent); font-weight: 600; }
"#;

pub fn render(report: &AuditReport) -> String {
    let w = &report.window;
    let c = &report.commits;
    let share = super::report::ai_share_pct(c.ai_assisted, c.total);
    // Open-graph / Twitter meta so a shared link previews with the hook. No
    // og:image — the card stays self-contained; the screenshot is the image.
    let social = match super::report::share_caption(report) {
        Some(cap) => format!(
            "<meta property=\"og:type\" content=\"website\">\n\
             <meta property=\"og:title\" content=\"argot audit\">\n\
             <meta property=\"og:description\" content=\"{cap}\">\n\
             <meta property=\"og:url\" content=\"https://argot.tmonier.com\">\n\
             <meta name=\"twitter:card\" content=\"summary\">\n\
             <meta name=\"twitter:title\" content=\"argot audit\">\n\
             <meta name=\"twitter:description\" content=\"{cap}\">\n",
            cap = esc(&cap)
        ),
        None => String::new(),
    };
    let window_label = match &w.requested {
        RequestedWindow::Commits(_) => format!("last {} commits", w.effective_commits),
        RequestedWindow::Since(s) => format!("since {} ({} commits)", esc(s), w.effective_commits),
    };

    let mut body = String::new();
    body.push_str(&format!(
        "<h1>argot audit <span>· {window_label}</span></h1>\n\
         <p class=\"meta\">{} → {} · generated by {}</p>\n",
        w.base_date,
        w.head_date,
        esc(&report.generated_by)
    ));

    let findings_stat = if report.hunks_scanned == 0 {
        "<b>—</b><span>no supported source changed</span>".to_string()
    } else {
        format!(
            "<b>{}</b><span>finding(s) argot would have raised</span>",
            report.findings.len()
        )
    };
    body.push_str(&format!(
        "<div class=\"stats\">\n\
         <div class=\"stat\"><b>{}</b><span>commits audited</span></div>\n\
         <div class=\"stat\"><b>{share}%</b><span>carry AI markers ({} of {})</span></div>\n\
         <div class=\"stat\">{findings_stat}</div>\n\
         </div>\n",
        c.total, c.ai_assisted, c.total
    ));
    if let Some(note) = &w.clamp_note {
        body.push_str(&format!("<p class=\"note\">note: {}</p>\n", esc(note)));
    }

    let group_rows: Vec<String> = report
        .groups
        .iter()
        .filter_map(|g| match g.status {
            GroupStatus::Scored if g.findings > 0 => Some(format!(
                "<div class=\"group\"><b>{}</b><span class=\"n\">{}</span>\
                 <span class=\"why\">{}</span></div>",
                g.group,
                g.findings,
                group_blurb(g.group)
            )),
            GroupStatus::Skipped => Some(format!(
                "<div class=\"group\"><b>{}</b><span class=\"n\">—</span>\
                 <span class=\"why\">skipped: {}</span></div>",
                g.group,
                esc(g.skip_reason.as_deref().unwrap_or(""))
            )),
            _ => None,
        })
        .collect();
    if !group_rows.is_empty() {
        body.push_str("<div class=\"groups\">\n");
        for r in group_rows {
            body.push_str(&r);
            body.push('\n');
        }
        body.push_str("</div>\n");
    }

    if report.hunks_scanned == 0 {
        body.push_str(&format!(
            "<div class=\"quiet\">These {} commit(s) touched no supported source files \
             (docs-only?).</div>\n",
            w.effective_commits
        ));
    } else if report.findings.is_empty() {
        body.push_str(&format!(
            "<div class=\"quiet\">Nothing argot would have raised — {} hunks audited. \
             A quiet audit is a bounded result, not proof that every change is ready to accept.</div>\n",
            report.hunks_scanned
        ));
    } else {
        let worst = &report.findings[0];
        body.push_str("<div class=\"worst\">\n<h2>Worst offender</h2>\n");
        body.push_str(&format!(
            "<div class=\"loc\">{}:{} · <span class=\"rule\">{}</span> · {} {}</div>\n",
            esc(&worst.path),
            span(worst),
            esc(&worst.rule),
            worst
                .commit
                .short
                .as_deref()
                .map(|s| format!("commit {}", esc(s)))
                .unwrap_or_default(),
            attribution_chip(worst.commit.attribution)
        ));
        if let Some(subject) = &worst.commit.subject {
            let marker = worst
                .commit
                .markers
                .first()
                .map(|m| format!(" — <code>{}</code>", esc(m)))
                .unwrap_or_else(|| " — no AI markers".to_string());
            body.push_str(&format!(
                "<div class=\"subject\">&ldquo;{}&rdquo;{marker}</div>\n",
                esc(subject)
            ));
        }
        if let Some(ev) = &worst.evidence {
            body.push_str(&format!("<div class=\"evidence\">{}</div>\n", esc(ev)));
        }
        body.push_str("</div>\n");

        if report.findings.len() > 1 {
            body.push_str(
                "<div class=\"rest\">\n<h2>All findings</h2>\n<div class=\"tablewrap\">\n\
                 <table>\n<tr><th>file</th><th>rule</th><th>commit</th><th>evidence</th></tr>\n",
            );
            for f in &report.findings {
                let commit = match &f.commit.short {
                    Some(s) => format!("{} {}", esc(s), attribution_chip(f.commit.attribution)),
                    None => attribution_chip(f.commit.attribution),
                };
                body.push_str(&format!(
                    "<tr><td class=\"mono\">{}:{}</td><td>{}</td><td class=\"mono\">{commit}</td>\
                     <td class=\"mono\">{}</td></tr>\n",
                    esc(&f.path),
                    span(f),
                    esc(&f.rule),
                    esc(f.evidence.as_deref().unwrap_or(""))
                ));
            }
            body.push_str("</table>\n</div>\n</div>\n");
        }
    }

    body.push_str(&format!(
        "<p class=\"frame\">Merged code is accepted code — read each finding as \
         &ldquo;would have prompted review before merge&rdquo;, not a bug list.<br>{method_note}<br>Next: <code>argot init</code> fits today's voice so \
         <code>argot check</code> raises these before they merge.<br>Then choose a recurring path you configure: pre-commit runs automatically at commit time once configured; the GitHub Action runs automatically in CI once configured. <a href=\"{ci_guide_url}\">CI and pre-commit guide</a>.</p>\n",
        method_note = esc(METHOD_NOTE),
        ci_guide_url = CI_GUIDE_URL,
    ));
    body.push_str(
        "<p class=\"brand\"><b>argot</b> · repository-grounded checks from git \
         history · argot.tmonier.com</p>\n",
    );

    format!(
        "<!doctype html>\n<html lang=\"en\">\n<head>\n<meta charset=\"utf-8\">\n\
         <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
         {social}<title>argot audit</title>\n<style>{STYLE}</style>\n</head>\n<body>\n\
         <div class=\"card\">\n{body}</div>\n</body>\n</html>\n"
    )
}

fn group_blurb(group: &str) -> &'static str {
    match group {
        "voice" => "code foreign to how this repo writes",
        "semantic" => "functions you already had, or code filed oddly",
        "architecture" => "imports that break the repo's layering",
        "integrity" => "tests deleted, disabled, or weakened",
        _ => "",
    }
}

#[cfg(test)]
mod tests {
    use super::super::report::*;
    use super::*;

    #[test]
    fn html_is_self_contained_and_escaped() {
        let f = Finding {
            rule: "foreign-import".into(),
            rule_label: "unfamiliar import".into(),
            group: "voice",
            confidence: "foreign".into(),
            severity: "error".into(),
            path: "src/<odd>&name.py".into(),
            line_start: 1,
            line_end: 2,
            evidence: Some("↳ requests <0 of 74>".into()),
            commit: FindingCommit {
                sha: Some("c".repeat(40)),
                short: Some("ccccccc".into()),
                subject: Some("add \"client\" & more".into()),
                attribution: super::super::attribution::Attribution::AiAssisted,
                markers: vec!["Co-Authored-By: Claude <noreply@anthropic.com>".into()],
            },
        };
        let report = AuditReport {
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
                total: 100,
                ai_assisted: 40,
                human: 60,
                unknown: 0,
            },
            hunks_scanned: 200,
            over_firing: Vec::new(),
            groups: vec![GroupReport {
                group: "semantic",
                status: GroupStatus::Skipped,
                findings: 0,
                skip_reason: Some("no semantic index was built at fit".into()),
            }],
            findings: vec![f],
        };
        let html = render(&report);
        assert!(html.starts_with("<!doctype html>"));
        // Self-contained: no external fetches; the CTA is a user-followed link.
        for banned in ["http-equiv", "src=\"http", "@import", "url("] {
            assert!(!html.contains(banned), "{banned}");
        }
        assert!(html.contains("prefers-color-scheme: dark"));
        // User content is escaped.
        assert!(html.contains("src/&lt;odd&gt;&amp;name.py"));
        assert!(html.contains("Claude &lt;noreply@anthropic.com&gt;"));
        assert!(html.contains("40%"));
        assert!(html.contains("Worst offender"));
        // Social preview meta + brand footer, still self-contained (no og:image).
        assert!(html.contains("og:description"));
        assert!(html.contains("class=\"brand\""));
        assert!(html.contains("findings are patterns that survive the audited base-to-head change"));
        assert!(html.contains("no marker was found"));
        assert!(html.contains("pre-commit runs automatically at commit time once configured"));
        assert!(html.contains("GitHub Action runs automatically in CI once configured"));
        assert!(html.contains(CI_GUIDE_URL));
        assert!(html.contains(&format!("href=\"{CI_GUIDE_URL}\"")));
        assert!(html.contains("no semantic index was built at fit"));
        assert!(!html.contains("og:image"));
    }
}
