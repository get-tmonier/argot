//! argot CLI — clap-based entry point.
//!
//! One statically-linked binary, no subprocess: the full pipeline
//! (`extract` → `train` → `calibrate` → `check`, plus the `fit` one-shot and
//! the suppression commands) runs in-process against `argot-core`.

mod audit;
mod auto_refit;
mod cache_cmd;
mod describe;
mod hook;
mod mcp;
mod review;
mod uninstall;
#[cfg(feature = "self-update")]
mod update;
#[cfg(feature = "self-update")]
mod update_check;
mod voice_diff;
mod worktree;

use clap::{Args, CommandFactory, Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::io::{BufWriter, IsTerminal, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::extract::{write_dataset, ExtractError};
use argot_core::git_walk::{head_sha, repo_exists};
use argot_core::inspect::{
    format_shares, inspect_model, inspect_repo, InspectReport, ModelReport, ReasonLevel, Verdict,
};
use argot_core::output::OutputFormat;
use argot_core::scoring::adapters::c::CAdapter;
use argot_core::scoring::adapters::cpp::CppAdapter;
use argot_core::scoring::adapters::csharp::CSharpAdapter;
use argot_core::scoring::adapters::go::GoAdapter;
use argot_core::scoring::adapters::java::JavaAdapter;
use argot_core::scoring::adapters::javascript::JavaScriptAdapter;
use argot_core::scoring::adapters::pascal::PascalAdapter;
use argot_core::scoring::adapters::php::PhpAdapter;
use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::adapters::ruby::RubyAdapter;
use argot_core::scoring::adapters::rust::RustAdapter;
use argot_core::scoring::adapters::typescript::TypeScriptAdapter;
use argot_core::scoring::adapters::{Language, LanguageAdapter};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use argot_core::suppress::{suggest_ignores, IgnoreSuggestions};
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

/// A fit older than this many days earns a soft "consider re-fitting" hint —
/// generous on purpose (a stale model is lower-confidence, not wrong). Never
/// affects exit codes or the verdict; it's a nudge for the "fit once, forgot"
/// case. Internal, not a user knob.
const STALE_FIT_DAYS: i64 = 90;

/// Days since the Unix epoch for a civil date — inverse of `civil_from_days`
/// (Howard Hinnant's `days_from_civil`).
fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = (if y >= 0 { y } else { y - 399 }) / 400;
    let yoe = y - era * 400;
    let mp = if m > 2 { m - 3 } else { m + 9 };
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146097 + doe - 719468
}

/// Whole days between a fit timestamp (ISO `YYYY-MM-DD…`) and `today`
/// (`YYYY-MM-DD`). `None` if either can't be parsed.
fn days_since_fit(fit_ts: &str, today: &str) -> Option<i64> {
    let parse = |s: &str| -> Option<(i64, i64, i64)> {
        let mut it = s.get(..10)?.split('-');
        Some((
            it.next()?.parse().ok()?,
            it.next()?.parse().ok()?,
            it.next()?.parse().ok()?,
        ))
    };
    let (fy, fm, fd) = parse(fit_ts)?;
    let (ty, tm, td) = parse(today)?;
    Some(days_from_civil(ty, tm, td) - days_from_civil(fy, fm, fd))
}

/// The calibration timestamp from a fitted `scorer-config.json` (any language;
/// they share the fit time). Used only for the staleness hint.
fn fit_timestamp(argot_dir: &Path) -> Option<String> {
    let bytes = fs::read(argot_dir.join("scorer-config.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("languages")?.as_object()?.values().find_map(|lc| {
        lc.get("calibration")?
            .get("timestamp_utc")?
            .as_str()
            .map(String::from)
    })
}

#[derive(Parser)]
#[command(
    name = "argot",
    version,
    about = "Surface repository-grounded divergence before code is accepted.",
    // The docs site serves an agent-readable mirror, and nothing local used to
    // point at it — so an agent configuring argot fetched HTML and read it
    // through a converter, several tool calls after giving up on `--help`.
    after_help = "Docs: https://argot.tmonier.com/docs/\n\
                  Coding agents: https://argot.tmonier.com/llms.txt"
)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Audit recent history for patterns that would have prompted review.
    /// It scores the surviving base-to-head change and classifies only
    /// explicit AI markers; informational, exits 0.
    Audit(AuditCmd),
    /// Set up argot for this repo: fit the voice model and report its health
    /// (`--suggest` lists directories you may want to exclude first).
    Init(InitCmd),
    /// Extract dataset from git history. Plumbing for the bench; hidden —
    /// the fit → check flow never consumes the dataset.
    #[command(hide = true)]
    Extract(ExtractArgs),
    /// Collect the repo corpus + generic baseline. Plumbing behind `fit`; hidden.
    #[command(hide = true)]
    Train(TrainCmd),
    /// Calibrate the per-language threshold. Plumbing behind `fit`; hidden.
    #[command(hide = true)]
    Calibrate(CalibrateCmd),
    /// Fit the voice model to this repo (train + calibrate, one-shot).
    Fit(FitCmd),
    /// Check code changes against the calibrated scorers.
    Check(CheckCmd),
    /// List every rule with its group and effective severity for this repo.
    Rules(RulesCmd),
    /// Score a PR (or diff range) against the local voice without checking it out.
    Review(ReviewCmd),
    /// PR-level out-of-voice metric plus ranked hot-spots for a ref/range.
    /// Informational: always exits 0 (use `check`/`review` to gate).
    #[command(name = "voice-diff")]
    VoiceDiff(VoiceDiffCmd),
    /// Report corpus composition, calibration health, and repo suitability.
    Inspect(InspectCmd),
    /// Accept a finding: by hash for one hit, or `--path` for a standing rule
    /// covering a whole tree (appends a [[mute]] to argot.toml).
    Mute(MuteCmd),
    /// List active suppressions across argot.toml [exclude], inline comments,
    /// and argot.toml [[mute]].
    #[command(name = "list-mutes")]
    ListMutes,
    /// Re-score muted files and report which suppressions no longer fire.
    #[command(name = "review-mutes")]
    ReviewMutes(ReviewMutesCmd),
    /// Batch-score hunks from stdin (benchmark harness seam). Hidden.
    #[command(hide = true)]
    Score(ScoreCmd),
    /// Show the current repository's argot state.
    Status(StatusCmd),
    /// List all registered repositories.
    List(ListCmd),
    /// Update the argot CLI to the latest release.
    Update,
    /// Inspect or clear the machine-wide embedding cache (`~/.cache/argot`).
    Cache(cache_cmd::CacheCmd),
    /// Remove argot from this machine: every repo's artifacts, the machine
    /// cache, global state, and the binary (shows the full list first).
    Uninstall(UninstallCmd),
    /// Refresh the cached version.json state (spawned detached; hidden).
    #[cfg(feature = "self-update")]
    #[command(name = "refresh-version-cache", hide = true)]
    RefreshVersionCache,
    /// Refit the voice model in the background (spawned detached; hidden).
    #[command(name = "background-refit", hide = true)]
    BackgroundRefit(BackgroundRefitCmd),
    /// Run a Model Context Protocol server for LLM coding agents (stdio).
    Mcp(McpCmd),
    /// Generate a STYLE.md describing the repo's learned voice.
    #[command(name = "describe-voice")]
    DescribeVoice(DescribeVoiceCmd),
    /// List the conventions this repo already follows — naming, syntax idioms,
    /// familiar imports, and typical calls by area — from the fitted model.
    Conventions(ConventionsCmd),
    /// Pre-write guardrail for coding agents: reads a Claude Code PreToolUse
    /// event on stdin and asks before a dependency foreign to the repo is
    /// written. Wired into `.claude/settings.json` by `argot-setup` on opt-in.
    #[command(hide = true)]
    Hook(HookCmd),
}

#[derive(Args)]
struct DescribeVoiceCmd {
    /// Path to the repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Typical callees listed per cluster.
    #[arg(long = "top", value_name = "N", default_value_t = describe::DEFAULT_TOP)]
    top: usize,
    /// Write to this file instead of stdout (e.g. `STYLE.md`).
    #[arg(long, value_name = "FILE")]
    out: Option<PathBuf>,
}

