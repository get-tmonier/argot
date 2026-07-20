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
use argot_core::config::ArgotConfig;
use argot_core::scoring::evidence::format_evidence;

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
    let detect = ArgotConfig::load(repo).detect;
    let mut scorers = RepoScorers::load(&repo.join(".argot"), &detect).ok()?;
    scorers.language_for(file_path)?;
    let scored = scorers.score(file_path, content, Some(content))?;
    if !scored.flagged
        || argot_core::rules::code_for_reason(scored.reason.as_str()) != "foreign-import"
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_write_family_tools_are_scored() {
        assert!(is_write_tool("Write"));
        assert!(is_write_tool("Edit"));
        assert!(is_write_tool("MultiEdit"));
        assert!(!is_write_tool("Read"));
        assert!(!is_write_tool("Bash"));
    }

    #[test]
    fn proposed_content_reads_write_and_edit_shapes() {
        assert_eq!(
            proposed_content(&json!({"content": "import x\n"})),
            "import x\n"
        );
        let edit = json!({"edits": [
            {"old_string": "a", "new_string": "import y"},
            {"old_string": "b", "new_string": "z"}
        ]});
        assert_eq!(proposed_content(&edit), "import y\nz");
        assert_eq!(
            proposed_content(&json!({"new_string": "import q"})),
            "import q"
        );
        assert_eq!(proposed_content(&json!({})), "");
    }
}
