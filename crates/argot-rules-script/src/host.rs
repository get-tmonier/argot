//! The Rhai host — sandboxed execution of one rule's script over one file.
//!
//! The contract with rule authors (host API v1):
//! - The script's top-level statements run once per in-scope changed file.
//! - In scope: `file` (a map: `path`, `language` — `""` for a file argot
//!   doesn't score, `ext`, `new_text`, `old_text` —
//!   the pre-image, `()` for added files) and `hunks`
//!   (an array of maps: `start`, `end`, `text` — the changed line ranges,
//!   1-indexed inclusive, post-image).
//! - Host functions: `ts_query(query)` / `ts_query_old(query)` (tree-sitter
//!   query against the post-/pre-image,
//!   → array of `#{capture, text, line, end_line}` matches),
//!   `import_attested(module)` / `callee_attested(name)` (the fitted voice
//!   model's learned facts for this file's language),
//!   `changeset_paths()` (every path in the changeset), and
//!   `report(line, message)` / `report_span(start, end, message, opts)`
//!   (`opts` map: optional `evidence` array, optional `symbol`).
//!
//! Sandbox: no filesystem, no network, no module imports (Rhai's core has
//! none, and `print`/`debug` are captured, never reaching stdout). Runaway
//! scripts trip hard operation / depth / size caps or the per-file
//! wall-clock budget; the caller disables the rule for the rest of the run.

use argot_engine::detector::ModelFacts;
use rhai::{Array, Dynamic, Engine, Map, Scope, AST};
use std::cell::RefCell;
use std::rc::Rc;
use std::sync::Arc;
use std::time::{Duration, Instant};
use streaming_iterator::StreamingIterator;

/// Hard operation cap per (rule, file) — far above any sane pattern rule.
const MAX_OPERATIONS: u64 = 1_000_000;
/// Call-depth cap (recursion guard).
const MAX_CALL_LEVELS: usize = 32;
/// Wall-clock budget per (rule, file).
pub const FILE_BUDGET: Duration = Duration::from_millis(100);

/// One `report(...)` from a script.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScriptFinding {
    pub line: usize,
    pub line_end: usize,
    pub message: String,
    pub evidence: Vec<String>,
    pub symbol: Option<String>,
}

/// One file's inputs, as the script sees them.
pub struct FileInput<'a> {
    /// Repo-relative path.
    pub path: &'a str,
    /// Scoring language name (`"python"`, …).
    pub language: &'a str,
    /// Post-image source.
    pub new_text: &'a str,
    /// Pre-image source (`None` = added file, or the mode can't resolve it).
    pub old_text: Option<&'a str>,
    /// Changed line ranges, 1-indexed inclusive.
    pub hunks: &'a [(usize, usize)],
}

/// Compile a rule script once per run (the AST is reused across files).
pub fn compile(script: &str) -> Result<AST, String> {
    sandboxed_engine()
        .compile(script)
        .map_err(|e| e.to_string())
}

/// The sandboxed engine: full Rhai core (arithmetic, strings, arrays, maps —
/// no I/O exists in the core language), with output captured and hard limits.
fn sandboxed_engine() -> Engine {
    let mut engine = Engine::new();
    engine.set_max_operations(MAX_OPERATIONS);
    engine.set_max_call_levels(MAX_CALL_LEVELS);
    engine.set_max_string_size(1 << 20);
    engine.set_max_array_size(100_000);
    engine.set_max_map_size(10_000);
    // `print`/`debug` must never reach stdout (a parity surface) — swallow.
    engine.on_print(|_| {});
    engine.on_debug(|_, _, _| {});
    engine
}

