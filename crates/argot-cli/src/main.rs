//! argot CLI — clap-based entry point.
//!
//! Replaces both the TypeScript/Bun shell (`cli/src`) and the Python engine
//! entry points (`argot-extract`, `argot-train`, `argot-calibrate`,
//! `argot-check`). One statically-linked binary, no subprocess.
//!
//! Subcommands are wired in as the port progresses. The user-facing command
//! surface is reconciled with the TS CLI in the CLI phase; the engine-level
//! commands mirror the Python entry points exactly (args, messages, exit
//! codes) so the benchmark harness and `just` recipes can drive the binary.

use clap::{Args, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, IsTerminal, Write};
use std::path::PathBuf;
use std::process::{Command as ProcCommand, ExitCode};
use std::time::{SystemTime, UNIX_EPOCH};

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::extract::{write_dataset, ExtractError};
use argot_core::git_walk::{head_sha, repo_exists};
use argot_core::inspect::{format_shares, inspect_repo, InspectReport, ReasonLevel, Verdict};
use argot_core::output::OutputFormat;
use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::adapters::typescript::TypeScriptAdapter;
use argot_core::scoring::adapters::{Language, LanguageAdapter};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use argot_core::text::read_text_lossy;
use argot_core::train::run_train;

/// Civil date (y, m, d) for a day count since the Unix epoch — Howard
/// Hinnant's civil-from-days algorithm.
fn civil_from_days(days: i64) -> (i64, i64, i64) {
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
    (y, m, d)
}

