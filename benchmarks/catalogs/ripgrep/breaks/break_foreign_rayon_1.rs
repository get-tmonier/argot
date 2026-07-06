/*!
Break fixture — not for compilation against the real workspace.
*/

use std::path::{Path, PathBuf};

/// Decoy: a single ignore rule in gitignore.rs's voice.
#[derive(Clone, Debug)]
struct IgnoreRule {
    original: String,
    is_whitelist: bool,
    is_only_dir: bool,
}

/// Decoy: rule construction helper.
fn rule_from_line(line: &str) -> Option<IgnoreRule> {
    let line = line.trim();
    if line.is_empty() || line.starts_with('#') {
        return None;
    }
    Some(IgnoreRule {
        is_whitelist: line.starts_with('!'),
        is_only_dir: line.ends_with('/'),
        original: line.to_string(),
    })
}

// Break: rayon par_iter for parallel matching. Verified foreign at the
// pinned SHA: `name = "rayon"` has zero occurrences in Cargo.lock; the
// repo's only parallelism primitives are crossbeam-channel and its own
// crossbeam-deque work-stealing walker in crates/ignore/src/walk.rs.
// Break: begin
use rayon::prelude::*;

fn matched_paths(rules: &[IgnoreRule], paths: &[PathBuf]) -> Vec<PathBuf> {
    paths
        .par_iter()
        .filter(|path| {
            rules.par_iter().any(|rule| {
                !rule.is_whitelist && path_matches(rule, path)
            })
        })
        .cloned()
        .collect()
}
// Break: end

/// Decoy: sequential matcher used by the rule set.
fn path_matches(rule: &IgnoreRule, path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    if rule.is_only_dir && !path.is_dir() {
        return false;
    }
    rule.original.trim_start_matches('!').trim_end_matches('/') == name
}
