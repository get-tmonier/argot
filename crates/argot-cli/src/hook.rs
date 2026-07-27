//! `argot hook` — the pre-write guardrail for coding agents.
//!
//! Reads a Claude Code `PreToolUse` event on stdin, scores the code the agent
//! is about to write against the repo's fitted voice, and — only for a
//! genuinely foreign dependency (the highest-precision signal argot has) —
//! returns an `ask` decision so the human confirms before it lands. argot never
//! auto-blocks: the reviewer keeps the last word, applied a step earlier.
//!
//! Wired only when you opt in at setup (a `PreToolUse` entry in the repo's
//! `.claude/settings.json`). Any problem (no model, unsupported file, an
//! unparseable event) degrades to a silent allow — the hook never breaks or
//! stalls the agent.

use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{json, Value};

use argot_core::check::RepoScorers;
use argot_core::config::{ArgotConfig, MigrationKind};
use argot_core::rules::Severity;
use argot_core::scoring::evidence::format_evidence;
use argot_core::suppress::PathScope;

/// Tools whose input carries code we can score before it's written.
fn is_write_tool(name: &str) -> bool {
    matches!(name, "Write" | "Edit" | "MultiEdit")
}

/// The proposed new content from a Write/Edit/MultiEdit tool input.
fn proposed_content(tool_input: &Value) -> String {
    // Write: the whole file.
    if let Some(c) = tool_input.get("content").and_then(Value::as_str) {
        return c.to_string();
    }
    // MultiEdit: `edits: [{ old_string, new_string }]` (Claude Code's shape).
    if let Some(edits) = tool_input.get("edits").and_then(Value::as_array) {
        return edits
            .iter()
            .filter_map(|e| e.get("new_string").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n");
    }
    // Older single-edit shape.
    if let Some(ns) = tool_input.get("new_string").and_then(Value::as_str) {
        return ns.to_string();
    }
    String::new()
}

/// Run the pre-write hook. Always exits 0 (allow); the only non-silent outcome
/// is an `ask` decision printed as JSON on stdout.
pub fn run_hook(repo: PathBuf) -> ExitCode {
    let mut input = String::new();
    if std::io::stdin().read_to_string(&mut input).is_err() {
        return ExitCode::SUCCESS;
    }
    let Ok(event) = serde_json::from_str::<Value>(&input) else {
        return ExitCode::SUCCESS;
    };
    let tool = event.get("tool_name").and_then(Value::as_str).unwrap_or("");
    if !is_write_tool(tool) {
        return ExitCode::SUCCESS;
    }
    let tool_input = event.get("tool_input").cloned().unwrap_or(Value::Null);
    let Some(file_path) = tool_input.get("file_path").and_then(Value::as_str) else {
        return ExitCode::SUCCESS;
    };
    let content = proposed_content(&tool_input);
    if content.trim().is_empty() {
        return ExitCode::SUCCESS;
    }

    if let Some(reason) = assess(&repo, file_path, &content) {
        let out = json!({
            "hookSpecificOutput": {
                "hookEventName": "PreToolUse",
                "permissionDecision": "ask",
                "permissionDecisionReason": reason,
            }
        });
        println!("{out}");
    }
    ExitCode::SUCCESS
}

/// Score the proposed content; return an `ask` reason iff it introduces a
/// dependency foreign to the repo (the `foreign-import` signal — 98% catch /
/// 0.29% false-alarm). Everything else stays silent so the hook never nags.
fn assess(repo: &Path, file_path: &str, content: &str) -> Option<String> {
    // The run's full vocabulary, not the built-ins: a repo's own rule names
    // must mean the same thing here as they do in `check`, or `argot.toml`
    // reads differently depending on which command opened it.
    let (config, registry) = argot_core::compose::load_config(repo);
    let relative_path = repo_relative_path(repo, file_path)?;
    if let Err(decline) = can_assess(&config, &registry, &relative_path) {
        if let Some(msg) = decline.message() {
            eprintln!("{msg}");
        }
        return None;
    }

    let mut scorers = RepoScorers::load(
        &repo.join(".argot"),
        &config.detect,
        &config.exclude.check_only,
    )
    .ok()?;
    scorers.language_for(file_path)?;
    let scored = scorers.score(file_path, content, Some(content))?;
    if !scored.flagged
        || argot_core::rules::code_for_reason(scored.reason.as_str()) != "foreign-import"
        || only_declared_replacements(&config, &scored.foreign_import_modules)
    {
        return None;
    }
    let evidence = scored
        .evidence
        .as_ref()
        .map(|ev| {
            format_evidence(ev, false, 1)
                .into_iter()
                .map(|l| l.trim().to_string())
                .collect::<Vec<_>>()
                .join(" · ")
        })
        .filter(|s| !s.is_empty());
    let file = Path::new(file_path)
        .file_name()
        .and_then(|f| f.to_str())
        .unwrap_or(file_path);
    Some(match evidence {
        Some(ev) => format!(
            "argot: `{file}` reaches for a dependency new to this repo — {ev}. \
             Intentional? The repo has its own idioms for this."
        ),
        None => {
            format!(
                "argot: `{file}` reaches for a dependency this repo has never used. Intentional?"
            )
        }
    })
}

/// Why the hook declined to assess a write, when the reason is the repo's
/// configuration rather than the code. Reported on stderr — never the
/// permission channel, so it interrupts nothing — because a guardrail that is
/// off is otherwise indistinguishable from one that had nothing to say.
enum Decline {
    /// The config file could not be parsed at all; every value fell back to a
    /// default the repo may not want. Fail open rather than ask on a guess.
    ConfigUnreadable,
    /// `foreign-import` is off repo-wide — the hook can never fire.
    RuleDisabled,
    /// This particular path is out of scope (`[exclude]`, or a `[rules]` path
    /// scope). Deliberate, per-path, and visible in the config the author
    /// wrote, so it is not worth a line per write.
    PathOutOfScope,
}

impl Decline {
    /// Announce only the states in which the guardrail is dead for *every*
    /// write. Those are the ones indistinguishable from silence; a path the
    /// author explicitly excluded is not a surprise worth narrating.
    fn message(&self) -> Option<&'static str> {
        match self {
            Decline::ConfigUnreadable => Some(
                "argot hook: argot.toml could not be parsed — pre-write check skipped for every \
                 file (run `argot check` to see the error)",
            ),
            Decline::RuleDisabled => Some(
                "argot hook: foreign-import is off in this repo — pre-write check does nothing",
            ),
            Decline::PathOutOfScope => None,
        }
    }
}