fn epoch_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Format the current time as an ISO 8601 UTC string (calibration metadata;
/// not parity-relevant).
fn iso_now() -> String {
    let secs = epoch_secs();
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
    let (y, m, d) = civil_from_days((secs / 86400) as i64);
    format!("{y:04}-{m:02}-{d:02}T{h:02}:{mi:02}:{s:02}+00:00")
}

/// Today's UTC date as `YYYY-MM-DD` — passed into core suppression-expiry
/// logic (core never calls system time itself).
fn today_utc() -> String {
    let (y, m, d) = civil_from_days((epoch_secs() / 86400) as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

/// The UTC date `days` days from now as `YYYY-MM-DD` (`mute --expires <N>d`).
fn date_days_from_now(days: u64) -> String {
    let (y, m, d) = civil_from_days((epoch_secs() / 86400) as i64 + days as i64);
    format!("{y:04}-{m:02}-{d:02}")
}

#[derive(Parser)]
#[command(
    name = "argot",
    version,
    about = "Voice linter that learns a repo's voice from git history."
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Extract dataset from git history (mirrors `argot-extract`).
    Extract(ExtractArgs),
    /// Collect the repo corpus + generic baseline (mirrors `argot-train`).
    Train(TrainCmd),
    /// Calibrate the per-language threshold (mirrors `argot-calibrate`).
    Calibrate(CalibrateCmd),
    /// Fit the voice model to this repo (train + calibrate, one-shot).
    Fit(FitCmd),
    /// Check code changes against the calibrated scorers (mirrors `argot-check`).
    Check(CheckCmd),
    /// Report corpus composition, calibration health, and repo suitability.
    Inspect(InspectCmd),
    /// Mute a hit by hash (appends to .argot/suppressions.yaml).
    Mute(MuteCmd),
    /// List active suppressions across .argotignore, inline comments, and
    /// suppressions.yaml.
    #[command(name = "list-mutes")]
    ListMutes,
    /// Re-score muted files and report which suppressions no longer fire.
    #[command(name = "review-mutes")]
    ReviewMutes(ReviewMutesCmd),
    /// Batch-score hunks from stdin (benchmark harness seam). Hidden.
    #[command(hide = true)]
    Score(ScoreCmd),
    /// Show the current repository's argot state.
    Status,
    /// List all registered repositories.
    List,
    /// Show the CLI version (self-update is handled by the installer).
    Update,
}

// --- repo context / registry (port of fs-repo-context.adapter.ts) ---

#[derive(Serialize, Deserialize, Default, Clone)]
struct RepoEntry {
    name: String,
    #[serde(rename = "registeredAt", default)]
    registered_at: String,
    #[serde(rename = "lastUsedAt", default)]
    last_used_at: String,
}

#[derive(Serialize, Deserialize, Default)]
struct GlobalSettings {
    #[serde(default)]
    repos: BTreeMap<String, RepoEntry>,
}

struct RepoCtx {
    git_root: String,
    name: String,
    argot_dir: PathBuf,
    dataset_path: PathBuf,
    repo_corpus_path: PathBuf,
}

fn settings_path() -> PathBuf {
    let home = std::env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_default();
    home.join(".argot").join("settings.json")
}

fn read_settings() -> GlobalSettings {
    match fs::read_to_string(settings_path()) {
        Ok(s) => serde_json::from_str(&s).unwrap_or_default(),
        Err(_) => GlobalSettings::default(),
    }
}

fn write_settings(s: &GlobalSettings) {
    let path = settings_path();
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(json) = serde_json::to_string_pretty(s) {
        let _ = fs::write(path, json);
    }
}

fn git_toplevel() -> Option<String> {
    let out = ProcCommand::new("git")
        .args(["rev-parse", "--show-toplevel"])
        .output()
        .ok()?;
    if !out.status.success() {
        return None;
    }
    Some(String::from_utf8_lossy(&out.stdout).trim().to_string())
}

fn resolve_context() -> RepoCtx {
    let git_root = git_toplevel().unwrap_or_else(|| {
        std::env::current_dir()
            .unwrap_or_default()
            .display()
            .to_string()
    });
    let mut settings = read_settings();
    let now = iso_now();
    let basename = std::path::Path::new(&git_root)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| git_root.clone());
    let entry = settings
        .repos
        .entry(git_root.clone())
        .or_insert_with(|| RepoEntry {
            name: basename.clone(),
            registered_at: now.clone(),
            last_used_at: now.clone(),
        });
    entry.last_used_at = now;
    let name = entry.name.clone();
    write_settings(&settings);

    let argot_dir = PathBuf::from(&git_root).join(".argot");
    RepoCtx {
        dataset_path: argot_dir.join("dataset.jsonl"),
        repo_corpus_path: argot_dir.join("repo-corpus.txt"),
        argot_dir,
        git_root,
        name,
    }
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

fn run_status() -> ExitCode {
    let ctx = resolve_context();
    println!("Repo:     {} ({})", ctx.name, ctx.git_root);
    match fs::metadata(&ctx.dataset_path) {
        Ok(m) => {
            let count = fs::read_to_string(&ctx.dataset_path)
                .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
                .unwrap_or(0);
            println!("Dataset:  {} records · {}", count, format_bytes(m.len()));
        }
        Err(_) => println!("Dataset:  —"),
    }
    match fs::metadata(&ctx.repo_corpus_path) {
        Ok(m) => println!("Model:    trained · {}", format_bytes(m.len())),
        Err(_) => println!("Model:    not trained"),
    }
    let config_path = ctx.argot_dir.join("scorer-config.json");
    if config_path.exists() {
        println!("Calibrated: yes");
    } else {
        println!("Calibrated: not calibrated — run `argot fit`");
    }
    ExitCode::SUCCESS
}

fn run_list() -> ExitCode {
    let current = git_toplevel();
    let settings = read_settings();
    let mut repos: Vec<(&String, &RepoEntry)> = settings.repos.iter().collect();
    repos.sort_by(|a, b| a.1.name.cmp(&b.1.name));
    if repos.is_empty() {
        println!("No repositories registered yet.");
        return ExitCode::SUCCESS;
    }
    for (path, entry) in repos {
        let marker = if Some(path) == current.as_ref() {
            "* "
        } else {
            "  "
        };
        println!("{}{} ({})", marker, entry.name, path);
    }
    ExitCode::SUCCESS
}

fn run_update() -> ExitCode {
    println!("argot {}", env!("CARGO_PKG_VERSION"));
    println!("Self-update is handled by the installer (install.sh / package manager).");
    ExitCode::SUCCESS
}

fn print_help_banner() {
    let version = env!("CARGO_PKG_VERSION");
    println!(
        "argot v{version}\n\nCOMMANDS\n  extract       Walk git history into a training dataset (.argot/dataset.jsonl)\n  fit           Fit the voice model to this repo (= train + calibrate, one-shot)\n  check         Check changes against the fitted voice\n  inspect       Report corpus composition, calibration health, and suitability\n  mute          Mute a hit by hash (appends to .argot/suppressions.yaml)\n  list-mutes    List active suppressions across all surfaces\n  review-mutes  Report (and --prune) muted hits that no longer fire\n  status        Show current repository's argot state\n  list          List all registered repositories\n  update        Update the argot CLI\n\nTypical first run: argot extract && argot fit && argot check\nRun `argot <command> --help` for details on any command."
    );
}

#[derive(Args)]
struct TrainCmd {
    /// Path to the target repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Output file listing repo corpus source paths.
    #[arg(long = "repo-corpus-out", default_value = ".argot/repo-corpus.txt")]
    repo_corpus_out: PathBuf,
    /// Output path for the BPE generic baseline JSON.
    #[arg(
        long = "generic-baseline-out",
        default_value = ".argot/generic-baseline.json"
    )]
    generic_baseline_out: PathBuf,
}