#[derive(Args)]
struct HookCmd {
    /// Path to the repository (defaults to the project dir).
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

#[derive(Args)]
struct ConventionsCmd {
    /// Path to the repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Output format: human (terminal) or json (stable machine-readable).
    #[arg(long, default_value = "human", value_parser = ["human", "json"])]
    format: String,
    /// Callees listed per cluster and idioms listed per language.
    #[arg(long = "top", value_name = "N", default_value_t = describe::DEFAULT_TOP)]
    top: usize,
}

fn run_conventions_cmd(c: ConventionsCmd) -> ExitCode {
    let catalog = match argot_core::convention_catalog::build_catalog(&c.repo, c.top) {
        Ok(cat) => cat,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    if c.format == "json" {
        match serde_json::to_string_pretty(&catalog) {
            Ok(s) => {
                println!("{s}");
                return ExitCode::SUCCESS;
            }
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
    }

    if catalog.languages.is_empty() {
        println!("No conventions learned yet — run `argot fit` first.");
        return ExitCode::SUCCESS;
    }

    println!("Conventions of this repository");
    println!(
        "Model {}. The repo's own API — the shared helpers and objects it routes through.\n",
        catalog.model_hash
    );
    let conv_line = |label: &str, items: &[argot_core::convention_catalog::Convention]| {
        if items.is_empty() {
            return;
        }
        let shown: Vec<String> = items
            .iter()
            .take(10)
            .map(|c| {
                // "Effect (232 files, called in 190)" — show the receiver signal
                // only when the symbol is both imported and routed through.
                if c.imported_in > 0 && c.called_in > 0 {
                    format!(
                        "{} ({} files, called in {})",
                        c.name, c.imported_in, c.called_in
                    )
                } else {
                    format!("{} ({} files)", c.name, c.imported_in.max(c.called_in))
                }
            })
            .collect();
        println!("  {label}  {}", shown.join(", "));
    };
    for (lang, conv) in &catalog.languages {
        println!("── {lang} ({} files) ──", conv.corpus_files);
        if !conv.naming.is_empty() {
            let shapes: Vec<String> = conv
                .naming
                .iter()
                .map(|s| format!("{} {:.0}%", s.shape, s.fraction * 100.0))
                .collect();
            println!("  Naming             {}", shapes.join(" · "));
        }
        // `idioms` (raw node-kind frequencies) stay in --format json only: the
        // top kinds are always the generic ones (identifier, call_expression),
        // which read as noise in the terse human list, and there is no
        // corpus-agnostic way to rank distinctiveness without a hardcoded
        // node-kind denylist. The useful idiom-conventions ("use Effect.gen",
        // "decorators live here") already surface via placement + vocabulary.
        if !conv.familiar_imports.is_empty() {
            let shown = 12.min(conv.familiar_imports.len());
            let mut imports = conv.familiar_imports[..shown].join(", ");
            if conv.familiar_imports.len() > shown {
                imports.push_str(&format!(", +{} more", conv.familiar_imports.len() - shown));
            }
            println!("  Familiar imports   {imports}");
        }
        conv_line("Vocabulary        ", &conv.vocabulary);
        conv_line("Type funnels      ", &conv.type_funnels);
        conv_line("Routing objects   ", &conv.routing_objects);
        for m in &conv.migrations {
            println!(
                "  In migration       {} → {} ({} commits, {}..{}) — {} file(s) still to migrate",
                m.old, m.new, m.commits, m.first, m.last, m.leftover_count
            );
            if !m.leftovers.is_empty() {
                let shown = 5.min(m.leftovers.len());
                let mut files = m.leftovers[..shown].join(", ");
                if m.leftover_count > shown {
                    files.push_str(&format!(", +{} more", m.leftover_count - shown));
                }
                println!("                     {files}");
            }
        }
        println!();
    }

    if !catalog.declared_migrations.is_empty() {
        println!("── declared migrations (argot.toml) ──");
        for m in &catalog.declared_migrations {
            println!(
                "  {} → {} ({}) — {} · {} file(s) still to migrate",
                m.from, m.to, m.kind, m.reason, m.leftover_count
            );
            if !m.leftovers.is_empty() {
                let shown = 5.min(m.leftovers.len());
                let mut files = m.leftovers[..shown].join(", ");
                if m.leftover_count > shown {
                    files.push_str(&format!(", +{} more", m.leftover_count - shown));
                }
                println!("    {files}");
            }
        }
        println!();
    }

    if !catalog.placement.is_empty() {
        println!("── placement (where a kind of code lives) ──");
        for p in &catalog.placement {
            let sig: Vec<String> = p
                .signature
                .iter()
                .take(5)
                .map(|f| f.feature.clone())
                .collect();
            println!("  {:<26} ({}f)  {}", p.location, p.files, sig.join(", "));
        }
        println!();
    }
    ExitCode::SUCCESS
}

#[derive(Args)]
struct UninstallCmd {
    /// Show everything that would be removed, remove nothing.
    #[arg(long)]
    dry_run: bool,
    /// Skip the confirmation prompt (required when not on a terminal).
    #[arg(long)]
    yes: bool,
}

#[derive(Args)]
struct McpCmd {
    /// Path to the repository whose fitted voice the server scores against.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

// --- repo context / registry ---

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

/// The user's home directory across platforms: `$HOME` (Unix), then
/// `%USERPROFILE%` and `%HOMEDRIVE%%HOMEPATH%` (Windows, where `HOME` is usually
/// unset). Empty values are ignored so the global registry never silently lands
/// in the current directory.
fn home_dir() -> Option<PathBuf> {
    let non_empty = |var: &str| std::env::var_os(var).filter(|v| !v.is_empty());
    if let Some(h) = non_empty("HOME").or_else(|| non_empty("USERPROFILE")) {
        return Some(PathBuf::from(h));
    }
    match (non_empty("HOMEDRIVE"), non_empty("HOMEPATH")) {
        (Some(drive), Some(path)) => {
            let mut home = PathBuf::from(drive);
            home.push(path);
            Some(home)
        }
        _ => None,
    }
}

fn settings_path() -> PathBuf {
    // With no discoverable home the global registry is a no-op convenience
    // (status/list); fall back to the cwd rather than crash.
    home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".argot")
        .join("settings.json")
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

/// The repo root of the current directory, resolved in-process via libgit2
/// (`argot_core::git_walk::repo_workdir`) — no external `git` binary, so it
/// behaves identically on Windows, macOS, and Linux.
fn git_toplevel() -> Option<String> {
    git_toplevel_at(".")
}

fn git_toplevel_at(path: &str) -> Option<String> {
    argot_core::git_walk::repo_workdir(path)
}

/// Upsert `git_root` into the global registry (`~/.argot/settings.json`) —
/// how `argot list`/`status` and `argot uninstall` know where artifacts live.
/// Returns the repo's display name.
fn register_repo(git_root: &str) -> String {
    let mut settings = read_settings();
    let now = iso_now();
    let basename = std::path::Path::new(git_root)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| git_root.to_string());
    let entry = settings
        .repos
        .entry(git_root.to_string())
        .or_insert_with(|| RepoEntry {
            name: basename.clone(),
            registered_at: now.clone(),
            last_used_at: now.clone(),
        });
    entry.last_used_at = now;
    let name = entry.name.clone();
    write_settings(&settings);
    name
}

fn resolve_context() -> RepoCtx {
    resolve_context_at(".")
}

/// The repo context for an explicitly named path, so a command can answer for
/// a repository the process is not sitting in — what every other `--repo`
/// taking command already does.
fn resolve_context_at(path: &str) -> RepoCtx {
    let git_root = git_toplevel_at(path).unwrap_or_else(|| {
        std::fs::canonicalize(path)
            .unwrap_or_else(|_| std::env::current_dir().unwrap_or_default())
            .display()
            .to_string()
    });
    let name = register_repo(&git_root);
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

#[derive(Args)]
struct StatusCmd {
    /// Path to the repository to report on.
    #[arg(long, default_value = ".")]
    repo: String,
    /// Output format: human (terminal) or json (stable machine-readable).
    #[arg(long, default_value = "human", value_parser = ["human", "json"])]
    format: String,
}

fn run_status(c: StatusCmd) -> ExitCode {
    let ctx = resolve_context_at(&c.repo);
    let dataset = fs::metadata(&ctx.dataset_path).ok().map(|m| {
        let count = fs::read_to_string(&ctx.dataset_path)
            .map(|s| s.lines().filter(|l| !l.trim().is_empty()).count())
            .unwrap_or(0);
        (count, m.len())
    });
    let model_bytes = fs::metadata(&ctx.repo_corpus_path).ok().map(|m| m.len());
    let calibrated = ctx.argot_dir.join("scorer-config.json").exists();

    // The one-stop "is my setup healthy / is it time to recalibrate?" answer:
    // freshness (commits behind), config sync, and calibration drift — all
    // read from what the last fit persisted, no tree walk here.
    let health = argot_core::health::read(&ctx.argot_dir);
    let (status_config, _) = argot_core::compose::load_config(Path::new(&ctx.git_root));
    let behind = health.as_ref().and_then(|h| {
        (!h.fit_sha.is_empty())
            .then(|| {
                argot_core::check::accepted_source_commits_behind(
                    &ctx.git_root,
                    &h.fit_sha,
                    &status_config,
                    status_config.fit_refresh_after,
                )
            })
            .flatten()
    });
    let config_in_sync = health.as_ref().map(|h| {
        h.config_fingerprint.is_empty()
            || h.config_fingerprint == argot_core::health::config_fingerprint(&status_config)
    });
    let drift: Vec<String> = health
        .as_ref()
        .map(|h| h.drift_candidates.clone())
        .unwrap_or_default();

    // In-progress migrations: mined supersessions from the artifact plus
    // declared [[migration]] entries — the freshness surface's "the repo is
    // moving on" counterpart.
    let (mined_migrations, leftover_files) = if calibrated {
        fs::read(ctx.argot_dir.join("scorer-config.json"))
            .ok()
            .and_then(|b| serde_json::from_slice::<serde_json::Value>(&b).ok())
            .and_then(|v| {
                v.get("languages").and_then(|l| l.as_object()).map(|langs| {
                    let mut mined = 0usize;
                    let mut leftovers = 0u64;
                    for lc in langs.values() {
                        if let Some(arr) = lc.get("supersessions").and_then(|s| s.as_array()) {
                            mined += arr.len();
                            leftovers += arr
                                .iter()
                                .filter_map(|s| s.get("leftover_count").and_then(|c| c.as_u64()))
                                .sum::<u64>();
                        }
                    }
                    (mined, leftovers as usize)
                })
            })
            .unwrap_or((0, 0))
    } else {
        (0, 0)
    };
    let declared_migrations = status_config.migrations().active.len();

    if wants_json(&c.format) {
        let doc = serde_json::json!({
            "repo": { "name": ctx.name, "path": ctx.git_root },
            "dataset": dataset.map(|(count, bytes)| serde_json::json!({
                "records": count, "bytes": bytes,
            })),
            "model": { "trained": model_bytes.is_some(), "bytes": model_bytes },
            "calibrated": calibrated,
            "health": health.as_ref().map(|h| serde_json::json!({
                "fit_sha": h.fit_sha,
                "commits_behind": behind,
                "config_in_sync": config_in_sync,
                "drift_candidates": drift,
            })),
            "migrations": {
                "mined": mined_migrations,
                "declared": declared_migrations,
                "leftover_files": leftover_files,
            },
        });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
        return ExitCode::SUCCESS;
    }

    println!("Repo:     {} ({})", ctx.name, ctx.git_root);
    match dataset {
        Some((count, bytes)) => println!("Dataset:  {} records · {}", count, format_bytes(bytes)),
        None => println!("Dataset:  —"),
    }
    match model_bytes {
        Some(bytes) => println!("Model:    trained · {}", format_bytes(bytes)),
        None => println!("Model:    not trained"),
    }
    if calibrated {
        println!("Calibrated: yes");
    } else {
        println!("Calibrated: not calibrated — run `argot fit`");
    }
    if let Some(h) = &health {
        let fresh = match behind {
            Some(0) => "fresh (nothing accepted since the fit)".to_string(),
            Some(n) => {
                let plus = if n >= status_config.fit_refresh_after {
                    "+"
                } else {
                    ""
                };
                format!("{n}{plus} accepted source commit(s) behind — auto-refresh will refit")
            }
            None => "unknown".to_string(),
        };
        println!(
            "Voice:    fitted at {} · {fresh}",
            &h.fit_sha[..12.min(h.fit_sha.len())]
        );
        match config_in_sync {
            Some(true) => println!("Config:   in sync with the fit"),
            Some(false) => {
                println!("Config:   argot.toml changed since the fit — auto-refresh will refit")
            }
            None => {}
        }
        if drift.is_empty() {
            println!("Hygiene:  no unexcluded generated/data-heavy/vendored directories");
        } else {
            println!(
                "Hygiene:  {} director{} shaping the voice that look generated, data-heavy, or vendored — `argot init --suggest`",
                drift.len(),
                if drift.len() != 1 { "ies" } else { "y" }
            );
        }
    }
    if mined_migrations + declared_migrations > 0 {
        println!(
            "Migrations: {} in progress ({} mined, {} declared) · {} file(s) still to migrate — `argot conventions`",
            mined_migrations + declared_migrations,
            mined_migrations,
            declared_migrations,
            leftover_files
        );
    }
    ExitCode::SUCCESS
}

#[derive(Args)]
struct ListCmd {
    /// Output format: human (terminal) or json (stable machine-readable).
    #[arg(long, default_value = "human", value_parser = ["human", "json"])]
    format: String,
}

fn run_list(c: ListCmd) -> ExitCode {
    let current = git_toplevel();
    let settings = read_settings();
    let mut repos: Vec<(&String, &RepoEntry)> = settings.repos.iter().collect();
    repos.sort_by(|a, b| a.1.name.cmp(&b.1.name));

    if wants_json(&c.format) {
        let items: Vec<serde_json::Value> = repos
            .iter()
            .map(|(path, entry)| {
                serde_json::json!({
                    "name": entry.name,
                    "path": path,
                    "current": Some(*path) == current.as_ref(),
                })
            })
            .collect();
        let doc = serde_json::json!({ "repos": items });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap_or_default());
        return ExitCode::SUCCESS;
    }

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

/// npm installs live under `node_modules`; the receipt-based updater would
/// touch a different install, so those must be updated through npm itself.
fn is_npm_install(exe: Option<&Path>) -> bool {
    exe.is_some_and(|p| p.components().any(|c| c.as_os_str() == "node_modules"))
}

/// Builds without the `self-update` feature (dev/CI) keep the command but
/// can only point at the other update channels.
#[cfg(not(feature = "self-update"))]
fn run_update() -> ExitCode {
    let current = env!("CARGO_PKG_VERSION");

    if is_npm_install(std::env::current_exe().ok().as_deref()) {
        println!("argot {current} was installed via npm; update it with:");
        println!("  npm install -g @tmonier/argot@latest");
        return ExitCode::SUCCESS;
    }

    eprintln!("argot {current}: this build was compiled without self-update support.");
    eprintln!("Update via the installer:");
    eprintln!("  curl -LsSf https://github.com/get-tmonier/argot/releases/latest/download/argot-installer.sh | sh");
    ExitCode::FAILURE
}

#[cfg(feature = "self-update")]
fn run_update() -> ExitCode {
    update::run_update()
}

/// Whether a command should emit its JSON document (the shared `--format
/// json` idiom).
fn wants_json(format: &str) -> bool {
    format == "json"
}

/// End-of-command freshness hook (notice + detached refresh). A no-op in
/// builds without `self-update` — dev/CI binaries never phone home.
fn freshness_hook() {
    #[cfg(feature = "self-update")]
    update_check::maybe_notify_and_refresh();
}

fn print_help_banner() {
    print!("{}", root_help());
}

fn root_help() -> String {
    Cli::command().render_help().to_string()
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
    #[arg(long = "n-cal", default_value_t = 100)]
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
    /// ≤ N cluster files is treated as cluster-absent. 0 disables the rule; the
    /// default of 2 treats callees in one or two cluster files as rare.
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
    /// Calibrate an extra threshold per slice (repeatable): `path:<prefix>`,
    /// `author:<email>`, or `auto`.
    #[arg(long = "slice", value_name = "SPEC")]
    slice: Vec<String>,
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
        slices: c.slice.clone(),
        ..CalibrateOptions::default()
    };
    match run_calibrate(&c.repo, &c.repo_corpus, &generic, &c.output, &opts) {
        Ok(thresholds) => {
            for (lang, t) in &thresholds {
                println!("[{lang}] threshold: {t:.4}");
            }
            let langs: Vec<&str> = thresholds.iter().map(|(l, _)| l.as_str()).collect();
            println!(
                "scorer-config.json (languages: {}) → {}",
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
    /// Calibrate an extra threshold for a slice of the repo (repeatable):
    /// `path:<prefix>` (a subdirectory), `author:<email>`, or `auto` (one per
    /// top-level directory). Hunks in a slice are judged against its threshold.
    #[arg(long = "slice", value_name = "SPEC")]
    slice: Vec<String>,
}

/// Warn that a dirty working tree will be folded into the learned voice.
/// argot calibrates from files as they are on disk, so uncommitted foreign
/// code (an agent's just-written change, a vendored paste) would be laundered
/// into "known-good" and never flagged. Advisory only — the fit still runs.
fn warn_uncommitted(paths: &[String]) {
    eprintln!(
        "warning: {} uncommitted source change(s) will be learned as part of this repo's voice:",
        paths.len()
    );
    for p in paths.iter().take(5) {
        eprintln!("           {p}");
    }
    if paths.len() > 5 {
        eprintln!("           … and {} more", paths.len() - 5);
    }
    eprintln!("         Commit or stash work in progress first — otherwise code you're about to");
    eprintln!("         check gets baked into the baseline it's checked against.");
}

/// Train + calibrate the repo's voice model into `.argot/`, printing the
/// two-step progress and protecting the (rebuildable, heavy) model dir from
/// version control. Returns the scorer-config path on success. Shared by `fit`
/// and `init`; failures are always setup errors (exit 2).
fn fit_repo(repo: &Path, slices: &[String]) -> Result<PathBuf, ()> {
    // Every fitted repo lands in the global registry so `argot list`/`status`
    // — and `argot uninstall` — know where artifacts live. argot's own
    // throwaway worktrees (audit, the background refresh) live under the OS
    // temp dir and are not user repos.
    let canon = |p: &Path| fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let repo_canon = canon(repo);
    if !repo_canon.starts_with(canon(&std::env::temp_dir())) {
        register_repo(&repo_canon.to_string_lossy());
    }
    let argot_dir = repo.join(".argot");
    if let Err(e) = ensure_model_gitignored(&argot_dir) {
        eprintln!(
            "warning: could not protect {} from git: {e}",
            argot_dir.display()
        );
    }
    // NB: `fit` is deliberately side-effect-free on the user's tracked tree —
    // it writes only inside the gitignored `.argot/`. Scaffolding `argot.toml`
    // and gitignoring the local override is `init`'s job (see `run_init_cmd`).
    // `fit` runs non-interactively on arbitrary detached checkouts (CI fits on
    // the PR base, then restores the head), so writing an untracked `argot.toml`
    // or editing the root `.gitignore` here would make the follow-up checkout
    // abort — a bootstrap-only failure the first time a repo adds argot.toml.
    // When `argot.toml` is absent, `config::load` already returns the default
    // in-memory, so nothing needs to be persisted for the fit to be correct.
    // argot fits from files as they are on disk. If the working tree is dirty,
    // uncommitted code — e.g. an agent's just-written foreign change — would be
    // folded into the learned voice and then read as familiar. Warn so the user
    // commits/stashes first; never block (best-effort, advisory).
    let dirty = argot_core::git_walk::uncommitted_source_paths(&repo.to_string_lossy());
    if !dirty.is_empty() {
        warn_uncommitted(&dirty);
    }
    // Same laundering risk one level up: a manual fit on a feature branch
    // learns its unmerged commits, and argot stops flagging what it has
    // learned. Advisory only — agents, scripts, and hooks drive fit too, so
    // never a prompt — and silent when the branch adds nothing in scope,
    // on detached HEAD (audit/refresh worktrees), or when the repo declared
    // branch fits intended via `[fit] refresh-from = "current-branch"`.
    let (fit_config, _) = argot_core::compose::load_config(repo);
    if let Some((branch, n)) = argot_core::check::unmerged_branch_source_commits(
        &repo.to_string_lossy(),
        &fit_config,
        argot_core::check::FRESHNESS_SCAN_CAP,
    ) {
        eprintln!(
            "warning: fitting on branch '{branch}' — its {n} unmerged source commit(s) will be"
        );
        eprintln!("         learned as this repo's voice, and argot stops flagging what it has");
        eprintln!("         learned. Merged code is the safer voice: fit from the default");
        eprintln!("         branch, or declare branch fits intended with");
        eprintln!("         `[fit] refresh-from = \"current-branch\"` in argot.toml.");
    }
    let repo_corpus = argot_dir.join("repo-corpus.txt");
    let generic = argot_dir.join("generic-baseline.json");
    let scorer_config = argot_dir.join("scorer-config.json");

    // Progress goes to stderr: fit runs inside commands whose stdout is a
    // machine document (audit --format json) — status must never mix in.
    eprintln!("Step 1/2: training voice model …");
    let t_train = argot_core::timing::phase("fit: train");
    if let Err(e) = run_train(repo, &repo_corpus, &generic) {
        eprintln!("error: {e}");
        return Err(());
    }
    t_train.done();

    eprintln!("Step 2/2: calibrating threshold …");
    let t_cal = argot_core::timing::phase("fit: calibrate (total)");
    let generic_bytes = match fs::read(&generic) {
        Ok(b) => b,
        Err(_) => {
            eprintln!("error: generic baseline missing after train");
            return Err(());
        }
    };
    let repo_sha = head_sha(&repo.to_string_lossy()).unwrap_or_else(|| "unknown".to_string());
    let opts = CalibrateOptions {
        repo_sha,
        timestamp_utc: iso_now(),
        slices: slices.to_vec(),
        ..CalibrateOptions::default()
    };
    if let Err(e) = run_calibrate(repo, &repo_corpus, &generic_bytes, &scorer_config, &opts) {
        eprintln!("error: {e}");
        return Err(());
    }
    t_cal.done();
    Ok(scorer_config)
}

fn run_fit_cmd(c: FitCmd) -> ExitCode {
    match fit_repo(&c.repo, &c.slice) {
        Ok(scorer_config) => {
            println!("Done. Scorer config: {}", scorer_config.display());
            drift_suggestions_note(&c.repo);
            config_in_voice_note(&c.repo);
            freshness_hook();
            ExitCode::SUCCESS
        }
        Err(()) => ExitCode::from(2),
    }
}

#[derive(Args)]
struct InitCmd {
    /// Path to the repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Don't fit — instead list directories you may want to add to
    /// `argot.toml [exclude].paths` first (statistical evidence only; you decide).
    #[arg(long)]
    suggest: bool,
    /// Output format for `--suggest`: human (terminal) or json (stable,
    /// machine-readable — consumed by the setup skill).
    #[arg(long, default_value = "human", value_parser = ["human", "json"])]
    format: String,
}

/// Config quality IS the experience: a fit that ingests vendored, generated,
/// or data-heavy directories speaks with the wrong voice and flags the wrong
/// things — and a repo DRIFTS into that state (a new `gen/`, a vendored SDK
/// added last month). `suggest_ignores` only returns directories NOT already
/// excluded, so this stays quiet on a well-configured repo and speaks up the
/// fit after the tree grew something the voice shouldn't learn.
fn drift_suggestions_note(repo: &Path) {
    // The fit that just ran persisted its own scan (`.argot/health.json`);
    // read it rather than walking the tree twice. Fallback: live scan.
    let candidates: Vec<String> = argot_core::health::read(&repo.join(".argot"))
        .map(|h| h.drift_candidates)
        .unwrap_or_else(|| {
            suggest_ignores(repo)
                .candidates
                .into_iter()
                .map(|c| c.path)
                .collect()
        });
    if candidates.is_empty() {
        return;
    }
    let plural = if candidates.len() != 1 { "ies" } else { "y" };
    let names: Vec<&str> = candidates.iter().take(3).map(String::as_str).collect();
    let more = if candidates.len() > 3 { ", …" } else { "" };
    println!();
    println!(
        "note: {} director{plural} look generated, data-heavy, or vendored and are shaping the voice ({}{more}).",
        candidates.len(),
        names.join(", ")
    );
    println!("      Review with `argot init --suggest`, or let the setup skill decide:");
    println!("      npx skills add get-tmonier/argot   then run /argot-setup");
}

/// After a fit, report files the built-in `argot:recommended` set would exclude
/// that are shaping the voice anyway — config/tooling/doc files the corpus walk
/// keeps because it applies only `[exclude].paths`. A misconfiguration argot
/// detects and surfaces rather than silently auto-dropping: the reader decides
/// whether to add them to `[exclude].paths`. Quiet on a clean repo.
fn config_in_voice_note(repo: &Path) {
    let leaked = argot_core::corpus::recommended_excluded_in_corpus(repo);
    if leaked.is_empty() {
        return;
    }
    let names: Vec<&str> = leaked.iter().take(3).map(String::as_str).collect();
    let more = if leaked.len() > 3 { ", …" } else { "" };
    let plural = if leaked.len() != 1 { "s" } else { "" };
    println!();
    println!(
        "note: {} file{plural} argot:recommended would exclude are shaping the voice ({}{more}).",
        leaked.len(),
        names.join(", ")
    );
    println!("      These look like config/tooling, not authored code. Drop them from the");
    println!("      model by adding them to argot.toml [exclude].paths, then re-run argot init.");
}

fn run_init_cmd(c: InitCmd) -> ExitCode {
    if c.suggest {
        return run_init_suggest(&c);
    }

    let scorer_config = match fit_repo(&c.repo, &[]) {
        Ok(p) => p,
        Err(()) => return ExitCode::from(2),
    };

    // `init` is the interactive, one-time setup — this is where the config is
    // scaffolded (a default `argot.toml` when absent) and the personal-override
    // file is kept out of version control. `fit` deliberately does neither.
    match ensure_config_present(&c.repo) {
        Ok((wrote_config, appended_gitignore)) => {
            if wrote_config {
                println!("Wrote argot.toml — the commented default config; commit it.");
            }
            if appended_gitignore {
                println!(
                    "Added argot.local.toml to .gitignore — personal overrides stay uncommitted."
                );
            }
        }
        Err(e) => eprintln!("warning: could not write argot.toml: {e}"),
    }

    let report = match inspect_repo(&c.repo) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let use_color = std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    println!();
    print!("{}", render_inspect_human(&report, use_color, &today_utc()));
    println!();

    match report.verdict {
        Verdict::Ready => {
            println!("Voice model fitted → {}", scorer_config.display());
            drift_suggestions_note(&c.repo);
            config_in_voice_note(&c.repo);
            print_init_next_actions(Verdict::Ready);
        }
        Verdict::ReadyWithNotes => {
            println!("Voice model fitted → {}", scorer_config.display());
            drift_suggestions_note(&c.repo);
            config_in_voice_note(&c.repo);
            print_init_next_actions(Verdict::ReadyWithNotes);
            println!("The notes above are tuning hints, not blockers. To refine the corpus:");
            println!(
                "  • argot init --suggest    # directories you may want to exclude from the voice"
            );
            println!("  • edit argot.toml [exclude], then re-run  argot init");
        }
        Verdict::NotRecommended => {
            println!("Voice model fitted → {}", scorer_config.display());
            config_in_voice_note(&c.repo);
            println!("The corpus looks not recommended yet. Next steps:");
            println!(
                "  • argot init --suggest    # directories you may want to exclude from the voice"
            );
            println!("  • edit argot.toml [exclude], then re-run  argot init");
            println!("  • argot check             # manual smoke check for your working changes");
            print_init_recurring_actions(Verdict::NotRecommended);
        }
    }
    ExitCode::SUCCESS
}

fn print_init_next_actions(verdict: Verdict) {
    println!("Next:  argot check          # manual smoke check for your working changes");
    print_init_recurring_actions(verdict);
}

fn print_init_recurring_actions(verdict: Verdict) {
    for line in init_recurring_actions(verdict) {
        println!("{line}");
    }
}

fn init_recurring_actions(verdict: Verdict) -> [&'static str; 4] {
    [
        match verdict {
            Verdict::Ready => "Then choose a recurring path you configure:",
            Verdict::ReadyWithNotes => {
                "After reviewing the notes, choose a recurring path you configure:"
            }
            Verdict::NotRecommended => {
                "After the corpus is ready, choose a recurring path you configure:"
            }
        },
        "  • pre-commit              # runs at commit time after user configuration",
        "  • GitHub Action           # runs in a user-configured CI workflow",
        "    https://argot.tmonier.com/docs/ci/",
    ]
}

fn run_init_suggest(c: &InitCmd) -> ExitCode {
    let suggestions = suggest_ignores(&c.repo);
    if wants_json(&c.format) {
        match serde_json::to_string_pretty(&suggestions) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::from(2)
            }
        }
    } else {
        print!("{}", render_suggestions_human(&suggestions));
        ExitCode::SUCCESS
    }
}

fn render_suggestions_human(s: &IgnoreSuggestions) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    if s.candidates.is_empty() {
        let _ = writeln!(
            out,
            "No directories stood out as generated, data-heavy, or vendored."
        );
        let _ = writeln!(out);
        let _ = writeln!(
            out,
            "argot already skips test/docs/build directories (argot:recommended) and drops"
        );
        let _ = writeln!(
            out,
            "auto-generated or data-dominant files on its own, and this repo's history shows"
        );
        let _ = writeln!(
            out,
            "no large directory it stores without writing. If one should still be kept out"
        );
        let _ = writeln!(
            out,
            "of the voice, add it to argot.toml [exclude].paths by hand — see the Configure"
        );
        let _ = writeln!(out, "guide.");
        return out;
    }
    let _ = writeln!(
        out,
        "Directories you may want to add to argot.toml [exclude].paths (evidence only —"
    );
    let _ = writeln!(
        out,
        "any ordinary code in these directories would then be dropped from the voice):"
    );
    let _ = writeln!(out);
    for c in &s.candidates {
        let _ = writeln!(out, "  {}", c.path);
        let _ = writeln!(out, "    {} · {}", c.reason, c.detail);
    }
    let _ = writeln!(out);
    let _ = writeln!(
        out,
        "Add the entries you agree with to argot.toml [exclude].paths, then re-run  argot init"
    );
    out
}

/// Keep the rebuildable model directory — and the heavy `argot extract`
/// dataset — out of version control. Everything under `.argot/` is now a build
/// artifact (the human-authored config and mutes moved to the committed
/// `argot.toml` at the repo root), so the whole directory is ignored. Never
/// clobbers an existing `.gitignore`, so a user who wants to commit their model
/// can delete it and stay in control.
fn ensure_model_gitignored(argot_dir: &Path) -> std::io::Result<()> {
    fs::create_dir_all(argot_dir)?;
    let gitignore = argot_dir.join(".gitignore");
    if gitignore.exists() {
        return Ok(());
    }
    fs::write(
        &gitignore,
        "# argot writes a rebuildable voice model here — regenerate any time with `argot fit`.\n\
         # The model and the heavy `argot extract` dataset are build artifacts, not source;\n\
         # CI restores them from cache or re-fits. Delete this file to commit them yourself.\n\
         # (Your excludes and mutes live in argot.toml at the repo root — that one is committed.)\n\
         *\n",
    )
}

/// Surface the effective config on fit: write a default `argot.toml` when
/// absent (idempotent), and keep the personal-override `argot.local.toml` out
/// of version control. Best-effort; a read-only tree must not fail the fit.
/// Returns (wrote `argot.toml`, appended to `.gitignore`) so `init` can say
/// out loud which files it touched.
fn ensure_config_present(repo: &Path) -> Result<(bool, bool), String> {
    let wrote_config = argot_core::config::write_default_if_absent(repo)?;
    let appended_gitignore = ensure_local_config_gitignored(repo);
    Ok((wrote_config, appended_gitignore))
}

/// Add `argot.local.toml` to the repo-root `.gitignore` when not already
/// covered — the local overrides are personal and uncommitted by design.
/// Returns true when the entry was appended.
fn ensure_local_config_gitignored(repo: &Path) -> bool {
    use argot_core::config::LOCAL_CONFIG_FILE;
    let gitignore = repo.join(".gitignore");
    let existing = fs::read_to_string(&gitignore).unwrap_or_default();
    if existing
        .lines()
        .any(|l| l.trim() == LOCAL_CONFIG_FILE || l.trim() == format!("/{LOCAL_CONFIG_FILE}"))
    {
        return false;
    }
    let mut content = existing;
    if !content.is_empty() && !content.ends_with('\n') {
        content.push('\n');
    }
    content.push_str(&format!(
        "\n# argot personal config overrides — not shared with the team.\n{LOCAL_CONFIG_FILE}\n"
    ));
    fs::write(&gitignore, content).is_ok()
}

#[derive(Args)]
struct CheckCmd {
    /// Optional git ref or range (e.g. abc1234 or a..b). Empty = workdir.
    #[arg(default_value = "")]
    reference: String,
    /// Path to git repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
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
    #[arg(long, allow_hyphen_values = true, hide = true)]
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
    /// Only show hits at or above this confidence tier.
    #[arg(
        long = "min-confidence",
        default_value = "unusual",
        value_parser = ["unusual", "suspicious", "foreign"]
    )]
    min_confidence: String,
    /// Override a rule's severity for this run: `--rule <name>=<error|warn|off>`
    /// (repeatable; accepts rule or group names — see `argot rules`).
    #[arg(long = "rule", value_name = "NAME=SEVERITY", value_parser = parse_rule_override)]
    rule: Vec<(String, argot_core::rules::Severity)>,
    /// Exit non-zero when `warn`-severity findings are present (CI strictness).
    #[arg(long = "error-on-warnings")]
    error_on_warnings: bool,
    /// Insert an inline ignore comment above every current finding instead of
    /// reporting (adopting argot on an existing codebase). Working-tree only.
    #[arg(long = "add-ignores")]
    add_ignores: bool,
    /// Output format: human (terminal), json (stable machine-readable), sarif
    /// (SARIF 2.1.0 for code-scanning uploads), or github (Actions workflow
    /// commands → inline PR annotations). Machine formats write nothing but
    /// the document to stdout.
    #[arg(
        long,
        default_value = "human",
        value_parser = ["human", "json", "sarif", "github"]
    )]
    format: String,
    /// Suppress informational stderr notes (model line, staleness nudges,
    /// suppression counts). Errors still print.
    #[arg(short = 'q', long)]
    quiet: bool,
}

