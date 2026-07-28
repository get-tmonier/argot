//! The audit report — one data model, four renderers.
//!
//! Everything the card shows lives here, computed once; the terminal,
//! markdown, and HTML renderers are pure functions of this struct and the
//! JSON format is its serde serialization (`schema_version` 1, stable: add
//! fields, never rename or repurpose).

use serde::Serialize;

use super::attribution::Attribution;
use super::window::WindowSpec;

/// How the audited window was chosen and what it resolved to.
#[derive(Debug, Clone, Serialize)]
pub struct WindowReport {
    /// What the user asked for: `{"commits": 50}` or `{"since": "6m"}`.
    pub requested: RequestedWindow,
    /// First-parent steps from HEAD to the base actually audited.
    pub effective_commits: usize,
    /// Why `effective_commits` differs from the request (`null` = it doesn't).
    pub clamp: Option<&'static str>,
    /// Human sentence for the clamp, also printed on the card.
    pub clamp_note: Option<String>,
    pub base: String,
    pub head: String,
    /// Commit dates (`YYYY-MM-DD`, UTC) of base and head.
    pub base_date: String,
    pub head_date: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RequestedWindow {
    Commits(usize),
    Since(String),
}

impl RequestedWindow {
    pub fn of(spec: &WindowSpec) -> Self {
        match spec {
            WindowSpec::Commits(n) => RequestedWindow::Commits(*n),
            WindowSpec::Since(s) => RequestedWindow::Since(s.clone()),
        }
    }
}

/// Attribution split over every commit in `base..head` (all parents, not
/// just the first-parent line — merged branch commits carry trailers too).
#[derive(Debug, Clone, Copy, Default, Serialize)]
pub struct CommitsReport {
    pub total: usize,
    pub ai_assisted: usize,
    pub human: usize,
    pub unknown: usize,
}

/// One rule group's outcome on the card.
#[derive(Debug, Clone, Serialize)]
pub struct GroupReport {
    /// Registry group name (`voice` / `semantic` / `architecture` / `integrity`).
    pub group: &'static str,
    /// `scored` or `skipped` — a skipped group is marked, never shown as 0.
    pub status: GroupStatus,
    pub findings: usize,
    /// Present iff `status == skipped`.
    pub skip_reason: Option<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupStatus {
    Scored,
    Skipped,
}

/// The introducing commit of a finding, with its attribution.
#[derive(Debug, Clone, Serialize)]
pub struct FindingCommit {
    /// Full sha, or `null` fields below when unresolvable (`unknown`).
    pub sha: Option<String>,
    pub short: Option<String>,
    pub subject: Option<String>,
    #[serde(serialize_with = "ser_attribution")]
    pub attribution: Attribution,
    /// The concrete markers that matched (empty for human/unknown).
    pub markers: Vec<String>,
}

fn ser_attribution<S: serde::Serializer>(a: &Attribution, s: S) -> Result<S::Ok, S::Error> {
    s.serialize_str(a.as_str())
}

/// One finding, already joined with its group and introducing commit.
/// Sorted worst-first in `AuditReport::findings` — the first element IS the
/// card's worst offender.
#[derive(Debug, Clone, Serialize)]
pub struct Finding {
    pub rule: String,
    pub rule_label: String,
    pub group: &'static str,
    pub confidence: String,
    pub severity: String,
    pub path: String,
    pub line_start: usize,
    pub line_end: usize,
    /// First rendered evidence line from check (when the scorer had one).
    pub evidence: Option<String>,
    pub commit: FindingCommit,
}

/// The whole audit, renderer-agnostic.
#[derive(Debug, Clone, Serialize)]
pub struct AuditReport {
    pub schema_version: u32,
    pub generated_by: String,
    pub window: WindowReport,
    pub commits: CommitsReport,
    pub hunks_scanned: u64,
    pub groups: Vec<GroupReport>,
    pub findings: Vec<Finding>,
    /// Rules firing on more of this repository's own accepted history than a
    /// healthy repository's do — calibration evidence, computed here because
    /// audit is the only surface that has both numerator and denominator.
    /// Empty is the normal case and renders as nothing.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub over_firing: Vec<OverFiringRule>,
}

/// A rule that fires too often on accepted history to be read as a signal.
#[derive(Debug, Clone, Serialize)]
pub struct OverFiringRule {
    pub rule: String,
    pub rule_label: String,
    pub findings: usize,
    /// Share of scanned hunks this rule flagged, as a percentage.
    pub rate_pct: f64,
}

/// Above this share of scanned hunks, a rule is over-firing *for this
/// repository* — its findings have stopped being a signal and started being
/// the background.
///
/// From the benchmark's own temporal-holdout replays, per-rule over-fire on
/// accepted history across the 18 corpora with ≥300 eligible hunks:
///
/// | rule | median | p75 | p90 | max |
/// |---|--:|--:|--:|--:|
/// | `unfamiliar-callee` | 0,43 % | 0,63 % | 4,50 % | 5,62 % |
/// | `foreign-import` | 0,31 % | 0,53 % | 1,06 % | 1,18 % |
/// | `rare-tokens` | 0,28 % | 0,66 % | 1,18 % | 1,69 % |
///
/// A healthy repository sits at 0,3–0,7 % per rule, so 2 % is roughly three
/// times the p75 of every rule and above the p90 of two of the three. It is
/// also the **same ≤2 % bar the RUBRIC already publishes** for over-fire —
/// reusing the project's existing gate beats inventing a second number.
pub const OVER_FIRE_PCT: f64 = 2.0;

/// Rules whose share of scanned hunks exceeds [`OVER_FIRE_PCT`], worst first.
///
/// Reported, never acted on: the fix is a judgment call between scoping the
/// rule to the tree that trips it, softening it to `warn`, and accepting it —
/// and argot does not get to make that call for a repository.
pub fn over_firing_rules(findings: &[Finding], hunks_scanned: u64) -> Vec<OverFiringRule> {
    if hunks_scanned == 0 {
        return Vec::new();
    }
    let mut counts: std::collections::BTreeMap<(&str, &str), usize> = Default::default();
    for f in findings {
        *counts
            .entry((f.rule.as_str(), f.rule_label.as_str()))
            .or_default() += 1;
    }
    let mut out: Vec<OverFiringRule> = counts
        .into_iter()
        .map(|((rule, label), findings)| OverFiringRule {
            rule: rule.to_string(),
            rule_label: label.to_string(),
            findings,
            rate_pct: 100.0 * findings as f64 / hunks_scanned as f64,
        })
        .filter(|r| r.rate_pct > OVER_FIRE_PCT)
        .collect();
    out.sort_by(|a, b| {
        b.rate_pct
            .partial_cmp(&a.rate_pct)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.rule.cmp(&b.rule))
    });
    out
}

pub const SCHEMA_VERSION: u32 = 1;

/// Shared human-renderer boundary: audit scores the net change that survives
/// between its historical base and HEAD, and markers are evidence, not a full
/// authorship census.
pub const METHOD_NOTE: &str = "Method: findings are patterns that survive the audited base-to-head change. AI-marker attribution is a floor, not a census; \"human\" means no marker was found.";

/// Tested, user-configured recurring check options for the audit-to-habit CTA.
pub const CI_GUIDE_URL: &str = "https://argot.tmonier.com/docs/ci/";

impl AuditReport {
    pub fn to_json(&self) -> String {
        let mut s = serde_json::to_string_pretty(self).expect("audit report serializes");
        s.push('\n');
        s
    }

