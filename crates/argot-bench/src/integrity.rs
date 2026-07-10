//! Test-integrity validation (`--mode integrity-verify` / `integrity-fp`).
//! Feature-gated: drives argot-core's `integrity` sense on the production
//! fit→check path.
//!
//! * `integrity-verify` — the fast recall guard: fit each corpus at its
//!   pinned SHA, then apply every authored gaming fixture
//!   (`benchmarks/catalogs/<corpus>/integrity_fixtures.yaml`) as a real
//!   before/after edit of a REAL test file, stage it with git, and judge it
//!   with `run_check --staged`. Controls (legitimate test-touching edits)
//!   must stay silent.
//! * `integrity-fp` — the co-headline: replay accepted test-touching commits
//!   OUTSIDE the fit-time calibration window through the fitted gates and
//!   count how many would have been flagged.

use crate::production::{check_args, fit_clone};
use crate::run::{ensure_clone, ensure_sha_checked_out, sync_corpus_config};
use crate::targets::Target;
use anyhow::{Context, Result};
use argot_core::check::run_check;
use argot_core::scoring::integrity::{
    changeset_events, FileChange, IntegrityModel, HISTORY_WINDOW, INTEGRITY_FILE,
};
use argot_core::scoring::test_inventory::{is_test_path, language_for_path};
use serde::Deserialize;
use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::Path;
use std::process::Command;

const INTEGRITY_FIXTURES_FILE: &str = "integrity_fixtures.yaml";
/// integrity-fp replays this many accepted first-parent commits per corpus,
/// starting where the fit's calibration window ends (no leakage).
const FP_REPLAY_WINDOW: usize = 600;

// ---------------------------------------------------------------- fixture format

#[derive(Debug, Deserialize)]
struct IntegrityCatalog {
    #[serde(default)]
    fixtures: Vec<IntegrityFix>,
    #[serde(default)]
    controls: Vec<IntegrityFix>,
}

#[derive(Debug, Deserialize)]
struct IntegrityFix {
    id: String,
    /// Fixtures: the rubric tactic. Controls: the legit-edit kind.
    #[serde(default)]
    tactic: String,
    /// The rule expected to fire (fixtures only).
    #[serde(default)]
    rule: String,
    test_file: String,
    #[serde(default)]
    delete_file: bool,
    #[serde(default)]
    edits: Vec<Edit>,
    /// The idiomatic production co-change (optional on tests-only controls).
    #[serde(default)]
    prod_edit: Option<ProdEdit>,
    #[serde(default)]
    #[allow(dead_code)]
    rationale: String,
}

#[derive(Debug, Deserialize)]
struct Edit {
    old: String,
    new: String,
}

#[derive(Debug, Deserialize)]
struct ProdEdit {
    file: String,
    old: String,
    new: String,
}

// ---------------------------------------------------------------- verify mode

#[derive(Debug, Clone, PartialEq)]
enum Outcome {
    /// The expected integrity rule fired on the test file.
    Caught,
    /// An integrity rule fired, but not the declared one.
    CaughtOtherRule(String),
    /// No integrity rule fired — a recall miss to report.
    Missed,
    /// The fixture no longer grounds (edit `old` absent or ambiguous).
    Invalid(String),
}

