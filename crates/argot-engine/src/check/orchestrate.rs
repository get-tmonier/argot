//! `run_check` (the `check` entry point), the freshness/staleness plumbing,
//! and `argot review-mutes`.

use super::collect::{
    batch_scope, collect_patches, passes_filters, patches_langs_without_model, BatchScope,
};
use super::render::{
    add_ignore_comments, confidence, hit_records, render_machine, render_results, report_meta,
};
use super::{CheckArgs, CheckOutcome, PatchBatch};
use crate::config::ArgotConfig;
use crate::detector::{run_detectors, CheckContext, RegisteredDetector, ScanReport};
use crate::finding::{Finding, SuppressedBy};
use crate::git_walk::{open_repo, SUPPORTED_EXTENSIONS};
use crate::output::{CheckResult, OutputFormat};
use crate::rules::{self, RuleSettings, Severity as RuleSeverity};
use crate::suppress::{write_last_check, LastCheckHit, SuppressionRule};
use argot_lang::adapters::LanguageAdapter;
use std::collections::HashMap;
use std::path::Path;

/// Confidence tier ordering, weakest first. Confidence grades how strong the
/// evidence is (`unusual` / `suspicious` / `foreign`); it is display-only —
/// whether a finding fails the check is decided by its rule's configured
/// severity (`error` / `warn`), never by the tier.
const CONFIDENCE_ORDER: [&str; 3] = ["unusual", "suspicious", "foreign"];
fn confidence_index(s: &str) -> usize {
    CONFIDENCE_ORDER.iter().position(|x| *x == s).unwrap_or(0)
}
/// The check exit code for the visible findings: 1 when any finding's rule is
/// configured `error` (or when `--error-on-warnings` promotes a warn-only
/// run), 0 otherwise. Unregistered reasons gate as `error` — a finding never
/// silently loses its gate.
fn gate_exit_code(hits: &[&Finding], settings: &RuleSettings, error_on_warnings: bool) -> i32 {
    let fails = hits
        .iter()
        .any(|h| settings.severity_of_reason(&h.reason) == RuleSeverity::Error)
        || (error_on_warnings && !hits.is_empty());
    if fails {
        1
    } else {
        0
    }
}

pub(super) fn result_summary(
    unsuppressed: &[&Finding],
    visible: &[&Finding],
    suppressed_hits: usize,
    settings: &RuleSettings,
    error_on_warnings: bool,
) -> CheckResult {
    let error_hits = unsuppressed
        .iter()
        .filter(|h| settings.severity_of_reason(&h.reason) == RuleSeverity::Error)
        .count();
    let warn_hits = unsuppressed.len() - error_hits;
    let exit_code = gate_exit_code(unsuppressed, settings, error_on_warnings);
    CheckResult {
        exit_code,
        unsuppressed_hits: unsuppressed.len(),
        visible_hits: visible.len(),
        hidden_hits: unsuppressed.len() - visible.len(),
        suppressed_hits,
        error_hits,
        warn_hits,
        gating_hits: if exit_code == 0 {
            0
        } else if error_hits > 0 {
            error_hits
        } else {
            warn_hits
        },
    }
}
/// Could the changeset's OLD side have carried locks the new config lost?
/// Cheap gate for the tamper pass: when the current config has no locks, the
/// pass still must run if the change touches the sensitive surfaces at all —
/// removing every lock is precisely the tamper case.
fn detected_locks_possible(args: &CheckArgs) -> bool {
    // The only cheap, mode-agnostic signal without re-diffing: does the
    // repo's argot.toml (either side) mention `locked` at all? Read the
    // committed file; the two-sided pass does the exact work.
    std::fs::read_to_string(std::path::Path::new(&args.repo_path).join("argot.toml"))
        .map(|t| t.contains("locked"))
        .unwrap_or(false)
        || {
            // A deleted/edited argot.toml in the diff can hide the marker in
            // the workdir — fall back to git HEAD's copy.
            let repo = crate::git_walk::open_repo(&args.repo_path).ok();
            repo.and_then(|r| {
                let head = r.head().ok()?.peel_to_tree().ok()?;
                let entry = head.get_path(std::path::Path::new("argot.toml")).ok()?;
                let blob = r.find_blob(entry.id()).ok()?;
                Some(String::from_utf8_lossy(blob.content()).contains("locked"))
            })
            .unwrap_or(false)
        }
}

/// Commit-context walks stop visiting here. Adaptive freshness does not use
/// this cap; an explicit `[fit] refresh-after` backstop and status context do.
pub const FRESHNESS_SCAN_CAP: usize = 200;

