//! Temporal-holdout FP bench (#92) — leak-free false-positive measurement.
//!
//! The catalog/production FP controls replay commits that are *ancestors* of
//! the fit SHA: the post-image lines they add are already in the training
//! corpus, so FP is ~0 by construction. This mode measures the number a team
//! adopting argot would actually see — "fit on history, check new PRs":
//!
//! 1. fit at an OLD commit (`window` first-parent steps behind the pinned
//!    head) on a full, non-shallow history;
//! 2. replay only non-merge commits STRICTLY AFTER the fit point through the
//!    production `check --commit` path (every replayed commit is asserted to
//!    not be an ancestor of the fit SHA — a leak aborts the run);
//! 3. split every scanned hunk by whether its file existed in the fit tree.
//!    New files and edits to existing files behave very differently, so each
//!    side gets its own rate. A renamed file counts as new (its path is not
//!    in the fit tree), matching what `check` itself would judge.
//!
//! Rates carry commit-level bootstrap 95% CIs (hunks within a commit are
//! correlated, so hunk-level resampling would understate the interval).

use crate::production::{check_args, fit_clone, git_stdout};
use crate::run::{ensure_clone, ensure_sha_checked_out, sync_corpus_config};
use crate::targets::Target;
use anyhow::{bail, Context, Result};
use argot_core::check::{ext_to_lang, extension, run_check};
use serde::Serialize;
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::process::Command;

/// Hits kept verbatim in the report for diagnosis, strongest first.
const MAX_KEPT_HITS: usize = 1000;

#[derive(Debug, Clone)]
pub struct HoldoutOptions {
    pub data_dir: PathBuf,
    /// Per-corpus `argot.toml` source root (`<catalogs_dir>/<corpus>/argot.toml`),
    /// so holdout fits each corpus with the same real-user config as production.
    pub catalogs_dir: PathBuf,
    /// First-parent commits to step back from the pinned head for the fit
    /// point. The replay set is every non-merge commit in `fit..head`.
    pub window: usize,
    /// Minimum eligible hunks for a trustworthy rate; below this the report
    /// is flagged `under_sampled` (widen the window and re-run).
    pub min_hunks: usize,
    pub bootstrap_reps: usize,
    pub seed: u64,
}

impl Default for HoldoutOptions {
    fn default() -> Self {
        HoldoutOptions {
            data_dir: PathBuf::from("benchmarks/data"),
            catalogs_dir: PathBuf::from("benchmarks/catalogs"),
            window: 120,
            min_hunks: 300,
            bootstrap_reps: 1000,
            seed: 0,
        }
    }
}

/// One split's false-positive tally.
#[derive(Debug, Clone, Default, Serialize)]
pub struct SplitFp {
    pub hunks: usize,
    pub hits: usize,
    pub rate: f64,
    /// Commit-level bootstrap 95% CI (percentile method); `None` when the
    /// split has no hunks.
    pub ci95: Option<(f64, f64)>,
}

/// Whether a hit's winning reason is an *over-fire* (a false alarm on the
/// repo's own code) rather than a *novel-pattern detection*.
///
/// The import and call-receiver stages fire only on a symbol/module verified
/// 0-usage in the repo at the fit SHA (a foreign import, an unattested
/// callee/namespace) — argot's one job — so a fire there on a real later commit
/// is a correct detection of a genuinely-new pattern (a new dependency, a new
/// stdlib API), not a false alarm. The bpe stage (distributional surprise over
/// the repo's *own* tokens) and the convention stage carry no 0-usage guarantee,
/// so a fire there is counted as over-fire. Conservative: this *over*-counts
/// over-fire (some bpe fires are real novel patterns), making the reported
/// over-fire rate an honest ceiling on argot's true false-alarm rate.
pub fn is_overfire(reason: &str) -> bool {
    matches!(reason, "bpe" | "convention")
}

/// Per-language tally within one commit. `*_overfire` counts the subset of
/// `*_hits` whose winning reason is an over-fire (see [`is_overfire`]); the
/// complement is a novel-pattern detection.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LangTally {
    pub existing_hunks: usize,
    pub existing_hits: usize,
    pub existing_overfire: usize,
    pub new_hunks: usize,
    pub new_hits: usize,
    pub new_overfire: usize,
}

