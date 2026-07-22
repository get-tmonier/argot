//! argot-bench — recall / false-positive evaluation over the pinned corpora.
//!
//! Rust successor to the retired Python `argot_bench` harness. Reads
//! `benchmarks/targets.yaml` + `benchmarks/catalogs/`, scores break fixtures
//! and real-PR control hunks with the production scorer, and writes per-corpus
//! JSON + a markdown summary under `--results-dir`.

use anyhow::{Context, Result};
use argot_bench::scorer::BenchKnobs;
use argot_bench::{accept_brief, holdout, pool, production, report, run, targets};

/// Current time as an ISO-8601 UTC string (dashboard metadata only).
fn iso_utc_now() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let (rem, days) = (secs % 86400, (secs / 86400) as i64);
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    // Howard Hinnant's civil-from-days.
    let z = days + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}+00:00")
}
use clap::Parser;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Parser, Debug)]
#[command(name = "argot-bench", about = "argot benchmark harness")]
struct Cli {
    /// Comma-separated corpus names (default: all targets).
    #[arg(long, value_delimiter = ',')]
    corpus: Option<Vec<String>>,

    /// Smoke mode: 1 fixture per category, 50 controls, small n_cal, primary PR only.
    #[arg(long)]
    quick: bool,

    /// Corpora to run concurrently — each corpus is an independent
    /// fit → replay, so this scales wall-clock near-linearly.
    /// 0 = auto (min(cores, 8)).
    #[arg(long, default_value_t = 0)]
    jobs: usize,

    /// Bench mode: `honest` (the headline numbers — production-path recall
    /// on the curated catalogs + leak-free temporal-holdout FP, merged into
    /// one dashboard), `production` (recall only), `catalog` (in-process
    /// scorer, historical continuity), `both` (catalog + production), or
    /// `holdout` (temporal-holdout FP only).
    #[arg(long, default_value = "honest")]
    mode: String,

    /// Holdout mode: first-parent commits to step back from the pinned head
    /// for the fit point (the replay window).
    #[arg(long, default_value_t = 120)]
    holdout_window: usize,

    /// Holdout mode: minimum eligible hunks before a corpus is flagged
    /// under-sampled.
    #[arg(long, default_value_t = 300)]
    min_holdout_hunks: usize,

    /// Holdout mode: bootstrap resamples for the 95% CIs.
    #[arg(long, default_value_t = 1000)]
    bootstrap_reps: usize,

    #[arg(long, default_value = "benchmarks/targets.yaml")]
    targets: PathBuf,

    #[arg(long, default_value = "benchmarks/catalogs")]
    catalogs_dir: PathBuf,

    #[arg(long, default_value = "benchmarks/data")]
    data_dir: PathBuf,

    #[arg(long, default_value = "benchmarks/results/latest")]
    results_dir: PathBuf,

    /// Pinned raw records to aggregate in `--mode accept-brief`.
    #[arg(long)]
    accept_brief_records: Option<PathBuf>,

    /// Pinned accepted-change manifest to replay in `--mode accept-brief`.
    /// Requires the repositories named in the manifest to already be present.
    #[arg(long)]
    accept_brief_manifest: Option<PathBuf>,

    /// Reservoir-free deterministic control subsample per PR snapshot.
    #[arg(long)]
    sample_controls: Option<usize>,

    /// Keep per-control scores in the report JSON (large).
    #[arg(long)]
    keep_controls: bool,

    /// List the corpora in targets.yaml and exit.
    #[arg(long)]
    list_corpora: bool,

