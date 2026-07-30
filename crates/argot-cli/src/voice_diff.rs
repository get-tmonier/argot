//! `argot voice-diff <target>` — an observed-findings summary with selected
//! locations. Pure aggregation over the per-hunk scores `check`
//! already produces (consumed via its stable `--format json` contract), so it
//! adds no new modeling.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;
use serde_json::Value;

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;

/// One scored, above-threshold hunk reduced to what the aggregator needs.
pub struct HitScore {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub score: f64,
    pub confidence: String,
    pub severity: String,
    pub rule: String,
    /// Rule-owned explanation lines from the full `argot check` result.
    pub evidence: Vec<String>,
    /// Content-based hit hash — `argot mute <hash>` accepts the hunk.
    pub hash: String,
}

/// Default hot-spots shown.
pub const DEFAULT_TOP: usize = 10;

#[derive(Serialize, Clone)]
pub struct HotSpot {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub score: f64,
    pub confidence: String,
    pub severity: String,
    pub rule: String,
    /// The evidence that explains why this specific location was selected.
    pub evidence: Vec<String>,
    /// Content-based hit hash — `argot mute <hash>` accepts the hunk.
    pub hash: String,
}

#[derive(Serialize)]
pub struct VoiceDiffSummary {
    pub scanned_hunks: usize,
    pub configured_findings: usize,
    pub findings_by_severity: BTreeMap<String, usize>,
    pub findings_by_rule: BTreeMap<String, usize>,
    /// Highest-scoring selected locations first.
    pub locations: Vec<HotSpot>,
}

/// Aggregate scored hits into the PR-level summary. Pure: the CLI just feeds it
/// what `check` produced.
pub fn summarize(hits: &[HitScore], hunks_total: usize, top_n: usize) -> VoiceDiffSummary {
    let mut findings_by_severity = BTreeMap::new();
    let mut findings_by_rule = BTreeMap::new();
    for hit in hits {
        *findings_by_severity
            .entry(hit.severity.clone())
            .or_insert(0) += 1;
        *findings_by_rule.entry(hit.rule.clone()).or_insert(0) += 1;
    }
    let mut sorted: Vec<&HitScore> = hits.iter().collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line_start.cmp(&b.line_start))
    });
    let hot_spots = sorted
        .into_iter()
        .take(top_n)
        .map(|h| HotSpot {
            file: h.file.clone(),
            line_start: h.line_start,
            line_end: h.line_end,
            score: h.score,
            confidence: h.confidence.clone(),
            severity: h.severity.clone(),
            rule: h.rule.clone(),
            evidence: h.evidence.clone(),
            hash: h.hash.clone(),
        })
        .collect();
    VoiceDiffSummary {
        scanned_hunks: hunks_total,
        configured_findings: hits.len(),
        findings_by_severity,
        findings_by_rule,
        locations: hot_spots,
    }
}

/// Compute the summary for a ref/range by running `check` in JSON mode and
/// aggregating its hits. `None` when the model can't be loaded (check errored).
pub fn summary_for_ref(repo: &Path, reference: &str, top_n: usize) -> Option<VoiceDiffSummary> {
    summary_for_ref_with_snapshot(repo, &repo.join(".argot"), reference, top_n)
}