/// One replayed commit's tallies.
#[derive(Debug, Clone, Serialize)]
pub struct CommitFp {
    pub sha: String,
    pub by_lang: BTreeMap<String, LangTally>,
}

/// One false-positive hit, kept for root-cause diagnosis.
#[derive(Debug, Clone, Serialize)]
pub struct HoldoutHit {
    pub commit: String,
    pub path: String,
    pub line_start: usize,
    pub line_end: usize,
    pub reason: String,
    pub score: f64,
    pub threshold: f64,
    pub new_file: bool,
}

/// Per-language existing/new split.
#[derive(Debug, Clone, Default, Serialize)]
pub struct LangReport {
    pub existing: SplitFp,
    pub new_file: SplitFp,
}

#[derive(Debug, Serialize)]
pub struct HoldoutReport {
    pub corpus: String,
    pub fit_sha: String,
    pub head_sha: String,
    pub window: usize,
    pub n_commits: usize,
    /// Calibrated per-language thresholds from the fit artifact.
    pub thresholds: BTreeMap<String, f64>,
    pub existing: SplitFp,
    pub new_file: SplitFp,
    pub overall: SplitFp,
    /// `existing`/`new_file` decomposed into over-fire (false alarm on the
    /// repo's own code) and novel-pattern detection (fire on a 0-usage symbol
    /// in a real commit — argot working). Each carries its own bootstrap CI;
    /// `*_overfire.rate + *_detection.rate == existing/new_file.rate`.
    pub existing_overfire: SplitFp,
    pub existing_detection: SplitFp,
    pub new_overfire: SplitFp,
    pub new_detection: SplitFp,
    pub languages: BTreeMap<String, LangReport>,
    pub min_hunks: usize,
    /// Total eligible hunks fell below `min_hunks` — widen the window.
    pub under_sampled: bool,
    /// Winning reason of every hit (e.g. `bpe`, `import`), split by file class.
    pub reason_counts_existing: BTreeMap<String, usize>,
    pub reason_counts_new: BTreeMap<String, usize>,
    pub commits: Vec<CommitFp>,
    /// Strongest hits first, capped at `MAX_KEPT_HITS`.
    pub hits: Vec<HoldoutHit>,
}

/// A shallow clone would silently truncate the training history — refuse to
/// fit on one. `git fetch --unshallow` repairs it in place.
pub fn ensure_full_history(repo_dir: &Path) -> Result<()> {
    let shallow = git_stdout(repo_dir, &["rev-parse", "--is-shallow-repository"])?;
    if shallow.trim() == "true" {
        eprintln!("[holdout] {} is shallow — unshallowing", repo_dir.display());
        let st = Command::new("git")
            .arg("-C")
            .arg(repo_dir)
            .args(["fetch", "--quiet", "--unshallow"])
            .status()
            .context("git fetch --unshallow")?;
        if !st.success() {
            bail!("git fetch --unshallow failed in {}", repo_dir.display());
        }
    }
    Ok(())
}

/// The temporal-holdout invariant: a replayed commit must not be an ancestor
/// of the fit SHA (its post-image would be inside the training corpus).
pub fn assert_not_ancestor(repo_dir: &Path, sha: &str, fit_sha: &str) -> Result<()> {
    let st = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(["merge-base", "--is-ancestor", sha, fit_sha])
        .status()
        .context("git merge-base --is-ancestor")?;
    match st.code() {
        Some(0) => bail!(
            "temporal leak: replay commit {sha} is an ancestor of fit SHA {fit_sha} — \
             its content is inside the training corpus"
        ),
        Some(1) => Ok(()),
        _ => bail!("git merge-base --is-ancestor {sha} {fit_sha} errored"),
    }
}

/// Every file path present in the fit tree (the training corpus tree).
pub fn fit_tree_files(repo_dir: &Path, fit_sha: &str) -> Result<HashSet<String>> {
    let out = git_stdout(repo_dir, &["ls-tree", "-r", "--name-only", fit_sha])?;
    Ok(out.lines().map(|l| l.to_string()).collect())
}

