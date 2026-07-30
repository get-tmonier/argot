//! `argot mcp` — a Model Context Protocol server over stdio.
//!
//! Exposes argot's voice signal to LLM coding agents (Claude Code, Cursor,
//! Aider, …) so they can (a) score generated code against the repo's voice and
//! (b) ask for the local voice *before* generating, writing in-voice from the
//! first token instead of writing-then-fixing.
//!
//! Transport: newline-delimited JSON-RPC 2.0 on stdin/stdout (the MCP stdio
//! transport). One JSON object per line; notifications (no `id`) get no reply.
//! It runs entirely in-process against the repo's fitted `.argot/` model — no
//! network, no separate runtime, same single binary.

use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde_json::{json, Value};

use argot_core::check::{run_check_read_only, CheckArgs, RepoScorers, DEFAULT_HUNK_LINES};
use argot_core::inspect::{inspect_model, inspect_repo};
use argot_core::output::OutputFormat;
use argot_core::scoring::evidence::format_evidence;

/// MCP protocol revision this server implements.
const PROTOCOL_VERSION: &str = "2024-11-05";

/// Run the stdio server loop until stdin closes.
pub fn run_mcp(repo: PathBuf) -> ExitCode {
    // Startup log → STDERR only. stdout is the JSON-RPC channel; anything
    // non-protocol printed there corrupts the stream. Clients capture this into
    // their MCP logs, and a manual terminal run shows the server actually
    // launched (a stdio server is otherwise silent, which reads as "hung").
    let fitted = repo.join(".argot").join("scorer-config.json").is_file();
    eprintln!("{}", startup_banner(&repo, fitted));

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    let mut out = stdout.lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else { break };
        if line.trim().is_empty() {
            continue;
        }
        let Some(reply) = handle_line(&line, &repo) else {
            // Notification or unparseable input → no response.
            continue;
        };
        if writeln!(out, "{reply}").is_err() || out.flush().is_err() {
            break;
        }
    }
    ExitCode::SUCCESS
}

/// The one-line startup log written to STDERR (never stdout — that's the
/// JSON-RPC channel). Reports version, the repo, and whether a model is fitted,
/// so a silent stdio server is visibly alive and its readiness is obvious.
fn startup_banner(repo: &Path, fitted: bool) -> String {
    format!(
        "argot {} · read-only MCP server ready on stdio · repo: {} · model: {} · tools: repository context + hunk and complete changeset checks; fitting remains an explicit local CLI/skill workflow",
        env!("CARGO_PKG_VERSION"),
        repo.display(),
        if fitted {
            "fitted"
        } else {
            "not fitted (run `argot init`)"
        },
    )
}

/// Turn one incoming JSON-RPC line into the serialized response line, or `None`
/// for notifications (no `id`) and unparseable input.
fn handle_line(line: &str, repo: &Path) -> Option<String> {
    let req: Value = serde_json::from_str(line).ok()?;
    let id = req.get("id").cloned();
    // Notifications carry no id and expect no reply (e.g. notifications/initialized).
    let id = id?;
    let method = req.get("method").and_then(Value::as_str).unwrap_or("");
    let params = req.get("params").cloned().unwrap_or(Value::Null);
    let response = match dispatch(method, &params, repo) {
        Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
        Err((code, message)) => {
            json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message } })
        }
    };
    Some(serde_json::to_string(&response).unwrap_or_default())
}

type RpcError = (i64, String);

/// Route a JSON-RPC method to its handler.
fn dispatch(method: &str, params: &Value, repo: &Path) -> Result<Value, RpcError> {
    match method {
        "initialize" => Ok(json!({
            "protocolVersion": PROTOCOL_VERSION,
            "capabilities": { "tools": {} },
            "serverInfo": { "name": "argot", "version": env!("CARGO_PKG_VERSION") },
            "instructions": "Argot is a read-only repository-context and checking server; the host must invoke each tool. Before writing, call argot.get_voice_context. Use argot.check_hunk for a fast voice-only snippet signal, argot.explain_hunk for deeper evidence about that snippet, and argot.check_changeset for the complete configured detector pipeline over a worktree, index, range, or commit. Call argot.get_fit_status before relying on learned state. Fitting is intentionally absent: use the argot-setup or argot-refresh workflow locally, review the snapshot diff, and commit it.",
        })),
        "tools/list" => Ok(json!({ "tools": tool_definitions() })),
        "tools/call" => tools_call(params, repo),
        // Unknown methods: -32601 Method not found.
        _ => Err((-32601, format!("method not found: {method}"))),
    }
}