/// Resolve `--argot-dir` against `--repo`: a relative artifacts dir (the default
/// `.argot`) is anchored to the target repo, not the process CWD. Absolute paths
/// pass through unchanged. This keeps `cd repo && argot check` identical
/// (`--repo .` → `./.argot`) while making `argot check --repo other/path` load
/// `other/path/.argot` instead of silently reading a stray `./.argot` in the CWD.
fn resolve_argot_dir(repo: &Path, argot_dir: PathBuf) -> PathBuf {
    if argot_dir.is_absolute() {
        argot_dir
    } else {
        repo.join(argot_dir)
    }
}

fn run_check_cmd(c: CheckCmd) -> ExitCode {
    // Color is enabled only when NO_COLOR is unset and stdout is a tty.
    let use_color = std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    let argot_dir = resolve_argot_dir(&c.repo, c.argot_dir);
    let human = c.format == "human";
    let quiet = c.quiet;
    let today = today_utc();
    let outcome = run_check(CheckArgs {
        repo_path: c.repo.to_string_lossy().into_owned(),
        reference: c.reference,
        staged: c.staged,
        unstaged: c.unstaged,
        commit: c.commit,
        only: c.only,
        exclude: c.exclude,
        threshold: c.threshold,
        argot_dir: argot_dir.clone(),
        hunk_lines: c.hunk_lines,
        verbose: c.verbose,
        min_confidence: c.min_confidence,
        rule_overrides: c.rule,
        error_on_warnings: c.error_on_warnings,
        add_ignores: c.add_ignores,
        use_color,
        // The value_parser restricts input to the known names, so this is
        // always Some; default to Human defensively.
        format: OutputFormat::parse(&c.format).unwrap_or_default(),
        today: today.clone(),
    });
    print!("{}", outcome.stdout);
    // --quiet drops informational stderr; hard errors (exit 2) always print.
    if !quiet || outcome.exit_code >= 2 {
        eprint!("{}", outcome.stderr);
    }
    // Soft staleness nudge — human output only, never affects the exit code or
    // the machine formats. Catches the "fit once, forgot for months" case.
    if human && !quiet && outcome.exit_code < 2 {
        if let Some(days) = fit_timestamp(&argot_dir).and_then(|ts| days_since_fit(&ts, &today)) {
            if days >= STALE_FIT_DAYS {
                eprintln!("note: voice model fitted {days} days ago — `argot fit` to refresh.");
            }
        }
        freshness_hook();
    }
    // Drifted model? Refresh it in the background so the next check is sharp
    // (detached, throttled, opt-out via [fit] auto-refresh = false).
    if outcome.exit_code < 2 {
        auto_refit::maybe_refit(&c.repo, &argot_dir, &today, quiet || !human);
    }
    ExitCode::from(outcome.exit_code as u8)
}

