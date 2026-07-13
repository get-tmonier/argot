//! Rule discovery — scan `.argot/rules/*/rule.toml`.
//!
//! Discovery degrades per rule: a malformed manifest, an unreadable script,
//! or an API mismatch skips that one rule with a warning; the run (and every
//! other rule) proceeds. No directory at all is the common case and is
//! silent.

use crate::manifest::{load_rule_dir, ScriptRule};
use std::path::Path;

/// The rules directory, under the repo's `.argot/`.
pub const RULES_DIR: &str = "rules";

/// Load every valid rule under `<argot_dir>/rules/`, sorted by name (a
/// stable order for the registry, `argot rules`, and error messages).
/// Warnings collect one line per skipped rule.
pub fn discover(argot_dir: &Path, warnings: &mut Vec<String>) -> Vec<ScriptRule> {
    let root = argot_dir.join(RULES_DIR);
    let Ok(entries) = std::fs::read_dir(&root) else {
        return Vec::new();
    };
    let mut rules: Vec<ScriptRule> = Vec::new();
    let mut dirs: Vec<_> = entries
        .flatten()
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .map(|e| e.path())
        .collect();
    dirs.sort();
    for dir in dirs {
        match load_rule_dir(&dir) {
            Ok(rule) => rules.push(rule),
            Err(msg) => warnings.push(format!("custom rule {msg}")),
        }
    }
    rules
}

#[cfg(test)]
mod tests;