/// Run one compiled rule over one file. `Err` is the verbatim engine error
/// (compile-stage errors surface from [`compile`]) — the caller turns it
/// into a rule-disabling diagnostic.
pub fn run_on_file(
    ast: &AST,
    file: &FileInput<'_>,
    changeset_paths: Vec<String>,
    facts: Option<Arc<dyn ModelFacts>>,
) -> Result<Vec<ScriptFinding>, String> {
    let mut engine = sandboxed_engine();

    // Wall-clock budget, checked on the operation ticks.
    let deadline = Instant::now() + FILE_BUDGET;
    engine.on_progress(move |_| {
        if Instant::now() > deadline {
            Some(Dynamic::from("file budget exceeded"))
        } else {
            None
        }
    });

    let findings: Rc<RefCell<Vec<ScriptFinding>>> = Rc::new(RefCell::new(Vec::new()));

    // report(line, message)
    {
        let findings = findings.clone();
        engine.register_fn("report", move |line: i64, message: &str| {
            findings.borrow_mut().push(ScriptFinding {
                line: line.max(1) as usize,
                line_end: line.max(1) as usize,
                message: message.to_string(),
                evidence: Vec::new(),
                symbol: None,
            });
        });
    }
    // report_span(start, end, message, #{evidence: [...], symbol: "..."})
    {
        let findings = findings.clone();
        engine.register_fn(
            "report_span",
            move |start: i64, end: i64, message: &str, opts: Map| {
                let evidence = opts
                    .get("evidence")
                    .and_then(|v| v.clone().try_cast::<Array>())
                    .map(|a| {
                        a.into_iter()
                            .filter_map(|d| d.try_cast::<String>())
                            .collect()
                    })
                    .unwrap_or_default();
                let symbol = opts
                    .get("symbol")
                    .and_then(|v| v.clone().try_cast::<String>());
                let start = start.max(1) as usize;
                findings.borrow_mut().push(ScriptFinding {
                    line: start,
                    line_end: (end.max(1) as usize).max(start),
                    message: message.to_string(),
                    evidence,
                    symbol,
                });
            },
        );
    }
    // ts_query(query) — native tree-sitter against the post-image.
    {
        let source = file.new_text.to_string();
        let language = file.language.to_string();
        engine.register_fn("ts_query", move |query: &str| -> Array {
            ts_query(&language, &source, query)
        });
    }
    // ts_query_old(query) — the same, against the pre-image (empty when the
    // file is new or the old side is unresolvable).
    {
        let old_source = file.old_text.unwrap_or_default().to_string();
        let language = file.language.to_string();
        engine.register_fn("ts_query_old", move |query: &str| -> Array {
            if old_source.is_empty() {
                Array::new()
            } else {
                ts_query(&language, &old_source, query)
            }
        });
    }
    // Learned voice-model facts for this file's language.
    {
        let facts_c = facts.clone();
        let language = file.language.to_string();
        engine.register_fn("import_attested", move |module: &str| -> bool {
            facts_c
                .as_ref()
                .map(|f| f.import_attested(&language, module))
                .unwrap_or(false)
        });
    }
    {
        let language = file.language.to_string();
        engine.register_fn("callee_attested", move |name: &str| -> bool {
            facts
                .as_ref()
                .map(|f| f.callee_attested(&language, name))
                .unwrap_or(false)
        });
    }
    // changeset_paths()
    {
        engine.register_fn("changeset_paths", move || -> Array {
            changeset_paths
                .iter()
                .map(|p| Dynamic::from(p.clone()))
                .collect()
        });
    }

    let mut scope = Scope::new();
    let mut file_map = Map::new();
    file_map.insert("path".into(), Dynamic::from(file.path.to_string()));
    file_map.insert("language".into(), Dynamic::from(file.language.to_string()));
    file_map.insert(
        "ext".into(),
        Dynamic::from(
            std::path::Path::new(file.path)
                .extension()
                .map(|e| format!(".{}", e.to_string_lossy()))
                .unwrap_or_default(),
        ),
    );
    file_map.insert("new_text".into(), Dynamic::from(file.new_text.to_string()));
    file_map.insert(
        "old_text".into(),
        file.old_text
            .map(|t| Dynamic::from(t.to_string()))
            .unwrap_or(Dynamic::UNIT),
    );
    scope.push_constant("file", file_map);
    let hunks: Array = file
        .hunks
        .iter()
        .map(|(start, end)| {
            let mut h = Map::new();
            h.insert("start".into(), Dynamic::from(*start as i64));
            h.insert("end".into(), Dynamic::from(*end as i64));
            let text = hunk_text(file.new_text, *start, *end);
            h.insert("text".into(), Dynamic::from(text));
            Dynamic::from_map(h)
        })
        .collect();
    scope.push_constant("hunks", hunks);

    engine
        .run_ast_with_scope(&mut scope, ast)
        .map_err(|e| e.to_string())?;
    // The engine's registered closures still hold Rc clones — read, don't
    // try_unwrap.
    let out = findings.borrow().clone();
    Ok(out)
}

/// The 1-indexed inclusive line range of `source` as one string.
fn hunk_text(source: &str, start: usize, end: usize) -> String {
    source
        .lines()
        .skip(start.saturating_sub(1))
        .take(end.saturating_sub(start) + 1)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Run a tree-sitter query over the file, returning one map per capture:
/// `#{capture, text, line, end_line}` (lines 1-indexed). An invalid query or
/// unsupported language yields an empty array — the authoring harness is the
/// place to catch query typos, not a hard error mid-check.
fn ts_query(language: &str, source: &str, query: &str) -> Array {
    let Some(lang) = argot_lang::adapters::Language::from_scoring_name(language) else {
        return Array::new();
    };
    let Some(tree) = argot_lang::ts_parse::parse(source, lang) else {
        return Array::new();
    };
    let ts_lang = argot_lang::ts_parse::ts_language(lang);
    let Ok(compiled) = tree_sitter::Query::new(&ts_lang, query) else {
        return Array::new();
    };
    let mut cursor = tree_sitter::QueryCursor::new();
    let names = compiled.capture_names();
    let mut out = Array::new();
    // tree-sitter 0.25+ returns a StreamingIterator from `matches`, not a
    // std Iterator — drive it with `.next()` via the streaming-iterator trait.
    let mut matches = cursor.matches(&compiled, tree.root_node(), source.as_bytes());
    while let Some(m) = matches.next() {
        for cap in m.captures {
            let node = cap.node;
            let mut entry = Map::new();
            entry.insert(
                "capture".into(),
                Dynamic::from(names[cap.index as usize].to_string()),
            );
            entry.insert(
                "text".into(),
                Dynamic::from(
                    node.utf8_text(source.as_bytes())
                        .unwrap_or_default()
                        .to_string(),
                ),
            );
            entry.insert(
                "line".into(),
                Dynamic::from((node.start_position().row + 1) as i64),
            );
            entry.insert(
                "end_line".into(),
                Dynamic::from((node.end_position().row + 1) as i64),
            );
            out.push(Dynamic::from_map(entry));
        }
    }
    out
}

#[cfg(test)]
mod tests;