#[derive(Args)]
struct AuditCmd {
    /// How many commits back to audit (first-parent line).
    #[arg(long, default_value_t = audit::DEFAULT_COMMITS, conflicts_with = "since")]
    commits: usize,
    /// Audit everything since a date (YYYY-MM-DD) or a duration (90d, 12w, 6m, 1y).
    #[arg(long)]
    since: Option<String>,
    /// Output format: terminal, json, markdown, or html.
    #[arg(long, default_value = "terminal")]
    format: String,
    /// Path to the repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

fn run_audit_cmd(c: AuditCmd) -> ExitCode {
    let Some(format) = audit::AuditFormat::parse(&c.format) else {
        eprintln!(
            "error: unknown --format '{}' — expected terminal, json, markdown, or html",
            c.format
        );
        return ExitCode::from(2);
    };
    let spec = match c.since {
        Some(s) => audit::window::WindowSpec::Since(s),
        None => audit::window::WindowSpec::Commits(c.commits),
    };
    audit::run_audit(&c.repo, spec, format)
}

#[derive(Args)]
struct BackgroundRefitCmd {
    /// Path to the repository to refit.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
}

#[derive(Args)]
struct RulesCmd {
    #[command(subcommand)]
    action: Option<RulesAction>,
    /// Path to the repository (its argot.toml decides the effective severities).
    #[arg(long, default_value = ".", global = true)]
    repo: PathBuf,
    /// Output format: human or json.
    #[arg(long, default_value = "human", value_parser = ["human", "json"])]
    format: String,
}

#[derive(Subcommand)]
enum RulesAction {
    /// Run the fixture tests of this repo's custom rules
    /// (`.argot/rules/<name>/tests/<case>/{input.<ext>, expected.json}`).
    Test {
        /// Test one rule by name (default: every discovered rule).
        name: Option<String>,
    },
}

/// `argot rules test [name]` — the custom-rule authoring loop.
#[cfg(feature = "script")]
fn run_rules_test_cmd(repo: &Path, name: Option<&str>) -> ExitCode {
    let mut warnings = Vec::new();
    let results = match argot_rules_script::harness::run_rule_tests(
        &repo.join(".argot"),
        name,
        &mut warnings,
    ) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    for w in &warnings {
        eprintln!("[argot] {w}");
    }
    if results.is_empty() {
        eprintln!("no custom rules discovered under .argot/rules/");
        return ExitCode::from(2);
    }
    let mut failed = 0usize;
    for r in &results {
        match &r.failure {
            None => println!("ok    {} :: {}", r.rule, r.case),
            Some(msg) => {
                failed += 1;
                println!(
                    "FAIL  {} :: {}
      {msg}",
                    r.rule, r.case
                );
            }
        }
    }
    println!(
        "
{} case(s), {} failed",
        results.len(),
        failed
    );
    // A setup problem (no tests/ dir, script won't compile) is exit 2 — distinct
    // from exit 1 for a genuine test-outcome mismatch — so CI can tell "the rule
    // is broken" from "a test failed".
    if results.iter().any(|r| r.setup) {
        return ExitCode::from(2);
    }
    if failed > 0 {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}

#[cfg(not(feature = "script"))]
fn run_rules_test_cmd(_repo: &Path, _name: Option<&str>) -> ExitCode {
    eprintln!("error: this argot was built without the scripted-rules feature");
    ExitCode::from(2)
}

fn run_rules_cmd(c: RulesCmd) -> ExitCode {
    if let Some(RulesAction::Test { name }) = &c.action {
        return run_rules_test_cmd(&c.repo, name.as_deref());
    }
    use argot_core::rules::{GROUPS, RULES};
    // Scripted community rules from `.argot/rules/` — same discovery `check`
    // runs, so the listing shows exactly the run vocabulary.
    #[cfg(feature = "script")]
    let (registry, custom_sources) = {
        use argot_core::detector::Detector as _;
        let mut warnings = Vec::new();
        let mut det = argot_rules_script::ScriptDetector::new();
        let custom = det.vocabulary(&c.repo.join(".argot"), &mut warnings);
        for w in &warnings {
            eprintln!("[argot] {w}");
        }
        let mut sources = std::collections::HashMap::new();
        for r in &custom {
            sources.insert(r.name.clone(), format!(".argot/rules/{}", r.name));
        }
        (
            argot_core::rules::Registry::with_custom(custom, &mut Vec::new()),
            sources,
        )
    };
    #[cfg(not(feature = "script"))]
    let registry = argot_core::rules::Registry::default();
    let config = argot_core::config::ArgotConfig::load_with(&c.repo, &registry);
    for w in &config.warnings {
        eprintln!("[argot] {w}");
    }
    let settings = config.rule_settings_with(&registry, &Vec::new());
    if c.format == "json" {
        #[allow(unused_mut)]
        let mut doc: Vec<serde_json::Value> = RULES
            .iter()
            .map(|r| {
                serde_json::json!({
                    "name": r.name,
                    "group": r.group,
                    "severity": settings.severity_of_rule(r).as_str(),
                    "label": r.label,
                    "description": r.description,
                })
            })
            .collect();
        #[cfg(feature = "script")]
        for r in registry.custom_rules() {
            let reason = format!("{}{}", argot_core::rules::CUSTOM_REASON_PREFIX, r.name);
            doc.push(serde_json::json!({
                "name": r.name,
                "group": argot_core::rules::GROUP_CUSTOM,
                "severity": settings.severity_of_reason(&reason).as_str(),
                "label": r.label,
                "description": r.description,
                "source": custom_sources.get(&r.name),
            }));
        }
        println!(
            "{}",
            serde_json::to_string_pretty(&doc).expect("static json")
        );
        return ExitCode::SUCCESS;
    }
    let name_w = RULES.iter().map(|r| r.name.len()).max().unwrap_or(0);
    let group_w = GROUPS.iter().map(|g| g.len()).max().unwrap_or(0);
    println!(
        "{:<name_w$}  {:<group_w$}  {:<8}  DESCRIPTION",
        "RULE", "GROUP", "SEVERITY"
    );
    for r in RULES {
        println!(
            "{:<name_w$}  {:<group_w$}  {:<8}  {}",
            r.name,
            r.group,
            settings.severity_of_rule(r).as_str(),
            r.description
        );
    }
    #[cfg(feature = "script")]
    for r in registry.custom_rules() {
        let reason = format!("{}{}", argot_core::rules::CUSTOM_REASON_PREFIX, r.name);
        println!(
            "{:<name_w$}  {:<group_w$}  {:<8}  {}  [{}]",
            r.name,
            argot_core::rules::GROUP_CUSTOM,
            settings.severity_of_reason(&reason).as_str(),
            r.description,
            custom_sources
                .get(&r.name)
                .map(String::as_str)
                .unwrap_or("scripted"),
        );
    }
    println!(
        "
Configure in argot.toml [rules] (e.g. `misplaced = \"warn\"`, `semantic = \"off\"`)\nor per run with `argot check --rule <name>=<severity>`."
    );
    ExitCode::SUCCESS
}

#[derive(Args)]
struct ReviewCmd {
    /// A PR URL, `#number` / `number`, a `base..head` range, or a commit sha.
    target: String,
    /// Path to the local repository (whose fitted voice scores the PR).
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Output format: human, json, sarif, or github (PR annotations).
    #[arg(long, default_value = "human", value_parser = ["human", "json", "sarif", "github"])]
    format: String,
}

#[derive(Args)]
struct VoiceDiffCmd {
    /// A ref or `base..head` range to summarize (e.g. `main..HEAD`).
    target: String,
    /// Path to the repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Output format: human, json, markdown (a PR-comment / job-summary score
    /// card), shields (shields.io endpoint JSON for a README badge), or svg (a
    /// self-contained badge).
    #[arg(long, default_value = "human", value_parser = ["human", "json", "markdown", "shields", "svg"])]
    format: String,
    /// Hot-spots to list.
    #[arg(long = "top", value_name = "N", default_value_t = voice_diff::DEFAULT_TOP)]
    top: usize,
}

#[derive(Args)]
struct InspectCmd {
    /// Path to the repository to inspect.
    #[arg(default_value = ".")]
    path: PathBuf,
    /// Inspect the fitted model artifact (hashes, provenance, and the typical
    /// callees per cluster) instead of repo suitability.
    #[arg(long)]
    model: bool,
    /// Typical callees shown per cluster under `--model`.
    #[arg(long = "top", value_name = "N", default_value_t = 8)]
    top: usize,
    /// List the files that would shape the voice, one per line, and exit. The
    /// same walk `fit` uses — a corpus preview that needs no fit.
    #[arg(long)]
    corpus: bool,
    /// Output format: human (terminal) or json (stable machine-readable).
    #[arg(long, default_value = "human", value_parser = ["human", "json"])]
    format: String,
}

/// `argot inspect --corpus` — the file list a fit would learn from, and the
/// one it would judge without learning from.
///
/// Before this, the only trustworthy answer to "what will shape the voice?"
/// was `.argot/repo-corpus.txt`, which exists only *after* a fit — so the
/// decision the setup flow asks you to make first (what to exclude) could only
/// be checked afterwards.
fn run_inspect_corpus(c: &InspectCmd) -> ExitCode {
    let config = argot_core::config::ArgotConfig::load(&c.path);
    let walk = argot_core::corpus::walk_corpus(&c.path, &config.path_suppressions());
    let rel = |p: &std::path::Path| argot_core::corpus::rel_to_repo(p, &c.path);
    if c.format == "json" {
        let doc = serde_json::json!({
            "voice": walk.voice.iter().map(|p| rel(p)).collect::<Vec<_>>(),
            "check_only": walk.check_only.iter().map(|p| rel(p)).collect::<Vec<_>>(),
        });
        println!("{}", serde_json::to_string_pretty(&doc).unwrap());
        return ExitCode::SUCCESS;
    }
    for p in &walk.voice {
        println!("{}", rel(p));
    }
    eprintln!(
        "[argot] {} file(s) shape the voice · {} checked but not learned from ([exclude].check-only)",
        walk.voice.len(),
        walk.check_only.len()
    );
    ExitCode::SUCCESS
}

fn run_inspect_cmd(c: InspectCmd) -> ExitCode {
    if c.model {
        return run_inspect_model(&c);
    }
    if c.corpus {
        return run_inspect_corpus(&c);
    }
    let report = match inspect_repo(&c.path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    if wants_json(&c.format) {
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
    print!("{}", render_inspect_human(&report, use_color, &today_utc()));
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

fn render_inspect_human(report: &InspectReport, use_color: bool, today: &str) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "Inspecting {}", report.path);
    let _ = writeln!(out);

    // Corpus composition.
    let c = &report.corpus;
    let _ = writeln!(out, "Corpus");
    let _ = writeln!(
        out,
        "  {} files in the repo · {} supported · {} unsupported extension",
        c.total_files, c.supported_files, c.unsupported_files
    );
    // Name what git puts outside the repo, so "42% javascript" can never again
    // be a reader's first news that their dependencies were counted as corpus.
    if !c.gitignored_dirs.is_empty() || c.gitignored_files > 0 {
        let shown = 4.min(c.gitignored_dirs.len());
        let mut skipped = c.gitignored_dirs[..shown].join(", ");
        if c.gitignored_dirs.len() > shown {
            let _ = write!(skipped, ", +{} more", c.gitignored_dirs.len() - shown);
        }
        if c.gitignored_files > 0 {
            if !skipped.is_empty() {
                skipped.push_str(" · ");
            }
            let _ = write!(skipped, "{} loose file(s)", c.gitignored_files);
        }
        let _ = writeln!(out, "  gitignored, not part of the repo: {skipped}");
    }
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
            // Soft freshness nudge — a stale model is lower-confidence, not
            // wrong. Advisory only; it never changes the verdict.
            if let Some(days) = cal
                .languages
                .values()
                .next()
                .and_then(|lc| days_since_fit(&lc.timestamp_utc, today))
            {
                if days >= STALE_FIT_DAYS {
                    let _ = writeln!(
                        out,
                        "  {} fitted {days} days ago — `argot fit` to refresh",
                        paint("stale:", ANSI_YELLOW, use_color)
                    );
                }
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
        Verdict::ReadyWithNotes => paint("Ready — with notes", ANSI_YELLOW, use_color),
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

fn run_inspect_model(c: &InspectCmd) -> ExitCode {
    let report = match inspect_model(&c.path, c.top) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    if wants_json(&c.format) {
        match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(e) => {
                eprintln!("error: {e}");
                return ExitCode::from(2);
            }
        }
        return ExitCode::SUCCESS;
    }
    let use_color = std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
    print!("{}", render_model_human(&report, use_color));
    ExitCode::SUCCESS
}

fn render_model_human(report: &ModelReport, use_color: bool) -> String {
    use std::fmt::Write as _;
    let mut out = String::new();
    let _ = writeln!(out, "Model artifact for {}", report.path);
    let _ = writeln!(out);

    match &report.manifest {
        Some(m) => {
            let _ = writeln!(
                out,
                "  {} {}",
                paint("model:", ANSI_BOLD, use_color),
                m.model_hash
            );
            let _ = writeln!(out, "  scorer-config: {}", m.scorer_config_hash);
            let _ = writeln!(
                out,
                "  format: manifest v{} · config v{}",
                m.manifest_version, m.config_version
            );
            let _ = writeln!(
                out,
                "  fitted at {} · repo sha {}",
                m.fit_timestamp, m.fit_commit_sha
            );
            let _ = writeln!(
                out,
                "  corpus: {} files · {} lines",
                m.corpus_files, m.corpus_lines
            );
        }
        None => {
            let _ = writeln!(
                out,
                "  (no manifest.json — re-run `argot fit` to write one)"
            );
        }
    }
    let _ = writeln!(out);

    for (lang, lm) in &report.languages {
        let _ = writeln!(
            out,
            "{} — threshold {:.4} · model {} · n_cal {}",
            paint(lang, ANSI_BOLD, use_color),
            lm.threshold,
            lm.model_hash,
            lm.n_cal
        );
        let _ = writeln!(
            out,
            "  {} distinct BPE tokens ({} total) · {} attested callees over {} corpus files · {} clusters",
            lm.distinct_tokens,
            lm.total_tokens,
            lm.attested_callees,
            lm.corpus_files,
            lm.clusters.len()
        );
        for cl in &lm.clusters {
            let callees: Vec<String> = cl
                .top_callees
                .iter()
                .map(|(name, n)| format!("{name} ({n})"))
                .collect();
            let _ = writeln!(
                out,
                "    cluster {} ({} files): {}",
                cl.id,
                cl.files,
                if callees.is_empty() {
                    "—".to_string()
                } else {
                    callees.join(", ")
                }
            );
        }
        for s in &lm.supersessions {
            let _ = writeln!(
                out,
                "    supersession: {} → {} ({} commits, {}..{}, e.g. {}) · {} leftover file(s)",
                s.old, s.new, s.commits, s.first, s.last, s.example_commit, s.leftover_count
            );
        }
        let _ = writeln!(out);
    }
    out
}

// --- suppression commands (mute / list-mutes / review-mutes) ---

#[derive(Args)]
struct MuteCmd {
    /// Hit hash from `argot check` output (the `[abc123def456]` on a hit line).
    /// Mutes exactly that hit — a per-hit acceptance, not a standing rule: the
    /// same finding in a sibling file gets its own hash. Omit it and pass
    /// `--path` instead for a durable, pattern-scoped mute.
    hash: Option<String>,
    /// Mute by path glob instead of by hash (`src/legacy/**`) — a standing
    /// `[[mute]]` that covers every future hit under it, not one hash.
    #[arg(long, value_name = "GLOB", conflicts_with = "hash")]
    path: Option<String>,
    /// Narrow a `--path` mute to one rule or group (e.g. `foreign-import`).
    /// Without it the mute covers every rule on those paths.
    #[arg(long, value_name = "RULE", requires = "path")]
    rule: Option<String>,
    /// Why this hit is muted (recorded in argot.toml [[mute]]).
    #[arg(long)]
    reason: Option<String>,
    /// Auto-expire the mute after N days (e.g. `30d`).
    #[arg(long, value_name = "DAYS", value_parser = parse_expires_days)]
    expires: Option<u64>,
}

/// Parse a `--rule <name>=<severity>` override against the registry — a CLI
/// typo is a hard usage error (config typos, by contrast, warn and degrade).
fn parse_rule_override(s: &str) -> Result<(String, argot_core::rules::Severity), String> {
    let (name, sev) = s
        .split_once('=')
        .ok_or_else(|| format!("expected <name>=<severity>, got '{s}'"))?;
    // Shape-check only: a kebab-case name may be a repo's custom rule, which
    // is only known once `.argot/rules/` is discovered — run_check validates
    // every selector against the run vocabulary and exits 2 on an unknown.
    let kebab = !name.is_empty()
        && name
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '-' || c.is_ascii_digit());
    if !argot_core::rules::known_selector(name) && !kebab {
        return Err(format!(
            "unknown rule '{name}' (known: {})",
            argot_core::rules::selector_names().join(", ")
        ));
    }
    let severity = argot_core::rules::Severity::parse(sev)
        .ok_or_else(|| format!("invalid severity '{sev}' (expected error, warn, or off)"))?;
    Ok((name.to_string(), severity))
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
    let repo_root = PathBuf::from(&ctx.git_root);
    let expires = c.expires.map(date_days_from_now);
    let (_, mute_registry) = argot_core::compose::load_config(&repo_root);

    // The path form is what a standing decision actually needs, and the CLI
    // used to be unable to write it: `argot mute <hash>` covers one hit, so
    // accepting the same dependency across a directory meant one committed
    // entry per file, forever — or hand-writing TOML the CLI never mentioned.
    let written = match (&c.hash, &c.path) {
        (_, Some(path)) => argot_core::suppress::mute_path(
            &repo_root,
            &mute_registry,
            path,
            c.rule.as_deref(),
            c.reason.as_deref(),
            expires.clone(),
        ),
        (Some(hash), None) => argot_core::suppress::mute_hash(
            &repo_root,
            &ctx.argot_dir,
            &mute_registry,
            hash,
            c.reason.as_deref(),
            expires.clone(),
            &today_utc(),
        ),
        (None, None) => Err(
            "expected a hit hash (from `argot check`) or --path <glob> for a durable mute"
                .to_string(),
        ),
    };

    match written {
        Ok(rule) => {
            let scope = match &rule.hash {
                Some(h) => format!("[{h}] in {}", rule.path),
                None => match &rule.rule {
                    Some(r) => format!("{} (rule {r})", rule.path),
                    None => rule.path.clone(),
                },
            };
            println!(
                "Muted {scope} — {}{}",
                rule.reason,
                expires
                    .map(|e| format!(" (expires {e})"))
                    .unwrap_or_default()
            );
            if rule.hash.is_some() {
                println!(
                    "  note: a hash mute covers this hit only — the same finding in another file \
                     gets its own hash. `argot mute --path <glob>` covers a whole tree."
                );
            }
            println!("  → commit argot.toml to share this decision with your team and CI.");
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
    use argot_core::suppress::parse_inline;

    let ctx = resolve_context();
    let repo_root = PathBuf::from(&ctx.git_root);
    let today = today_utc();

    let (config, registry) = argot_core::compose::load_config(&repo_root);
    for w in &config.warnings {
        eprintln!("[argot] {w}");
    }

    // 1. Path-level excludes: argot.toml [exclude] (recommended set + paths).
    println!("argot.toml  [exclude]");
    if config.exclude.recommended.is_empty() {
        println!("  recommended set: disabled (empty list)");
    } else {
        println!(
            "  recommended set: {} pattern(s) — {}",
            config.exclude.recommended.len(),
            config.exclude.recommended.join(" ")
        );
    }
    if config.exclude.paths.is_empty() {
        println!("  paths: (none)");
    } else {
        for p in &config.exclude.paths {
            println!("  path: {p}");
        }
    }
    if !config.from_file {
        println!("  (no argot.toml at repo root — using built-in defaults)");
    }

    // 2. Durable [[mute]] entries, with expiry status.
    let rules = config.mutes_with(&registry, &today);
    println!("\nargot.toml  [[mute]]");
    if rules.active.is_empty() && rules.expired.is_empty() {
        println!("  (no entries)");
    }
    for (label, list) in [("active", &rules.active), ("expired", &rules.expired)] {
        for r in list {
            let mut parts = vec![format!("path={}", r.path)];
            if let Some(s) = &r.rule {
                parts.push(format!("rule={s}"));
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
            Language::Javascript => Box::new(JavaScriptAdapter::new()),
            Language::Go => Box::new(GoAdapter::new()),
            Language::Rust => Box::new(RustAdapter::new()),
            Language::C => Box::new(CAdapter::new()),
            Language::Java => Box::new(JavaAdapter::new()),
            Language::CSharp => Box::new(CSharpAdapter::new()),
            Language::Php => Box::new(PhpAdapter::new()),
            Language::Cpp => Box::new(CppAdapter::new()),
            Language::Ruby => Box::new(RubyAdapter::new()),
            Language::Pascal => Box::new(PascalAdapter::new()),
        };
        // Cheap pre-filter before the full parse.
        if !source.contains("argot:") {
            continue;
        }
        let inline = parse_inline(
            &source,
            adapter.line_comment_prefix(),
            argot_core::rules::Registry::builtin(),
        );
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
    /// Remove the dead mutes — those whose file is gone (rewrites argot.toml [[mute]]).
    #[arg(long)]
    prune: bool,
}

fn run_review_mutes_cmd(c: ReviewMutesCmd) -> ExitCode {
    let ctx = resolve_context();
    let (_, registry) = argot_core::compose::load_config(std::path::Path::new(&ctx.git_root));
    let outcome =
        argot_core::check::run_review_mutes(&ctx.git_root, &registry, &today_utc(), c.prune);
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
    /// Cluster-rare threshold; the bench auto-selects 0 or 2.
    #[arg(long = "cluster-rare-threshold", default_value_t = 0)]
    cluster_rare_threshold: usize,
    /// Repo root for resolving the repo's own module names (package.json name /
    /// workspace packages). Used so self-package imports (`import … from
    /// 'ink'`) are attested, not foreign.
    #[arg(long = "repo-root")]
    repo_root: Option<PathBuf>,
}

fn run_score_cmd(c: ScoreCmd) -> ExitCode {
    use std::io::BufRead;
    let language = match c.language.as_str() {
        "python" => Language::Python,
        "typescript" => Language::Typescript,
        "go" => Language::Go,
        "rust" => Language::Rust,
        "c" => Language::C,
        "java" => Language::Java,
        "csharp" => Language::CSharp,
        "php" => Language::Php,
        "cpp" => Language::Cpp,
        "ruby" => Language::Ruby,
        "pascal" => Language::Pascal,
        other => {
            eprintln!(
                "error: --language must be python|typescript|go|rust|c|java|csharp|php|cpp|ruby|pascal (got '{other}')"
            );
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
        Language::Javascript => Box::new(JavaScriptAdapter::new()),
        Language::Go => Box::new(GoAdapter::new()),
        Language::Rust => Box::new(RustAdapter::new()),
        Language::C => Box::new(CAdapter::new()),
        Language::Java => Box::new(JavaAdapter::new()),
        Language::CSharp => Box::new(CSharpAdapter::new()),
        Language::Php => Box::new(PhpAdapter::new()),
        Language::Cpp => Box::new(CppAdapter::new()),
        Language::Ruby => Box::new(RubyAdapter::new()),
        Language::Pascal => Box::new(PascalAdapter::new()),
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
    // Repo-module resolution: the repo's own package name and workspace
    // packages attest self-imports so e.g. `import … from 'ink'` in an ink
    // checkout is internal, not foreign.
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
        conventions: None,
        convention_bonus: 0.0,
        import_modules: mods.into_iter().collect(),
        import_module_prefixes,
        // Plumbing scores single hunks with no repo scope of their own.
        check_only_import_modules: Vec::new(),
        check_only_patterns: Vec::new(),
        // Bench feature extraction reads `stages.bpe_score`; no evidence needed.
        evidence_corpus: None,
        // Bench corpora don't customize [detect]; the built-in defaults match
        // how production fits (data-threshold 0.65 + the standard markers).
        detect: c
            .repo_root
            .as_ref()
            .map(|r| argot_core::config::ArgotConfig::load(r).detect)
            .unwrap_or_default(),
    };
    let scorer = match SequentialImportBpeScorer::from_config(&repo_files, &generic, adapter, cfg) {
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
    /// Optional git ref or range (e.g. abc1234 or a..b). Defaults to full history.
    #[arg(default_value = "")]
    reference: String,
    /// Path to git repository.
    #[arg(long, default_value = ".")]
    repo: PathBuf,
    /// Output JSONL path.
    #[arg(long, default_value = ".argot/dataset.jsonl")]
    out: PathBuf,
    /// Max number of records to emit.
    #[arg(long)]
    limit: Option<usize>,
}

fn run_extract(a: ExtractArgs) -> ExitCode {
    let repo_path = a.repo.to_string_lossy().into_owned();
    // No git repository at `path` → exit 2.
    if !repo_exists(&repo_path) {
        eprintln!("error: repository not found at '{}'", repo_path);
        return ExitCode::from(2);
    }

    if let Some(parent) = a.out.parent() {
        let _ = fs::create_dir_all(parent);
        // The dataset is heavy and rebuildable — keep the default `.argot/`
        // model dir out of version control (leave custom --out paths alone).
        if parent.file_name().is_some_and(|n| n == ".argot") {
            let _ = ensure_model_gitignored(parent);
        }
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

    let stats = match write_dataset(&repo_path, reference, a.limit, &mut writer) {
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
        Some(Command::Init(c)) => run_init_cmd(c),
        Some(Command::Extract(a)) => run_extract(a),
        Some(Command::Train(c)) => run_train_cmd(c),
        Some(Command::Calibrate(c)) => run_calibrate_cmd(c),
        Some(Command::Fit(c)) => run_fit_cmd(c),
        Some(Command::Check(c)) => run_check_cmd(c),
        Some(Command::Rules(c)) => run_rules_cmd(c),
        Some(Command::Review(c)) => {
            let use_color =
                std::env::var_os("NO_COLOR").is_none() && std::io::stdout().is_terminal();
            review::run_review(&c.target, c.repo, &c.format, use_color)
        }
        Some(Command::VoiceDiff(c)) => {
            voice_diff::run_voice_diff(&c.target, c.repo, &c.format, c.top)
        }
        Some(Command::Inspect(c)) => run_inspect_cmd(c),
        Some(Command::Mute(c)) => run_mute_cmd(c),
        Some(Command::ListMutes) => run_list_mutes(),
        Some(Command::ReviewMutes(c)) => run_review_mutes_cmd(c),
        Some(Command::Score(c)) => run_score_cmd(c),
        Some(Command::Status(c)) => run_status(c),
        Some(Command::List(c)) => run_list(c),
        Some(Command::Update) => run_update(),
        Some(Command::Cache(c)) => cache_cmd::run(c.action),
        Some(Command::Uninstall(c)) => uninstall::run_uninstall(c.dry_run, c.yes),
        #[cfg(feature = "self-update")]
        Some(Command::RefreshVersionCache) => {
            update_check::run_refresh();
            ExitCode::SUCCESS
        }
        Some(Command::Audit(c)) => run_audit_cmd(c),
        Some(Command::BackgroundRefit(c)) => {
            auto_refit::run_background_refit(&c.repo);
            ExitCode::SUCCESS
        }
        Some(Command::Mcp(c)) => mcp::run_mcp(c.repo),
        Some(Command::DescribeVoice(c)) => describe::run_describe_voice(c.repo, c.top, c.out),
        Some(Command::Conventions(c)) => run_conventions_cmd(c),
        Some(Command::Hook(c)) => hook::run_hook(c.repo),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        days_since_fit, ensure_local_config_gitignored, ensure_model_gitignored, fit_repo,
        init_recurring_actions, is_npm_install, resolve_argot_dir, root_help, wants_json, Cli,
        Verdict,
    };
    use clap::CommandFactory;

    /// `argot fit` must be side-effect-free on the user's tracked tree — it may
    /// write only inside the gitignored `.argot/`. Scaffolding `argot.toml` or
    /// editing the root `.gitignore` during fit breaks the CI Action's
    /// fit-on-base → restore-head checkout (get-tmonier/vigie#39): the first PR
    /// to introduce argot.toml aborts the restore and fails a non-blocking check.
    #[test]
    fn fit_leaves_the_tracked_tree_clean() {
        let dir = std::env::temp_dir().join(format!("argot_fit_clean_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let repo = git2::Repository::init(&dir).unwrap();
        for (name, body) in [
            ("a.py", "def alpha(x):\n    return x + 1\n"),
            ("b.py", "def beta(x):\n    return x * 2\n"),
            (
                "c.py",
                "import os\n\ndef gamma():\n    return os.getpid()\n",
            ),
        ] {
            std::fs::write(dir.join(name), body).unwrap();
        }
        let mut index = repo.index().unwrap();
        index
            .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
            .unwrap();
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@t").unwrap();
        repo.commit(Some("HEAD"), &sig, &sig, "init", &tree, &[])
            .unwrap();

        // Best-effort fit (the result doesn't matter — the invariant is that it
        // scaffolds nothing on the tracked tree, whether it succeeds or not).
        let _ = fit_repo(&dir, &[]);

        assert!(
            !dir.join("argot.toml").exists(),
            "fit must not scaffold argot.toml"
        );
        assert!(
            !dir.join(".gitignore").exists(),
            "fit must not create/modify the root .gitignore"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn model_gitignore_hides_the_whole_model_dir() {
        let dir = std::env::temp_dir().join(format!("argot_gitignore_test_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let argot_dir = dir.join(".argot");
        ensure_model_gitignored(&argot_dir).expect("write .gitignore");
        let body = std::fs::read_to_string(argot_dir.join(".gitignore")).unwrap();
        // The whole `.argot/` dir is rebuildable now — config and mutes moved to
        // the committed argot.toml at the repo root, so nothing is re-included.
        assert!(body.contains("*\n"), "blanket * ignore present");
        assert!(!body.contains('!'), "nothing re-included from .argot/");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn local_config_is_added_to_root_gitignore() {
        let dir = std::env::temp_dir().join(format!("argot_local_gi_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(".gitignore"), "target/\n").unwrap();
        assert!(ensure_local_config_gitignored(&dir), "first call appends");
        let body = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
        assert!(body.contains("target/"), "existing entries preserved");
        assert!(body.contains("argot.local.toml"), "local config ignored");
        // Idempotent — a second call adds nothing.
        assert!(
            !ensure_local_config_gitignored(&dir),
            "second call is a no-op"
        );
        let body2 = std::fs::read_to_string(dir.join(".gitignore")).unwrap();
        assert_eq!(body, body2, "second call is a no-op");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn days_since_fit_counts_calendar_days() {
        // Same day, ISO timestamp with a time part.
        assert_eq!(
            days_since_fit("2026-07-06T07:39:27+00:00", "2026-07-06"),
            Some(0)
        );
        assert_eq!(days_since_fit("2026-01-01", "2026-01-31"), Some(30));
        // A year (2024 is a leap year → 366 from mid-2024, but 2025→2026 is 365).
        assert_eq!(days_since_fit("2025-07-06", "2026-07-06"), Some(365));
        // Unparseable input never panics.
        assert_eq!(days_since_fit("not-a-date", "2026-07-06"), None);
    }
    use std::path::{Path, PathBuf};

    #[test]
    fn argot_dir_resolves_relative_to_repo_not_cwd() {
        // Default `.argot` under an explicit repo anchors to that repo, so
        // `argot check --repo other/path` reads `other/path/.argot`, not a
        // stray `./.argot` in the process CWD.
        assert_eq!(
            resolve_argot_dir(Path::new("other/path"), PathBuf::from(".argot")),
            PathBuf::from("other/path/.argot")
        );
        // The common `--repo .` case is byte-identical to the old behaviour.
        assert_eq!(
            resolve_argot_dir(Path::new("."), PathBuf::from(".argot")),
            PathBuf::from("./.argot")
        );
        // An absolute --argot-dir is honoured verbatim (escape hatch).
        assert_eq!(
            resolve_argot_dir(Path::new("/repo"), PathBuf::from("/tmp/custom")),
            PathBuf::from("/tmp/custom")
        );
    }

    #[test]
    fn wants_json_honors_format() {
        assert!(wants_json("json"));
        assert!(!wants_json("human"));
    }

    #[test]
    fn npm_install_detected_by_node_modules_component() {
        let npm = Path::new(
            "/Users/x/.nvm/versions/node/v26.1.0/lib/node_modules/@tmonier/argot/bin/argot",
        );
        assert!(is_npm_install(Some(npm)));
    }

    #[test]
    fn cargo_home_install_is_not_npm() {
        assert!(!is_npm_install(Some(Path::new(
            "/Users/x/.cargo/bin/argot"
        ))));
        assert!(!is_npm_install(None));
    }

    #[test]
    fn init_recurring_actions_preserve_each_verdict_boundary() {
        for (verdict, opening) in [
            (Verdict::Ready, "Then choose"),
            (Verdict::ReadyWithNotes, "After reviewing the notes"),
            (Verdict::NotRecommended, "After the corpus is ready"),
        ] {
            let actions = init_recurring_actions(verdict).join("\n");
            assert!(actions.starts_with(opening), "{actions}");
            assert!(actions.contains("pre-commit"));
            assert!(actions.contains("runs at commit time after user configuration"));
            assert!(actions.contains("GitHub Action"));
            assert!(actions.contains("runs in a user-configured CI workflow"));
            assert!(actions.contains("https://argot.tmonier.com/docs/ci/"));
            assert!(!actions.contains("automatic lifecycle"));
        }
    }

    #[test]
    fn root_help_lists_the_audit_first_public_command_registry() {
        let command = Cli::command();
        let command_ids: Vec<_> = command
            .get_subcommands()
            .filter(|command| !command.is_hide_set())
            .map(|command| command.get_name())
            .collect();

        assert_eq!(command_ids[0], "audit");
        for command in [
            "audit",
            "init",
            "fit",
            "check",
            "rules",
            "review",
            "voice-diff",
            "inspect",
            "mute",
            "list-mutes",
            "review-mutes",
            "status",
            "list",
            "update",
            "cache",
            "uninstall",
            "mcp",
            "describe-voice",
            "conventions",
        ] {
            assert!(command_ids.contains(&command), "missing {command}");
        }

        let root = root_help();
        assert!(root.starts_with("Surface repository-grounded divergence"));
        assert!(root.contains("  audit           Audit recent history"));
        assert!(root.contains("  conventions     List the conventions"));
    }
}
