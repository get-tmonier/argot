//! The rule-authoring harness — `argot rules test`.
//!
//! Fixtures live inside the rule directory:
//!
//! ```text
//! .argot/rules/no-raw-sql/
//!   rule.toml
//!   check.rhai
//!   tests/
//!     fires-on-execute/
//!       input.py          # the file the rule runs over (whole file = one hunk)
//!       expected.json     # [{"line": 2, "message": "raw SQL — …"}]
//!     silent-on-builder/
//!       input.py
//!       expected.json     # []
//! ```
//!
//! An optional `old.<ext>` sibling supplies the pre-image for
//! `file.old_text` / `ts_query_old` (absent = added file).
//!
//! Each case runs the rule over `input.<ext>` exactly as `check` would (the
//! extension picks the language; the whole file is one hunk) and compares
//! the reported `(line, message)` pairs against `expected.json`, order-
//! independent. Learned-model facts are absent in the harness —
//! `import_attested`/`callee_attested` return `false` — so a rule keying on
//! them should test the unattested path here and the attested path live.

use crate::host;
use crate::manifest::ScriptRule;
use serde::Deserialize;
use std::path::Path;

/// One expected finding.
#[derive(Debug, Clone, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
pub struct Expected {
    pub line: usize,
    pub message: String,
}

/// One executed case.
#[derive(Debug)]
pub struct CaseResult {
    pub rule: String,
    pub case: String,
    /// `None` = pass; `Some` = a human-readable failure.
    pub failure: Option<String>,
}

/// Run every test case of every discovered rule (or just `filter`'s).
/// `Err` is a setup problem (unknown rule name); per-case problems land in
/// the results as failures.
pub fn run_rule_tests(
    argot_dir: &Path,
    filter: Option<&str>,
    warnings: &mut Vec<String>,
) -> Result<Vec<CaseResult>, String> {
    let rules = crate::discover::discover(argot_dir, warnings);
    if let Some(name) = filter {
        if !rules.iter().any(|r| r.name == name) {
            return Err(format!(
                "unknown custom rule '{name}' (discovered: {})",
                rules
                    .iter()
                    .map(|r| r.name.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }
    }
    let mut results = Vec::new();
    for rule in &rules {
        if filter.is_some_and(|f| f != rule.name) {
            continue;
        }
        results.extend(run_one_rule(argot_dir, rule));
    }
    Ok(results)
}

fn run_one_rule(argot_dir: &Path, rule: &ScriptRule) -> Vec<CaseResult> {
    let tests_dir = argot_dir.join("rules").join(&rule.name).join("tests");
    let Ok(entries) = std::fs::read_dir(&tests_dir) else {
        return vec![CaseResult {
            rule: rule.name.clone(),
            case: "(no tests)".to_string(),
            failure: Some(format!(
                "no tests/ directory — add {}/tests/<case>/{{input.<ext>, expected.json}}",
                rule.name
            )),
        }];
    };
    let ast = match host::compile(&rule.script) {
        Ok(a) => a,
        Err(e) => {
            return vec![CaseResult {
                rule: rule.name.clone(),
                case: "(compile)".to_string(),
                failure: Some(format!("script failed to compile: {e}")),
            }]
        }
    };
    let mut cases: Vec<_> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    cases.sort();
    cases
        .into_iter()
        .map(|dir| run_case(rule, &ast, &dir))
        .collect()
}

fn run_case(rule: &ScriptRule, ast: &rhai::AST, dir: &Path) -> CaseResult {
    let case = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let fail = |msg: String| CaseResult {
        rule: rule.name.clone(),
        case: case.clone(),
        failure: Some(msg),
    };

    // The single `input.<ext>` file drives the case.
    let Some(input) = std::fs::read_dir(dir).ok().and_then(|entries| {
        entries
            .flatten()
            .map(|e| e.path())
            .find(|p| p.file_stem().is_some_and(|s| s == "input"))
    }) else {
        return fail("no input.<ext> file".to_string());
    };
    let ext = format!(
        ".{}",
        input.extension().unwrap_or_default().to_string_lossy()
    );
    // Language from the extension; a rule with `files` globs may legitimately
    // target extensions argot doesn't score (`.env`, CI configs…) — the
    // fixture then runs with `file.language == ""` and no tree-sitter.
    let language = argot_lang::ext::ext_to_lang(&ext);
    match language {
        Some(l) if !rule.covers_language(l) => {
            return fail(format!(
                "input is {l}, but the rule's manifest scopes languages = {:?}",
                rule.languages
            ));
        }
        None if rule.files.is_empty() => {
            return fail(format!(
                "unsupported input extension '{ext}' — scope the rule with `files` globs to \
                 run it on unscored files"
            ));
        }
        _ => {}
    }
    let language = language.unwrap_or("");
    let source = match std::fs::read_to_string(&input) {
        Ok(s) => s,
        Err(e) => return fail(format!("reading input failed: {e}")),
    };
    let expected: Vec<Expected> = match std::fs::read_to_string(dir.join("expected.json"))
        .map_err(|e| e.to_string())
        .and_then(|raw| serde_json::from_str(&raw).map_err(|e| e.to_string()))
    {
        Ok(v) => v,
        Err(e) => return fail(format!("expected.json: {e}")),
    };

    // Optional pre-image fixture: a sibling `old.<ext>` feeds `file.old_text`
    // / `ts_query_old` (absent = the rule sees an added file).
    let old_source = std::fs::read_dir(dir).ok().and_then(|entries| {
        entries
            .flatten()
            .map(|e| e.path())
            .find(|p| p.file_stem().is_some_and(|s| s == "old"))
            .and_then(|p| std::fs::read_to_string(p).ok())
    });
    let lines = source.lines().count().max(1);
    let file = host::FileInput {
        path: &format!(
            "tests/{case}/{}",
            input.file_name().unwrap().to_string_lossy()
        ),
        language,
        new_text: &source,
        old_text: old_source.as_deref(),
        hunks: &[(1, lines)],
    };
    let got = match host::run_on_file(ast, &file, vec![file.path.to_string()], None) {
        Ok(v) => v,
        Err(e) => return fail(format!("script failed: {e}")),
    };

    let mut got: Vec<Expected> = got
        .into_iter()
        .map(|f| Expected {
            line: f.line,
            message: f.message,
        })
        .collect();
    got.sort();
    let mut want = expected;
    want.sort();
    if got == want {
        CaseResult {
            rule: rule.name.clone(),
            case,
            failure: None,
        }
    } else {
        fail(format!(
            "findings differ\n      expected: {}\n      got:      {}",
            render(&want),
            render(&got)
        ))
    }
}

fn render(findings: &[Expected]) -> String {
    if findings.is_empty() {
        return "(none)".to_string();
    }
    findings
        .iter()
        .map(|f| format!("L{} {:?}", f.line, f.message))
        .collect::<Vec<_>>()
        .join(", ")
}

#[cfg(test)]
mod tests;