/// As [`summary_for_ref`], but against an explicit fitted snapshot.  The
/// Action uses the base branch's extracted snapshot so a PR cannot influence
/// either the check or the scorecard by editing `.argot/`.
pub fn summary_for_ref_with_snapshot(
    repo: &Path,
    argot_dir: &Path,
    reference: &str,
    top_n: usize,
) -> Option<VoiceDiffSummary> {
    let outcome = run_check(CheckArgs {
        repo_path: repo.to_string_lossy().into_owned(),
        reference: reference.to_string(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: argot_dir.to_path_buf(),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_confidence: "unusual".to_string(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: OutputFormat::Json,
        today: crate::today_utc(),
    });
    if outcome.exit_code >= 2 {
        return None;
    }
    let doc: Value = serde_json::from_str(&outcome.stdout).ok()?;
    let hunks_total = doc
        .get("hunks_scanned")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let hits: Vec<HitScore> = doc
        .get("hits")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|h| HitScore {
                    file: h
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    line_start: h.get("line_start").and_then(Value::as_u64).unwrap_or(0) as usize,
                    line_end: h.get("line_end").and_then(Value::as_u64).unwrap_or(0) as usize,
                    score: h.get("score").and_then(Value::as_f64).unwrap_or(0.0),
                    confidence: h
                        .get("confidence")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    severity: h
                        .get("severity")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    rule: h
                        .get("rule")
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                        .to_string(),
                    evidence: h
                        .get("evidence")
                        .and_then(Value::as_array)
                        .map(|lines| {
                            lines
                                .iter()
                                .filter_map(Value::as_str)
                                .map(str::to_owned)
                                .collect()
                        })
                        .unwrap_or_default(),
                    hash: h
                        .get("hash")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(summarize(&hits, hunks_total, top_n))
}

/// One-line headline for the human render + `argot review` header.
pub fn one_liner(s: &VoiceDiffSummary) -> String {
    format!(
        "{} pattern{} worth reviewing · {} scanned hunks",
        s.configured_findings,
        if s.configured_findings == 1 { "" } else { "s" },
        s.scanned_hunks,
    )
}

/// The concrete review decision a finding asks the author to make. This is
/// deliberately rule-based: confidence is evidence strength, not a remedy.
fn review_action(rule: &str) -> &'static str {
    match rule {
        "foreign-import" => {
            "Compare the named dependency with the familiar imports above; use the established option unless this adoption is deliberate."
        }
        "unfamiliar-callee" => {
            "Compare this call with the common callees above; use the repository's established API if it serves the same purpose."
        }
        "rare-tokens" | "convention" => {
            "Read the highlighted vocabulary and rewrite it in the repository's established form if the difference is unintended."
        }
        "superseded" => {
            "Follow the replacement named in the evidence; this is advisory unless the repository configured it to gate."
        }
        "redundant" => {
            "Open the cited duplicate and reuse it, or explain the material difference that requires a separate implementation."
        }
        "misplaced" => {
            "Move the code to the named home area, or explain why this location is the intentional exception."
        }
        "layering" => {
            "Route through the intended layer or invert the dependency; do not introduce the reversed import by accident."
        }
        "test-deleted" | "test-disabled" | "test-weakened" => {
            "Restore the test strength, or explain why the production behavior and its test legitimately changed together."
        }
        "rule-tampered" => "Restore the locked rule or its severity. This finding cannot be muted.",
        _ => "Read the evidence and the rule's repository policy before deciding whether this is an intentional exception.",
    }
}

/// A GitHub-flavoured observed-findings card for a PR comment or Actions job
/// summary. It keeps the full check's evidence and next decision close to each
/// finding, rather than making reviewers reconstruct them from a count table.
pub fn markdown_card(s: &VoiceDiffSummary) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "### 🎙️ argot review\n");

    if s.configured_findings == 0 {
        let _ = writeln!(
            out,
            "> 🟢 **No patterns requiring review** across {} scanned hunks.\n",
            s.scanned_hunks
        );
        let _ = writeln!(out, "<sub>Non-blocking by default: this scan found no configured foreignness signal. It does not prove the change is correct or fully idiomatic.</sub>");
        return out;
    }

    let review_glyph = if s.findings_by_severity.contains_key("error") {
        "🔴"
    } else {
        "🟡"
    };
    let _ = writeln!(
        out,
        "> {review_glyph} **{} review decision{}** across {} scanned hunk{}.\n",
        s.configured_findings,
        if s.configured_findings == 1 { "" } else { "s" },
        s.scanned_hunks,
        if s.scanned_hunks == 1 { "" } else { "s" },
    );
    let _ = writeln!(
        out,
        "> **Start here:** open each row in the review queue. Argot is advisory — not a merge gate.\n"
    );
    let severity_counts = s
        .findings_by_severity
        .iter()
        .map(|(severity, count)| {
            let glyph = if severity == "error" { "🔴" } else { "🟡" };
            format!("{glyph} {count} {severity}")
        })
        .collect::<Vec<_>>()
        .join(", ");
    let rule_counts = s
        .findings_by_rule
        .iter()
        .map(|(rule, count)| format!("{count} {rule}"))
        .collect::<Vec<_>>()
        .join(", ");
    let _ = writeln!(out, "### 🔎 Review queue\n");
    let _ = writeln!(out, "**Signals:** {severity_counts} · {rule_counts}\n");

    for (index, h) in s.locations.iter().enumerate() {
        let glyph = match h.severity.as_str() {
            "error" => "🔴",
            _ => "🟡",
        };
        let loc = if h.line_start == h.line_end {
            format!("{}:{}", h.file, h.line_start)
        } else {
            format!("{}:{}-{}", h.file, h.line_start, h.line_end)
        };
        let label = h.rule.replace('-', " ");
        let _ = writeln!(out, "<details>");
        let _ = writeln!(
            out,
            "<summary><strong>{}. {glyph} {label}</strong> · <code>{loc}</code> · {} severity</summary>\n",
            index + 1,
            h.severity,
        );
        let _ = writeln!(out, "**Evidence signal:** `{}`\n", h.confidence);
        if h.evidence.is_empty() {
            let _ = writeln!(
                out,
                "**Evidence:** no additional rule evidence was available.\n"
            );
        } else {
            let _ = writeln!(out, "**Evidence**");
            for line in &h.evidence {
                let _ = writeln!(out, "> {line}");
            }
            let _ = writeln!(out);
        }
        let _ = writeln!(out, "**Review:** {}", review_action(&h.rule));
        if h.rule == "rule-tampered" {
            let _ = writeln!(out);
        } else if h.hash.is_empty() {
            let _ = writeln!(
                out,
                "\n**If intentional:** explain the exception in the PR or commit.\n"
            );
        } else {
            let _ = writeln!(
                out,
                "\n**If intentional:** `argot mute {} --reason \"why this is on purpose\"`\n",
                h.hash
            );
        }
        let _ = writeln!(out, "</details>\n");
    }
    let _ = writeln!(out, "> 💬 Argot is probabilistic: findings are prompts to review, not proof of defects. · [What this means](https://argot.tmonier.com/docs/reading-the-output/)");
    out
}

