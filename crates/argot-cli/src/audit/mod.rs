//! `argot audit` — what did AI sneak into your repo?
//!
//! The history scorecard: fit the voice **as of a base commit** in a
//! temporary `git worktree` (the user's tree and `.argot/` are never
//! touched; the current `argot.toml` rides along so today's excludes judge
//! the past, and the current semantic index seeds the fit), score
//! `base..HEAD` with every rule group, attribute each finding to its
//! introducing commit (ai-assisted / human / unknown, concrete markers
//! only), and render the card — terminal, json, markdown, or html.
//!
//! Zero-setup by design: on a fresh clone with no `.argot/` and no
//! `argot.toml`, the fit happens at base inside the worktree. Informational
//! by design: always exits 0 on success, 2 when it can't run — merged code
//! is accepted code, so each finding is "would have prompted review before
//! merge", never a bug list.

pub mod attribution;
mod html;
mod markdown;
mod report;
mod term;
pub mod window;

use std::collections::HashMap;
use std::path::Path;
use std::process::ExitCode;

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;
use argot_core::rules;

use crate::worktree::TempWorktree;
use attribution::Attribution;
use report::{
    AuditReport, CommitsReport, Finding, FindingCommit, GroupReport, GroupStatus, RequestedWindow,
    WindowReport, SCHEMA_VERSION,
};
use window::{Clamp, WindowSpec, MAX_WINDOW};

pub use window::DEFAULT_COMMITS;

/// Audit output formats. The terminal card is the product; the others are
/// the same report in machine/pasteable/screenshot form.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuditFormat {
    Terminal,
    Json,
    Markdown,
    Html,
}

impl AuditFormat {
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "terminal" => Some(Self::Terminal),
            "json" => Some(Self::Json),
            "markdown" | "md" => Some(Self::Markdown),
            "html" => Some(Self::Html),
            _ => None,
        }
    }
}

/// True when the tree at `sha` holds at least one file today's config counts
/// as corpus source — i.e. a fit at that commit can succeed.
fn tree_has_scoped_source(
    git_repo: &git2::Repository,
    sha: &str,
    suppressions: &argot_core::suppress::PathSuppressions,
) -> bool {
    let Ok(oid) = git2::Oid::from_str(sha) else {
        return false;
    };
    let Ok(tree) = git_repo.find_commit(oid).and_then(|c| c.tree()) else {
        return false;
    };
    let mut found = false;
    let _ = tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            if let Some(name) = entry.name() {
                let rel = format!("{root}{name}");
                if argot_core::train::is_corpus_source(&rel, suppressions) {
                    found = true;
                    return git2::TreeWalkResult::Abort;
                }
            }
        }
        git2::TreeWalkResult::Ok
    });
    found
}

/// The widest window whose base commit can actually be fitted: the largest
/// `k ≤ chain.len()` such that the tree `k` commits back still holds
/// in-scope source under today's config. Repos whose in-scope code is
/// younger than the requested window (a rewrite, or early history today's
/// excludes mute entirely) shrink instead of failing. 0 = no ancestor
/// qualifies.
fn max_fittable_window(
    repo: &Path,
    chain: &[window::ChainCommit],
    suppressions: &argot_core::suppress::PathSuppressions,
) -> usize {
    let Ok(git_repo) = git2::Repository::discover(repo) else {
        return 0;
    };
    (1..=chain.len())
        .rev()
        .find(|&k| tree_has_scoped_source(&git_repo, &chain[k - 1].sha, suppressions))
        .unwrap_or(0)
}

/// One parsed hit from check's JSON document.
struct CheckHit {
    rule: String,
    rule_label: String,
    confidence: String,
    severity: String,
    path: String,
    line_start: usize,
    line_end: usize,
    evidence: Option<String>,
}