fn run_train_cmd(c: TrainCmd) -> ExitCode {
    match run_train(&c.repo, &c.repo_corpus_out, &c.generic_baseline_out) {
        Ok(o) => {
            println!(
                "repo corpus: {} source files → {}",
                o.source_file_count,
                c.repo_corpus_out.display()
            );
            println!("generic baseline: {}", c.generic_baseline_out.display());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

#[derive(Args)]
struct CalibrateCmd {
    /// Path to the target repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Number of calibration hunks.
    #[arg(long = "n-cal", default_value_t = 500)]
    n_cal: usize,
    /// RNG seed for hunk sampling.
    #[arg(long, default_value_t = 0)]
    seed: u64,
    /// Number of independent calibration seeds.
    #[arg(long = "threshold-n-seeds", default_value_t = 7)]
    n_seeds: usize,
    /// File listing repo corpus source paths.
    #[arg(long = "repo-corpus", default_value = ".argot/repo-corpus.txt")]
    repo_corpus: PathBuf,
    /// Path to the BPE generic baseline JSON.
    #[arg(
        long = "generic-baseline",
        default_value = ".argot/generic-baseline.json"
    )]
    generic_baseline: PathBuf,
    /// Output path for scorer-config.json.
    #[arg(long, default_value = ".argot/scorer-config.json")]
    output: PathBuf,
    /// Number of top entries per dimension in the evidence corpus.
    #[arg(long = "evidence-top-n", default_value_t = 50)]
    evidence_top_n: usize,
    /// Rare-branch threshold for the check-time scorer: a callee attested in
    /// ≤ N cluster files is treated as cluster-absent. 0 disables (pre-13.5
    /// baseline). Default 2 (era-13.5 setting).
    #[arg(long = "call-receiver-cluster-rare-threshold", default_value_t = 2)]
    cluster_rare_threshold: usize,
    /// Minimum cluster size for the rare rule to fire (0 = no floor).
    #[arg(long = "call-receiver-cluster-size-min", default_value_t = 0)]
    cluster_size_min: usize,
    /// Disable the per-corpus rare-rule auto-detect probe (probe is on by default).
    #[arg(long = "no-auto-select-asym-cal")]
    no_auto_select_asym_cal: bool,
    /// Auto-detect cutoff: keep the rare rule when its calibration fire rate
    /// is below this fraction of cal hunks.
    #[arg(long = "asym-fire-rate-threshold", default_value_t = 0.05)]
    asym_fire_rate_threshold: f64,
}

fn run_calibrate_cmd(c: CalibrateCmd) -> ExitCode {
    let generic = match fs::read(&c.generic_baseline) {
        Ok(b) => b,
        Err(_) => {
            eprintln!(
                "error: generic baseline not found: {}",
                c.generic_baseline.display()
            );
            return ExitCode::from(2);
        }
    };
    let repo_sha = head_sha(&c.repo.to_string_lossy()).unwrap_or_else(|| "unknown".to_string());
    let opts = CalibrateOptions {
        n_cal: c.n_cal,
        seed: c.seed,
        n_seeds: c.n_seeds,
        evidence_top_n: c.evidence_top_n,
        repo_sha,
        timestamp_utc: iso_now(),
        cluster_rare_threshold: c.cluster_rare_threshold,
        cluster_size_min: c.cluster_size_min,
        auto_select_asym_cal: !c.no_auto_select_asym_cal,
        asym_fire_rate_threshold: c.asym_fire_rate_threshold,
    };
    match run_calibrate(&c.repo, &c.repo_corpus, &generic, &c.output, &opts) {
        Ok(thresholds) => {
            for (lang, t) in &thresholds {
                println!("[{lang}] threshold: {t:.4}");
            }
            let langs: Vec<&str> = thresholds.iter().map(|(l, _)| l.as_str()).collect();
            println!(
                "scorer-config.json (v3, languages: {}) → {}",
                langs.join(", "),
                c.output.display()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

#[derive(Args)]
struct FitCmd {
    /// Path to the target repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

fn run_fit_cmd(c: FitCmd) -> ExitCode {
    let argot_dir = c.repo.join(".argot");
    let repo_corpus = argot_dir.join("repo-corpus.txt");
    let generic = argot_dir.join("generic-baseline.json");
    let scorer_config = argot_dir.join("scorer-config.json");

    println!("Step 1/2: training voice model …");
    if let Err(e) = run_train(&c.repo, &repo_corpus, &generic) {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }

    println!("Step 2/2: calibrating threshold …");
    let generic_bytes = match fs::read(&generic) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("error: generic baseline missing after train");
            return ExitCode::from(2);
        }
    };
    let repo_sha = head_sha(&c.repo.to_string_lossy()).unwrap_or_else(|| "unknown".to_string());
    let opts = CalibrateOptions {
        repo_sha,
        timestamp_utc: iso_now(),
        ..CalibrateOptions::default()
    };
    if let Err(e) = run_calibrate(&c.repo, &repo_corpus, &generic_bytes, &scorer_config, &opts) {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }
    println!("Done. Scorer config: {}", scorer_config.display());
    ExitCode::SUCCESS
}

#[derive(Args)]
struct CheckCmd {
    /// Path to git repository.
    repo_path: String,
    /// Optional git ref or range (e.g. abc1234 or a..b). Empty = workdir.
    #[arg(default_value = "")]
    reference: String,
    /// Check staged changes only.
    #[arg(long)]
    staged: bool,
    /// Check unstaged changes only.
    #[arg(long)]
    unstaged: bool,
    /// Check a single commit.
    #[arg(long, value_name = "SHA")]
    commit: Option<String>,
    /// Restrict to matching files (glob, repeatable).
    #[arg(long, value_name = "GLOB")]
    only: Vec<String>,
    /// Drop matching files (glob, repeatable).
    #[arg(long, value_name = "GLOB")]
    exclude: Vec<String>,
    /// Override the calibrated threshold (bench/dev escape hatch).
    #[arg(long, allow_hyphen_values = true)]
    threshold: Option<f64>,
    /// Directory containing argot artifacts.
    #[arg(long = "argot-dir", default_value = ".argot")]
    argot_dir: PathBuf,
    /// Hunk-body lines shown under each above-threshold hit (0 to suppress).
    #[arg(long = "hunk-lines", value_name = "N", default_value_t = DEFAULT_HUNK_LINES)]
    hunk_lines: usize,
    /// Show full hunk contents (no truncation; overrides --hunk-lines).
    #[arg(short = 'v', long)]
    verbose: bool,
    /// Only show hits at or above this severity.
    #[arg(
        long = "min-severity",
        default_value = "unusual",
        value_parser = ["unusual", "suspicious", "foreign"]
    )]
    min_severity: String,
    /// Output format: human (terminal), json (stable machine-readable), or
    /// sarif (SARIF 2.1.0 for code-scanning uploads). Machine formats write
    /// nothing but the document to stdout.
    #[arg(
        long,
        default_value = "human",
        value_parser = ["human", "json", "sarif"]
    )]
    format: String,
}