/// The fit point (`window` first-parent steps behind `head`) and the replay
/// set (non-merge commits strictly after it, oldest first).
pub fn plan_holdout(
    repo_dir: &Path,
    head_sha: &str,
    window: usize,
) -> Result<(String, Vec<String>)> {
    let fit_sha = git_stdout(repo_dir, &["rev-parse", &format!("{head_sha}~{window}")])
        .with_context(|| format!("resolving {head_sha}~{window} — history shorter than window?"))?
        .trim()
        .to_string();
    let replay = git_stdout(
        repo_dir,
        &[
            "rev-list",
            "--no-merges",
            "--reverse",
            &format!("{fit_sha}..{head_sha}"),
        ],
    )?;
    let shas: Vec<String> = replay
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.to_string())
        .collect();
    if shas.is_empty() {
        bail!("no commits between fit {fit_sha} and head {head_sha}");
    }
    for sha in &shas {
        assert_not_ancestor(repo_dir, sha, &fit_sha)?;
    }
    Ok((fit_sha, shas))
}

fn splitmix64(state: &mut u64) -> u64 {
    *state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    z ^ (z >> 31)
}

/// Commit-level percentile bootstrap over (hunks, hits) pairs. Returns `None`
/// when the population has no hunks. Deterministic for a given seed.
pub fn bootstrap_ci(per_commit: &[(usize, usize)], reps: usize, seed: u64) -> Option<(f64, f64)> {
    let total_hunks: usize = per_commit.iter().map(|(h, _)| h).sum();
    if total_hunks == 0 || per_commit.is_empty() {
        return None;
    }
    let n = per_commit.len();
    let mut state = seed ^ 0xD1B5_4A32_D192_ED03;
    let mut rates: Vec<f64> = Vec::with_capacity(reps);
    for _ in 0..reps {
        let (mut hunks, mut hits) = (0usize, 0usize);
        for _ in 0..n {
            let (h, k) = per_commit[(splitmix64(&mut state) % n as u64) as usize];
            hunks += h;
            hits += k;
        }
        if hunks > 0 {
            rates.push(hits as f64 / hunks as f64);
        }
    }
    if rates.is_empty() {
        return None;
    }
    rates.sort_by(|a, b| a.partial_cmp(b).expect("rates are finite"));
    let lo = rates[((rates.len() as f64) * 0.025) as usize];
    let hi_idx = (((rates.len() as f64) * 0.975).ceil() as usize).saturating_sub(1);
    let hi = rates[hi_idx.min(rates.len() - 1)];
    Some((lo, hi))
}

fn split_fp(per_commit: &[(usize, usize)], reps: usize, seed: u64) -> SplitFp {
    let hunks: usize = per_commit.iter().map(|(h, _)| h).sum();
    let hits: usize = per_commit.iter().map(|(_, k)| k).sum();
    SplitFp {
        hunks,
        hits,
        rate: if hunks == 0 {
            0.0
        } else {
            hits as f64 / hunks as f64
        },
        ci95: bootstrap_ci(per_commit, reps, seed),
    }
}