/// How many commits HEAD is ahead of the fit SHA (`None` when either end
/// cannot be resolved — shallow clones, rewritten history, detached states
/// must never break check). This is context for adaptive freshness and powers
/// an explicit commit-count backstop when a team opts into one.
pub fn commits_since_fit(repo_path: &str, fit_sha: &str) -> Option<usize> {
    let repo = open_repo(repo_path).ok()?;
    let head = repo.head().ok()?.peel_to_commit().ok()?;
    let fit_oid = git2::Oid::from_str(fit_sha).ok()?;
    if head.id() == fit_oid {
        return Some(0);
    }
    repo.find_commit(fit_oid).ok()?;
    let (ahead, _) = repo.graph_ahead_behind(head.id(), fit_oid).ok()?;
    Some(ahead)
}
/// The repo's default branch, by shorthand name — `origin/HEAD`'s target when
/// the remote declares one, else a local `main`/`master`. `None` when neither
/// exists (unusual layouts keep today's HEAD-relative behaviour).
fn default_branch_shorthand(repo: &git2::Repository) -> Option<String> {
    if let Ok(r) = repo.find_reference("refs/remotes/origin/HEAD") {
        if let Some(target) = r.symbolic_target() {
            if let Some(name) = target.strip_prefix("refs/remotes/origin/") {
                return Some(name.to_string());
            }
        }
    }
    ["main", "master"]
        .iter()
        .find(|name| repo.find_reference(&format!("refs/heads/{name}")).is_ok())
        .map(|s| s.to_string())
}
/// The trunk whose line counts as accepted history: the branch named in
/// `[fit] refresh-from` when it exists (locally or on origin), else the
/// auto-detected default branch — a named trunk missing from this clone
/// (a fork, a typo) degrades to detection rather than silently anchoring
/// at HEAD.
fn trunk_shorthand(repo: &git2::Repository, config: &crate::config::ArgotConfig) -> Option<String> {
    if let crate::config::FitRefreshFrom::Branch(name) = &config.fit_refresh_from {
        let exists = repo.find_reference(&format!("refs/heads/{name}")).is_ok()
            || repo
                .find_reference(&format!("refs/remotes/origin/{name}"))
                .is_ok();
        if exists {
            return Some(name.clone());
        }
    }
    default_branch_shorthand(repo)
}
/// The newest **accepted** commit the current work builds on. On the trunk
/// (or when no trunk is discernible) that's HEAD; on any other branch it's
/// the merge-base with the trunk. Feature-branch commits are deliberately not
/// accepted history — a voice refreshed against this anchor never learns
/// unreviewed work-in-progress, so `check` keeps judging it instead of
/// treating it as the repo's own. `None` when history can't be resolved
/// (shallow clones, disjoint roots).
pub fn accepted_anchor(repo_path: &str, config: &crate::config::ArgotConfig) -> Option<String> {
    let repo = open_repo(repo_path).ok()?;
    let head_ref = repo.head().ok()?;
    let head = head_ref.peel_to_commit().ok()?;
    let Some(trunk) = trunk_shorthand(&repo, config) else {
        return Some(head.id().to_string());
    };
    if head_ref.is_branch() && head_ref.shorthand() == Some(trunk.as_str()) {
        return Some(head.id().to_string());
    }
    let tip = repo
        .find_reference(&format!("refs/heads/{trunk}"))
        .or_else(|_| repo.find_reference(&format!("refs/remotes/origin/{trunk}")))
        .ok()?
        .peel_to_commit()
        .ok()?
        .id();
    let base = repo.merge_base(head.id(), tip).ok()?;
    Some(base.to_string())
}
/// How many commits in `from..to` touch corpus source under the given
/// suppressions — the staleness measure freshness decisions run on: docs,
/// CI config, and changelog churn don't age a voice; accepted source changes
/// do. Bounded twice so it never weighs on check: the count stops at
/// `stop_at` (callers only need "did it cross the threshold"), and the walk
/// itself gives up after [`FRESHNESS_SCAN_CAP`] commits (a fit that far
/// behind is stale regardless of the exact count). `None` when either end is
/// unresolvable — callers must leave the fit alone rather than guess.
pub fn in_scope_commits_between(
    repo_path: &str,
    from_sha: &str,
    to_sha: &str,
    suppressions: &crate::suppress::PathSuppressions,
    stop_at: usize,
) -> Option<usize> {
    let repo = open_repo(repo_path).ok()?;
    let from = git2::Oid::from_str(from_sha).ok()?;
    let to = git2::Oid::from_str(to_sha).ok()?;
    if from == to || stop_at == 0 {
        return Some(0);
    }
    repo.find_commit(from).ok()?;
    let mut walk = repo.revwalk().ok()?;
    walk.push(to).ok()?;
    walk.hide(from).ok()?;
    let mut in_scope = 0usize;
    // Adaptive freshness never needs this walk. The normal status context is
    // capped, while an explicitly configured commit backstop is allowed to
    // scan up to the value the team deliberately chose.
    for oid in walk.flatten().take(FRESHNESS_SCAN_CAP.max(stop_at)) {
        let commit = repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        let parent_tree = commit.parent(0).ok().and_then(|p| p.tree().ok());
        let diff = repo
            .diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None)
            .ok()?;
        let touches = diff.deltas().any(|d| {
            d.new_file()
                .path()
                .or(d.old_file().path())
                .and_then(|p| p.to_str())
                .is_some_and(|rel| crate::corpus::is_corpus_source(rel, suppressions))
        });
        if touches {
            in_scope += 1;
            if in_scope >= stop_at {
                break;
            }
        }
    }
    Some(in_scope)
}
/// The anchor freshness is measured against — and the commit a deliberate
/// refresh fits at. [`accepted_anchor`] under the default
/// `[fit] refresh-from = "default-branch"`; plain HEAD when the repo opted
/// into `"current-branch"`.
pub fn freshness_anchor(repo_path: &str, config: &crate::config::ArgotConfig) -> Option<String> {
    match &config.fit_refresh_from {
        crate::config::FitRefreshFrom::DefaultBranch | crate::config::FitRefreshFrom::Branch(_) => {
            accepted_anchor(repo_path, config)
        }
        crate::config::FitRefreshFrom::CurrentBranch => {
            let repo = open_repo(repo_path).ok()?;
            let head = repo.head().ok()?.peel_to_commit().ok()?;
            Some(head.id().to_string())
        }
    }
}
/// The laundering advisory's evidence: when HEAD is a named branch other than
/// the default and its unmerged commits touch in-scope source, returns
/// `(branch, count)` (count stops at `cap`). `None` whenever a fit here is
/// unremarkable — on the default branch, detached HEAD (replay worktrees),
/// a branch with nothing in-scope of its own, or a repo that opted into
/// `[fit] refresh-from = "current-branch"`.
pub fn unmerged_branch_source_commits(
    repo_path: &str,
    config: &crate::config::ArgotConfig,
    cap: usize,
) -> Option<(String, usize)> {
    if config.fit_refresh_from == crate::config::FitRefreshFrom::CurrentBranch {
        return None;
    }
    let repo = open_repo(repo_path).ok()?;
    let head_ref = repo.head().ok()?;
    if !head_ref.is_branch() {
        return None;
    }
    let branch = head_ref.shorthand()?.to_string();
    if branch == trunk_shorthand(&repo, config)? {
        return None;
    }
    let head_sha = head_ref.peel_to_commit().ok()?.id().to_string();
    let anchor = accepted_anchor(repo_path, config)?;
    if anchor == head_sha {
        return None;
    }
    let n = in_scope_commits_between(
        repo_path,
        &anchor,
        &head_sha,
        &config.path_suppressions(),
        cap,
    )?;
    (n > 0).then_some((branch, n))
}
/// The shared freshness measure: commits of **accepted, in-scope** source the
/// fit hasn't seen — [`freshness_anchor`] composed with
/// [`in_scope_commits_between`]. Adaptive freshness reports it as context and
/// uses it only when the team explicitly configures a commit backstop.
pub fn accepted_source_commits_behind(
    repo_path: &str,
    fit_sha: &str,
    config: &crate::config::ArgotConfig,
    stop_at: usize,
) -> Option<usize> {
    let anchor = freshness_anchor(repo_path, config)?;
    // Cheap gate: no commits at all between fit and anchor (the common,
    // fresh case) answers 0 without a single tree diff.
    let repo = open_repo(repo_path).ok()?;
    let fit = git2::Oid::from_str(fit_sha).ok()?;
    let anchor_oid = git2::Oid::from_str(&anchor).ok()?;
    if fit == anchor_oid {
        return Some(0);
    }
    repo.find_commit(fit).ok()?;
    let (ahead, _) = repo.graph_ahead_behind(anchor_oid, fit).ok()?;
    if ahead == 0 {
        return Some(0);
    }
    in_scope_commits_between(
        repo_path,
        fit_sha,
        &anchor,
        &config.path_suppressions(),
        stop_at,
    )
}
/// Entry point (`check.py:main`). Never exits the process — returns the
/// outcome. The composition root: `detectors` is the run's registered rule
/// groups, decided one layer up by `argot-core`'s `compose::default_detectors`
/// (which rule groups a given build wires in), not by this engine.
pub fn run_check(args: CheckArgs, detectors: Vec<RegisteredDetector<'_>>) -> CheckOutcome {
    // Mutual-exclusion validation — fail fast with a clear message (exit 2).
    let ref_nonempty = !args.reference.is_empty();
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    if args.staged && args.unstaged {
        return CheckOutcome::err(
            "error: --staged and --unstaged are mutually exclusive\n".to_string(),
            2,
        );
    }
    if commit_set && ref_nonempty {
        return CheckOutcome::err(
            "error: --commit and ref positional are mutually exclusive\n".to_string(),
            2,
        );
    }
    if commit_set && (args.staged || args.unstaged) {
        return CheckOutcome::err(
            "error: --commit is mutually exclusive with --staged/--unstaged\n".to_string(),
            2,
        );
    }
    if ref_nonempty && (args.staged || args.unstaged) {
        return CheckOutcome::err(
            "error: ref positional is mutually exclusive with --staged/--unstaged\n".to_string(),
            2,
        );
    }

    let mut detectors = detectors;

    // The run's rule vocabulary: built-ins plus whatever custom rules the
    // registered detectors discover (the scripted-rules slice reads
    // `.argot/rules/`). Built BEFORE config loads, so custom `[rules]` keys,
    // severities, and suppression selectors resolve like built-in ones.
    let mut vocab_warnings: Vec<String> = Vec::new();
    let custom: Vec<rules::CustomRule> = detectors
        .iter_mut()
        .flat_map(|r| r.detector.vocabulary(&args.argot_dir, &mut vocab_warnings))
        .collect();
    let registry = &rules::Registry::with_custom(custom, &mut vocab_warnings);

    // argot.toml config: excludes + `[detect]` heuristics + `[rules]` +
    // `[[mute]]`. Loaded once here — the `[detect]` markers gate the check-time
    // auto-generated skip built into each scorer, so they must be in place
    // before load_scorers.
    let config = ArgotConfig::load_with(Path::new(&args.repo_path), registry);
    // Effective per-rule severities: defaults ⊕ [rules] ⊕ CLI --rule overrides.
    // Locked rules freeze at the committed value; refusals surface below.
    let (settings, lock_warnings) = config.rule_settings_resolved(registry, &args.rule_overrides);
    // Unknown --rule selectors fail fast (exit 2), same contract as before —
    // but judged against the run vocabulary, so custom rules are addressable.
    for (name, _) in &args.rule_overrides {
        if !registry.known_selector(name) {
            return CheckOutcome::err(
                format!(
                    "error: --rule: unknown rule '{name}' (known: {})\n",
                    registry.selector_names().join(", ")
                ),
                2,
            );
        }
    }

    // The resolved path scope, shared by the load lifecycle (which files shape
    // the voice vs. are only judged by it) and the changeset filter below.
    let path_suppressions = config.path_suppressions();

    // Load lifecycle: the base model is mandatory (its Err fails the check);
    // additive groups degrade inside their pass instead.
    let t_load = crate::timing::phase("check: load scorers");
    let load_ctx = crate::detector::LoadContext {
        argot_dir: &args.argot_dir,
        detect: &config.detect,
        path_suppressions: &path_suppressions,
    };
    for reg in &mut detectors {
        if let Err((msg, code)) = reg.detector.load(&load_ctx) {
            return CheckOutcome::err(msg, code);
        }
    }
    t_load.done();
    // Learned-model facts for passes that consume them (the scripted rules'
    // host API) — provided by the base detector once loaded.
    let facts = detectors.iter().find_map(|r| r.detector.model_facts());
    let Some(base) = detectors
        .iter()
        .find_map(|r| r.detector.base_info().cloned())
    else {
        return CheckOutcome::err(
            "error: no base detector registered — this is an argot bug\n".to_string(),
            2,
        );
    };
    // The fitted-language adapter map: comment prefixes for suppression
    // parsing and per-pass file parsing. Fitted languages only — an unfitted
    // language's batches are dropped by batch_scope, never parsed.
    let filter_adapters: HashMap<String, Box<dyn LanguageAdapter>> = base
        .fitted_languages
        .iter()
        .filter_map(|l| argot_lang::adapters::adapter_for(l).map(|a| (l.clone(), a)))
        .collect();

    let t_patches = crate::timing::phase("check: collect patches");
    let (patches, scan_label) = match collect_patches(&args) {
        Ok(v) => v,
        Err(outcome) => {
            // Machine formats own stdout: the only non-error early exit (an
            // explicit range with no commits, exit 0) still emits a complete,
            // hit-free document. Hard errors (exit != 0) stay stderr-only.
            if args.format.is_machine() && outcome.exit_code == 0 {
                let meta = report_meta(
                    &args,
                    format!("0 commit(s) ({})", args.reference),
                    0,
                    Vec::new(),
                    &base.model_hash,
                    CheckResult {
                        exit_code: 0,
                        unsuppressed_hits: 0,
                        visible_hits: 0,
                        hidden_hits: 0,
                        suppressed_hits: 0,
                        error_hits: 0,
                        warn_hits: 0,
                        gating_hits: 0,
                    },
                );
                return CheckOutcome {
                    stdout: render_machine(args.format, &meta, &[]),
                    stderr: outcome.stderr,
                    exit_code: 0,
                };
            }
            return outcome;
        }
    };
    t_patches.done();

    let mut stderr = String::new();

    // Name the model that judged this diff — reproducibility + "is my model the
    // same as my colleague's?". On stderr (human) so stdout stays byte-parity;
    // machine formats carry it in the report meta instead.
    if !args.format.is_machine() {
        stderr.push_str(&format!("[argot] model: {}\n", base.model_hash));
    }

    // Fit-time health and one shared, content-driven freshness assessment.
    // `watch` remains visible in status/MCP only; ordinary checks speak only
    // when maintenance is actually recommended.
    if let Some(health) = crate::health::read(&args.argot_dir) {
        let refresh = crate::refresh::assess(Path::new(&args.repo_path), &health, &config);
        match refresh.compatibility {
            crate::refresh::Compatibility::ConfigChanged => stderr.push_str(
                "[argot] argot.toml changed since the last fit — use the `argot-refresh` skill to review scope and mutes before fitting, then commit the refreshed `.argot/` snapshot\n",
            ),
            crate::refresh::Compatibility::ProfileMissing => stderr.push_str(
                "[argot] fit snapshot has no adaptive freshness profile — use the `argot-refresh` skill locally, then review and commit `.argot/`\n",
            ),
            crate::refresh::Compatibility::LineageDiverged => stderr.push_str(
                "[argot] fit snapshot belongs to a different accepted history — use the `argot-refresh` skill on the accepted branch, then review and commit `.argot/`\n",
            ),
            _ => {
                if refresh.recommendation.is_some_and(|r| r.notifies_check()) {
                    let reason = refresh
                        .primary_reason()
                        .map(crate::refresh::RefreshReason::human_summary)
                        .unwrap_or_else(|| "material learned-surface drift detected".to_string());
                    let action = if refresh.next_action
                        == crate::refresh::NextAction::ReviewScopeThenFit
                    {
                        "use the `argot-refresh` skill to review scope and mutes before fitting"
                    } else {
                        "use the `argot-refresh` skill, or run `argot fit` locally on the accepted branch"
                    };
                    stderr.push_str(&format!(
                        "[argot] fit refresh recommended — {reason}; {action}, then review and commit `.argot/`\n"
                    ));
                }
            }
        }
        if !health.drift_candidates.is_empty() {
            let shown: Vec<&str> = health
                .drift_candidates
                .iter()
                .take(3)
                .map(String::as_str)
                .collect();
            let more = if health.drift_candidates.len() > 3 {
                ", …"
            } else {
                ""
            };
            stderr.push_str(&format!(
                "[argot] {} director{} look generated, data-heavy, or vendored and are shaping the voice     ({}{more}) — review `argot init --suggest`
",
                health.drift_candidates.len(),
                if health.drift_candidates.len() != 1 {
                    "ies"
                } else {
                    "y"
                },
                shown.join(", "),
            ));
        }
    }

    // Suppression surfaces from argot.toml (config loaded above): the resolved
    // path set (recommended built-ins + `[exclude].paths`, the same set
    // calibration samples from) and the `[[mute]]` rules (expiry vs `args.today`).
    for w in &vocab_warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    for w in &lock_warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    for w in &config.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let mutes = config.mutes_with(registry, &args.today);
    for w in &mutes.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let migrations = config.migrations();
    for w in &migrations.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }

    // A supported language with no model in this fit is silently dropped by
    // batch_scope below — correct scoring, but the user must know their new
    // Go file has zero coverage until the next fit. (Computed pre-filter:
    // those batches don't survive it.)
    {
        let mut unfitted: Vec<&str> = patches_langs_without_model(&patches, &base.fitted_languages);
        unfitted.sort_unstable();
        unfitted.dedup();
        if !unfitted.is_empty() {
            stderr.push_str(&format!(
                "[argot] this change touches {} file(s) — no model in the current fit; run `argot fit` to cover them\n",
                unfitted.join("/"),
            ));
        }
    }

    // Scope + only/exclude filters. User-ignored files stay scored (marked) so
    // their suppressed hits are countable.
    let filtered: Vec<PatchBatch> = patches
        .into_iter()
        .filter_map(|mut b| {
            match batch_scope(&b.file_path, &base.language_extensions, &path_suppressions) {
                BatchScope::Drop => return None,
                BatchScope::ScoreSuppressed => b.ignored_by_pattern = true,
                BatchScope::Score => {}
            }
            passes_filters(&b.file_path, &args.only, &args.exclude).then_some(b)
        })
        .collect();

    // Files argot doesn't score (unsupported extension: `.env`, CI configs,
    // lockfiles…) but that a registered detector wants — today the scripted
    // rules' `files` globs. Collected only on demand (`wants_unscored_files`),
    // so no non-script build pays for the extra diff; whole file is one hunk.
    let extra_batches: Vec<PatchBatch> =
        if detectors.iter().any(|r| r.detector.wants_unscored_files()) {
            let ps = &path_suppressions;
            let unscored = super::two_sided::collect_two_sided(&args, &|path| {
                super::ext_to_lang(&super::extension(path)).is_none()
                    && matches!(
                        ps.classify(path),
                        crate::suppress::PathScope::InScope
                            | crate::suppress::PathScope::UserIgnored
                    )
                    && passes_filters(path, &args.only, &args.exclude)
            });
            let suppressed_paths: std::collections::HashSet<String> = unscored
                .iter()
                .flat_map(|(_, files)| files.iter().map(|f| f.path.clone()))
                .filter(|p| ps.is_suppressed(p))
                .collect();
            unscored
                .into_iter()
                .flat_map(|(source, files)| {
                    let suppressed_paths = &suppressed_paths;
                    files.into_iter().filter_map(move |f| {
                        let content = f.new?.into_bytes();
                        let lines = content.iter().filter(|&&b| b == b'\n').count().max(1) as u32;
                        Some(PatchBatch {
                            file_path: f.path.clone(),
                            content,
                            hunks: vec![crate::git_walk::HunkSpan {
                                new_start: 1,
                                new_lines: lines,
                            }],
                            source: source.clone(),
                            ignored_by_pattern: suppressed_paths.contains(&f.path),
                        })
                    })
                })
                .collect()
        } else {
            Vec::new()
        };

    // `.h` and `.inc` route to the same model calibrate built them into (the
    // repo's own translation units) — computed once from the working tree.
    let repo_langs = crate::corpus::repo_langs(Path::new(&args.repo_path));

    let mut scan = ScanReport::default();
    // The guardrail's self-protection: does THIS change weaken a rule that
    // was locked before it? Runs outside the detector loop (it guards the
    // framework, not a domain), pinned error, unsuppressable — in CI this is
    // the big red annotation on the PR.
    let tampered = if config.rule_locks.is_empty() && !detected_locks_possible(&args) {
        Vec::new()
    } else {
        let _t = crate::timing::phase("check: tamper pass");
        super::tamper::tamper_findings(&args)
    };
    if !tampered.is_empty() {
        stderr.push_str(&format!(
            "[argot] ⚠ this change weakens a locked guardrail — {} rule-tampered finding(s)\n",
            tampered.len()
        ));
    }

    let hits = {
        let mut ctx = CheckContext {
            batches: &filtered,
            args: &args,
            filter_adapters: &filter_adapters,
            mute_rules: &mutes.active,
            migrations: &migrations.active,
            detect: &config.detect,
            repo_langs,
            settings: &settings,
            registry,
            stderr: &mut stderr,
            scan: &mut scan,
            facts: facts.clone(),
            extra_batches: &extra_batches,
        };
        run_detectors(&mut detectors, &mut ctx)
    };
    drop(detectors);
    let hits: Vec<Finding> = tampered.into_iter().chain(hits).collect();
    let ScanReport {
        hunk_count,
        files_scanned,
    } = scan;

    // A rule set to `off` emits nothing: its findings are dropped entirely
    // (an off rule inside an otherwise-enabled group reaches this filter;
    // internal reasons like `none` have no rule and always pass).
    let hits = {
        let mut hits = hits;
        // A rule set to `off` emits nothing; a rule with a [rules] path scope
        // only keeps findings inside it (applies to every rule — built-in or
        // custom). `rule-tampered` is never path-scoped away (a guardrail's
        // alarm must not have a blind spot).
        hits.retain(|h| {
            settings.severity_of_reason(&h.reason) != RuleSeverity::Off
                && (h.reason == "rule_tampered" || settings.covers_path(&h.reason, &h.file_path))
        });
        hits
    };

    // Display gate: --threshold widens to every hit >= N; otherwise show flagged.
    let threshold_override = args.threshold;
    let above_all: Vec<&Finding> = if let Some(t) = threshold_override {
        hits.iter().filter(|h| h.score >= t).collect()
    } else {
        hits.iter().filter(|h| h.flagged).collect()
    };

    // Suppressed ≠ deleted: drop muted hits from output and exit-code
    // consideration, but say how many were muted (and by which surface).
    let (above, suppressed): (Vec<&Finding>, Vec<&Finding>) = above_all
        .into_iter()
        .partition(|h| h.suppressed_by.is_none());
    if !suppressed.is_empty() {
        let count = |s: SuppressedBy| {
            suppressed
                .iter()
                .filter(|h| h.suppressed_by == Some(s))
                .count()
        };
        stderr.push_str(&format!(
            "{} hits suppressed ({} by argot.toml excludes, {} inline, {} by argot.toml mutes)\n",
            suppressed.len(),
            count(SuppressedBy::Exclude),
            count(SuppressedBy::Inline),
            count(SuppressedBy::Mute),
        ));
    }

    // --min-confidence drops weaker tiers from both output and banner counts.
    let min_idx = confidence_index(&args.min_confidence);
    let visible: Vec<&Finding> = above
        .iter()
        .copied()
        .filter(|h| {
            let t = threshold_override.unwrap_or(h.threshold);
            confidence_index(confidence(&h.reason, h.score, t)) >= min_idx
        })
        .collect();
    let result = result_summary(
        &above,
        &visible,
        suppressed.len(),
        &settings,
        args.error_on_warnings,
    );

    // --add-ignores: edit the working tree instead of reporting.
    if args.add_ignores {
        return add_ignore_comments(&args, &visible, &filter_adapters, stderr);
    }

    // Cache the visible hits for `argot mute <hash>` — written on every check
    // run (best-effort; a read-only tree must not fail the check).
    let last_check: Vec<LastCheckHit> = visible
        .iter()
        .map(|h| LastCheckHit {
            path: h.file_path.clone(),
            reason: h.reason.clone(),
            hash: h.hash.clone(),
            line_start: h.line,
            line_end: h.line_end,
        })
        .collect();
    let _ = write_last_check(&args.argot_dir, &last_check);

    // Machine formats: the serialized document is the entire stdout; skip
    // warnings stay on stderr. Exit semantics match the human path (rule
    // severities decide, see gate_exit_code).
    if args.format.is_machine() {
        let records = hit_records(&visible, &settings, registry);
        let meta = report_meta(
            &args,
            scan_label,
            hunk_count,
            files_scanned,
            &base.model_hash,
            result.clone(),
        );
        let mut stdout = render_machine(args.format, &meta, &records);
        // In the github format, the health notes ("model drifted", "config
        // changed since fit", "language not fitted") become run-level notices —
        // CI logs bury stderr, PR annotations don't.
        if args.format == OutputFormat::Github {
            for line in stderr.lines() {
                if let Some(note) = line.strip_prefix("[argot] ") {
                    stdout.push_str(&format!(
                        "::notice title=argot::{}
",
                        note.replace('%', "%25")
                    ));
                }
            }
        }
        return CheckOutcome {
            stdout,
            stderr,
            exit_code: result.exit_code,
        };
    }

    if visible.is_empty() {
        let mut sorted_exts: Vec<&str> = SUPPORTED_EXTENSIONS.to_vec();
        sorted_exts.sort_unstable();
        let exts = sorted_exts.join(" ");
        let stdout = if hunk_count == 0 {
            format!(
                "No changes to supported files found ({scan_label} scanned).\nSupported extensions: {exts}\n"
            )
        } else if !above.is_empty() {
            format!(
                "All {} hit(s) below confidence '{}' — pass a lower --min-confidence to see them.\n",
                above.len(),
                args.min_confidence
            )
        } else if let Some(t) = threshold_override {
            format!(
                "No configured findings on {hunk_count} scanned hunk{} (threshold {t:.2}).\n",
                if hunk_count == 1 { "" } else { "s" }
            )
        } else {
            format!(
                "No configured findings on {hunk_count} scanned hunk{} — this scan found nothing configured to report.\n",
                if hunk_count == 1 { "" } else { "s" }
            )
        };
        if result.hidden_hits > 0 && result.gating_hits > 0 {
            stderr.push_str(&format!(
                "[argot] {} finding(s) hidden by --min-confidence affect this run's status; lower --min-confidence to reveal them\n",
                result.hidden_hits
            ));
        }
        return CheckOutcome {
            stdout,
            stderr,
            exit_code: result.exit_code,
        };
    }

    let hunk_lines = if args.verbose {
        None
    } else {
        Some(args.hunk_lines)
    };
    let mut stdout = String::new();
    let any_truncated =
        render_results(&visible, hunk_lines, args.use_color, &settings, &mut stdout);

    if any_truncated && !args.verbose {
        stdout.push('\n');
        stdout.push_str("tip: pass --verbose (-v) to expand truncated hunks.\n");
    }
    if result.hidden_hits > 0 && result.gating_hits > 0 {
        stdout.push_str(&format!(
            "\nnote: {} finding(s) hidden by --min-confidence affect this run's status; lower --min-confidence to reveal them.\n",
            result.hidden_hits
        ));
    }

    CheckOutcome {
        stdout,
        stderr,
        exit_code: result.exit_code,
    }
}
/// Outcome of `argot review-mutes` — mute-rot cleanup over the hash-scoped
/// `argot.toml` `[[mute]]` entries.
pub struct ReviewOutcome {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}
/// Re-run the check scoring over the files behind the currently-muted
/// hash-scoped mutes and report which no longer fire. With `prune`, stale hash
/// entries are removed from `argot.toml` (the `[[mute]]` array is rewritten;
/// expired and non-hash entries, and every other section, are kept).
///
/// A mute "still fires" when re-scoring the file's current content (as one
/// full-file hunk plus each sampleable range — stable, reproducible hunk
/// boundaries) yields a flagged hit with the entry's hash. Hits muted from
/// transient diff hunks whose boundaries no longer exist resolve as "no longer
/// fires" — which is exactly mute-rot.
pub fn run_review_mutes(
    repo_path: &str,
    registry: &rules::Registry,
    today: &str,
    prune: bool,
) -> ReviewOutcome {
    let mut stdout = String::new();
    let mut stderr = String::new();

    let repo_root = Path::new(repo_path);
    let config = ArgotConfig::load_with(repo_root, registry);
    for w in &config.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let mutes = config.mutes_with(registry, today);
    for w in &mutes.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let hash_entries: Vec<&SuppressionRule> =
        mutes.active.iter().filter(|r| r.hash.is_some()).collect();
    if hash_entries.is_empty() {
        stdout.push_str("No hash-scoped suppressions to review.\n");
        return ReviewOutcome {
            stdout,
            stderr,
            exit_code: 0,
        };
    }

    stdout.push_str(&format!(
        "Reviewing {} hash-scoped suppression(s)…\n",
        hash_entries.len()
    ));
    // A hash-scoped mute names the exact file `argot mute` minted it from, and
    // its stored hash is a one-way digest of that specific diff hunk — there is
    // no way to recover the hunk from the hash, so re-scoring the live tree can
    // only *guess* at staleness (and would wrongly flag every mute of an
    // edited-but-still-present region, which `--prune` would then delete). The
    // one thing we can assert soundly is existence: a mute can never fire again
    // once its file is gone from both the working tree and HEAD. `--prune` acts
    // on that alone, so it never removes a mute still guarding live code.
    let mut dead_hashes: Vec<String> = Vec::new();
    for entry in &hash_entries {
        let hash = entry.hash.as_deref().expect("filtered on hash presence");
        let present = mute_path_present(repo_path, &entry.path);
        stdout.push_str(&format!(
            "  [{hash}]  {}  {}\n",
            entry.path,
            if present {
                "file present"
            } else {
                "file gone — dead"
            }
        ));
        if !present {
            dead_hashes.push(hash.to_string());
        }
    }

    if dead_hashes.is_empty() {
        stdout.push_str("Every muted file still exists — nothing to prune.\n");
    } else if prune {
        let mut kept: Vec<SuppressionRule> = Vec::new();
        for rule in mutes.active.iter().chain(mutes.expired.iter()) {
            let dead = rule
                .hash
                .as_deref()
                .is_some_and(|h| dead_hashes.iter().any(|s| s == h));
            if !dead {
                kept.push(rule.clone());
            }
        }
        match crate::config::write_mutes(repo_root, &kept) {
            Ok(()) => stdout.push_str(&format!(
                "Pruned {} dead mute(s) from argot.toml.\n",
                dead_hashes.len()
            )),
            Err(e) => {
                stderr.push_str(&format!("error: {e}\n"));
                return ReviewOutcome {
                    stdout,
                    stderr,
                    exit_code: 2,
                };
            }
        }
    } else {
        stdout.push_str(&format!(
            "{} dead mute(s) (file gone) — run `argot review-mutes --prune` to remove them.\n",
            dead_hashes.len()
        ));
    }

    ReviewOutcome {
        stdout,
        stderr,
        exit_code: 0,
    }
}
/// Does the repo still contain the file a hash-scoped mute names? `argot mute`
/// records the hit's exact path, so a plain path is checked against both the
/// working tree and `HEAD` — the mute is only "gone" when the file exists in
/// neither (a file still in HEAD can re-appear in a diff, so its mute is not
/// yet dead). A glob path (only ever hand-edited into a hash entry) is always
/// treated as present so `--prune` never reasons about a pattern.
fn mute_path_present(repo_path: &str, mute_path: &str) -> bool {
    if mute_path.contains(['*', '?', '[']) {
        return true;
    }
    if Path::new(repo_path).join(mute_path).is_file() {
        return true;
    }
    open_repo(repo_path)
        .ok()
        .and_then(|repo| {
            let tree = repo.head().ok()?.peel_to_commit().ok()?.tree().ok()?;
            Some(tree.get_path(Path::new(mute_path)).is_ok())
        })
        .unwrap_or(false)
}
