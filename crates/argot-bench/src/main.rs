//! argot-bench — recall / false-positive evaluation over the pinned corpora.
//!
//! Rust successor to the retired Python `argot_bench` harness. Reads
//! `benchmarks/targets.yaml` + `benchmarks/catalogs/`, scores break fixtures
//! and real-PR control hunks with the production scorer, and writes per-corpus
//! JSON + a markdown summary under `--results-dir`.

mod catalog;
mod metrics;
mod report;
mod run;
mod scorer;
mod targets;

use anyhow::{Context, Result};
use clap::Parser;
use scorer::BenchKnobs;
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

    #[arg(long, default_value = "benchmarks/targets.yaml")]
    targets: PathBuf,

    #[arg(long, default_value = "benchmarks/catalogs")]
    catalogs_dir: PathBuf,

    #[arg(long, default_value = "benchmarks/data")]
    data_dir: PathBuf,

    #[arg(long, default_value = "benchmarks/results/latest")]
    results_dir: PathBuf,

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
    };
    let opts = run::RunOptions {
        data_dir: cli.data_dir,
        catalogs_dir: cli.catalogs_dir,
        knobs,
        quick: cli.quick,
        sample_controls: cli.sample_controls,
        keep_control_results: cli.keep_controls,
    };

    let mut reports = Vec::new();
    for target in &selected {
        let started = std::time::Instant::now();
        let mut rs = run::run_corpus(target, &opts)
            .with_context(|| format!("corpus {}", target.name))?;
        eprintln!(
            "[{}] done in {:.0}s",
            target.name,
            started.elapsed().as_secs_f64()
        );
        reports.append(&mut rs);
    }

    report::write_reports(&cli.results_dir, &reports)?;
    print!("{}", report::summary_markdown(&reports));
    eprintln!("results → {}", cli.results_dir.display());
    Ok(ExitCode::SUCCESS)
}