/// The tools, with JSON-Schema input shapes.
fn tool_definitions() -> Value {
    let hunk_schema = json!({
        "type": "object",
        "properties": {
            "file_path": { "type": "string", "description": "Repo-relative path of the file the hunk belongs to. Its extension selects the language and that language's model (e.g. `.py`→Python, `.ts`→TypeScript, `.rs`→Rust); an unsupported extension yields no score." },
            "hunk_content": { "type": "string", "description": "The changed code to score — the added/edited lines, not the whole file. A few lines is enough; argot scores the hunk, not the file." },
            "file_source": { "type": "string", "description": "Optional: the full current text of the file. Provide it so argot can resolve callees against the surrounding code (more accurate when the hunk references names defined elsewhere in the file); omit it and the hunk is scored in isolation." }
        },
        "required": ["file_path", "hunk_content"]
    });
    let read_only = json!({
        "readOnlyHint": true,
        "destructiveHint": false,
        "idempotentHint": true,
        "openWorldHint": false
    });
    json!([
        {
            "name": "argot.check_changeset",
            "description": "Run Argot's complete configured detector pipeline over one selected changeset. Use this after editing when you need the same voice, semantic, architecture, integrity, and custom-rule findings as the CLI. Returns the stable check JSON, including findings, evidence, suppressions, and result counts. Read-only: it does not fit, mute, edit files, or update the last-check cache; requires a fitted repository.",
            "inputSchema": {
                "type": "object",
                "properties": {
                    "target": { "type": "string", "enum": ["worktree", "staged", "unstaged", "range", "commit"], "default": "worktree", "description": "Changeset to inspect. worktree includes current tracked and untracked changes; staged and unstaged select one side of the index; range and commit require reference." },
                    "reference": { "type": "string", "description": "Git range such as main..HEAD when target=range, or commit SHA/ref when target=commit." },
                    "only": { "type": "array", "items": { "type": "string" }, "description": "Optional repo-relative glob filters; inspect only matching paths." },
                    "exclude": { "type": "array", "items": { "type": "string" }, "description": "Optional repo-relative glob filters to omit from this invocation." },
                    "min_confidence": { "type": "string", "enum": ["unusual", "suspicious", "foreign"], "default": "unusual", "description": "Display filter only; rule severity still determines the check result." }
                },
                "additionalProperties": false
            },
            "annotations": read_only,
        },
        {
            "name": "argot.check_hunk",
            "description": "Score one supplied code hunk with Argot's fast fitted voice model. Use while drafting or for an isolated snippet when no Git changeset exists. Returns out_of_voice, score, threshold, rule, and concise evidence; it does not run semantic, architecture, integrity, or custom rules. For a real change, prefer check_changeset. Read-only; requires a fitted repository.",
            "inputSchema": hunk_schema,
            "annotations": read_only,
        },
        {
            "name": "argot.explain_hunk",
            "description": "Explain one hunk-level voice result with untruncated structured evidence, including surprising identifiers and attestation counts. Use as a follow-up when check_hunk flags or nearly flags a snippet; do not use it as a second independent check. It covers the fitted voice model only. Read-only; requires a fitted repository.",
            "inputSchema": hunk_schema,
            "annotations": read_only,
        },
        {
            "name": "argot.get_voice_context",
            "description": "Get the repository vocabulary relevant to a file before writing: typical callees, familiar imports, and active replacement guidance for the file's language. This is generation context, not a verdict about code and not a changeset check. Read-only; requires a fitted repository.",
            "inputSchema": json!({
                "type": "object",
                "properties": {
                    "file_path": { "type": "string", "description": "Repo-relative path of the file you're about to create or edit. Its extension selects the language; the returned voice is that language's typical callees and familiar imports." },
                    "top": { "type": "integer", "description": "How many typical callees to return per cluster (default 10). Raise for a fuller picture, lower to keep the injected context small." }
                },
                "required": ["file_path"]
            }),
            "annotations": read_only,
        },
        {
            "name": "argot.get_fit_status",
            "description": "Get repository readiness before using learned tools: fit suitability, committed-snapshot completeness, configuration compatibility, and the adaptive refresh recommendation with structured reasons and next_action. This tool diagnoses setup and maintenance only; it never fits or writes.",
            "inputSchema": json!({ "type": "object", "properties": {} }),
            "annotations": read_only,
        },
        {
            "name": "argot.list_conventions",
            "description": "List conventions learned from the repository: internal API vocabulary, placement concentrations, and migrations still in progress. Use this to discover candidate team conventions or understand where code belongs; it does not evaluate a change. Read-only; requires a fitted repository.",
            "inputSchema": json!({ "type": "object", "properties": {} }),
            "annotations": read_only,
        },
    ])
}