    // --- scorer knobs (era-13.5 canonical defaults) ---
    #[arg(long, default_value_t = 100)]
    n_cal: usize,
    #[arg(long, default_value_t = 0)]
    seed: u64,
    #[arg(long, default_value_t = 7)]
    threshold_n_seeds: usize,
    #[arg(long, default_value_t = 2.0)]
    call_receiver_alpha: f64,
    #[arg(long, default_value_t = 5)]
    call_receiver_cap: usize,
    #[arg(long, default_value_t = 2.0)]
    call_receiver_root_bonus: f64,
    #[arg(long, default_value_t = 8)]
    call_receiver_clusters: usize,
    #[arg(long, default_value_t = 0)]
    call_receiver_cluster_seed: u64,
    #[arg(long, default_value_t = 5.0)]
    call_receiver_cluster_bonus: f64,
    #[arg(long, default_value_t = 2)]
    call_receiver_cluster_rare_threshold: usize,
    #[arg(long, default_value_t = 0)]
    call_receiver_cluster_size_min: usize,
    #[arg(long)]
    no_typicality_filter: bool,
    /// Symmetric calibration: apply optional contributions on the cal side too.
    #[arg(long)]
    apply_optional_contributions_to_cal: bool,
    /// Disable the per-corpus cluster-rare auto-detect probe.
    #[arg(long)]
    no_auto_select_asym_cal: bool,
    #[arg(long, default_value_t = 0.05)]
    asym_fire_rate_threshold: f64,
    #[arg(long, default_value_t = 1000)]
    asym_probe_n: usize,
    /// Rarity weighting for the cluster branches: off | linear-df |
    /// gated-df:<min_df> | log-df.
    #[arg(long, default_value = "off")]
    rarity_weighting: String,
    /// Calibration-hunk source: random | diff.
    #[arg(long, default_value = "random")]
    calibration_source: String,
    /// Comma-separated shape-primitive names to enable on the scoring path.
    #[arg(long, default_value = "", value_delimiter = ',')]
    enable_shape_primitives: Vec<String>,
    /// Disable the parse-error host fallback.
    #[arg(long)]
    no_parse_error_fallback: bool,
    /// Disable the convention-rarity stage.
    #[arg(long)]
    no_conventions: bool,
    /// Convention-rarity bonus magnitude.
    #[arg(long, default_value_t = 5.0)]
    convention_bonus: f64,
}

fn parse_rarity_weighting(s: &str) -> Result<argot_core::scoring::call_receiver::RarityWeighting> {
    use argot_core::scoring::call_receiver::RarityWeighting as W;
    Ok(match s {
        "off" => W::Off,
        "linear-df" => W::LinearDf,
        "log-df" => W::LogDf,
        other => match other.strip_prefix("gated-df:") {
            Some(n) => W::GatedDf {
                min_df: n.parse().context("gated-df:<min_df> needs an integer")?,
            },
            None => anyhow::bail!("unknown rarity weighting {other:?}"),
        },
    })
}

fn main() -> ExitCode {
    match real_main() {
        Ok(code) => code,
        Err(e) => {
            eprintln!("error: {e:#}");
            ExitCode::FAILURE
        }
    }
}