fn run_check_cmd(c: CheckCmd) -> ExitCode {
    // Color is enabled only when NO_COLOR is unset and stdout is a tty.
    let use_color = std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    let outcome = run_check(CheckArgs {
        repo_path: c.repo_path,
        reference: c.reference,
        staged: c.staged,
        unstaged: c.unstaged,
        commit: c.commit,
        only: c.only,
        exclude: c.exclude,
        threshold: c.threshold,
        argot_dir: c.argot_dir,
        hunk_lines: c.hunk_lines,
        verbose: c.verbose,
        min_severity: c.min_severity,
        use_color,
        // The value_parser restricts input to the known names, so this is
        // always Some; default to Human defensively.
        format: OutputFormat::parse(&c.format).unwrap_or_default(),
        today: today_utc(),
    });
    print!("{}", outcome.stdout);
    eprint!("{}", outcome.stderr);
    ExitCode::from(outcome.exit_code as u8)
}

#[derive(Args)]
struct InspectCmd {
    /// Path to the repository to inspect.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Emit a stable machine-readable JSON document.
    #[arg(long)]
    json: bool,
}

fn run_inspect_cmd(c: InspectCmd) -> ExitCode {
    let report = match inspect_repo(&c.path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    if c.json {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }
    // Same color policy as `check`: NO_COLOR unset and stdout is a tty.
    let use_color = std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    print!("{}", render_inspect_human(&report, use_color));
    ExitCode::SUCCESS
}

const ANSI_RED: &str = "\x1b[31m";
const ANSI_YELLOW: &str = "\x1b[33m";
const ANSI_GREEN: &str = "\x1b[32m";
const ANSI_BOLD: &str = "\x1b[1m";
const ANSI_RESET: &str = "\x1b[0m";

fn paint(text: &str, color: &str, use_color: bool) -> String {
    if use_color {
        format!("{color}{text}{ANSI_RESET}")
    } else {
        text.to_string()
    }
}

fn render_inspect_human(report: &InspectReport, use_color: bool) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "Inspecting {}", report.path);
    let _ = writeln!(out);

    // Corpus composition.
    let c = &report.corpus;
    let _ = writeln!(out, "Corpus");
    let _ = writeln!(
        out,
        "  {} files scanned · {} supported · {} unsupported extension",
        c.total_files, c.supported_files, c.unsupported_files
    );
    for (lang, stats) in &c.languages {
        let _ = writeln!(
            out,
            "  {lang}: {} files ({:.0}%) · {} included · excluded: {} path, {} auto-generated, {} data-dominant",
            stats.files,
            stats.share_of_supported * 100.0,
            stats.included,
            stats.excluded_path,
            stats.auto_generated,
            stats.data_dominant,
        );
        let _ = writeln!(
            out,
            "    calibration candidates: {} hunks",
            stats.candidate_hunks
        );
    }
    if !c.languages.is_empty() {
        let mixed = if c.meaningfully_mixed {
            " — meaningfully mixed"
        } else {
            ""
        };
        let _ = writeln!(out, "  polyglotism: {}{mixed}", format_shares(c));
    }
    let _ = writeln!(out);

    // Calibration health (post-fit only).
    let _ = writeln!(out, "Calibration");
    match &report.calibration {
        Some(cal) => {
            let _ = writeln!(out, "  config: {}", cal.config_path);
            for (lang, lc) in &cal.languages {
                let _ = writeln!(
                    out,
                    "  {lang}: threshold {:.4} · n_cal {} (candidates now: {}) · {} seeds (base {})",
                    lc.threshold, lc.n_cal, lc.candidate_hunks_now, lc.n_seeds, lc.seed
                );
                let _ = writeln!(
                    out,
                    "    calibrated at {} · repo sha {}",
                    lc.timestamp_utc, lc.repo_sha
                );
                let _ = writeln!(
                    out,
                    "    phrasing headroom: {:+.2} (BPE ceiling {:.2} + callee cap {:.0} vs threshold {:.2})",
                    lc.phrasing_headroom, lc.bpe_ceiling, lc.contribution_cap, lc.threshold
                );
            }
        }
        None => {
            let _ = writeln!(out, "  not fitted — run `argot fit` to calibrate");
        }
    }
    let _ = writeln!(out);

    // Verdict.
    let verdict_label = match report.verdict {
        Verdict::Ready => paint("Ready", ANSI_GREEN, use_color),
        Verdict::Marginal => paint("Marginal", ANSI_YELLOW, use_color),
        Verdict::NotRecommended => paint("Not recommended", ANSI_RED, use_color),
    };
    let _ = writeln!(
        out,
        "{} {verdict_label}",
        paint("Verdict:", ANSI_BOLD, use_color)
    );
    for reason in &report.reasons {
        let (label, color) = match reason.level {
            ReasonLevel::Red => ("red", ANSI_RED),
            ReasonLevel::Yellow => ("yellow", ANSI_YELLOW),
        };
        let _ = writeln!(
            out,
            "  {} {} — {}",
            paint(label, color, use_color),
            reason.signal,
            reason.message
        );
    }
    out
}

