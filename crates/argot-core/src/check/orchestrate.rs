//! `run_check` (the `check` entry point), the freshness/staleness plumbing,
//! and `argot review-mutes`.

#[cfg(feature = "arch")]
use super::arch::ArchDetector;
use super::collect::{batch_scope, collect_patches, passes_filters, BatchScope};
#[cfg(feature = "integrity")]
use super::integrity::IntegrityDetector;
use super::load::{load_scorers, patches_langs_without_model, Loaded};
use super::render::{
    add_ignore_comments, confidence, hit_records, render_machine, render_results, report_meta,
};
#[cfg(feature = "semantic")]
use super::semantic::SemanticDetector;
use super::voice::VoiceDetector;
use super::{ext_to_lang, extension, CheckArgs, CheckOutcome, PatchBatch};
use crate::config::ArgotConfig;
use crate::detector::{run_detectors, CheckContext, RegisteredDetector, ScanReport};
use crate::finding::{Finding, SuppressedBy};
use crate::git_walk::{open_repo, SUPPORTED_EXTENSIONS};
use crate::output::OutputFormat;
use crate::rules::{self, RuleSettings, Severity as RuleSeverity};
use crate::suppress::{write_last_check, LastCheckHit, SuppressionRule};
use std::collections::{HashMap, HashSet};
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
fn gate_exit_code(visible: &[&Finding], settings: &RuleSettings, error_on_warnings: bool) -> i32 {
    let fails = visible
        .iter()
        .any(|h| settings.severity_of_reason(&h.reason) == RuleSeverity::Error)
        || (error_on_warnings && !visible.is_empty());
    if fails {
        1
    } else {
        0
    }
}
/// Freshness walks stop visiting commits here — far past every threshold.
/// The stale-after threshold itself is `[fit] refresh-after` in argot.toml.
pub const FRESHNESS_SCAN_CAP: usize = 200;

