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
    /// The named symbol the finding is about (integrity rules: the test).
    symbol: Option<String>,
    /// Verbatim flagged module specifiers (foreign-import findings only).
    foreign_specifiers: Vec<String>,
    /// Nearest-function cosine similarity (semantic `redundant` findings only).
    similarity: Option<f32>,
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
                        symbol: h["symbol"].as_str().map(String::from),
                        foreign_specifiers: h["foreign_specifiers"]
                            .as_array()
                            .map(|arr| {
                                arr.iter()
                                    .filter_map(|s| s.as_str().map(String::from))
                                    .collect()
                            })
                            .unwrap_or_default(),
                        similarity: h["similarity"].as_f64().map(|v| v as f32),
                    })
                })
                .collect()
        })
        .unwrap_or_default();
    (hits, hunks)
}

/// Minimum nearest-function similarity for a `redundant` finding to survive the
/// history sweep. A per-commit `check` runs the rule at its liberal tier (a
/// strong structural match fires down to cos 0.70), which reads right when
/// you're looking at one change; swept across every hunk of a window, that
/// per-hunk noise floor stacks into a wall of low-confidence "you already have
/// this" matches — many of them generic helpers or a monorepo's deliberate
/// per-package parallels. A scorecard needs per-finding precision, so it keeps
/// only clearly-close matches — the 0.85 bar the rule's own conservative mode
/// uses.
const AUDIT_REDUNDANT_MIN_SIMILARITY: f32 = 0.85;

/// The package identity of a module specifier: `@scope/name` for a scoped
/// package, the first path segment otherwise. Subpath entry points
/// (`@scope/name/sub`, `pkg/sub`) share their package's identity, so importing a
/// new entry point of a package the repo already owns resolves as internal.
fn package_identity(spec: &str) -> &str {
    let mut slashes = spec.match_indices('/').map(|(i, _)| i);
    if spec.starts_with('@') {
        slashes.next(); // end of `@scope`
        match slashes.next() {
            Some(i) => &spec[..i], // `@scope/name`
            None => spec,
        }
    } else {
        match slashes.next() {
            Some(i) => &spec[..i], // first segment
            None => spec,
        }
    }
}

/// True when a module specifier resolves to repo-internal code: a workspace /
/// own package name, a subpath of one, or a path-alias prefix. Relative and
/// `#…` specifiers never reach the foreign-import surface, so they aren't here.
fn spec_is_internal(spec: &str, internal: &argot_core::scoring::adapters::RepoModules) -> bool {
    internal.exact.contains(spec)
        || internal.exact.contains(package_identity(spec))
        || internal.prefixes.iter().any(|p| spec.starts_with(p))
}

/// Curate raw per-hunk `check` hits into the scorecard's findings: a history
/// sweep over every hunk surfaces a rule's per-hunk noise floor at scale, so the
/// audit (unlike `check`) drops findings that aren't actionable in aggregate.
/// `check` itself is untouched — the pre-commit gate still sees everything.
fn curate_hits(
    hits: Vec<CheckHit>,
    internal: &argot_core::scoring::adapters::RepoModules,
) -> Vec<CheckHit> {
    hits.into_iter().filter(|h| keep_hit(h, internal)).collect()
}

/// Whether a curated audit keeps `h` (see [`curate_hits`]).
fn keep_hit(h: &CheckHit, internal: &argot_core::scoring::adapters::RepoModules) -> bool {
    match h.rule.as_str() {
        // Drop when every flagged specifier resolves to repo-internal code — a
        // workspace package (possibly created after the base commit, so the
        // base-time fit couldn't know it) or a subpath of one. Importing your
        // own package is never a foreign dependency. Findings with no structured
        // specifiers are left alone.
        "foreign-import" => {
            h.foreign_specifiers.is_empty()
                || !h
                    .foreign_specifiers
                    .iter()
                    .all(|s| spec_is_internal(s, internal))
        }
        // Keep only findings that actually name the unfamiliar callee. When the
        // scorer can't (the foreignness is distributional, not a single foreign
        // call), the evidence degrades to "common here: …" — what's NORMAL, not
        // what's off — which isn't actionable on a scorecard.
        "unfamiliar-callee" => h
            .evidence
            .as_deref()
            .map(|e| e.starts_with('↳'))
            .unwrap_or(false),
        // History sweep keeps only clearly-close reinventions (see the const).
        "redundant" => h
            .similarity
            .map(|s| s >= AUDIT_REDUNDANT_MIN_SIMILARITY)
            .unwrap_or(true),
        _ => true,
    }
}