// --- suppression commands (mute / list-mutes / review-mutes) ---

#[derive(Args)]
struct MuteCmd {
    /// Hit hash from `argot check` output (the `[abc123def456]` on a hit line).
    hash: String,
    /// Why this hit is muted (recorded in suppressions.yaml).
    #[arg(long)]
    reason: Option<String>,
    /// Auto-expire the mute after N days (e.g. `30d`).
    #[arg(long, value_name = "DAYS", value_parser = parse_expires_days)]
    expires: Option<u64>,
}

/// Parse `--expires 30d` (a plain number is accepted too).
fn parse_expires_days(s: &str) -> Result<u64, String> {
    s.strip_suffix('d')
        .unwrap_or(s)
        .parse::<u64>()
        .map_err(|_| format!("expected a day count like '30d', got '{s}'"))
}

fn run_mute_cmd(c: MuteCmd) -> ExitCode {
    let ctx = resolve_context();
    let expires = c.expires.map(date_days_from_now);
    match argot_core::suppress::mute_hash(
        &ctx.argot_dir,
        &c.hash,
        c.reason.as_deref(),
        expires.clone(),
        &today_utc(),
    ) {
        Ok(rule) => {
            println!(
                "Muted [{}] in {} — {}{}",
                c.hash,
                rule.path,
                rule.reason,
                expires
                    .map(|e| format!(" (expires {e})"))
                    .unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::from(2)
        }
    }
}

/// Recursively collect supported-language source files (skips `.git` and the
/// argot dir itself) for the inline-comment scan.
fn supported_files(root: &std::path::Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        let Ok(entries) = fs::read_dir(&dir) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            match entry.file_type() {
                Ok(t) if t.is_dir() => {
                    if name != ".git" && name != ".argot" {
                        stack.push(path);
                    }
                }
                Ok(t)
                    if t.is_file()
                        && argot_core::scoring::calibration::language_for_filename(&name)
                            .is_some() =>
                {
                    out.push(path);
                }
                _ => {}
            }
        }
    }
    out.sort();
    out
}

