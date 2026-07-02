//! Production-path bench mode — fixtures scored through the exact `argot
//! check` pipeline.
//!
//! Catalog mode scores fixture hunks through a scorer the harness assembles
//! in-process, which historically gave fixtures a signal surface the real
//! `check` command could not reproduce (fit-time attestation, cluster
//! routing). This mode closes that gap by measurement: for every catalog
//! fixture, the fixture content is spliced into its host file **on disk**,
//! staged with real git, and judged by `run_check --staged` against a real
//! `argot fit` artifact — self-attestation conditions and all. The false-
//! positive control replays the corpus's own recent commits through
//! `run_check --commit`.
//!
//! The recall/FP gap between catalog mode and this mode is itself a tracked
//! metric: it must shrink toward zero as the check path gains the signal
//! surface the bench always had.

use crate::catalog::load_catalog;
use crate::run::{ensure_clone, ensure_sha_checked_out, fixture_scoring_input, RunOptions};
use crate::targets::Target;
use anyhow::{bail, Context, Result};
use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;
use std::process::Command;

/// Fixed "today" for suppression expiry — production checks are date-driven
/// only through suppressions.yaml, which the bench clones never carry.
const BENCH_TODAY: &str = "2026-01-01";

#[derive(Debug, Clone, Serialize)]
pub struct ProdFixtureResult {
    pub id: String,
    pub category: String,
    pub language: Option<String>,
    pub flagged: bool,
    /// Winning reasons of the hits on the host file (empty when uncaught).
    pub reasons: Vec<String>,
    /// Highest hit score on the host file (0.0 when uncaught).
    pub max_score: f64,
}

#[derive(Debug, Serialize)]
pub struct ProductionReport {
    pub corpus: String,
    pub n_fixtures: usize,
    pub n_caught: usize,
    pub uncaught: Vec<String>,
    /// Calibrated per-language thresholds from the fit artifact.
    pub thresholds: BTreeMap<String, f64>,
    /// Per-language resolved cluster-rare thresholds from the fit artifact.
    pub resolved_rare: BTreeMap<String, u64>,
    /// FP control: real history commits replayed through `check --commit`.
    pub fp_commits: usize,
    pub fp_hunks: usize,
    pub fp_hits: usize,
    pub fp_rate: f64,
    pub fixture_results: Vec<ProdFixtureResult>,
}

pub(crate) fn git_ok(repo_dir: &Path, args: &[&str]) -> Result<()> {
    let st = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(args)
        .status()
        .with_context(|| format!("running git {args:?}"))?;
    if !st.success() {
        bail!("git {args:?} failed in {}", repo_dir.display());
    }
    Ok(())
}