/// Run one corpus through the temporal-holdout FP flow.
pub fn run_corpus_holdout(target: &Target, opts: &HoldoutOptions) -> Result<HoldoutReport> {
    let repo_dir = ensure_clone(&opts.data_dir, &target.name, &target.url)?;
    let head_sha = target.prs[0].sha.clone();
    ensure_full_history(&repo_dir)?;
    // Detached checkout also fetches once if the pinned object is missing.
    ensure_sha_checked_out(&repo_dir, &head_sha)?;

    let window = target.holdout_window.unwrap_or(opts.window);
    let (fit_sha, replay) = plan_holdout(&repo_dir, &head_sha, window)?;
    eprintln!(
        "[{}] holdout: fit @ {} (head~{}), replaying {} commits",
        target.name,
        &fit_sha[..8],
        window,
        replay.len()
    );
    let fit_files = fit_tree_files(&repo_dir, &fit_sha)?;

    // Fit at the OLD sha. A stale artifact must not leak into this fit.
    ensure_sha_checked_out(&repo_dir, &fit_sha)?;
    let argot_dir = repo_dir.join(".argot");
    if argot_dir.exists() {
        std::fs::remove_dir_all(&argot_dir)?;
    }
    sync_corpus_config(&opts.catalogs_dir, &target.name, &repo_dir)?;
    let (thresholds, _) = fit_clone(&repo_dir, &fit_sha)?;

    let mut commits: Vec<CommitFp> = Vec::new();
    let mut hits: Vec<HoldoutHit> = Vec::new();
    for (i, sha) in replay.iter().enumerate() {
        let mut args = check_args(&repo_dir);
        args.commit = Some(sha.clone());
        let outcome = run_check(args);
        let doc: serde_json::Value = serde_json::from_str(&outcome.stdout).with_context(|| {
            format!(
                "check --commit {sha} did not emit JSON (exit {}): {}",
                outcome.exit_code, outcome.stderr
            )
        })?;

        let mut by_lang: BTreeMap<String, LangTally> = BTreeMap::new();
        for f in doc["files_scanned"].as_array().into_iter().flatten() {
            let (Some(path), Some(hunks)) = (f["path"].as_str(), f["hunks"].as_u64()) else {
                continue;
            };
            let Some(lang) = ext_to_lang(&extension(path)) else {
                continue;
            };
            let tally = by_lang.entry(lang.to_string()).or_default();
            if fit_files.contains(path) {
                tally.existing_hunks += hunks as usize;
            } else {
                tally.new_hunks += hunks as usize;
            }
        }
        for h in doc["hits"].as_array().into_iter().flatten() {
            let Some(path) = h["path"].as_str() else {
                continue;
            };
            let Some(lang) = ext_to_lang(&extension(path)) else {
                continue;
            };
            let new_file = !fit_files.contains(path);
            let reason = h["reason"].as_str().unwrap_or("");
            let overfire = is_overfire(reason);
            let tally = by_lang.entry(lang.to_string()).or_default();
            if new_file {
                tally.new_hits += 1;
                tally.new_overfire += usize::from(overfire);
            } else {
                tally.existing_hits += 1;
                tally.existing_overfire += usize::from(overfire);
            }
            hits.push(HoldoutHit {
                commit: sha[..8].to_string(),
                path: path.to_string(),
                line_start: h["line_start"].as_u64().unwrap_or(0) as usize,
                line_end: h["line_end"].as_u64().unwrap_or(0) as usize,
                reason: reason.to_string(),
                score: h["score"].as_f64().unwrap_or(0.0),
                threshold: h["threshold"].as_f64().unwrap_or(0.0),
                new_file,
            });
        }
        commits.push(CommitFp {
            sha: sha.clone(),
            by_lang,
        });
        if (i + 1) % 25 == 0 {
            eprintln!(
                "[{}] holdout: {}/{} commits",
                target.name,
                i + 1,
                replay.len()
            );
        }
    }

    // Aggregate. Seeds are offset per split so intervals are independent.
    let pairs = |f: &dyn Fn(&LangTally) -> (usize, usize), lang: Option<&str>| {
        commits
            .iter()
            .map(|c| {
                c.by_lang
                    .iter()
                    .filter(|(l, _)| lang.is_none_or(|want| want == l.as_str()))
                    .fold((0usize, 0usize), |(h, k), (_, t)| {
                        let (th, tk) = f(t);
                        (h + th, k + tk)
                    })
            })
            .collect::<Vec<_>>()
    };
    let existing_of = |t: &LangTally| (t.existing_hunks, t.existing_hits);
    let new_of = |t: &LangTally| (t.new_hunks, t.new_hits);
    let overall_of = |t: &LangTally| (t.existing_hunks + t.new_hunks, t.existing_hits + t.new_hits);
    let existing_overfire_of = |t: &LangTally| (t.existing_hunks, t.existing_overfire);
    let existing_detection_of =
        |t: &LangTally| (t.existing_hunks, t.existing_hits - t.existing_overfire);
    let new_overfire_of = |t: &LangTally| (t.new_hunks, t.new_overfire);
    let new_detection_of = |t: &LangTally| (t.new_hunks, t.new_hits - t.new_overfire);
    let reps = opts.bootstrap_reps;
    let existing = split_fp(&pairs(&existing_of, None), reps, opts.seed);
    let new_file = split_fp(&pairs(&new_of, None), reps, opts.seed.wrapping_add(1));
    let overall = split_fp(&pairs(&overall_of, None), reps, opts.seed.wrapping_add(2));
    let existing_overfire = split_fp(
        &pairs(&existing_overfire_of, None),
        reps,
        opts.seed.wrapping_add(3),
    );
    let existing_detection = split_fp(
        &pairs(&existing_detection_of, None),
        reps,
        opts.seed.wrapping_add(4),
    );
    let new_overfire = split_fp(
        &pairs(&new_overfire_of, None),
        reps,
        opts.seed.wrapping_add(5),
    );
    let new_detection = split_fp(
        &pairs(&new_detection_of, None),
        reps,
        opts.seed.wrapping_add(6),
    );

    let lang_names: HashSet<String> = commits
        .iter()
        .flat_map(|c| c.by_lang.keys().cloned())
        .collect();
    let mut languages = BTreeMap::new();
    for (i, lang) in lang_names.into_iter().enumerate() {
        let seed = opts.seed.wrapping_add(10 + 2 * i as u64);
        languages.insert(
            lang.clone(),
            LangReport {
                existing: split_fp(&pairs(&existing_of, Some(&lang)), reps, seed),
                new_file: split_fp(&pairs(&new_of, Some(&lang)), reps, seed.wrapping_add(1)),
            },
        );
    }

    let mut reason_counts_existing: BTreeMap<String, usize> = BTreeMap::new();
    let mut reason_counts_new: BTreeMap<String, usize> = BTreeMap::new();
    for h in &hits {
        let m = if h.new_file {
            &mut reason_counts_new
        } else {
            &mut reason_counts_existing
        };
        *m.entry(h.reason.clone()).or_insert(0) += 1;
    }
    hits.sort_by(|a, b| b.score.partial_cmp(&a.score).expect("scores are finite"));
    hits.truncate(MAX_KEPT_HITS);

    let under_sampled = overall.hunks < opts.min_hunks;
    if under_sampled {
        eprintln!(
            "[{}] holdout UNDER-SAMPLED: {} hunks < {} — widen --holdout-window",
            target.name, overall.hunks, opts.min_hunks
        );
    }

    // Leave the clone back on the pinned head for the next mode.
    ensure_sha_checked_out(&repo_dir, &head_sha)?;

    Ok(HoldoutReport {
        corpus: target.name.clone(),
        fit_sha,
        head_sha,
        window,
        n_commits: commits.len(),
        thresholds,
        existing,
        new_file,
        overall,
        existing_overfire,
        existing_detection,
        new_overfire,
        new_detection,
        languages,
        min_hunks: opts.min_hunks,
        under_sampled,
        reason_counts_existing,
        reason_counts_new,
        commits,
        hits,
    })
}