/// Dispatch a `tools/call`. Tool errors are returned as an `isError` content
/// result (per MCP) rather than a protocol error, so the agent can read them.
fn tools_call(params: &Value, repo: &Path) -> Result<Value, RpcError> {
    let name = params
        .get("name")
        .and_then(Value::as_str)
        .ok_or((-32602, "tools/call requires a 'name'".to_string()))?;
    let args = params.get("arguments").cloned().unwrap_or(Value::Null);

    let result = match name {
        "argot.check_changeset" => tool_check_changeset(&args, repo),
        "argot.check_hunk" => tool_check(&args, repo, false),
        "argot.explain_hunk" => tool_check(&args, repo, true),
        "argot.get_voice_context" => tool_voice_context(&args, repo),
        "argot.get_fit_status" => tool_fit_status(repo),
        "argot.list_conventions" => tool_conventions(repo),
        other => return Err((-32602, format!("unknown tool: {other}"))),
    };

    Ok(match result {
        Ok(value) => text_content(&value, false),
        Err(message) => text_content(&json!({ "error": message }), true),
    })
}

/// Wrap a JSON payload as MCP tool `content` (one text block of pretty JSON).
fn text_content(value: &Value, is_error: bool) -> Value {
    json!({
        "content": [{
            "type": "text",
            "text": serde_json::to_string_pretty(value).unwrap_or_default()
        }],
        "isError": is_error,
    })
}

fn argot_dir(repo: &Path) -> PathBuf {
    repo.join(".argot")
}

fn string_array(args: &Value, key: &str) -> Result<Vec<String>, String> {
    let Some(value) = args.get(key) else {
        return Ok(Vec::new());
    };
    let values = value
        .as_array()
        .ok_or_else(|| format!("{key} must be an array of strings"))?;
    values
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} must contain only strings"))
        })
        .collect()
}