fn run_list_mutes() -> ExitCode {
    use argot_core::scoring::calibration::language_for_filename;
    use argot_core::suppress::{
        load_suppressions_file, parse_inline, PathSuppressions, SUPPRESSIONS_FILE,
    };

    let ctx = resolve_context();
    let repo_root = PathBuf::from(&ctx.git_root);
    let today = today_utc();

    // 1. Path-level mutes (.argotignore + the built-in recommended set).
    let paths = PathSuppressions::load(&repo_root);
    println!(".argotignore");
    println!(
        "  recommended set: {}",
        if paths.recommended_active() {
            "active (built-in test/docs/examples/… exclusions)"
        } else {
            "disabled (!argot:recommended)"
        }
    );
    if paths.from_file {
        let patterns = paths.user_patterns();
        if patterns.is_empty() {
            println!("  patterns: (none)");
        } else {
            for p in patterns {
                println!("  pattern: {p}");
            }
        }
    } else {
        println!("  no .argotignore file at repo root");
    }

    // 2. suppressions.yaml rules, with expiry status.
    let rules_path = ctx.argot_dir.join(SUPPRESSIONS_FILE);
    let rules = load_suppressions_file(&rules_path, &today);
    println!("\nsuppressions.yaml ({})", rules_path.display());
    if rules.active.is_empty() && rules.expired.is_empty() {
        println!("  (no entries)");
    }
    for (label, list) in [("active", &rules.active), ("expired", &rules.expired)] {
        for r in list {
            let mut parts = vec![format!("path={}", r.path)];
            if let Some(s) = &r.scorer {
                parts.push(format!("scorer={s}"));
            }
            if let Some(h) = &r.hash {
                parts.push(format!("hash={h}"));
            }
            if let Some(e) = &r.expires {
                parts.push(format!("expires={e}"));
            }
            println!("  [{label}] {} — {}", parts.join("  "), r.reason);
        }
    }
    for w in &rules.warnings {
        if !w.contains("expired") {
            eprintln!("[argot] {w}");
        }
    }

    // 3. Inline suppression comments (cheap line scan over supported files).
    println!("\ninline comments");
    let mut any_inline = false;
    for file in supported_files(&repo_root) {
        let name = file
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default();
        let Some(language) = language_for_filename(&name) else {
            continue;
        };
        let Ok(source) = read_text_lossy(&file) else {
            continue;
        };
        let adapter: Box<dyn LanguageAdapter> = match language {
            Language::Python => Box::new(PythonAdapter::new()),
            Language::Typescript => Box::new(TypeScriptAdapter::new()),
        };
        // Cheap pre-filter before the full parse.
        if !source.contains("argot:") {
            continue;
        }
        let inline = parse_inline(&source, adapter.line_comment_prefix());
        if inline.rules.is_empty() {
            continue;
        }
        any_inline = true;
        let rel = file
            .strip_prefix(&repo_root)
            .unwrap_or(&file)
            .display()
            .to_string();
        println!("  {rel}: {} suppression comment(s)", inline.rules.len());
    }
    if !any_inline {
        println!("  (none found)");
    }
    ExitCode::SUCCESS
}

#[derive(Args)]
struct ReviewMutesCmd {
    /// Remove the suppressions that no longer fire (rewrites suppressions.yaml).
    #[arg(long)]
    prune: bool,
}

fn run_review_mutes_cmd(c: ReviewMutesCmd) -> ExitCode {
    let ctx = resolve_context();
    let outcome =
        argot_core::check::run_review_mutes(&ctx.git_root, &ctx.argot_dir, &today_utc(), c.prune);
    print!("{}", outcome.stdout);
    eprint!("{}", outcome.stderr);
    ExitCode::from(outcome.exit_code as u8)
}