fn fmt_split(s: &SplitFp) -> String {
    let ci = match s.ci95 {
        Some((lo, hi)) => format!(" [{:.2}–{:.2}]", 100.0 * lo, 100.0 * hi),
        None => String::new(),
    };
    format!("{:.2}%{ci} ({}/{})", 100.0 * s.rate, s.hits, s.hunks)
}

pub fn holdout_summary_markdown(reports: &[HoldoutReport]) -> String {
    let mut out = String::new();
    out.push_str("# argot-bench temporal-holdout report (leak-free FP)\n\n");
    out.push_str(
        "Fit at an old SHA, replay only commits strictly after it. \
         Rates carry commit-level bootstrap 95% CIs.\n\n",
    );
    out.push_str(
        "| Corpus | Fit → head | Commits | FP existing-file | FP new-file | FP overall | Flags |\n\
         |:---|:---|---:|---:|---:|---:|:---|\n",
    );
    for r in reports {
        out.push_str(&format!(
            "| {} | {}…{} (~{} back) | {} | {} | {} | {} | {} |\n",
            r.corpus,
            &r.fit_sha[..8.min(r.fit_sha.len())],
            &r.head_sha[..8.min(r.head_sha.len())],
            r.window,
            r.n_commits,
            fmt_split(&r.existing),
            fmt_split(&r.new_file),
            fmt_split(&r.overall),
            if r.under_sampled {
                format!("⚠️ under-sampled (<{} hunks)", r.min_hunks)
            } else {
                "—".to_string()
            },
        ));
    }
    out.push('\n');
    for r in reports {
        if r.languages.len() > 1 {
            out.push_str(&format!("## {} per-language split\n\n", r.corpus));
            out.push_str("| Language | FP existing-file | FP new-file |\n|:---|---:|---:|\n");
            for (lang, lr) in &r.languages {
                out.push_str(&format!(
                    "| {} | {} | {} |\n",
                    lang,
                    fmt_split(&lr.existing),
                    fmt_split(&lr.new_file),
                ));
            }
            out.push('\n');
        }
    }
    out
}