fn git(repo_dir: &Path, args: &[&str]) -> Result<()> {
    let out = Command::new("git")
        .args(args)
        .current_dir(repo_dir)
        .output()
        .with_context(|| format!("git {args:?}"))?;
    anyhow::ensure!(
        out.status.success(),
        "git {args:?} failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    Ok(())
}

/// Apply one fixture's edits to the working tree. Returns an error string
/// (→ Invalid) when an edit doesn't ground.
fn apply_fix(repo_dir: &Path, fix: &IntegrityFix) -> std::result::Result<(), String> {
    let test_path = repo_dir.join(&fix.test_file);
    if fix.delete_file {
        std::fs::remove_file(&test_path).map_err(|e| format!("delete {}: {e}", fix.test_file))?;
    } else {
        let mut text = std::fs::read_to_string(&test_path)
            .map_err(|e| format!("read {}: {e}", fix.test_file))?;
        for (i, e) in fix.edits.iter().enumerate() {
            let n = text.matches(&e.old).count();
            if n != 1 {
                return Err(format!("edit {} of {}: old matches {n}×", i + 1, fix.id));
            }
            text = text.replacen(&e.old, &e.new, 1);
        }
        std::fs::write(&test_path, text).map_err(|e| e.to_string())?;
    }
    if let Some(p) = &fix.prod_edit {
        let prod_path = repo_dir.join(&p.file);
        let mut text =
            std::fs::read_to_string(&prod_path).map_err(|e| format!("read {}: {e}", p.file))?;
        let n = text.matches(&p.old).count();
        if n != 1 {
            return Err(format!("prod_edit of {}: old matches {n}×", fix.id));
        }
        text = text.replacen(&p.old, &p.new, 1);
        std::fs::write(&prod_path, text).map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Stage the fixture's own paths (never `.argot/` — a later fixture's
/// `reset --hard` would delete staged fit artifacts) and run the production
/// check; return the integrity rules that fired on the fixture's test file.
fn judge(repo_dir: &Path, fix: &IntegrityFix) -> Result<Vec<String>> {
    let test_file = fix.test_file.as_str();
    let mut add: Vec<&str> = vec!["add", "-A", "--", test_file];
    if let Some(p) = &fix.prod_edit {
        add.push(&p.file);
    }
    git(repo_dir, &add)?;
    let mut args = check_args(repo_dir);
    args.staged = true;
    let outcome = run_check(args);
    let doc: serde_json::Value = serde_json::from_str(&outcome.stdout)
        .with_context(|| format!("check JSON: {}", outcome.stderr))?;
    let mut fired = Vec::new();
    if let Some(hits) = doc["hits"].as_array() {
        for h in hits {
            let rule = h["rule"].as_str().unwrap_or_default();
            let path = h["path"].as_str().unwrap_or_default();
            if rule.starts_with("test-") && path == test_file {
                fired.push(rule.to_string());
            }
        }
    }
    Ok(fired)
}

/// Skip the (expensive) per-corpus fit when the existing artifacts were
/// already fitted at this SHA — fixture-authoring iteration reuses them; a
/// fresh clone or SHA bump refits.
fn ensure_fitted(repo_dir: &Path, sha: &str) -> Result<()> {
    let manifest = repo_dir.join(".argot").join("manifest.json");
    if let Ok(raw) = std::fs::read_to_string(&manifest) {
        if let Ok(doc) = serde_json::from_str::<serde_json::Value>(&raw) {
            if doc["fit_commit_sha"].as_str() == Some(sha)
                && repo_dir.join(".argot").join("integrity.json").exists()
            {
                return Ok(());
            }
        }
    }
    fit_clone(repo_dir, sha)?;
    Ok(())
}

fn reset(repo_dir: &Path) -> Result<()> {
    git(repo_dir, &["reset", "-q", "--hard", "HEAD"])?;
    // Keep the fit artifacts and the synced corpus config across fixtures.
    git(
        repo_dir,
        &["clean", "-qfd", "-e", ".argot", "-e", "argot.toml"],
    )
}

pub fn run_integrity_verify(
    targets: &[Target],
    data_dir: &Path,
    catalogs_dir: &Path,
) -> Result<String> {
    let mut md = String::new();
    let _ = writeln!(md, "# integrity-verify — gaming-fixture recall guard\n");
    let _ = writeln!(
        md,
        "| corpus | fixtures | caught | other-rule | missed | invalid | controls | control-FP |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|");

    let mut total = (0usize, 0usize, 0usize, 0usize, 0usize, 0usize, 0usize);
    let mut per_tactic: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    let mut notes: Vec<String> = Vec::new();

    for t in targets {
        let catalog_path = catalogs_dir.join(&t.name).join(INTEGRITY_FIXTURES_FILE);
        let Ok(raw) = std::fs::read_to_string(&catalog_path) else {
            continue; // corpus without integrity fixtures
        };
        let catalog: IntegrityCatalog =
            serde_yaml::from_str(&raw).with_context(|| catalog_path.display().to_string())?;
        let repo_dir = ensure_clone(data_dir, &t.name, &t.url)?;
        let sha = &t.prs[0].sha;
        ensure_sha_checked_out(&repo_dir, sha)?;
        sync_corpus_config(catalogs_dir, &t.name, &repo_dir)?;
        ensure_fitted(&repo_dir, sha)?;

        let (mut caught, mut other, mut missed, mut invalid) = (0usize, 0usize, 0usize, 0usize);
        for fix in &catalog.fixtures {
            reset(&repo_dir)?;
            let outcome = match apply_fix(&repo_dir, fix) {
                Err(e) => Outcome::Invalid(e),
                Ok(()) => {
                    let fired = judge(&repo_dir, fix)?;
                    if fired.iter().any(|r| r == &fix.rule) {
                        Outcome::Caught
                    } else if let Some(r) = fired.first() {
                        Outcome::CaughtOtherRule(r.clone())
                    } else {
                        Outcome::Missed
                    }
                }
            };
            let tactic = per_tactic.entry(fix.tactic.clone()).or_default();
            tactic.1 += 1;
            match &outcome {
                Outcome::Caught => {
                    caught += 1;
                    tactic.0 += 1;
                }
                Outcome::CaughtOtherRule(r) => {
                    other += 1;
                    notes.push(format!(
                        "{}/{}: fired `{r}`, expected `{}`",
                        t.name, fix.id, fix.rule
                    ));
                }
                Outcome::Missed => {
                    missed += 1;
                    notes.push(format!("{}/{}: MISSED ({})", t.name, fix.id, fix.tactic));
                }
                Outcome::Invalid(e) => {
                    invalid += 1;
                    notes.push(format!("{}/{}: INVALID — {e}", t.name, fix.id));
                }
            }
        }

        let mut control_fp = 0usize;
        for fix in &catalog.controls {
            reset(&repo_dir)?;
            match apply_fix(&repo_dir, fix) {
                Err(e) => {
                    notes.push(format!("{}/{} (control): INVALID — {e}", t.name, fix.id));
                }
                Ok(()) => {
                    let fired = judge(&repo_dir, fix)?;
                    if !fired.is_empty() {
                        control_fp += 1;
                        notes.push(format!(
                            "{}/{} (control {}): fired {:?}",
                            t.name, fix.id, fix.tactic, fired
                        ));
                    }
                }
            }
        }
        reset(&repo_dir)?;

        let n = catalog.fixtures.len();
        let nc = catalog.controls.len();
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} | {} |",
            t.name, n, caught, other, missed, invalid, nc, control_fp
        );
        total.0 += n;
        total.1 += caught;
        total.2 += other;
        total.3 += missed;
        total.4 += invalid;
        total.5 += nc;
        total.6 += control_fp;
        eprintln!(
            "[integrity-verify] {}: {}/{} caught, {} other-rule, {} missed, {} invalid, {}/{} control-FP",
            t.name, caught, n, other, missed, invalid, control_fp, nc
        );
    }

    let _ = writeln!(
        md,
        "| **total** | {} | {} | {} | {} | {} | {} | {} |",
        total.0, total.1, total.2, total.3, total.4, total.5, total.6
    );
    let valid = total.0.saturating_sub(total.4);
    if valid > 0 {
        let _ = writeln!(
            md,
            "\ncatch (expected rule) = {}/{} = {:.1}% · any-integrity-rule = {}/{} = {:.1}%",
            total.1,
            valid,
            100.0 * total.1 as f64 / valid as f64,
            total.1 + total.2,
            valid,
            100.0 * (total.1 + total.2) as f64 / valid as f64,
        );
    }
    let _ = writeln!(md, "\n## per tactic (caught/total)\n");
    for (tactic, (c, n)) in &per_tactic {
        let _ = writeln!(md, "- {tactic}: {c}/{n}");
    }
    if !notes.is_empty() {
        let _ = writeln!(md, "\n## findings\n");
        for n in &notes {
            let _ = writeln!(md, "- {n}");
        }
    }
    Ok(md)
}

// ---------------------------------------------------------------- fp mode

/// Replay accepted test-touching commits through the fitted gates, skipping
/// the fit's own calibration window (no leakage), and count flagged commits.
pub fn run_integrity_fp(
    targets: &[Target],
    data_dir: &Path,
    catalogs_dir: &Path,
) -> Result<String> {
    let mut md = String::new();
    let _ = writeln!(md, "# integrity-fp — accepted-history false positives\n");
    let _ = writeln!(
        md,
        "| corpus | replayed | test-touching | flagged(any) | any rate | flagged(gating) | gating rate | flagged events |"
    );
    let _ = writeln!(md, "|---|---|---|---|---|---|---|---|");

    let (mut sum_tt, mut sum_flagged, mut sum_gating) = (0usize, 0usize, 0usize);
    for t in targets {
        let repo_dir = ensure_clone(data_dir, &t.name, &t.url)?;
        let sha = &t.prs[0].sha;
        ensure_sha_checked_out(&repo_dir, sha)?;
        sync_corpus_config(catalogs_dir, &t.name, &repo_dir)?;
        ensure_fitted(&repo_dir, sha)?;
        let model_raw = std::fs::read_to_string(repo_dir.join(".argot").join(INTEGRITY_FILE))
            .unwrap_or_default();
        let model =
            IntegrityModel::from_json(&model_raw).unwrap_or_else(IntegrityModel::permissive);

        let ReplayStats {
            replayed,
            test_touching,
            flagged,
            flagged_gating,
            kinds,
        } = replay_history(&repo_dir, &model, HISTORY_WINDOW, FP_REPLAY_WINDOW)?;
        let rate = 100.0 * flagged as f64 / test_touching.max(1) as f64;
        let gating_rate = 100.0 * flagged_gating as f64 / test_touching.max(1) as f64;
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {:.2}% | {} | {:.2}% | {} |",
            t.name,
            replayed,
            test_touching,
            flagged,
            rate,
            flagged_gating,
            gating_rate,
            kinds
                .iter()
                .map(|(k, n)| format!("{k}×{n}"))
                .collect::<Vec<_>>()
                .join(" "),
        );
        sum_tt += test_touching;
        sum_flagged += flagged;
        sum_gating += flagged_gating;
        eprintln!(
            "[integrity-fp] {}: any {flagged}/{test_touching} ({rate:.2}%) · gating {flagged_gating} ({gating_rate:.2}%)",
            t.name
        );
    }
    let _ = writeln!(
        md,
        "\n**overall: any-severity {}/{} = {:.2}% · gating (error-severity rules) {}/{} = {:.2}%** (gate: gating ≤ 2%)",
        sum_flagged,
        sum_tt,
        100.0 * sum_flagged as f64 / sum_tt.max(1) as f64,
        sum_gating,
        sum_tt,
        100.0 * sum_gating as f64 / sum_tt.max(1) as f64,
    );
    Ok(md)
}