/// `argot.check_changeset`: the complete configured CLI check pipeline, minus
/// the last-check cache write used by the interactive `argot mute` workflow.
fn tool_check_changeset(args: &Value, repo: &Path) -> Result<Value, String> {
    let target = args
        .get("target")
        .and_then(Value::as_str)
        .unwrap_or("worktree");
    let supplied_reference = args.get("reference").and_then(Value::as_str).unwrap_or("");
    let (reference, staged, unstaged, commit) = match target {
        "worktree" if supplied_reference.is_empty() => (String::new(), false, false, None),
        "staged" if supplied_reference.is_empty() => (String::new(), true, false, None),
        "unstaged" if supplied_reference.is_empty() => (String::new(), false, true, None),
        "range" if !supplied_reference.is_empty() => {
            (supplied_reference.to_owned(), false, false, None)
        }
        "commit" if !supplied_reference.is_empty() => (
            String::new(),
            false,
            false,
            Some(supplied_reference.to_owned()),
        ),
        "range" | "commit" => return Err(format!("reference is required for target={target}")),
        "worktree" | "staged" | "unstaged" => {
            return Err(format!("reference is not valid for target={target}"));
        }
        _ => {
            return Err(
                "target must be one of: worktree, staged, unstaged, range, commit".to_string(),
            );
        }
    };
    let min_confidence = args
        .get("min_confidence")
        .and_then(Value::as_str)
        .unwrap_or("unusual");
    if !matches!(min_confidence, "unusual" | "suspicious" | "foreign") {
        return Err("min_confidence must be unusual, suspicious, or foreign".to_string());
    }

    let outcome = run_check_read_only(CheckArgs {
        repo_path: repo.to_string_lossy().into_owned(),
        reference,
        staged,
        unstaged,
        commit,
        only: string_array(args, "only")?,
        exclude: string_array(args, "exclude")?,
        threshold: None,
        argot_dir: argot_dir(repo),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_confidence: min_confidence.to_owned(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: OutputFormat::Json,
        today: crate::today_utc(),
    });
    if outcome.exit_code >= 2 {
        let message = outcome.stderr.trim();
        return Err(if message.is_empty() {
            "changeset check could not run".to_string()
        } else {
            message.to_string()
        });
    }
    let mut document: Value = serde_json::from_str(&outcome.stdout)
        .map_err(|error| format!("changeset check returned invalid JSON: {error}"))?;
    document["mcp"] = json!({
        "target": target,
        "read_only": true,
        "recorded_last_check": false
    });
    let notes: Vec<&str> = outcome
        .stderr
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .collect();
    if !notes.is_empty() {
        document["notes"] = json!(notes);
    }
    Ok(document)
}

/// `argot.check_hunk` / `argot.explain_hunk`: score one hunk against the model.
fn tool_check(args: &Value, repo: &Path, explain: bool) -> Result<Value, String> {
    let file_path = args
        .get("file_path")
        .and_then(Value::as_str)
        .ok_or("file_path is required")?;
    let hunk_content = args
        .get("hunk_content")
        .and_then(Value::as_str)
        .ok_or("hunk_content is required")?;
    let file_source = args.get("file_source").and_then(Value::as_str);

    let config = argot_core::config::ArgotConfig::load(repo);
    let migrations = config.migrations().active;
    let check_only = config.exclude.check_only.clone();
    let detect = config.detect;
    let mut scorers = RepoScorers::load(&argot_dir(repo), &detect, &check_only)?;
    if scorers.language_for(file_path).is_none() {
        return Err(format!(
            "unsupported file type for '{file_path}' — argot has no language adapter for this file"
        ));
    }
    let scored = scorers
        .score(file_path, hunk_content, file_source)
        .ok_or_else(|| format!("no fitted model for the language of '{file_path}'"))?;

    let mut out = json!({
        "model": scorers.model_hash,
        "file_path": file_path,
        "out_of_voice": scored.flagged,
        // The winning scorer's score and its bar — the same pair `check` reports,
        // so an agent can gauge how close a hit was (not the internal BPE stage).
        "score": scored.score,
        "threshold": scored.threshold,
        // The stable rule name (registry) — matches `check` JSON's `rule`.
        "rule": argot_core::rules::code_for_reason(scored.reason.as_str()),
    });
    // Evidence: the human summary for both `explain` and a fired `check`; plus,
    // for `explain` only, the full structured payload — every surprising
    // identifier / foreign specifier / unfamiliar callee with its raw attestation
    // count and rarity numerator/denominator, untruncated (the summary caps names
    // at three). That's what makes `explain` genuinely richer than `check` rather
    // than a byte-for-byte duplicate.
    if explain || scored.flagged {
        if let Some(ev) = &scored.evidence {
            let lines: Vec<String> = format_evidence(ev, false, 1)
                .into_iter()
                .map(|l| l.trim().to_string())
                .collect();
            out["evidence"] = json!(lines);
            if explain {
                out["evidence_detail"] = serde_json::to_value(ev).unwrap_or(Value::Null);
            }
        }
    }
    // Superseded patterns the hunk uses (mined supersessions + declared
    // [[migration]] entries) — the same signal `check`'s `superseded` rule
    // reports, so an agent hears "this repo moved on from X" before writing
    // more of it.
    let superseded = scorers.superseded_in_hunk(file_path, hunk_content, &migrations);
    if !superseded.is_empty() {
        out["superseded"] = serde_json::to_value(&superseded).unwrap_or(Value::Null);
    }
    Ok(out)
}

/// `argot.get_voice_context`: the local voice for a file — typical callees per
/// cluster (from the fitted model) plus the familiar import surface.
fn tool_voice_context(args: &Value, repo: &Path) -> Result<Value, String> {
    let file_path = args
        .get("file_path")
        .and_then(Value::as_str)
        .ok_or("file_path is required")?;
    let top = args
        .get("top")
        .and_then(Value::as_u64)
        .map(|n| n as usize)
        .unwrap_or(10);

    let config = argot_core::config::ArgotConfig::load(repo);
    let check_only = config.exclude.check_only.clone();
    let detect = config.detect;
    let scorers = RepoScorers::load(&argot_dir(repo), &detect, &check_only)?;
    let language = scorers
        .language_for(file_path)
        .ok_or_else(|| format!("unsupported file type for '{file_path}'"))?;

    let model = inspect_model(repo, top).map_err(|e| e.to_string())?;
    let lang_view = model
        .languages
        .get(language)
        .ok_or_else(|| format!("no fitted model for '{language}'"))?;

    let clusters: Vec<Value> = lang_view
        .clusters
        .iter()
        .map(|c| {
            json!({
                "cluster": c.id,
                "files": c.files,
                "typical_callees": c.top_callees.iter().map(|(n, _)| n).collect::<Vec<_>>(),
            })
        })
        .collect();

    let familiar_imports: Vec<&String> = lang_view.familiar_imports.iter().take(top).collect();
    let superseded: Vec<Value> = lang_view
        .supersessions
        .iter()
        .map(|s| json!({ "avoid": s.old, "use": s.new }))
        .collect();
    let mut out = json!({
        "file_path": file_path,
        "language": language,
        "model": model.manifest.as_ref().map(|m| m.model_hash.clone()),
        "typical_callees_by_cluster": clusters,
        "familiar_imports": familiar_imports,
        "note": "Prefer these callees and imports; code that reaches for names absent here will read as out of voice.",
    });
    if !superseded.is_empty() {
        out["superseded"] = json!(superseded);
        out["superseded_note"] =
            json!("The repo is migrating away from these — use the replacement.");
    }
    Ok(out)
}

/// `argot.get_fit_status`: suitability plus the snapshot contract agents need to
/// decide whether they may rely on this repository's learned state.
fn tool_fit_status(repo: &Path) -> Result<Value, String> {
    let report = inspect_repo(repo).map_err(|e| e.to_string())?;
    let mut out = serde_json::to_value(&report).map_err(|e| e.to_string())?;
    let snapshot_dir = repo.join(".argot");
    out["snapshot"] = crate::fit_snapshot_status_json(repo, &snapshot_dir);

    let health = argot_core::health::read(&snapshot_dir);
    let (config, _) = argot_core::compose::load_config(repo);
    out["refresh"] = health
        .as_ref()
        .map(|h| argot_core::refresh::assess(repo, h, &config))
        .map(|assessment| serde_json::to_value(assessment).unwrap_or(Value::Null))
        .unwrap_or(Value::Null);
    Ok(out)
}

/// `argot.list_conventions`: the repo's vocabulary + placement conventions.
fn tool_conventions(repo: &Path) -> Result<Value, String> {
    let catalog =
        argot_core::convention_catalog::build_catalog(repo, 10).map_err(|e| e.to_string())?;
    serde_json::to_value(&catalog).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn initialize_advertises_tools_and_server_info() {
        let repo = PathBuf::from(".");
        let result = dispatch("initialize", &Value::Null, &repo).unwrap();
        assert_eq!(result["protocolVersion"], PROTOCOL_VERSION);
        assert_eq!(result["serverInfo"]["name"], "argot");
        assert!(result["capabilities"]["tools"].is_object());
    }

    #[test]
    fn tools_list_exposes_every_tool() {
        let repo = PathBuf::from(".");
        let result = dispatch("tools/list", &Value::Null, &repo).unwrap();
        let names: Vec<&str> = result["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|t| t["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "argot.check_changeset",
                "argot.check_hunk",
                "argot.explain_hunk",
                "argot.get_voice_context",
                "argot.get_fit_status",
                "argot.list_conventions",
            ]
        );
        for tool in result["tools"].as_array().unwrap() {
            let description = tool["description"].as_str().unwrap();
            assert!(!description.is_empty());
            assert_eq!(tool["annotations"]["readOnlyHint"], true);
            assert_eq!(tool["annotations"]["destructiveHint"], false);
        }
        assert!(result["tools"][0]["description"]
            .as_str()
            .unwrap()
            .contains("complete configured detector pipeline"));
        assert!(result["tools"][1]["description"]
            .as_str()
            .unwrap()
            .contains("voice model"));
        assert!(result["tools"][2]["description"]
            .as_str()
            .unwrap()
            .contains("follow-up"));
    }

    #[test]
    fn unknown_method_is_a_json_rpc_method_not_found() {
        let repo = PathBuf::from(".");
        let err = dispatch("does/not/exist", &Value::Null, &repo).unwrap_err();
        assert_eq!(err.0, -32601);
    }

    #[test]
    fn notifications_get_no_reply() {
        // No `id` → notification → no response line.
        let line = r#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
        assert!(handle_line(line, &PathBuf::from(".")).is_none());
    }

    #[test]
    fn a_request_with_an_id_gets_a_reply_line() {
        let line = r#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
        let reply = handle_line(line, &PathBuf::from(".")).expect("reply");
        let doc: Value = serde_json::from_str(&reply).unwrap();
        assert_eq!(doc["id"], 1);
        assert_eq!(doc["result"]["serverInfo"]["name"], "argot");
    }

    #[test]
    fn tools_call_on_a_repo_without_a_model_reports_an_iserror_result() {
        // No `.argot/` under a bare temp dir → check tool returns isError, not a
        // protocol error (the agent can read the message).
        let tmp = std::env::temp_dir().join("argot_mcp_no_model_test");
        let _ = std::fs::create_dir_all(&tmp);
        let params = json!({
            "name": "argot.check_hunk",
            "arguments": { "file_path": "a.py", "hunk_content": "x = 1\n" }
        });
        let result = dispatch("tools/call", &params, &tmp).unwrap();
        assert_eq!(result["isError"], true);
    }

    #[test]
    fn startup_banner_reports_version_repo_and_fit() {
        let f = startup_banner(Path::new("/x"), true);
        assert!(f.contains(env!("CARGO_PKG_VERSION")));
        assert!(f.contains("/x"));
        assert!(f.contains("stdio"));
        assert!(f.contains("model: fitted"));
        assert!(f.contains("read-only MCP server"));
        assert!(f.contains("complete changeset checks"));
        assert!(f.contains("fitting remains an explicit local CLI/skill workflow"));
        let u = startup_banner(Path::new("."), false);
        assert!(u.contains("not fitted"));
        assert!(u.contains("argot init"));
    }

    #[test]
    fn protocol_wording_contract_keeps_names_and_capability_boundaries() {
        let repo = PathBuf::from(".");
        let initialized = dispatch("initialize", &Value::Null, &repo).unwrap();
        let instructions = initialized["instructions"].as_str().unwrap();
        assert!(instructions.contains("read-only repository-context and checking server"));
        assert!(instructions.contains("argot.check_changeset"));
        assert!(instructions.contains("Fitting is intentionally absent"));

        let tools = dispatch("tools/list", &Value::Null, &repo).unwrap();
        let names: Vec<&str> = tools["tools"]
            .as_array()
            .unwrap()
            .iter()
            .map(|tool| tool["name"].as_str().unwrap())
            .collect();
        assert_eq!(
            names,
            [
                "argot.check_changeset",
                "argot.check_hunk",
                "argot.explain_hunk",
                "argot.get_voice_context",
                "argot.get_fit_status",
                "argot.list_conventions",
            ]
        );
        assert!(names
            .iter()
            .all(|name| name.split('.').nth(1).unwrap().contains('_')));
        assert!(!names.contains(&"argot.check"));
        assert!(!names.contains(&"argot.voice_context"));
    }

    #[test]
    fn changeset_tool_requires_a_reference_for_range_or_commit() {
        let repo = PathBuf::from(".");
        for target in ["range", "commit"] {
            let params = json!({
                "name": "argot.check_changeset",
                "arguments": { "target": target }
            });
            let result = dispatch("tools/call", &params, &repo).unwrap();
            assert_eq!(result["isError"], true);
            let text = result["content"][0]["text"].as_str().unwrap();
            assert!(text.contains("reference is required"));
        }
    }
}