fn real_main() -> Result<ExitCode> {
    let cli = Cli::parse();

    if cli.mode == "accept-brief" {
        let records = match (&cli.accept_brief_records, &cli.accept_brief_manifest) {
            (Some(records), None) => accept_brief::load_records(records)?,
            (None, Some(manifest)) => {
                let raw = std::fs::read_to_string(manifest)
                    .with_context(|| format!("reading {}", manifest.display()))?;
                let manifest: accept_brief::ReplayManifest = serde_json::from_str(&raw)
                    .context("accept-brief manifest must be JSON")?;
                let output = cli.results_dir.join("raw");
                let records = accept_brief::replay_manifest(&manifest, &output)?;
                std::fs::create_dir_all(&cli.results_dir)?;
                let jsonl = records
                    .iter()
                    .map(serde_json::to_string)
                    .collect::<Result<Vec<_>, _>>()?
                    .join("\n");
                std::fs::write(cli.results_dir.join("records.jsonl"), format!("{jsonl}\n"))?;
                records
            }
            _ => anyhow::bail!(
                "accept-brief needs exactly one of --accept-brief-records or --accept-brief-manifest"
            ),
        };
        let aggregate = accept_brief::aggregate(&records)?;
        std::fs::create_dir_all(&cli.results_dir)?;
        std::fs::write(
            cli.results_dir.join("combined.json"),
            format!("{}\n", serde_json::to_string_pretty(&aggregate)?),
        )?;
        println!("accept-brief records: {}", aggregate.accepted_changes);
        eprintln!("results → {}", cli.results_dir.display());
        return Ok(ExitCode::SUCCESS);
    }

    let all_targets = targets::load_targets(&cli.targets)?;

    if cli.list_corpora {
        for t in &all_targets {
            println!("{} ({}, {} PRs)", t.name, t.language, t.prs.len());
        }
        return Ok(ExitCode::SUCCESS);
    }

    let selected: Vec<_> = match &cli.corpus {
        Some(names) => {
            let mut sel = Vec::new();
            for n in names {
                let t = all_targets
                    .iter()
                    .find(|t| &t.name == n)
                    .with_context(|| format!("unknown corpus {n:?}"))?;
                sel.push(t.clone());
            }
            sel
        }
        None => all_targets,
    };

    let knobs = BenchKnobs {
        n_cal: cli.n_cal,
        seed: cli.seed,
        threshold_n_seeds: cli.threshold_n_seeds,
        alpha: cli.call_receiver_alpha,
        cap: cli.call_receiver_cap,
        root_bonus: cli.call_receiver_root_bonus,
        n_clusters: cli.call_receiver_clusters,
        cluster_seed: cli.call_receiver_cluster_seed,
        cluster_bonus: cli.call_receiver_cluster_bonus,
        cluster_rare_threshold: cli.call_receiver_cluster_rare_threshold,
        cluster_size_min: cli.call_receiver_cluster_size_min,
        enable_typicality_filter: !cli.no_typicality_filter,
        apply_optional_contributions_to_cal: cli.apply_optional_contributions_to_cal,
        auto_select_asym_cal: !cli.no_auto_select_asym_cal,
        asym_fire_rate_threshold: cli.asym_fire_rate_threshold,
        asym_probe_n: cli.asym_probe_n,
        rarity_weighting: parse_rarity_weighting(&cli.rarity_weighting)?,
        calibration_source: match cli.calibration_source.as_str() {
            "random" => argot_bench::scorer::CalibrationSource::Random,
            "diff" => argot_bench::scorer::CalibrationSource::Diff,
            other => anyhow::bail!("unknown calibration source {other:?}"),
        },
        shape_primitive_names: cli
            .enable_shape_primitives
            .iter()
            .filter(|s| !s.is_empty())
            .cloned()
            .collect(),
        parse_error_host_fallback: !cli.no_parse_error_fallback,
        enable_conventions: !cli.no_conventions,
        convention_bonus: cli.convention_bonus,
    };
    let opts = run::RunOptions {
        data_dir: cli.data_dir,
        catalogs_dir: cli.catalogs_dir,
        knobs,
        quick: cli.quick,
        sample_controls: cli.sample_controls,
        keep_control_results: cli.keep_controls,
    };

    #[cfg(feature = "structural")]
    if cli.mode == "structural" {
        let md =
            argot_bench::structural::run_structural(&selected, &opts.data_dir, &cli.results_dir)?;
        print!("{md}");
        eprintln!("results → {}", cli.results_dir.display());
        return Ok(ExitCode::SUCCESS);
    }

    #[cfg(feature = "arch")]
    if cli.mode == "arch" {
        let md = argot_bench::arch::run_arch(
            &selected,
            &opts.data_dir,
            &opts.catalogs_dir,
            &cli.results_dir,
        )?;
        print!("{md}");
        eprintln!("results → {}", cli.results_dir.display());
        return Ok(ExitCode::SUCCESS);
    }

    #[cfg(feature = "arch")]
    if cli.mode == "arch-candidates" {
        let md = argot_bench::arch::run_arch_candidates(
            &selected,
            &opts.data_dir,
            &opts.catalogs_dir,
            &cli.results_dir,
        )?;
        print!("{md}");
        eprintln!("results → {}", cli.results_dir.display());
        return Ok(ExitCode::SUCCESS);
    }

    #[cfg(feature = "arch")]
    if cli.mode == "arch-verify" {
        let md = argot_bench::arch::run_arch_verify(&selected, &opts.data_dir, &opts.catalogs_dir)?;
        print!("{md}");
        return Ok(ExitCode::SUCCESS);
    }

    #[cfg(feature = "integrity")]
    if cli.mode == "integrity-verify" {
        let md = argot_bench::integrity::run_integrity_verify(
            &selected,
            &opts.data_dir,
            &opts.catalogs_dir,
        )?;
        print!("{md}");
        return Ok(ExitCode::SUCCESS);
    }

    #[cfg(feature = "integrity")]
    if cli.mode == "integrity-fp" {
        let md = argot_bench::integrity::run_integrity_fp(
            &selected,
            &opts.data_dir,
            &opts.catalogs_dir,
        )?;
        print!("{md}");
        return Ok(ExitCode::SUCCESS);
    }

    let jobs = if cli.jobs == 0 {
        pool::default_jobs()
    } else {
        cli.jobs
    };

    if cli.mode == "holdout" || cli.mode == "honest" {
        let hopts = holdout::HoldoutOptions {
            data_dir: opts.data_dir.clone(),
            catalogs_dir: opts.catalogs_dir.clone(),
            window: cli.holdout_window,
            min_hunks: cli.min_holdout_hunks,
            bootstrap_reps: cli.bootstrap_reps,
            seed: cli.seed,
        };
        let mut reports = Vec::new();
        // Quick honest mode skips holdout (a full old-SHA fit + replay per
        // corpus is minutes, not the PR-comment budget) — recall only.
        let run_holdout = cli.mode == "holdout" || !cli.quick;
        if run_holdout {
            let results = pool::run_indexed(&selected, jobs, |target| {
                let started = std::time::Instant::now();
                let r = holdout::run_corpus_holdout(target, &hopts)
                    .with_context(|| format!("corpus {} (holdout)", target.name));
                if r.is_ok() {
                    eprintln!(
                        "[{}] holdout done in {:.0}s",
                        target.name,
                        started.elapsed().as_secs_f64()
                    );
                }
                r
            });
            for r in results {
                reports.push(r?);
            }
            let md = holdout::write_holdout_reports(&cli.results_dir, &reports)?;
            print!("{md}");
        }
        if cli.mode == "honest" {
            let results = pool::run_indexed(&selected, jobs, |target| {
                if !opts.catalogs_dir.join(&target.name).is_dir() {
                    eprintln!("[{}] no catalog — holdout-only corpus", target.name);
                    return None;
                }
                let started = std::time::Instant::now();
                let r = production::run_corpus_production(target, &opts)
                    .with_context(|| format!("corpus {} (production)", target.name));
                if r.is_ok() {
                    eprintln!(
                        "[{}] production done in {:.0}s",
                        target.name,
                        started.elapsed().as_secs_f64()
                    );
                }
                Some(r)
            });
            let mut prod_reports = Vec::new();
            for r in results.into_iter().flatten() {
                prod_reports.push(r?);
            }
            let md = production::write_production_reports(
                &cli.results_dir,
                &prod_reports,
                &std::collections::BTreeMap::new(),
            )?;
            print!("{md}");
            let languages: std::collections::BTreeMap<String, String> = selected
                .iter()
                .map(|t| (t.name.clone(), t.language.clone()))
                .collect();
            let commit =
                std::env::var("ARGOT_BENCH_COMMIT").unwrap_or_else(|_| "local".to_string());
            let dash = argot_bench::dashboard::BenchDashboard::from_honest(
                &prod_reports,
                &reports,
                &languages,
                commit,
                iso_utc_now(),
            );
            std::fs::write(
                cli.results_dir.join("dashboard.json"),
                serde_json::to_string_pretty(&dash)?,
            )?;
        }
        eprintln!("results → {}", cli.results_dir.display());
        return Ok(ExitCode::SUCCESS);
    }

    let (run_catalog, run_production) = match cli.mode.as_str() {
        "catalog" => (true, false),
        "production" => (false, true),
        "both" => (true, true),
        other => {
            anyhow::bail!("unknown mode {other:?} (honest | catalog | production | both | holdout)")
        }
    };

    // Catalog/production modes need break fixtures; holdout-only targets
    // (language-port corpora) have none — skip them loudly, don't fail.
    let selected: Vec<_> = selected
        .into_iter()
        .filter(|t| {
            let has_catalog = opts.catalogs_dir.join(&t.name).is_dir();
            if !has_catalog {
                eprintln!(
                    "[{}] no catalog under {} — holdout-only corpus, skipping",
                    t.name,
                    opts.catalogs_dir.display()
                );
            }
            has_catalog
        })
        .collect();

    // Per-corpus catalog recall (caught, total) for the gap column. Catalog
    // reports are per (corpus, language); the production path checks whole
    // staged diffs, so the gap is compared at corpus granularity.
    let mut catalog_recall: std::collections::BTreeMap<String, (usize, usize)> =
        std::collections::BTreeMap::new();

    if run_catalog {
        let results = pool::run_indexed(&selected, jobs, |target| {
            let started = std::time::Instant::now();
            let rs =
                run::run_corpus(target, &opts).with_context(|| format!("corpus {}", target.name));
            if rs.is_ok() {
                eprintln!(
                    "[{}] catalog done in {:.0}s",
                    target.name,
                    started.elapsed().as_secs_f64()
                );
            }
            rs
        });
        let mut reports = Vec::new();
        for (target, rs) in selected.iter().zip(results) {
            let mut rs = rs?;
            for r in &rs {
                let entry = catalog_recall.entry(target.name.clone()).or_insert((0, 0));
                entry.0 += r.n_flagged_fixtures;
                entry.1 += r.n_fixtures;
            }
            reports.append(&mut rs);
        }
        report::write_reports(&cli.results_dir, &reports)?;
        // Versioned dashboard summary (#64): the compact, schema-stable payload
        // the public benchmark page + PR comment read. Commit sha from CI env.
        let commit = std::env::var("ARGOT_BENCH_COMMIT").unwrap_or_else(|_| "local".to_string());
        let dash =
            argot_bench::dashboard::BenchDashboard::from_reports(&reports, commit, iso_utc_now());
        std::fs::write(
            cli.results_dir.join("dashboard.json"),
            serde_json::to_string_pretty(&dash)?,
        )?;
        print!("{}", report::summary_markdown(&reports));
    }

    if run_production {
        let results = pool::run_indexed(&selected, jobs, |target| {
            let started = std::time::Instant::now();
            let r = production::run_corpus_production(target, &opts)
                .with_context(|| format!("corpus {} (production)", target.name));
            if r.is_ok() {
                eprintln!(
                    "[{}] production done in {:.0}s",
                    target.name,
                    started.elapsed().as_secs_f64()
                );
            }
            r
        });
        let mut prod_reports = Vec::new();
        for r in results {
            prod_reports.push(r?);
        }
        let md =
            production::write_production_reports(&cli.results_dir, &prod_reports, &catalog_recall)?;
        // Versioned dashboard summary (#64) — production path is the headline.
        let commit = std::env::var("ARGOT_BENCH_COMMIT").unwrap_or_else(|_| "local".to_string());
        let dash = argot_bench::dashboard::BenchDashboard::from_production(
            &prod_reports,
            commit,
            iso_utc_now(),
        );
        std::fs::write(
            cli.results_dir.join("dashboard.json"),
            serde_json::to_string_pretty(&dash)?,
        )?;
        print!("{md}");
    }

    eprintln!("results → {}", cli.results_dir.display());
    Ok(ExitCode::SUCCESS)
}