/// A [shields.io endpoint](https://shields.io/badges/endpoint-badge) JSON
/// document. Published by CI to a stable URL, it lets a README badge stay fresh
/// without argot hosting anything: `img.shields.io/endpoint?url=<the JSON>`
/// renders the current configured-finding count. `schemaVersion` 1 is shields'
/// stable contract.
pub fn shields_endpoint(s: &VoiceDiffSummary) -> String {
    let doc = serde_json::json!({
        "schemaVersion": 1,
        "label": "argot",
        "message": format!("{} findings", s.configured_findings),
        "color": "blue",
    });
    format!("{doc}\n")
}

/// A self-contained flat SVG badge — zero external requests (same ethos as the
/// audit HTML card), for a static badge committed to a repo or embedded in
/// docs. The shields endpoint is the fresher path; this is the offline one.
pub fn badge_svg(s: &VoiceDiffSummary) -> String {
    let label = "argot";
    let message = format!("{} findings", s.configured_findings);
    let hex = "#007ec6";
    let escape = |value: &str| {
        value
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;")
            .replace('"', "&quot;")
    };
    // Approximate Verdana-11 advance width; the shields endpoint renders
    // pixel-perfect, so a rough width here only affects the offline SVG.
    let char_w = 7u32;
    let pad = 12u32; // 6px each side
    let tw = |t: &str| t.chars().count() as u32 * char_w;
    let (ltw, mtw) = (tw(label), tw(&message));
    let (lw, mw) = (ltw + pad, mtw + pad);
    let total = lw + mw;
    // ×10 coordinates for the scale(.1) trick shields uses for crisp text.
    let lcx = lw * 5;
    let mcx = lw * 10 + mw * 5;
    let (ltl, mtl) = (ltw * 10, mtw * 10);
    let label_e = escape(label);
    let message_e = escape(&message);
    let aria = format!("{label_e}: {message_e}");
    format!(
        r##"<svg xmlns="http://www.w3.org/2000/svg" width="{total}" height="20" role="img" aria-label="{aria}">
<title>{aria}</title>
<linearGradient id="s" x2="0" y2="100%"><stop offset="0" stop-color="#bbb" stop-opacity=".1"/><stop offset="1" stop-opacity=".1"/></linearGradient>
<clipPath id="r"><rect width="{total}" height="20" rx="3" fill="#fff"/></clipPath>
<g clip-path="url(#r)"><rect width="{lw}" height="20" fill="#555"/><rect x="{lw}" width="{mw}" height="20" fill="{hex}"/><rect width="{total}" height="20" fill="url(#s)"/></g>
<g fill="#fff" text-anchor="middle" font-family="Verdana,Geneva,DejaVu Sans,sans-serif" font-size="110" text-rendering="geometricPrecision">
<text aria-hidden="true" x="{lcx}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{ltl}">{label_e}</text>
<text x="{lcx}" y="140" transform="scale(.1)" textLength="{ltl}">{label_e}</text>
<text aria-hidden="true" x="{mcx}" y="150" fill="#010101" fill-opacity=".3" transform="scale(.1)" textLength="{mtl}">{message_e}</text>
<text x="{mcx}" y="140" transform="scale(.1)" textLength="{mtl}">{message_e}</text></g>
</svg>
"##
    )
}