/// Walk first-parent history from HEAD: skip `skip` commits (the calibration
/// window), then evaluate `take` commits through the fitted gates.
fn replay_history(
    repo_dir: &Path,
    model: &IntegrityModel,
    skip: usize,
    take: usize,
) -> Result<ReplayStats> {
    let repo = git2::Repository::discover(repo_dir)?;
    let head = repo.head()?.peel_to_commit()?;
    let mut revwalk = repo.revwalk()?;
    revwalk.set_sorting(git2::Sort::TOPOLOGICAL)?;
    revwalk.push(head.id())?;
    revwalk.simplify_first_parent()?;

    let mut walked = 0usize;
    let mut replayed = 0usize;
    let mut test_touching = 0usize;
    let mut flagged = 0usize;
    let mut flagged_gating = 0usize;
    let mut kinds: BTreeMap<String, usize> = BTreeMap::new();

    for oid in revwalk {
        let oid = oid?;
        let Ok(commit) = repo.find_commit(oid) else {
            continue;
        };
        if commit.parent_count() != 1 {
            continue;
        }
        walked += 1;
        if walked <= skip {
            continue;
        }
        if replayed >= take {
            break;
        }
        replayed += 1;
        let parent = commit.parent(0)?;
        let (old_tree, new_tree) = (parent.tree()?, commit.tree()?);
        let mut diff = repo.diff_tree_to_tree(Some(&old_tree), Some(&new_tree), None)?;
        let _ = diff.find_similar(Some(&mut git2::DiffFindOptions::new()));

        let mut files = Vec::new();
        let mut touches_test = false;
        for d in diff.deltas() {
            let new_path = d.new_file().path().map(|p| p.to_string_lossy().to_string());
            let old_path = d.old_file().path().map(|p| p.to_string_lossy().to_string());
            let path = new_path.clone().or(old_path.clone()).unwrap_or_default();
            let Some(lang) = language_for_path(&path) else {
                continue;
            };
            if is_test_path(&path, lang) {
                touches_test = true;
            }
            let old = old_path
                .as_deref()
                .filter(|_| d.status() != git2::Delta::Added)
                .and_then(|p| blob_text(&repo, &old_tree, p));
            let new = new_path
                .as_deref()
                .filter(|_| d.status() != git2::Delta::Deleted)
                .and_then(|p| blob_text(&repo, &new_tree, p));
            if old.is_none() && new.is_none() {
                continue;
            }
            files.push(FileChange { path, old, new });
        }
        let events: Vec<_> = changeset_events(&files)
            .into_iter()
            .filter(|e| model.enabled(e.kind))
            .collect();
        if !events.is_empty() {
            touches_test = true;
        }
        if !touches_test {
            continue;
        }
        test_touching += 1;
        if !events.is_empty() {
            flagged += 1;
            // Gating = events whose rule ships error-severity by default.
            if events.iter().any(|e| {
                argot_core::rules::rule_for_reason(e.kind.reason())
                    .map(|r| r.default_severity == argot_core::rules::Severity::Error)
                    .unwrap_or(true)
            }) {
                flagged_gating += 1;
            }
            let mut per: BTreeMap<&str, usize> = BTreeMap::new();
            for e in &events {
                *kinds.entry(e.kind.key().to_string()).or_default() += 1;
                *per.entry(e.kind.key()).or_default() += 1;
            }
            let sha7: String = oid.to_string().chars().take(7).collect();
            let detail: Vec<String> = per.iter().map(|(k, n)| format!("{k}x{n}")).collect();
            let sample = events
                .first()
                .map(|e| format!("{}::{}", e.file, e.test_name))
                .unwrap_or_default();
            eprintln!(
                "[integrity-fp]   flagged {sha7} {} e.g. {sample}",
                detail.join(" ")
            );
        }
    }
    Ok(ReplayStats {
        replayed,
        test_touching,
        flagged,
        flagged_gating,
        kinds,
    })
}

/// Counters from one accepted-history replay.
struct ReplayStats {
    replayed: usize,
    test_touching: usize,
    flagged: usize,
    flagged_gating: usize,
    kinds: BTreeMap<String, usize>,
}

fn blob_text(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
    let entry = tree.get_path(Path::new(path)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    if blob.size() > 400_000 {
        return None;
    }
    Some(String::from_utf8_lossy(blob.content()).to_string())
}
