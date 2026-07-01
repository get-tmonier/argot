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
use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::adapters::typescript::TypeScriptAdapter;
use argot_core::scoring::adapters::{Language, LanguageAdapter};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use argot_core::text::read_text_lossy;
use argot_core::train::run_train;

/// Format the current time as an ISO 8601 UTC string (calibration metadata;
/// not parity-relevant). Uses Howard Hinnant's civil-from-days algorithm.
fn iso_now() -> String {
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = (secs / 86400) as i64;
    let rem = secs % 86400;
    let (h, mi, s) = (rem / 3600, (rem % 3600) / 60, rem % 60);
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
        "argot v{version}\n\nCOMMANDS\n  extract    Walk git history into a training dataset (.argot/dataset.jsonl)\n  fit        Fit the voice model to this repo (= train + calibrate, one-shot)\n  check      Check changes against the fitted voice\n  status     Show current repository's argot state\n  list       List all registered repositories\n  update     Update the argot CLI\n\nTypical first run: argot extract && argot fit && argot check\nRun `argot <command> --help` for details on any command."
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
    };
    match run_calibrate(&c.repo, &c.repo_corpus, &generic, &c.output, &opts) {
        Ok(thresholds) => {
            for (lang, t) in &thresholds {
                println!("[{lang}] threshold: {t:.4}");
            }
            let langs: Vec<&str> = thresholds.iter().map(|(l, _)| l.as_str()).collect();
            println!(
                "scorer-config.json (v2, languages: {}) → {}",
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
        n_cal: 500,
        seed: 0,
        n_seeds: 7,
        evidence_top_n: 50,
        repo_sha,
        timestamp_utc: iso_now(),
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
    });
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
        Some(Command::Score(c)) => run_score_cmd(c),
        Some(Command::Status) => run_status(),
        Some(Command::List) => run_list(),
        Some(Command::Update) => run_update(),
    }
}