// --- batch score (benchmark harness seam) ---
//
// Builds the production composite scorer from a repo corpus, then scores hunks
// read as JSONL from stdin, emitting one JSONL result per line. The benchmark's
// `score.py` adapter shells out to this so the harness runs against the Rust
// engine. Output fields mirror the bench `ScoreResult`.

#[derive(serde::Deserialize)]
struct HunkRequest {
    hunk_content: String,
    #[serde(default)]
    file_source: Option<String>,
    #[serde(default)]
    hunk_start_line: Option<usize>,
    #[serde(default)]
    hunk_end_line: Option<usize>,
    #[serde(default)]
    file_path: Option<String>,
}

#[derive(serde::Serialize)]
struct ScoreOut {
    import_score: f64,
    bpe_score: f64,
    flagged: bool,
    reason: String,
}

#[derive(Args)]
struct ScoreCmd {
    /// File listing repo corpus source paths (the scorer is built from these).
    #[arg(long = "repo-corpus")]
    repo_corpus: PathBuf,
    /// Path to the BPE generic baseline JSON.
    #[arg(long = "generic-baseline")]
    generic_baseline: PathBuf,
    /// Corpus language: python or typescript.
    #[arg(long)]
    language: String,
    /// Calibrated BPE threshold (drives the `flagged` decision → recall/fp).
    /// AUC is threshold-independent; pass the harness's threshold for a fair
    /// recall/fp comparison.
    #[arg(long, default_value_t = 0.0)]
    threshold: f64,
    /// Cluster-rare threshold (era-13.5); the bench auto-selects 0 or 2.
    #[arg(long = "cluster-rare-threshold", default_value_t = 0)]
    cluster_rare_threshold: usize,
    /// Repo root for resolving the repo's own module names (package.json name /
    /// workspace packages). Matches Python inference's `repo_root=` so
    /// self-package imports (`import … from 'ink'`) are attested, not foreign.
    #[arg(long = "repo-root")]
    repo_root: Option<PathBuf>,
}

fn run_score_cmd(c: ScoreCmd) -> ExitCode {
    use std::io::BufRead;
    let language = match c.language.as_str() {
        "python" => Language::Python,
        "typescript" => Language::Typescript,
        other => {
            eprintln!("error: --language must be python|typescript (got '{other}')");
            return ExitCode::from(2);
        }
    };
    let corpus_txt = match fs::read_to_string(&c.repo_corpus) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let files: Vec<PathBuf> = corpus_txt
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .collect();
    let repo_files: Vec<(PathBuf, String)> = files
        .iter()
        .filter_map(|p| read_text_lossy(p).ok().map(|s| (p.clone(), s)))
        .collect();
    let generic = match fs::read(&c.generic_baseline) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let adapter: Box<dyn LanguageAdapter> = match language {
        Language::Python => Box::new(PythonAdapter::new()),
        Language::Typescript => Box::new(TypeScriptAdapter::new()),
    };
    // import_modules = union of extract_imports over the corpus (matches the
    // bench's ImportGraphScorer.fit). AUC is threshold-independent; the
    // threshold only drives `flagged` (recall/fp), so it is passed in.
    let mut mods = std::collections::BTreeSet::new();
    for (_, s) in &repo_files {
        for m in adapter.extract_imports(s) {
            mods.insert(m);
        }
    }
    // Repo-module resolution (matches Python inference's `repo_root=`): the
    // repo's own package name and workspace packages attest self-imports so
    // e.g. `import … from 'ink'` in an ink checkout is internal, not foreign.
    let mut import_module_prefixes: Vec<String> = Vec::new();
    if let Some(root) = &c.repo_root {
        let repo_mods = adapter.resolve_repo_modules(root);
        mods.extend(repo_mods.exact);
        import_module_prefixes = repo_mods.prefixes.into_iter().collect();
    }
    let cfg = SequentialConfig {
        bpe_threshold: c.threshold,
        enable_typicality: true,
        exclude_data_dominant: true,
        call_receiver_alpha: 2.0,
        call_receiver_cap: 5,
        call_receiver_root_bonus: 2.0,
        call_receiver_n_clusters: 8,
        call_receiver_cluster_seed: 0,
        call_receiver_cluster_bonus: 5.0,
        call_receiver_cluster_rare_threshold: c.cluster_rare_threshold,
        call_receiver_cluster_size_min: 0,
        call_receiver_rarity_weighting: argot_core::scoring::call_receiver::RarityWeighting::Off,
        call_receiver_shape_primitive_names: Vec::new(),
        call_receiver_parse_error_host_fallback: false,
        import_modules: mods.into_iter().collect(),
        import_module_prefixes,
        // Bench feature extraction reads `stages.bpe_score`; no evidence needed.
        evidence_corpus: None,
    };
    let mut scorer =
        match SequentialImportBpeScorer::from_config(&repo_files, &generic, adapter, cfg) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        };

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut w = BufWriter::new(stdout.lock());
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };
        if line.trim().is_empty() {
            continue;
        }
        let req: HunkRequest = match serde_json::from_str(&line) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("error: bad request json: {e}");
                return ExitCode::from(2);
            }
        };
        let fp = req.file_path.as_ref().map(PathBuf::from);
        let scored = scorer.score_hunk(
            &req.hunk_content,
            req.file_source.as_deref(),
            req.hunk_start_line,
            req.hunk_end_line,
            fp.as_deref(),
        );
        let out_rec = ScoreOut {
            import_score: scored.stages.import_score,
            bpe_score: scored.stages.bpe_score,
            flagged: scored.flagged,
            reason: scored.reason.as_str().to_string(),
        };
        if writeln!(w, "{}", serde_json::to_string(&out_rec).unwrap()).is_err() {
            break;
        }
        // Flush per line so the harness coprocess reads each response as its
        // request arrives (no deadlock on a full pipe buffer).
        if w.flush().is_err() {
            break;
        }
    }
    ExitCode::SUCCESS
}

