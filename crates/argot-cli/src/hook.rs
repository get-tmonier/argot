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
    let config = ArgotConfig::load(repo);
    let relative_path = repo_relative_path(repo, file_path)?;
    if !can_assess(&config, &relative_path) {
        return None;
    }

    let mut scorers = RepoScorers::load(&repo.join(".argot"), &config.detect).ok()?;
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

/// Apply the configuration subset that can be decided before a write. The
/// hook has no final diff hunk or stable hit hash, so inline and hash mutes are
/// deliberately unsupported here; treating a proposed partial edit as either
/// would claim parity the hook cannot provide.
fn can_assess(config: &ArgotConfig, relative_path: &str) -> bool {
    // Check reports configuration diagnostics to its caller; the hook has no
    // diagnostic channel that should interrupt an agent, so malformed config
    // fails open rather than silently falling back to a possibly unwanted ask.
    if !config.warnings.is_empty() {
        return false;
    }
    let settings = config.rule_settings(&Vec::new());
    settings.severity_of_reason("import") != Severity::Off
        && settings.covers_path("import", relative_path)
        && config.path_suppressions().classify(relative_path) == PathScope::InScope
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

/// Normalize a Claude file path to the repository-relative, slash-separated
/// form used by Argot's path scopes and exclusions. Files outside the repo (or
/// relative paths escaping it) are not candidates for a pre-write ask.
fn repo_relative_path(repo: &Path, file_path: &str) -> Option<String> {
    let path = Path::new(file_path);
    let candidate = if path.is_absolute() {
        path.strip_prefix(repo).ok()?.to_path_buf()
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