/// Apply the configuration subset that can be decided before a write: the
/// hook has no final diff hunk or stable hit hash, so inline and hash mutes are
/// deliberately unsupported here (treating a proposed partial edit as either
/// would claim parity the hook cannot provide).
///
/// `Ok(())` when the write can be assessed, `Err(reason)` when configuration
/// rules it out. Only a config that failed to *parse* fails open: a per-entry
/// warning ("unknown rule 'x' — ignored", a malformed `[[mute]]`) leaves the
/// import decision perfectly well-defined, and taking the guardrail down for
/// one unrelated typo is a far larger blast radius than the diagnostic
/// deserves.
fn can_assess(
    config: &ArgotConfig,
    registry: &argot_core::rules::Registry,
    relative_path: &str,
) -> Result<(), Decline> {
    if config.degraded {
        return Err(Decline::ConfigUnreadable);
    }
    let settings = config.rule_settings_with(registry, &Vec::new());
    if settings.severity_of_reason("import") == Severity::Off {
        return Err(Decline::RuleDisabled);
    }
    if !settings.covers_path("import", relative_path)
        || config.path_suppressions().classify(relative_path) != PathScope::InScope
    {
        return Err(Decline::PathOutOfScope);
    }
    Ok(())
}

/// Full check attests declared replacement imports before scoring. At pre-write
/// time, retain a prompt when any other foreign import remains in the proposed
/// content; suppress only when every foreign module is a declared replacement.
fn only_declared_replacements(config: &ArgotConfig, foreign_modules: &[String]) -> bool {
    !foreign_modules.is_empty()
        && foreign_modules.iter().all(|module| {
            config
                .migrations()
                .active
                .iter()
                .any(|migration| migration.kind == MigrationKind::Import && module == &migration.to)
        })
}

/// Canonicalize as much of `path` as exists, re-appending the rest verbatim.
///
/// Plain `canonicalize` is not enough here: the pre-write hook is called for
/// files that do not exist yet — creating one is the whole point — and on
/// macOS the repo root canonicalizes through `/var` → `/private/var` while the
/// unresolvable payload path does not, so the two would never share a prefix.
fn resolve_existing(path: &Path) -> PathBuf {
    if let Ok(resolved) = std::fs::canonicalize(path) {
        return resolved;
    }
    let mut tail: Vec<&std::ffi::OsStr> = Vec::new();
    let mut cursor = path;
    while let Some(parent) = cursor.parent() {
        let Some(name) = cursor.file_name() else {
            break;
        };
        tail.push(name);
        if let Ok(resolved) = std::fs::canonicalize(parent) {
            let mut out = resolved;
            out.extend(tail.iter().rev());
            return out;
        }
        cursor = parent;
    }
    path.to_path_buf()
}

/// Normalize a Claude file path to the repository-relative, slash-separated
/// form used by Argot's path scopes and exclusions. Files outside the repo (or
/// relative paths escaping it) are not candidates for a pre-write ask.
fn repo_relative_path(repo: &Path, file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    let candidate = if path.is_absolute() {
        // Both sides resolved: `--repo` defaults to `.`, and every Claude Code
        // payload carries an absolute `file_path`, so a raw `strip_prefix(".")`
        // would fail on the CLI's own default and the hook would go silently
        // dead. Keep the raw values as a last resort so a repo path that can't
        // be resolved still strips a literal prefix.
        let repo_abs = resolve_existing(repo);
        let path_abs = resolve_existing(path);
        path_abs
            .strip_prefix(&repo_abs)
            .or_else(|_| path.strip_prefix(repo))
            .ok()?
            .to_path_buf()
    } else {
        if path
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return None;
        }
        path.to_path_buf()
    };
    let parts = candidate
        .components()
        .filter_map(|part| match part {
            std::path::Component::Normal(name) => name.to_str(),
            _ => None,
        })
        .collect::<Vec<_>>();
    (!parts.is_empty()).then(|| parts.join("/"))
}

#[cfg(test)]
mod tests;