/// The external package identities a foreign-import hit introduces — its flagged
/// specifiers minus any that resolve to repo-internal code, collapsed to package
/// identity and de-duplicated. Empty when every specifier is internal.
fn external_package_ids(
    h: &CheckHit,
    internal: &argot_core::scoring::adapters::RepoModules,
) -> Vec<String> {
    let mut ids: Vec<String> = h
        .foreign_specifiers
        .iter()
        .filter(|s| !spec_is_internal(s, internal))
        .map(|s| package_identity(s).to_string())
        .collect();
    ids.sort();
    ids.dedup();
    ids
}

/// Collapse per-file foreign-import hits into one finding per distinct new
/// dependency. A history sweep re-flags the same adoption in every file that
/// imports it (a UI kit landing in 8 components, a data layer in 8 modules) and
/// re-lists each subpath — one adoption, told once, is the scorecard's job.
/// Hits are grouped by their set of external package identities (internal
/// specifiers, e.g. a co-imported workspace package, drop out of both the key
/// and the rendered evidence); the first hit of each group represents it, its
/// evidence rewritten to the package(s) plus a file count. Hits with no
/// structured specifiers pass through untouched.
fn dedup_foreign_imports(
    hits: Vec<CheckHit>,
    internal: &argot_core::scoring::adapters::RepoModules,
) -> Vec<CheckHit> {
    let mut out: Vec<CheckHit> = Vec::new();
    // (external-id key, distinct paths, representative hit), first-seen order.
    let mut groups: Vec<(Vec<String>, Vec<String>, CheckHit)> = Vec::new();
    for h in hits {
        if h.rule != "foreign-import" {
            out.push(h);
            continue;
        }
        let ids = external_package_ids(&h, internal);
        if ids.is_empty() {
            // No external specifier survives (all internal) — nothing to report.
            // `curate_hits` already drops these; belt and braces.
            continue;
        }
        if let Some((_, paths, _)) = groups.iter_mut().find(|(key, ..)| *key == ids) {
            if !paths.contains(&h.path) {
                paths.push(h.path.clone());
            }
        } else {
            groups.push((ids, vec![h.path.clone()], h));
        }
    }
    for (ids, paths, mut rep) in groups {
        let joined = ids.join(", ");
        let files = paths.len();
        rep.foreign_specifiers = ids;
        rep.evidence = Some(if files > 1 {
            format!("↳ {joined} — new to the repo ({files} files)")
        } else {
            format!("↳ {joined} — new to the repo")
        });
        out.push(rep);
    }
    out
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
            // A deletion finding has no lines at head for blame to see — and
            // blaming the *neighbouring* lines credits whoever last edited
            // around the deletion. Resolve by content instead: the commit
            // whose diff dropped the named test.
            if h.rule == "test-deleted" {
                return h
                    .symbol
                    .as_deref()
                    .and_then(|sym| attribution::symbol_removal_commit(r, base, head, &h.path, sym))
                    .or_else(|| attribution::sole_touching_commit(r, base, head, &h.path));
            }
            let blame = blames
                .entry(h.path.as_str())
                .or_insert_with(|| attribution::blame_file(r, base, head, &h.path));
            blame
                .as_ref()
                .and_then(|b| attribution::introducing_commit(b, base, h.line_start, h.line_end))
                .or_else(|| attribution::sole_touching_commit(r, base, head, &h.path))
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
    let t_fittable = argot_core::timing::phase("audit: max-fittable-window scan");
    let fittable = max_fittable_window(repo, &chain[..walked], &suppressions);
    t_fittable.done();
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
    let t_wt = argot_core::timing::phase("audit: worktree add + seed");
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
    t_wt.done();

    eprintln!("argot: fitting the historical voice (one-off, in a temp worktree)…");
    let t_fit = argot_core::timing::phase("audit: fit (total)");
    if crate::fit_repo(&worktree.path, &[]).is_err() {
        eprintln!("error: fitting at {base_short} failed");
        return ExitCode::from(2);
    }
    t_fit.done();

    let t_check = argot_core::timing::phase("audit: check (total)");
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

    t_check.done();
    let (hits, hunks_scanned) = parse_hits(&outcome.stdout);
    // Curate the raw per-hunk hits into scorecard findings. Internal modules are
    // resolved from the real repo (HEAD), not the base worktree, so a workspace
    // package created within the window still reads as internal.
    let internal_modules = argot_core::inspect::resolve_repo_internal_modules(repo);
    let hits = curate_hits(hits, &internal_modules);
    let hits = dedup_foreign_imports(hits, &internal_modules);
    // One row per affected symbol: a single test can trip several `test-weakened`
    // detectors (assertions removed *and* a literal retargeted), which land as
    // distinct hits on different lines. Collapse repeats that name the same
    // symbol; hits without a symbol (foreign/redundant/…) are never touched.
    let hits = {
        let mut seen = std::collections::HashSet::new();
        hits.into_iter()
            .filter(|h| {
                h.symbol.is_none()
                    || seen.insert((h.rule.clone(), h.path.clone(), h.symbol.clone()))
            })
            .collect::<Vec<_>>()
    };

    eprintln!("argot: attributing {walked} commit(s) of history…");
    let t_attr = argot_core::timing::phase("audit: attribution");
    let (counts, by_sha) = match attribution::attribute_range(repo, &base.sha, &head.sha) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let findings = attribute_findings(repo, &base.sha, &head.sha, &hits, &by_sha);
    t_attr.done();
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

#[cfg(test)]
mod tests {
    use super::*;
    use argot_core::scoring::adapters::RepoModules;

    fn internal_set(exact: &[&str], prefixes: &[&str]) -> RepoModules {
        RepoModules {
            exact: exact.iter().map(|s| s.to_string()).collect(),
            prefixes: prefixes.iter().map(|s| s.to_string()).collect(),
        }
    }

    fn hit(rule: &str) -> CheckHit {
        CheckHit {
            rule: rule.to_string(),
            rule_label: rule.to_string(),
            confidence: "unusual".to_string(),
            severity: "error".to_string(),
            path: "src/x.ts".to_string(),
            line_start: 1,
            line_end: 2,
            evidence: None,
            symbol: None,
            foreign_specifiers: Vec::new(),
            similarity: None,
        }
    }

    #[test]
    fn package_identity_scoped_unscoped_and_subpaths() {
        assert_eq!(package_identity("@acme/core"), "@acme/core");
        assert_eq!(package_identity("@acme/core/sub"), "@acme/core");
        assert_eq!(package_identity("@acme/core/deep/path"), "@acme/core");
        assert_eq!(package_identity("react"), "react");
        assert_eq!(package_identity("react-dom/client"), "react-dom");
        assert_eq!(package_identity("@scope"), "@scope");
    }

    #[test]
    fn internal_specifier_matches_exact_subpath_and_prefix() {
        let internal = internal_set(&["@acme/core", "@acme/http"], &["#/"]);
        assert!(spec_is_internal("@acme/core", &internal));
        // Subpath of an owned package resolves internal via package identity.
        assert!(spec_is_internal("@acme/http/serve-static", &internal));
        assert!(spec_is_internal("#/utils", &internal));
        // A real third-party dependency is not internal.
        assert!(!spec_is_internal("@base-ui/react/tooltip", &internal));
        assert!(!spec_is_internal("zod", &internal));
    }

    #[test]
    fn foreign_import_dropped_only_when_all_specifiers_internal() {
        let internal = internal_set(&["@acme/core", "@acme/http"], &[]);
        // All internal → dropped.
        let mut all_internal = hit("foreign-import");
        all_internal.foreign_specifiers =
            vec!["@acme/core".into(), "@acme/http/serve-static".into()];
        assert!(!keep_hit(&all_internal, &internal));
        // Mixed internal + external → kept (a real dependency remains).
        let mut mixed = hit("foreign-import");
        mixed.foreign_specifiers = vec!["@acme/core".into(), "zod".into()];
        assert!(keep_hit(&mixed, &internal));
        // No structured specifiers → left alone.
        assert!(keep_hit(&hit("foreign-import"), &internal));
    }

    #[test]
    fn unfamiliar_callee_dropped_without_a_named_callee() {
        let internal = internal_set(&[], &[]);
        let mut named = hit("unfamiliar-callee");
        named.evidence = Some("↳ store.findMany — 0 of 42 callees in this cluster".into());
        assert!(keep_hit(&named, &internal));
        let mut common_only = hit("unfamiliar-callee");
        common_only.evidence = Some("common here: Effect.gen (117×)".into());
        assert!(!keep_hit(&common_only, &internal));
        // No evidence at all → not actionable → dropped.
        assert!(!keep_hit(&hit("unfamiliar-callee"), &internal));
    }

    #[test]
    fn redundant_dropped_below_the_similarity_bar() {
        let internal = internal_set(&[], &[]);
        let mut strong = hit("redundant");
        strong.similarity = Some(0.92);
        assert!(keep_hit(&strong, &internal));
        let mut weak = hit("redundant");
        weak.similarity = Some(0.74);
        assert!(!keep_hit(&weak, &internal));
        // Exactly at the bar survives.
        let mut boundary = hit("redundant");
        boundary.similarity = Some(AUDIT_REDUNDANT_MIN_SIMILARITY);
        assert!(keep_hit(&boundary, &internal));
    }

    #[test]
    fn unrelated_rules_pass_through_untouched() {
        let internal = internal_set(&["@acme/core"], &[]);
        assert!(keep_hit(&hit("rare-tokens"), &internal));
        assert!(keep_hit(&hit("test-deleted"), &internal));
        assert!(keep_hit(&hit("layering"), &internal));
    }

    fn fi(path: &str, specs: &[&str]) -> CheckHit {
        let mut h = hit("foreign-import");
        h.path = path.to_string();
        h.foreign_specifiers = specs.iter().map(|s| s.to_string()).collect();
        h
    }

    #[test]
    fn dedup_collapses_a_dependency_adopted_across_files() {
        let internal = internal_set(&["@acme/core"], &[]);
        let hits = vec![
            fi("src/a.tsx", &["@base-ui/react/input"]),
            fi("src/b.tsx", &["@base-ui/react/switch"]),
            fi(
                "src/c.tsx",
                &["@base-ui/react/toggle", "@base-ui/react/toggle-group"],
            ),
            fi("src/x.ts", &["@vendor/xlsx"]),
        ];
        let out = dedup_foreign_imports(hits, &internal);
        assert_eq!(out.len(), 2, "base-ui collapses to one, excel stands alone");
        let base = out
            .iter()
            .find(|h| h.foreign_specifiers == ["@base-ui/react"])
            .expect("base-ui group");
        assert_eq!(
            base.evidence.as_deref(),
            Some("↳ @base-ui/react — new to the repo (3 files)")
        );
        let excel = out
            .iter()
            .find(|h| h.foreign_specifiers == ["@vendor/xlsx"])
            .expect("excel group");
        assert_eq!(
            excel.evidence.as_deref(),
            Some("↳ @vendor/xlsx — new to the repo")
        );
    }

    #[test]
    fn dedup_drops_internal_specifiers_from_a_mixed_finding() {
        let internal = internal_set(&["@acme/core", "@acme/rpc"], &[]);
        // A file importing a real new dep alongside its own workspace packages:
        // the finding stays (real dep), the internal names leave the evidence.
        let hits = vec![fi(
            "src/server.ts",
            &["@acme/rpc", "@acme/core", "@vendor/queue"],
        )];
        let out = dedup_foreign_imports(hits, &internal);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].foreign_specifiers, ["@vendor/queue"]);
        assert_eq!(
            out[0].evidence.as_deref(),
            Some("↳ @vendor/queue — new to the repo")
        );
    }

    #[test]
    fn dedup_leaves_other_rules_alone() {
        let internal = internal_set(&[], &[]);
        let mut red = hit("redundant");
        red.similarity = Some(0.9);
        let out = dedup_foreign_imports(vec![red], &internal);
        assert_eq!(out.len(), 1);
        assert_eq!(out[0].rule, "redundant");
    }
}