fn parse_hits(json: &str) -> (Vec<CheckHit>, u64) {
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(json) else {
        return (Vec::new(), 0);
    };
    let hunks = doc["hunks_scanned"].as_u64().unwrap_or(0);
    let hits = doc["hits"]
        .as_array()
        .map(|hits| {
            hits.iter()
                .filter_map(|h| {
                    Some(CheckHit {
                        rule: h["rule"].as_str()?.to_string(),
                        rule_label: h["rule_label"].as_str().unwrap_or("").to_string(),
                        confidence: h["confidence"].as_str().unwrap_or("unusual").to_string(),
                        severity: h["severity"].as_str().unwrap_or("error").to_string(),
                        path: h["path"].as_str()?.to_string(),
                        line_start: h["line_start"].as_u64().unwrap_or(0) as usize,
                        line_end: h["line_end"].as_u64().unwrap_or(0) as usize,
                        evidence: h["evidence"]
                            .as_array()
                            .and_then(|e| e.first())
                            .and_then(|e| e.as_str())
                            .map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (hits, hunks)
}

/// Attribute every finding to its introducing commit: one bounded blame per
/// distinct file, then most-lines-wins per span. Unresolvable ⇒ `unknown`.
fn attribute_findings(
    repo: &Path,
    base: &str,
    head: &str,
    hits: &[CheckHit],
    by_sha: &HashMap<String, attribution::CommitInfo>,
) -> Vec<Finding> {
    let git_repo = git2::Repository::discover(repo).ok();
    let mut blames: HashMap<&str, Option<git2::Blame>> = HashMap::new();
    let mut findings = Vec::with_capacity(hits.len());
    for h in hits {
        let sha = git_repo.as_ref().and_then(|r| {
            let blame = blames
                .entry(h.path.as_str())
                .or_insert_with(|| attribution::blame_file(r, base, head, &h.path));
            blame
                .as_ref()
                .and_then(|b| attribution::introducing_commit(b, base, h.line_start, h.line_end))
        });
        let commit = match sha.as_ref().and_then(|s| by_sha.get(s)) {
            Some(info) => FindingCommit {
                sha: sha.clone(),
                short: Some(info.short.clone()),
                subject: Some(info.subject.clone()),
                attribution: info.attribution,
                markers: info.markers.clone(),
            },
            None => FindingCommit {
                sha: None,
                short: None,
                subject: None,
                attribution: Attribution::Unknown,
                markers: Vec::new(),
            },
        };
        findings.push(Finding {
            group: rules::rule_named(&h.rule)
                .map(|r| r.group)
                .unwrap_or("voice"),
            rule: h.rule.clone(),
            rule_label: h.rule_label.clone(),
            confidence: h.confidence.clone(),
            severity: h.severity.clone(),
            path: h.path.clone(),
            line_start: h.line_start,
            line_end: h.line_end,
            evidence: h.evidence.clone(),
            commit,
        });
    }
    AuditReport::sort_findings(&mut findings);
    findings
}

/// Per-group outcome: scored (with its finding count) or skipped (with the
/// honest reason). A skipped group is marked on the card, never shown as 0.
fn group_statuses(
    config: &argot_core::config::ArgotConfig,
    worktree_argot: &Path,
    findings: &[Finding],
) -> Vec<GroupReport> {
    let settings = config.rule_settings(&Vec::new());
    // Not a `matches!`: each arm is a different cfg!, they only collapse to
    // identical literals in builds with no optional features compiled in.
    #[allow(clippy::match_like_matches_macro)]
    let compiled = |group: &str| match group {
        rules::GROUP_SEMANTIC => cfg!(feature = "semantic"),
        rules::GROUP_ARCHITECTURE => cfg!(feature = "arch"),
        rules::GROUP_INTEGRITY => cfg!(feature = "integrity"),
        _ => true,
    };
    rules::GROUPS
        .iter()
        .map(|&group| {
            let count = findings.iter().filter(|f| f.group == group).count();
            let all_off = rules::RULES
                .iter()
                .filter(|r| r.group == group)
                .all(|r| settings.severity_of_reason(r.reason) == rules::Severity::Off);
            let skip_reason = if !compiled(group) {
                Some("not compiled into this build".to_string())
            } else if all_off {
                Some("disabled in argot.toml".to_string())
            } else if group == rules::GROUP_SEMANTIC
                && !worktree_argot.join("semantic-index.json").is_file()
            {
                Some("embedding model not available (offline?)".to_string())
            } else if group == rules::GROUP_INTEGRITY
                && !worktree_argot.join("integrity.json").is_file()
            {
                Some("integrity gates could not be learned".to_string())
            } else {
                None
            };
            GroupReport {
                group,
                status: if skip_reason.is_some() {
                    GroupStatus::Skipped
                } else {
                    GroupStatus::Scored
                },
                findings: count,
                skip_reason,
            }
        })
        .collect()
}

/// Run the whole audit. Informational: exits 0 on success even with findings.
pub fn run_audit(repo: &Path, spec: WindowSpec, format: AuditFormat) -> ExitCode {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0);
    // One past the cap so "hit the cap" and "exactly the cap" stay distinct.
    let (head, chain) = match window::resolve_chain(repo, MAX_WINDOW + 1) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let (mut walked, mut clamp) = match window::requested_window(&spec, &head, &chain, now) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let mut clamp_note = clamp.map(|c| match c {
        Clamp::Cap => format!(
            "window clamped to the audit cap of {MAX_WINDOW} commits — auditing the {walked} most recent"
        ),
        Clamp::History => format!(
            "history ends {walked} commit(s) back (the root, or a shallow clone's fetch \
             horizon) — auditing all of it"
        ),
        Clamp::FitShrink => String::new(), // set below
    });
    // Shrink to the oldest ancestor a fit can succeed on: today's config
    // rides into the historical worktree, so a base commit whose whole tree
    // is excluded or unsupported would otherwise die on "no source files".
    let config = argot_core::config::ArgotConfig::load(repo);
    let suppressions = config.path_suppressions();
    let fittable = max_fittable_window(repo, &chain[..walked], &suppressions);
    if fittable == 0 {
        eprintln!(
            "error: nothing to audit — none of the last {walked} commit(s) contain source files in \
             argot's scope; if this repo has source code, check the [exclude] patterns in argot.toml"
        );
        return ExitCode::from(2);
    }
    if fittable < walked {
        clamp = Some(Clamp::FitShrink);
        clamp_note = Some(format!(
            "the repo {walked} commit(s) ago had no code in argot's scope yet — window shrunk to {fittable} commit(s)"
        ));
        walked = fittable;
    }
    if let Some(note) = &clamp_note {
        eprintln!("argot: {note}");
    }
    let base = &chain[walked - 1];
    let base_short = &base.sha[..12.min(base.sha.len())];

    eprintln!("argot: auditing your last {walked} commit(s) against the voice as of {base_short}…");
    let worktree = match TempWorktree::create(repo, &base.sha, "argot-audit") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    // Today's config judges the past, and the current semantic index seeds
    // the historical fit (embeddings reuse: seconds, not a full re-embed).
    worktree.adopt_current_config(repo);

    eprintln!("argot: fitting the historical voice (one-off, in a temp worktree)…");
    if crate::fit_repo(&worktree.path, &[]).is_err() {
        eprintln!("error: fitting at {base_short} failed");
        return ExitCode::from(2);
    }

    let outcome = run_check(CheckArgs {
        repo_path: worktree.path.to_string_lossy().into_owned(),
        reference: format!("{}..{}", base.sha, head.sha),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: worktree.path.join(".argot"),
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
        eprint!("{}", outcome.stderr);
        eprintln!("error: audit check failed");
        return ExitCode::from(2);
    }

    let (hits, hunks_scanned) = parse_hits(&outcome.stdout);

    eprintln!("argot: attributing {walked} commit(s) of history…");
    let (counts, by_sha) = match attribution::attribute_range(repo, &base.sha, &head.sha) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let findings = attribute_findings(repo, &base.sha, &head.sha, &hits, &by_sha);
    let groups = group_statuses(&config, &worktree.path.join(".argot"), &findings);

    let report = AuditReport {
        schema_version: SCHEMA_VERSION,
        generated_by: format!("argot v{}", env!("CARGO_PKG_VERSION")),
        window: WindowReport {
            requested: RequestedWindow::of(&spec),
            effective_commits: walked,
            clamp: clamp.map(Clamp::as_str),
            clamp_note,
            base: base.sha.clone(),
            head: head.sha.clone(),
            base_date: window::format_date(base.time),
            head_date: window::format_date(head.time),
        },
        commits: CommitsReport {
            total: counts.total,
            ai_assisted: counts.ai_assisted,
            human: counts.human,
            unknown: 0,
        },
        hunks_scanned,
        groups,
        findings,
    };

    let color = std::env::var_os("NO_COLOR").is_none()
        && std::io::IsTerminal::is_terminal(&std::io::stdout());
    match format {
        AuditFormat::Json => print!("{}", report.to_json()),
        AuditFormat::Markdown => print!("{}", markdown::render(&report)),
        AuditFormat::Html => print!("{}", html::render(&report)),
        AuditFormat::Terminal => print!("{}", term::render(&report, color)),
    }
    ExitCode::SUCCESS
}