pub(crate) fn git_stdout(repo_dir: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(args)
        .output()
        .with_context(|| format!("running git {args:?}"))?;
    if !out.status.success() {
        bail!("git {args:?} failed in {}", repo_dir.display());
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

pub(crate) fn check_args(repo_dir: &Path) -> CheckArgs {
    CheckArgs {
        repo_path: repo_dir.to_string_lossy().into_owned(),
        reference: String::new(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: repo_dir.join(".argot"),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_severity: "unusual".to_string(),
        use_color: false,
        format: argot_core::output::OutputFormat::Json,
        today: BENCH_TODAY.to_string(),
    }
}

/// `argot fit` (train → calibrate at production defaults) into the clone's
/// `.argot/`. Returns (thresholds, resolved rare thresholds) per language.
pub(crate) fn fit_clone(
    repo_dir: &Path,
    primary_sha: &str,
) -> Result<(BTreeMap<String, f64>, BTreeMap<String, u64>)> {
    let argot_dir = repo_dir.join(".argot");
    std::fs::create_dir_all(&argot_dir)?;
    argot_core::train::run_train(
        repo_dir,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )?;
    let opts = CalibrateOptions {
        repo_sha: primary_sha.to_string(),
        timestamp_utc: "1970-01-01T00:00:00+00:00".to_string(),
        ..Default::default()
    };
    run_calibrate(
        repo_dir,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &opts,
    )?;
    let config: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(
        argot_dir.join("scorer-config.json"),
    )?)?;
    let mut thresholds = BTreeMap::new();
    let mut resolved_rare = BTreeMap::new();
    if let Some(langs) = config["languages"].as_object() {
        for (lang, cfg) in langs {
            if let Some(t) = cfg["threshold"].as_f64() {
                thresholds.insert(lang.clone(), t);
            }
            if let Some(r) = cfg["call_receiver_cluster_rare_threshold"].as_u64() {
                resolved_rare.insert(lang.clone(), r);
            }
        }
    }
    Ok((thresholds, resolved_rare))
}

/// Run one corpus through the production path. `fp_commits` real history
/// commits feed the FP control (0 disables it).
pub fn run_corpus_production(
    target: &Target,
    opts: &RunOptions,
    fp_commits: usize,
) -> Result<ProductionReport> {
    let catalog_dir = opts.catalogs_dir.join(&target.name);
    let catalog = load_catalog(&catalog_dir)?;
    let repo_dir = ensure_clone(&opts.data_dir, &target.name, &target.url)?;
    let primary = &target.prs[0];
    ensure_sha_checked_out(&repo_dir, &primary.sha)?;

    // A stale artifact from a previous run must not leak into this fit.
    let argot_dir = repo_dir.join(".argot");
    if argot_dir.exists() {
        std::fs::remove_dir_all(&argot_dir)?;
    }

    eprintln!(
        "[{}] production fit (train → calibrate) @ {}",
        target.name,
        &primary.sha[..8.min(primary.sha.len())]
    );
    let (thresholds, resolved_rare) = fit_clone(&repo_dir, &primary.sha)?;

    // --- Recall: plant each fixture on disk, stage, check, restore.
    let mut fixture_results = Vec::new();
    for fx in &catalog.fixtures {
        let input = fixture_scoring_input(&catalog_dir, fx, &repo_dir)?;
        let Some((spliced, _, _)) = input.host_context else {
            eprintln!(
                "[{}] skipping {}: no host-injection metadata (production mode plants on disk)",
                target.name, fx.id
            );
            continue;
        };
        let host_file = fx
            .host_file
            .as_deref()
            .expect("host_context implies host_file");
        let host_path = repo_dir.join(host_file);

        // Plant → stage → check --staged → restore (worktree AND index).
        let mut planted = spliced;
        if !planted.ends_with('\n') {
            planted.push('\n');
        }
        std::fs::write(&host_path, &planted)
            .with_context(|| format!("planting {} into {host_file}", fx.id))?;
        let staged_result = git_ok(&repo_dir, &["add", "--", host_file]).and_then(|()| {
            let mut args = check_args(&repo_dir);
            args.staged = true;
            let outcome = run_check(args);
            serde_json::from_str::<serde_json::Value>(&outcome.stdout)
                .context("check --staged emits JSON")
        });
        git_ok(&repo_dir, &["checkout", "-q", "HEAD", "--", host_file])?;
        let doc = staged_result?;

        let hits: Vec<&serde_json::Value> = doc["hits"]
            .as_array()
            .map(|a| a.iter().filter(|h| h["path"] == host_file).collect())
            .unwrap_or_default();
        let reasons: Vec<String> = hits
            .iter()
            .filter_map(|h| h["reason"].as_str().map(String::from))
            .collect();
        let max_score = hits
            .iter()
            .filter_map(|h| h["score"].as_f64())
            .fold(0.0f64, f64::max);
        fixture_results.push(ProdFixtureResult {
            id: fx.id.clone(),
            category: fx.category.clone(),
            language: fx.language.clone(),
            flagged: !hits.is_empty(),
            reasons,
            max_score,
        });
    }

    // --- FP control: replay real history commits through `check --commit`.
    let mut fp_hunks = 0usize;
    let mut fp_hits = 0usize;
    let mut fp_checked = 0usize;
    if fp_commits > 0 {
        let shas = git_stdout(
            &repo_dir,
            &[
                "rev-list",
                "--no-merges",
                "-n",
                &fp_commits.to_string(),
                &primary.sha,
            ],
        )?;
        for sha in shas.lines().filter(|l| !l.trim().is_empty()) {
            let mut args = check_args(&repo_dir);
            args.commit = Some(sha.to_string());
            let outcome = run_check(args);
            let Ok(doc) = serde_json::from_str::<serde_json::Value>(&outcome.stdout) else {
                continue;
            };
            fp_checked += 1;
            fp_hunks += doc["hunks_scanned"].as_u64().unwrap_or(0) as usize;
            fp_hits += doc["hits"].as_array().map(|a| a.len()).unwrap_or(0);
        }
    }

    let n_fixtures = fixture_results.len();
    let n_caught = fixture_results.iter().filter(|f| f.flagged).count();
    let uncaught: Vec<String> = fixture_results
        .iter()
        .filter(|f| !f.flagged)
        .map(|f| f.id.clone())
        .collect();
    Ok(ProductionReport {
        corpus: target.name.clone(),
        n_fixtures,
        n_caught,
        uncaught,
        thresholds,
        resolved_rare,
        fp_commits: fp_checked,
        fp_hunks,
        fp_hits,
        fp_rate: if fp_hunks == 0 {
            0.0
        } else {
            fp_hits as f64 / fp_hunks as f64
        },
        fixture_results,
    })
}

/// Persist production reports and render the summary (with the catalog-mode
/// gap column when catalog reports for the same corpora are supplied).
pub fn write_production_reports(
    results_dir: &Path,
    reports: &[ProductionReport],
    catalog_recall: &BTreeMap<String, (usize, usize)>,
) -> Result<String> {
    std::fs::create_dir_all(results_dir)?;
    for r in reports {
        let path = results_dir.join(format!("production-{}.json", r.corpus));
        std::fs::write(&path, serde_json::to_string_pretty(r)?)?;
    }
    let md = production_summary_markdown(reports, catalog_recall);
    std::fs::write(results_dir.join("production-report.md"), &md)?;
    Ok(md)
}

pub fn production_summary_markdown(
    reports: &[ProductionReport],
    catalog_recall: &BTreeMap<String, (usize, usize)>,
) -> String {
    let mut out = String::new();
    out.push_str("# argot-bench production-path report\n\n");
    out.push_str(
        "| Corpus | Recall (production) | Recall (catalog) | Gap | FP rate | FP hits/hunks (commits) | Uncaught |\n\
         |:---|---:|---:|---:|---:|---:|:---|\n",
    );
    let mut total_caught = 0usize;
    let mut total_fixtures = 0usize;
    for r in reports {
        total_caught += r.n_caught;
        total_fixtures += r.n_fixtures;
        let prod_pct = if r.n_fixtures > 0 {
            100.0 * r.n_caught as f64 / r.n_fixtures as f64
        } else {
            0.0
        };
        let (cat_cell, gap_cell) = match catalog_recall.get(&r.corpus) {
            Some((caught, total)) if *total > 0 => {
                let cat_pct = 100.0 * *caught as f64 / *total as f64;
                (
                    format!("{caught}/{total} ({cat_pct:.1}%)"),
                    format!("{:+.1}pp", prod_pct - cat_pct),
                )
            }
            _ => ("—".to_string(), "—".to_string()),
        };
        out.push_str(&format!(
            "| {} | {}/{} ({:.1}%) | {} | {} | {:.2}% | {}/{} ({}) | {} |\n",
            r.corpus,
            r.n_caught,
            r.n_fixtures,
            prod_pct,
            cat_cell,
            gap_cell,
            100.0 * r.fp_rate,
            r.fp_hits,
            r.fp_hunks,
            r.fp_commits,
            if r.uncaught.is_empty() {
                "—".to_string()
            } else {
                r.uncaught.join(", ")
            },
        ));
    }
    if total_fixtures > 0 {
        out.push_str(&format!(
            "\n**Total production recall: {total_caught}/{total_fixtures} ({:.1}%)**\n",
            100.0 * total_caught as f64 / total_fixtures as f64
        ));
    }
    out
}