    /// Findings ordered worst-first: confidence tier, then severity, then
    /// stable path/line order so renders are deterministic.
    pub fn sort_findings(findings: &mut [Finding]) {
        findings.sort_by(|a, b| {
            confidence_rank(&b.confidence)
                .cmp(&confidence_rank(&a.confidence))
                .then_with(|| severity_rank(&b.severity).cmp(&severity_rank(&a.severity)))
                .then_with(|| a.path.cmp(&b.path))
                .then_with(|| a.line_start.cmp(&b.line_start))
        });
    }
}

pub fn confidence_rank(c: &str) -> usize {
    match c {
        "foreign" => 2,
        "suspicious" => 1,
        _ => 0,
    }
}

/// Percentage of AI-marked commits, rounded to the nearest integer — the
/// card headline shared by every renderer.
pub fn ai_share_pct(ai_assisted: usize, total: usize) -> usize {
    (ai_assisted * 100 + total / 2)
        .checked_div(total)
        .unwrap_or(0)
}

/// The one-sentence, copy-pasteable caption for sharing an audit result — the
/// provocative hook (count of patterns foreign to the repo + the AI-marker
/// share). `None` when nothing was scored (a docs-only window has nothing to
/// share). Carries no URL; callers append their own.
pub fn share_caption(report: &AuditReport) -> Option<String> {
    if report.hunks_scanned == 0 {
        return None;
    }
    let share = ai_share_pct(report.commits.ai_assisted, report.commits.total);
    let commits = report.window.effective_commits;
    let n = report.findings.len();
    Some(if n == 0 {
        format!(
            "argot audited my last {commits} commits — 0 patterns it would have raised \
             for review ({share}% of commits carried AI markers)."
        )
    } else {
        format!(
            "argot audited my last {commits} commits: {n} pattern{} it would have raised \
             for review, {share}% of commits carried AI markers.",
            if n == 1 { "" } else { "s" }
        )
    })
}

fn severity_rank(s: &str) -> usize {
    match s {
        "error" => 1,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn finding(rule: &str, confidence: &str, severity: &str, path: &str) -> Finding {
        Finding {
            rule: rule.to_string(),
            rule_label: rule.to_string(),
            group: "voice",
            confidence: confidence.to_string(),
            severity: severity.to_string(),
            path: path.to_string(),
            line_start: 1,
            line_end: 4,
            evidence: Some("↳ evidence".to_string()),
            commit: FindingCommit {
                sha: Some("a".repeat(40)),
                short: Some("aaaaaaa".to_string()),
                subject: Some("subject".to_string()),
                attribution: Attribution::Human,
                markers: vec![],
            },
        }
    }

    #[test]
    fn findings_sort_worst_first_deterministically() {
        let mut f = vec![
            finding("a", "unusual", "warn", "z.py"),
            finding("b", "foreign", "error", "m.py"),
            finding("c", "suspicious", "error", "a.py"),
            finding("d", "foreign", "warn", "a.py"),
        ];
        AuditReport::sort_findings(&mut f);
        let order: Vec<&str> = f.iter().map(|x| x.rule.as_str()).collect();
        assert_eq!(order, ["b", "d", "c", "a"]);
    }

    #[test]
    fn json_schema_has_the_stable_top_level_shape() {
        let report = AuditReport {
            schema_version: SCHEMA_VERSION,
            generated_by: "argot v0.0.0".into(),
            window: WindowReport {
                requested: RequestedWindow::Commits(50),
                effective_commits: 50,
                clamp: Some("cap"),
                clamp_note: Some("clamped".into()),
                base: "b".repeat(40),
                head: "h".repeat(40),
                base_date: "2026-05-28".into(),
                head_date: "2026-07-12".into(),
            },
            commits: CommitsReport {
                total: 214,
                ai_assisted: 67,
                human: 147,
                unknown: 0,
            },
            hunks_scanned: 900,
            over_firing: Vec::new(),
            groups: vec![GroupReport {
                group: "voice",
                status: GroupStatus::Scored,
                findings: 1,
                skip_reason: None,
            }],
            findings: vec![finding("foreign-import", "foreign", "error", "x.py")],
        };
        let doc: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();
        assert_eq!(doc["schema_version"], 1);
        assert_eq!(doc["window"]["requested"]["commits"], 50);
        assert_eq!(doc["window"]["clamp"], "cap");
        assert_eq!(doc["commits"]["ai_assisted"], 67);
        assert_eq!(doc["groups"][0]["status"], "scored");
        let f = &doc["findings"][0];
        assert_eq!(f["rule"], "foreign-import");
        assert_eq!(f["commit"]["attribution"], "human");
        assert_eq!(f["commit"]["short"], "aaaaaaa");
    }

    fn report_with(findings: Vec<Finding>, hunks: u64) -> AuditReport {
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
            over_firing: Vec::new(),
            groups: vec![],
            findings,
        }
    }

    #[test]
    fn share_caption_leads_with_the_hook() {
        // ai_share_pct(66, 200) = 33.
        let mut report = report_with(
            vec![finding("foreign-import", "foreign", "error", "x.py")],
            900,
        );
        let cap = share_caption(&report).unwrap();
        assert!(
            cap.contains("1 pattern it would have raised for review"),
            "{cap}"
        );
        assert!(cap.contains("33% of commits carried AI markers"), "{cap}");
        // A quiet audit brags positively rather than going silent.
        report.findings.clear();
        assert!(share_caption(&report)
            .unwrap()
            .contains("0 patterns it would have raised for review"));
        // A docs-only window has nothing to share.
        report.hunks_scanned = 0;
        assert!(share_caption(&report).is_none());
    }

    #[test]
    fn renderer_contract_keeps_method_attribution_and_next_actions_in_sync() {
        let report = report_with(
            vec![finding(
                "foreign-import",
                "foreign",
                "error",
                "src/client.ts",
            )],
            900,
        );
        let terminal = super::super::term::render(&report, false);
        let markdown = super::super::markdown::render(&report);
        let html = super::super::html::render(&report);
        let json: serde_json::Value = serde_json::from_str(&report.to_json()).unwrap();

        for rendered in [&terminal, &markdown, &html] {
            assert!(
                rendered.contains("audited base-to-head change"),
                "{rendered}"
            );
            assert!(
                rendered.contains("attribution is a floor, not a census"),
                "{rendered}"
            );
            assert!(rendered.contains("no marker was found"), "{rendered}");
            assert!(rendered.contains("argot init"), "{rendered}");
            assert!(rendered.contains("pre-commit"), "{rendered}");
            assert!(rendered.contains("GitHub Action"), "{rendered}");
            assert!(rendered.contains(CI_GUIDE_URL), "{rendered}");
        }
        assert_eq!(json["schema_version"], SCHEMA_VERSION);
        assert_eq!(json["findings"][0]["commit"]["attribution"], "human");
        assert!(share_caption(&report)
            .unwrap()
            .contains("would have raised for review"));
    }

    #[test]
    fn transient_change_absent_from_net_diff_has_no_shareable_finding() {
        // A source change added and then removed has no surviving base-to-head
        // hunk. Every renderer must present that net-diff outcome consistently.
        let report = report_with(vec![], 0);
        let terminal = super::super::term::render(&report, false);
        let markdown = super::super::markdown::render(&report);
        let html = super::super::html::render(&report);

        for rendered in [&terminal, &markdown, &html] {
            assert!(rendered.contains("no supported source"), "{rendered}");
            assert!(!rendered.contains("Worst offender"), "{rendered}");
        }
        assert!(share_caption(&report).is_none());
    }
}