pub fn run_voice_diff(
    target: &str,
    repo: PathBuf,
    argot_dir: PathBuf,
    format: &str,
    top_n: usize,
) -> ExitCode {
    let Some(summary) = summary_for_ref_with_snapshot(&repo, &argot_dir, target, top_n) else {
        eprintln!("error: could not score '{target}' — run `argot fit` first?");
        return ExitCode::from(2);
    };
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }
    if format == "markdown" {
        print!("{}", markdown_card(&summary));
        return ExitCode::SUCCESS;
    }
    if format == "shields" {
        print!("{}", shields_endpoint(&summary));
        return ExitCode::SUCCESS;
    }
    if format == "svg" {
        print!("{}", badge_svg(&summary));
        return ExitCode::SUCCESS;
    }
    println!("{}", one_liner(&summary));
    if summary.locations.is_empty() {
        println!("  no patterns requiring review.");
    } else {
        println!("  selected locations:");
        for h in &summary.locations {
            let loc = if h.line_start == h.line_end {
                format!("L{}", h.line_start)
            } else {
                format!("L{}-L{}", h.line_start, h.line_end)
            };
            println!("    {:<20} {:<10}  {}:{}", h.rule, h.severity, h.file, loc);
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(file: &str, line: usize, score: f64, confidence: &str) -> HitScore {
        HitScore {
            file: file.to_string(),
            line_start: line,
            line_end: line,
            score,
            confidence: confidence.to_string(),
            severity: "error".to_string(),
            rule: "foreign-import".to_string(),
            evidence: vec![
                "↳ axios — 0 of 47 module specifiers in repo".to_string(),
                "common here: fetch (12×), request (8×)".to_string(),
            ],
            hash: "deadbeef".to_string(),
        }
    }

    #[test]
    fn clean_summary_reports_no_configured_findings() {
        let s = summarize(&[], 12, 10);
        assert_eq!(s.scanned_hunks, 12);
        assert_eq!(s.configured_findings, 0);
        assert!(s.findings_by_severity.is_empty());
        assert!(s.locations.is_empty());
    }

    #[test]
    fn findings_summary_has_no_percentage_fields() {
        let s = summarize(&[hit("a.py", 1, 9.0, "foreign")], 1, 10);
        let json = serde_json::to_value(s).unwrap();
        assert!(json.get("out_of_voice_pct").is_none());
        assert!(json.get("hunks_total").is_none());
        assert!(json.get("hunks_flagged").is_none());
        assert!(json.get("max_confidence").is_none());
        assert_eq!(json["configured_findings"], 1);
    }

    #[test]
    fn markdown_card_keeps_evidence_and_a_rule_aware_next_step() {
        let hits = vec![hit("src/http.ts", 42, 8.2, "foreign")];
        let card = markdown_card(&summarize(&hits, 40, 10));
        assert!(card.contains("🔴 **1 review decision** across 40 scanned hunks"));
        assert!(card.contains("### 🔎 Review queue"));
        assert!(card.contains("**Signals:** 🔴 1 error · 1 foreign-import"));
        assert!(card.contains("<details>"));
        assert!(card.contains("<summary><strong>1. 🔴 foreign import</strong> · <code>src/http.ts:42</code> · error severity</summary>"));
        assert!(card.contains("↳ axios — 0 of 47 module specifiers in repo"));
        assert!(card.contains("Compare the named dependency"));
        assert!(
            card.contains("Argot is advisory — not a merge gate"),
            "framed informational"
        );
        assert!(
            card.contains("argot mute deadbeef"),
            "offers the accept command"
        );
        assert!(!card.contains('%'), "never presents a percentage");
    }

    #[test]
    fn markdown_card_preserves_warn_and_error_counts() {
        let mut warning = hit("src/style.rs", 9, 4.0, "unusual");
        warning.severity = "warn".to_string();
        warning.rule = "convention".to_string();
        let error = hit("src/http.rs", 42, 8.2, "foreign");
        let card = markdown_card(&summarize(&[warning, error], 40, 10));
        assert!(
            card.contains("**Signals:** 🔴 1 error, 🟡 1 warn · 1 convention, 1 foreign-import")
        );
    }

    #[test]
    fn markdown_card_clean_diff_uses_the_bounded_clean_claim() {
        let card = markdown_card(&summarize(&[], 30, 10));
        assert!(card.contains("🟢 **No patterns requiring review** across 30 scanned hunks."));
        assert!(card.contains("Non-blocking by default"));
        assert!(!card.contains("in-voice"));
    }

    #[test]
    fn locked_rule_tampering_never_offers_a_mute() {
        let mut tampered = hit("argot.toml", 4, 1.0, "suspicious");
        tampered.rule = "rule-tampered".to_string();
        tampered.hash = "lockedhash".to_string();
        let card = markdown_card(&summarize(&[tampered], 1, 10));
        assert!(card.contains("Restore the locked rule or its severity"));
        assert!(!card.contains("argot mute lockedhash"));
    }

    #[test]
    fn hot_spots_are_ranked_by_score_and_capped() {
        let hits = vec![
            hit("a.py", 1, 5.0, "unusual"),
            hit("b.py", 2, 9.0, "foreign"),
            hit("c.py", 3, 7.0, "suspicious"),
        ];
        let s = summarize(&hits, 20, 2);
        assert_eq!(s.locations.len(), 2);
        assert_eq!(s.locations[0].file, "b.py"); // highest score first
        assert_eq!(s.locations[1].file, "c.py");
    }

    #[test]
    fn shields_endpoint_is_a_neutral_findings_count() {
        let json = shields_endpoint(&summarize(&[hit("a.py", 1, 6.0, "unusual")], 100, 10));
        let doc: Value = serde_json::from_str(&json).unwrap();
        assert_eq!(doc["schemaVersion"], 1);
        assert_eq!(doc["label"], "argot");
        let msg = doc["message"].as_str().unwrap();
        assert_eq!(msg, "1 findings");
        assert_eq!(doc["color"], "blue");
    }

    #[test]
    fn shields_endpoint_clean_diff_is_zero_findings() {
        let doc: Value = serde_json::from_str(&shields_endpoint(&summarize(&[], 30, 10))).unwrap();
        assert_eq!(doc["message"], "0 findings");
        assert_eq!(doc["color"], "blue");
    }

    #[test]
    fn badge_svg_is_self_contained() {
        let svg = badge_svg(&summarize(&[hit("a.py", 1, 6.0, "unusual")], 100, 10));
        assert!(svg.starts_with("<svg"));
        assert!(svg.trim_end().ends_with("</svg>"));
        assert!(svg.contains("1 findings"));
        assert!(svg.contains("argot"));
        // No external resource references (internal url(#…) refs are fine).
        for banned in ["href=", "src=", "<image", "url(http", "@import"] {
            assert!(!svg.contains(banned), "unexpected external ref: {banned}");
        }
    }
}