/// How many commits HEAD is ahead of the fit SHA (`None` when either end
/// cannot be resolved — shallow clones, rewritten history, detached states
/// must never break check). Public: the CLI's auto-refresh reads the same
/// staleness the in-check warning does.
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
    for oid in walk.flatten().take(FRESHNESS_SCAN_CAP) {
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
                .is_some_and(|rel| crate::train::is_corpus_source(rel, suppressions))
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
/// The anchor freshness is measured against — and the commit a background
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
/// [`in_scope_commits_between`]. Both check's drift warning and the CLI's
/// background auto-refresh read this, so a feature branch full of its own
/// commits reads as fresh (nothing accepted moved), and a docs-only sprint on
/// main does too. Cost on the check path: a couple of ref lookups plus one
/// commit-graph count; the per-commit tree diffs only run when accepted
/// history actually moved, and stop at `stop_at`.
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
/// Entry point (`check.py:main`). Never exits the process — returns the outcome.
pub fn run_check(args: CheckArgs) -> CheckOutcome {
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

    // argot.toml config: excludes + `[detect]` heuristics + `[rules]` +
    // `[[mute]]`. Loaded once here — the `[detect]` markers gate the check-time
    // auto-generated skip built into each scorer, so they must be in place
    // before load_scorers.
    // The run's rule vocabulary: built-ins plus (in a scripted-rules build)
    // whatever `.argot/rules/` carries — discovered before config validation
    // so custom [rules] keys and severities resolve like built-in ones.
    let registry = rules::Registry::builtin();
    let config = ArgotConfig::load_with(Path::new(&args.repo_path), registry);
    // Effective per-rule severities: defaults ⊕ [rules] ⊕ CLI --rule overrides.
    let settings = config.rule_settings_with(registry, &args.rule_overrides);

    let t_load = crate::timing::phase("check: load scorers");
    let Loaded {
        mut scorers,
        filter_adapters,
        language_extensions,
        fit_sha,
        model_hash,
        slices,
        new_file_thresholds,
        fit_corpus_files,
    } = match load_scorers(&args.argot_dir, &config.detect) {
        Ok(l) => l,
        Err((msg, code)) => return CheckOutcome::err(msg, code),
    };
    t_load.done();

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
                    &model_hash,
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
        stderr.push_str(&format!("[argot] model: {model_hash}\n"));
    }

    // Freshness: a stale model turns ordinary drift into noise (a month of
    // drift on a busy workspace measured ~14× the hit volume of a fresh
    // fit). Warn when ACCEPTED history has moved substantially since the fit
    // — commits touching in-scope source on the default-branch line. A
    // feature branch's own commits don't count (they're the code under
    // judgment, not the voice), and docs-only churn doesn't either.
    if let Some(fit_sha) = &fit_sha {
        let stale_after = config.fit_refresh_after;
        if let Some(behind) =
            accepted_source_commits_behind(&args.repo_path, fit_sha, &config, stale_after)
        {
            if behind >= stale_after {
                // No imperative here: the CLI's auto-refresh acts on this
                // drift itself (and says so right after this line).
                stderr.push_str(&format!(
                    "[argot] model fitted {behind}+ source commits ago — voice may have drifted\n"
                ));
            }
        }
    }

    // Fit-time health (persisted by the last fit — foreground OR background,
    // whose stdout is detached): the "is it time to recalibrate?" answer,
    // surfaced by the command users actually run.
    if let Some(health) = crate::health::read(&args.argot_dir) {
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
                "[argot] {} director{} look generated or data-heavy and are shaping the voice                  ({}{more}) — review `argot init --suggest`
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
        if !health.config_fingerprint.is_empty()
            && health.config_fingerprint != crate::health::config_fingerprint(&config)
        {
            stderr.push_str(
                "[argot] argot.toml changed since the last fit — the voice doesn't reflect                  your configuration yet (auto-refresh will refit, or run `argot fit`)
",
            );
        }
    }

    // Suppression surfaces from argot.toml (config loaded above): the resolved
    // path set (recommended built-ins + `[exclude].paths`, the same set
    // calibration samples from) and the `[[mute]]` rules (expiry vs `args.today`).
    for w in &config.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let path_suppressions = config.path_suppressions();
    let mutes = config.mutes(&args.today);
    for w in &mutes.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }

    // A supported language with no model in this fit is silently dropped by
    // batch_scope below — correct scoring, but the user must know their new
    // Go file has zero coverage until the next fit. (Computed pre-filter:
    // those batches don't survive it.)
    {
        let mut unfitted: Vec<&str> = patches_langs_without_model(&patches, &scorers);
        unfitted.sort_unstable();
        unfitted.dedup();
        if !unfitted.is_empty() {
            stderr.push_str(&format!(
                "[argot] this change touches {} file(s) — no model in the current fit;                  run `argot fit` to cover them
",
                unfitted.join("/"),
            ));
        }
    }

    // Scope + only/exclude filters. User-ignored files stay scored (marked) so
    // their suppressed hits are countable.
    let filtered: Vec<PatchBatch> = patches
        .into_iter()
        .filter_map(|mut b| {
            match batch_scope(&b.file_path, &language_extensions, &path_suppressions) {
                BatchScope::Drop => return None,
                BatchScope::ScoreSuppressed => b.ignored_by_pattern = true,
                BatchScope::Score => {}
            }
            passes_filters(&b.file_path, &args.only, &args.exclude).then_some(b)
        })
        .collect();

    // A supported language with no model in this fit gets silently dropped by
    // batch_scope — which is correct scoring, but the user must know their new
    // Go file has zero coverage until the next fit.
    {
        let mut unfitted: Vec<&str> = patches_langs_without_model(&filtered, &scorers);
        unfitted.sort_unstable();
        unfitted.dedup();
        if !unfitted.is_empty() {
            stderr.push_str(&format!(
                "[argot] this change touches {} file(s) — no model in the current fit;                  run `argot fit` to cover them
",
                unfitted.join("/"),
            ));
        }
    }

    // Changeset-wide local bindings: names any file in this change defines.
    // A change that calls what it also defines (a new feature naming its own
    // components) is new code, not foreign voice; only callees neither the
    // corpus nor the changeset knows keep contributing.
    let mut changeset_bindings: HashMap<&'static str, HashSet<String>> = HashMap::new();
    for b in &filtered {
        let ext = extension(&b.file_path);
        let Some(lang) = ext_to_lang(&ext) else {
            continue;
        };
        let Some(adapter) = filter_adapters.get(lang) else {
            continue;
        };
        let source = String::from_utf8_lossy(&b.content);
        changeset_bindings
            .entry(lang)
            .or_default()
            .extend(adapter.callable_definitions(&source));
    }
    for (lang, bindings) in changeset_bindings {
        if let Some(scorer) = scorers.get_mut(lang) {
            scorer.set_changeset_bindings(bindings);
        }
    }

    // `.h` routes to the same C/C++ model calibrate built it into (repo's
    // translation-unit majority) — computed once from the working tree.
    let header_cpp = crate::scoring::calibration::header_is_cpp(Path::new(&args.repo_path));

    // Register this run's detectors — the composition root. The rank pair is
    // the order table (see detector.rs): execution_rank runs additive passes
    // first and the base pass last (stderr interleave parity); merge_rank
    // puts the base pass's findings first (stdout parity). Deleting a rule
    // group deletes exactly its registration lines.
    let mut scan = ScanReport::default();
    let mut detectors: Vec<RegisteredDetector<'_>> = vec![RegisteredDetector {
        detector: Box::new(VoiceDetector {
            scorers: &mut scorers,
            slices: &slices,
            new_file_thresholds: &new_file_thresholds,
            fit_corpus_files: &fit_corpus_files,
        }),
        execution_rank: 3,
        merge_rank: 0,
    }];
    #[cfg(feature = "semantic")]
    detectors.push(RegisteredDetector {
        detector: Box::new(SemanticDetector),
        execution_rank: 0,
        merge_rank: 1,
    });
    #[cfg(feature = "arch")]
    detectors.push(RegisteredDetector {
        detector: Box::new(ArchDetector),
        execution_rank: 1,
        merge_rank: 2,
    });
    #[cfg(feature = "integrity")]
    detectors.push(RegisteredDetector {
        detector: Box::new(IntegrityDetector),
        execution_rank: 2,
        merge_rank: 3,
    });

    let hits = {
        let mut ctx = CheckContext {
            batches: &filtered,
            args: &args,
            filter_adapters: &filter_adapters,
            mute_rules: &mutes.active,
            detect: &config.detect,
            header_cpp,
            settings: &settings,
            stderr: &mut stderr,
            scan: &mut scan,
        };
        run_detectors(&mut detectors, &mut ctx)
    };
    drop(detectors);
    let ScanReport {
        hunk_count,
        files_scanned,
    } = scan;

    // A rule set to `off` emits nothing: its findings are dropped entirely
    // (an off rule inside an otherwise-enabled group reaches this filter;
    // internal reasons like `none` have no rule and always pass).
    let hits = {
        let mut hits = hits;
        hits.retain(|h| settings.severity_of_reason(&h.reason) != RuleSeverity::Off);
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
        let meta = report_meta(&args, scan_label, hunk_count, files_scanned, &model_hash);
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
            exit_code: gate_exit_code(&visible, &settings, args.error_on_warnings),
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
            format!("All {hunk_count} hunk(s) scored below threshold {t:.2} — looks clean.\n")
        } else {
            format!("All {hunk_count} hunk(s) scored below calibrated thresholds — looks clean.\n")
        };
        return CheckOutcome {
            stdout,
            stderr,
            exit_code: 0,
        };
    }

    let hunk_lines = if args.verbose {
        None
    } else {
        Some(args.hunk_lines)
    };
    let mut stdout = String::new();
    let any_truncated = render_results(&visible, hunk_lines, args.use_color, &mut stdout);

    if any_truncated && !args.verbose {
        stdout.push('\n');
        stdout.push_str("tip: pass --verbose (-v) to expand truncated hunks.\n");
    }

    CheckOutcome {
        stdout,
        stderr,
        exit_code: gate_exit_code(&visible, &settings, args.error_on_warnings),
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
pub fn run_review_mutes(repo_path: &str, today: &str, prune: bool) -> ReviewOutcome {
    let mut stdout = String::new();
    let mut stderr = String::new();

    let repo_root = Path::new(repo_path);
    let config = ArgotConfig::load(repo_root);
    for w in &config.warnings {
        stderr.push_str(&format!("[argot] {w}\n"));
    }
    let mutes = config.mutes(today);
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