#[derive(Args)]
struct ExtractArgs {
    /// Path to git repository.
    repo_path: String,
    /// Optional git ref or range (e.g. abc1234 or a..b). Defaults to full history.
    #[arg(default_value = "")]
    reference: String,
    /// Output JSONL path.
    #[arg(long, default_value = ".argot/dataset.jsonl")]
    out: PathBuf,
    /// Max number of records to emit.
    #[arg(long)]
    limit: Option<usize>,
}

fn run_extract(a: ExtractArgs) -> ExitCode {
    // `pygit2.Repository(path)` raising GitError → exit 2.
    if !repo_exists(&a.repo_path) {
        eprintln!("error: repository not found at '{}'", a.repo_path);
        return ExitCode::from(2);
    }

    if let Some(parent) = a.out.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Atomic write: stream to a per-PID tmp file, then rename.
    let out_name = a
        .out
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| "dataset.jsonl".to_string());
    let tmp = a
        .out
        .with_file_name(format!("{}.tmp.{}", out_name, std::process::id()));

    let file = match fs::File::create(&tmp) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let mut writer = BufWriter::new(file);

    let reference = if a.reference.is_empty() {
        None
    } else {
        Some(a.reference.as_str())
    };

    let stats = match write_dataset(&a.repo_path, reference, a.limit, &mut writer) {
        Ok(s) => s,
        Err(e) => {
            let _ = fs::remove_file(&tmp);
            if let Some(ExtractError::NoCommitsForRef(r)) = e.downcast_ref::<ExtractError>() {
                eprintln!("error: no commits found for ref '{r}' — try a wider range");
                return ExitCode::from(2);
            }
            eprintln!("error: {e:#}");
            return ExitCode::from(1);
        }
    };

    if let Err(e) = writer.flush() {
        let _ = fs::remove_file(&tmp);
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }
    drop(writer);

    if stats.count == 0 {
        let _ = fs::remove_file(&tmp);
        eprintln!("error: no hunks found — repository may have no history");
        return ExitCode::from(2);
    }

    if let Err(e) = fs::rename(&tmp, &a.out) {
        eprintln!("error: {e}");
        return ExitCode::from(2);
    }

    if stats.limit_reached {
        eprintln!("Reached limit of {} records", a.limit.unwrap_or(0));
    }
    println!("Wrote {} records to {}", stats.count, a.out.display());
    ExitCode::SUCCESS
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    match cli.command {
        None => {
            print_help_banner();
            ExitCode::SUCCESS
        }
        Some(Command::Extract(a)) => run_extract(a),
        Some(Command::Train(c)) => run_train_cmd(c),
        Some(Command::Calibrate(c)) => run_calibrate_cmd(c),
        Some(Command::Fit(c)) => run_fit_cmd(c),
        Some(Command::Check(c)) => run_check_cmd(c),
        Some(Command::Inspect(c)) => run_inspect_cmd(c),
        Some(Command::Mute(c)) => run_mute_cmd(c),
        Some(Command::ListMutes) => run_list_mutes(),
        Some(Command::ReviewMutes(c)) => run_review_mutes_cmd(c),
        Some(Command::Score(c)) => run_score_cmd(c),
        Some(Command::Status) => run_status(),
        Some(Command::List) => run_list(),
        Some(Command::Update) => run_update(),
    }
}
