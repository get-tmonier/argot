//! Production-path bench mode — fixtures scored through the exact `argot
//! check` pipeline.
//!
//! Catalog mode scores fixture hunks through a scorer the harness assembles
//! in-process, which historically gave fixtures a signal surface the real
//! `check` command could not reproduce (fit-time attestation, cluster
//! routing). This mode closes that gap by measurement: for every catalog
//! fixture, the fixture content is spliced into its host file **on disk**,
//! staged with real git, and judged by `run_check --staged` against a real
//! `argot fit` artifact — self-attestation conditions and all. Recall only:
//! the old false-positive control replayed commits that are ANCESTORS of the
//! fit SHA (train-on-test; FP ~0 by construction) and was deleted — honest FP
//! comes from the temporal-holdout mode (issue #92).
//!
//! The recall/FP gap between catalog mode and this mode is itself a tracked
//! metric: it must shrink toward zero as the check path gains the signal
//! surface the bench always had.

use crate::catalog::load_catalog;
use crate::run::{
    ensure_clone, ensure_sha_checked_out, fixture_scoring_input, sync_corpus_argotignore,
    RunOptions,
};
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

/// Run one corpus through the production path (recall only — honest FP is
/// the temporal-holdout mode's job).
pub fn run_corpus_production(target: &Target, opts: &RunOptions) -> Result<ProductionReport> {
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

    // Fit and check this corpus the way a real user of the repo would — with
    // the per-corpus `.argotignore` (e.g. vendored trees muted).
    sync_corpus_argotignore(&opts.catalogs_dir, &target.name, &repo_dir)?;

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

/// Rubric scope tier for a break class (see `benchmarks/catalogs/RUBRIC.md`):
/// `voice` = foreign-to-repo vocabulary argot's stages detect (gated ≥85%);
/// `semantic` = misuse of the repo's own/known vocabulary (reported, ungated —
/// a documented fundamental limit, not a pass/fail line).
pub fn tier_of(category: &str) -> &'static str {
    match category {
        // Gated foreign-symbol classes (RUBRIC v2): a foreign package/library
        // verified 0-usage in the repo — argot's reliable capability.
        "foreign_import" | "foreign_api" | "foreign_concurrency" => "gated",
        "naming_shape_break" => "naming",
        // v1 `wrong_concurrency` is mostly *attested* primitives (pthread where
        // attested, busy-wait) — semantic, not a foreign symbol. Reported.
        "wrong_error_discipline"
        | "wrong_api_within_known_lib"
        | "wrong_concurrency"
        | "semantic_convention" => "semantic",
        // Legacy Python/TS catalogs use an ad-hoc taxonomy predating the RUBRIC.
        _ => "other",
    }
}

/// `(caught, total)` over a report's fixtures in one scope tier.
fn tier_recall(r: &ProductionReport, tier: &str) -> (usize, usize) {
    let mut caught = 0;
    let mut total = 0;
    for f in &r.fixture_results {
        if tier_of(&f.category) == tier {
            total += 1;
            if f.flagged {
                caught += 1;
            }
        }
    }
    (caught, total)
}

fn pct(caught: usize, total: usize) -> f64 {
    if total > 0 {
        100.0 * caught as f64 / total as f64
    } else {
        0.0
    }
}

pub fn production_summary_markdown(
    reports: &[ProductionReport],
    _catalog_recall: &BTreeMap<String, (usize, usize)>,
) -> String {
    let mut out = String::new();
    out.push_str("# argot-bench production-path report\n\n");
    out.push_str(
        "argot's one job: catch code introducing a pattern **foreign to the \
         repo** — the \"unknown to this codebase\" thing an LLM agent drags in. \
         The headline is **novel-pattern catch rate** (foreign import/API/dep, \
         gated ≥85%) paired with **false-alarm rate** (temporal-holdout FP, \
         separate run). Naming/semantic are *secondary coverage*, never gated \
         (see `benchmarks/catalogs/RUBRIC.md`).\n\n",
    );
    let cell = |c: usize, t: usize| {
        if t == 0 {
            "—".to_string()
        } else {
            format!("{c}/{t} ({:.0}%)", pct(c, t))
        }
    };
    out.push_str(
        "| Corpus | Novel-pattern catch (≥85%) | Naming | Semantic | Legacy | Uncaught |\n\
         |:---|---:|---:|---:|---:|:---|\n",
    );
    let (mut g_c, mut g_t, mut n_c, mut n_t, mut s_c, mut s_t, mut o_c, mut o_t) =
        (0, 0, 0, 0, 0, 0, 0, 0);
    for r in reports {
        let (gc, gt) = tier_recall(r, "gated");
        let (nc, nt) = tier_recall(r, "naming");
        let (sc, st) = tier_recall(r, "semantic");
        let (oc, ot) = tier_recall(r, "other");
        g_c += gc;
        g_t += gt;
        n_c += nc;
        n_t += nt;
        s_c += sc;
        s_t += st;
        o_c += oc;
        o_t += ot;
        let gate = if gt == 0 {
            "" // legacy corpus: no gated fixtures
        } else if pct(gc, gt) >= 85.0 {
            " ✅"
        } else {
            " ❌"
        };
        out.push_str(&format!(
            "| {} | {}{} | {} | {} | {} | {} |\n",
            r.corpus,
            cell(gc, gt),
            gate,
            cell(nc, nt),
            cell(sc, st),
            cell(oc, ot),
            if r.uncaught.is_empty() {
                "—".to_string()
            } else {
                r.uncaught.join(", ")
            },
        ));
    }
    out.push_str(&format!(
        "\n**Novel-pattern catch rate (≥85%): {g_c}/{g_t} ({:.1}%)** — THE HEADLINE \
         (pair with false-alarm/FP from `--mode holdout`)\n\
         _secondary coverage (never gated): naming {n_c}/{n_t} ({:.1}%) · \
         semantic {s_c}/{s_t} ({:.1}%) · legacy {o_c}/{o_t} ({:.1}%)_\n",
        pct(g_c, g_t),
        pct(n_c, n_t),
        pct(s_c, s_t),
        pct(o_c, o_t),
    ));
    out
}