/// Persist per-corpus JSON + the markdown summary; returns the markdown.
pub fn write_holdout_reports(results_dir: &Path, reports: &[HoldoutReport]) -> Result<String> {
    std::fs::create_dir_all(results_dir)?;
    for r in reports {
        let path = results_dir.join(format!("holdout-{}.json", r.corpus));
        std::fs::write(&path, serde_json::to_string_pretty(r)?)?;
    }
    let md = holdout_summary_markdown(reports);
    std::fs::write(results_dir.join("holdout-report.md"), &md)?;
    Ok(md)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::targets::{PrPin, Target};

    fn git(dir: &Path, args: &[&str]) {
        let st = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .env("GIT_AUTHOR_DATE", "2021-01-01T00:00:00 +0000")
            .env("GIT_COMMITTER_DATE", "2021-01-01T00:00:00 +0000")
            .status()
            .expect("git runs");
        assert!(st.success(), "git {args:?} failed");
    }

    fn head(dir: &Path) -> String {
        git_stdout(dir, &["rev-parse", "HEAD"])
            .unwrap()
            .trim()
            .to_string()
    }

    /// A toy repo with enough idiomatic Python at the fit point for
    /// train/calibrate to work, then two post-fit commits: one edits an
    /// existing file, one adds a new file.
    fn build_toy_repo(dir: &Path) -> (String, String) {
        std::fs::create_dir_all(dir).unwrap();
        git(dir, &["init", "-q", "-b", "main"]);
        git(dir, &["config", "commit.gpgsign", "false"]);
        let files = [
            (
                "stats.py",
                "import math\n\n\ndef mean(values):\n    xs = list(values)\n    if not xs:\n        return 0.0\n    return math.fsum(xs) / len(xs)\n\n\ndef variance(values):\n    xs = list(values)\n    if not xs:\n        return 0.0\n    m = mean(xs)\n    return math.fsum((x - m) ** 2 for x in xs) / len(xs)\n",
            ),
            (
                "text.py",
                "def normalize(line):\n    stripped = line.strip()\n    if not stripped:\n        return \"\"\n    return \" \".join(stripped.split())\n\n\ndef shorten(text, width):\n    if len(text) <= width:\n        return text\n    return text[: width - 1] + \"…\"\n",
            ),
            (
                "store.py",
                "import json\nfrom pathlib import Path\n\n\nclass Store:\n    def __init__(self, path):\n        self.path = Path(path)\n        self.data = {}\n\n    def load(self):\n        if self.path.exists():\n            self.data = json.loads(self.path.read_text())\n        return self.data\n\n    def save(self):\n        self.path.write_text(json.dumps(self.data))\n",
            ),
            (
                "graph.py",
                "from collections import defaultdict\n\n\ndef build_adjacency(edges):\n    adj = defaultdict(list)\n    for a, b in edges:\n        adj[a].append(b)\n        adj[b].append(a)\n    return adj\n\n\ndef neighbors(adj, node):\n    return sorted(adj.get(node, []))\n",
            ),
        ];
        for (name, content) in files {
            std::fs::write(dir.join(name), content).unwrap();
        }
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", "initial modules"]);
        let fit_point = head(dir);

        // Post-fit commit 1: edit an existing file.
        std::fs::write(
            dir.join("text.py"),
            "def normalize(line):\n    stripped = line.strip()\n    if not stripped:\n        return \"\"\n    return \" \".join(stripped.split())\n\n\ndef shorten(text, width):\n    if len(text) <= width:\n        return text\n    return text[: width - 1] + \"…\"\n\n\ndef titlecase(text):\n    words = normalize(text).split()\n    return \" \".join(w.capitalize() for w in words)\n",
        )
        .unwrap();
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", "add titlecase"]);

        // Post-fit commit 2: add a new file.
        std::fs::write(
            dir.join("report.py"),
            "from stats import mean\nfrom text import normalize\n\n\ndef summarize(rows):\n    values = [len(normalize(r)) for r in rows]\n    return {\"count\": len(rows), \"avg_len\": mean(values)}\n",
        )
        .unwrap();
        git(dir, &["add", "-A"]);
        git(dir, &["commit", "-q", "-m", "add report"]);
        (fit_point, head(dir))
    }

    fn tmp(name: &str) -> PathBuf {
        // CARGO_TARGET_TMPDIR only exists for integration tests; unit tests
        // get the OS tempdir.
        std::env::temp_dir().join("argot_bench_holdout").join(name)
    }

    #[test]
    fn plan_holdout_picks_fit_point_and_strictly_future_commits() {
        let dir = tmp("holdout_plan_repo");
        let _ = std::fs::remove_dir_all(&dir);
        let (fit_point, head_sha) = build_toy_repo(&dir);

        let (fit_sha, replay) = plan_holdout(&dir, &head_sha, 2).unwrap();
        assert_eq!(fit_sha, fit_point);
        assert_eq!(replay.len(), 2, "exactly the two post-fit commits");
        assert!(!replay.contains(&fit_sha));
        for sha in &replay {
            assert_not_ancestor(&dir, sha, &fit_sha).unwrap();
        }
    }

    #[test]
    fn assert_not_ancestor_rejects_training_corpus_commits() {
        let dir = tmp("holdout_leak_repo");
        let _ = std::fs::remove_dir_all(&dir);
        let (fit_point, head_sha) = build_toy_repo(&dir);

        // The fit point itself is an ancestor of itself → leak.
        let err = assert_not_ancestor(&dir, &fit_point, &fit_point).unwrap_err();
        assert!(err.to_string().contains("temporal leak"));
        // And so is anything at-or-before it relative to a later fit.
        let err = assert_not_ancestor(&dir, &fit_point, &head_sha).unwrap_err();
        assert!(err.to_string().contains("temporal leak"));
        // A future commit relative to the fit point is fine.
        assert_not_ancestor(&dir, &head_sha, &fit_point).unwrap();
    }

    #[test]
    fn bootstrap_ci_is_deterministic_and_brackets_the_rate() {
        let per_commit: Vec<(usize, usize)> =
            (0..40).map(|i| (10, usize::from(i % 5 == 0))).collect();
        let a = bootstrap_ci(&per_commit, 500, 7).unwrap();
        let b = bootstrap_ci(&per_commit, 500, 7).unwrap();
        assert_eq!(a, b, "same seed → same interval");
        let rate = 8.0 / 400.0;
        assert!(a.0 <= rate && rate <= a.1, "CI {a:?} brackets {rate}");
        assert!(a.0 < a.1);
        // Degenerate cases.
        assert_eq!(bootstrap_ci(&[], 100, 0), None);
        assert_eq!(bootstrap_ci(&[(0, 0)], 100, 0), None);
    }

    /// End-to-end on the toy repo: fit at the old SHA, replay the two future
    /// commits through real `check --commit`, and verify the new/existing
    /// split sees each side.
    #[test]
    fn holdout_end_to_end_splits_new_and_existing_files() {
        let src = tmp("holdout_e2e_src");
        let _ = std::fs::remove_dir_all(&src);
        let (fit_point, head_sha) = build_toy_repo(&src);

        let data_dir = tmp("holdout_e2e_data");
        let _ = std::fs::remove_dir_all(&data_dir);
        let target = Target {
            name: "toy".to_string(),
            url: src.to_str().unwrap().to_string(),
            language: "python".to_string(),
            prs: vec![PrPin {
                pr: 0,
                sha: head_sha.clone(),
            }],
            holdout_window: None,
        };
        let opts = HoldoutOptions {
            data_dir: data_dir.clone(),
            catalogs_dir: data_dir.join("catalogs"),
            window: 2,
            min_hunks: 300,
            bootstrap_reps: 200,
            seed: 0,
        };
        let report = run_corpus_holdout(&target, &opts).unwrap();

        assert_eq!(report.fit_sha, fit_point);
        assert_eq!(report.head_sha, head_sha);
        assert_eq!(report.n_commits, 2);
        // The edited file existed at fit; report.py did not.
        assert!(report.existing.hunks >= 1, "existing-file hunks scanned");
        assert!(report.new_file.hunks >= 1, "new-file hunks scanned");
        assert_eq!(
            report.overall.hunks,
            report.existing.hunks + report.new_file.hunks
        );
        // Two tiny commits can never satisfy the 300-hunk bar.
        assert!(report.under_sampled);
        // The per-commit records classify report.py's hunks as new.
        let last = &report.commits[1];
        let py = last.by_lang.get("python").expect("python tally");
        assert!(py.new_hunks >= 1);
        assert_eq!(py.existing_hunks, 0);
        let first = &report.commits[0];
        let py0 = first.by_lang.get("python").expect("python tally");
        assert!(py0.existing_hunks >= 1);
        assert_eq!(py0.new_hunks, 0);
    }
}
