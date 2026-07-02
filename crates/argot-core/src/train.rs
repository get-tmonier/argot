//! `train` (a.k.a. `fit`) — port of `engine/argot/train.py`.
//!
//! Collects the repo's production source files into `repo-corpus.txt` and
//! emits the pre-baked BPE generic baseline. There is no model training here;
//! "train" is corpus collection + baseline copy.

use crate::suppress::PathSuppressions;
use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// The generic BPE baseline (`generic_tokens_bpe.json`), embedded so the
/// binary is self-contained (Python `train.py` copies this file).
pub const GENERIC_BASELINE_JSON: &[u8] = include_bytes!("../data/generic_tokens_bpe.json");

const SOURCE_EXTENSIONS: &[&str] = &[".py", ".ts", ".tsx", ".cs"];

const EXCLUDE_DIRS: &[&str] = &[
    "node_modules",
    ".git",
    ".tox",
    ".eggs",
    "__pycache__",
    "build",
    "dist",
    ".venv",
    "venv",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "test",
    "tests",
    "__tests__",
    "benchmarks",
];

/// Python `Path(name).suffix` (case-sensitive) — the substring from the last
/// dot in the basename when that dot is neither first nor last char.
fn suffix(name: &str) -> &str {
    match name.rfind('.') {
        Some(i) if i > 0 && i < name.len() - 1 => &name[i..],
        _ => "",
    }
}

fn is_test_filename(name: &str) -> bool {
    name.starts_with("test_") || name.contains(".test.") || name.contains(".spec.")
}

/// Recursively collect production source files under `repo_path`, mirroring
/// `_collect_source_files`: keep `.py/.ts/.tsx`, drop any path with an
/// excluded directory component, drop test/spec files, and drop paths the
/// user muted in `.argotignore` (user patterns only — the built-in
/// `argot:recommended` set governs calibration/check scope, not corpus
/// collection, so a repo without an `.argotignore` gets exactly the corpus
/// it always did). Vendored trees (`repos/`, editor-history dirs, …) would
/// otherwise attest their own voice into the model.
///
/// The result is sorted for reproducibility. Python's `rglob` order is
/// filesystem-dependent (non-deterministic) and downstream consumers only
/// build order-independent counters, so sorting is a justified, score-neutral
/// divergence.
pub fn collect_source_files(repo_path: &Path) -> Vec<PathBuf> {
    collect_source_files_with(repo_path, &PathSuppressions::load(repo_path))
}

/// [`collect_source_files`] against an already-resolved suppression set.
pub fn collect_source_files_with(
    repo_path: &Path,
    suppressions: &PathSuppressions,
) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_recursive(repo_path, repo_path, suppressions, &mut out);
    out.sort();
    out
}

/// True when the user's `.argotignore` mutes this path. Recommended-set
/// exclusions deliberately do NOT count here (corpus behaviour without an
/// `.argotignore` must stay byte-identical) — but a user pattern applies even
/// where the recommended set overlaps it (e.g. `.history/`), so vendored /
/// editor-state trees are pruned from the corpus the user explicitly muted.
fn user_ignored(path: &Path, root: &Path, suppressions: &PathSuppressions) -> bool {
    match crate::suppress::rel_string(path, root) {
        Some(rel) => suppressions.matches_user_pattern(&rel),
        None => false,
    }
}

fn collect_recursive(
    dir: &Path,
    root: &Path,
    suppressions: &PathSuppressions,
    out: &mut Vec<PathBuf>,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let file_type = match entry.file_type() {
            Ok(t) => t,
            Err(_) => continue,
        };
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if file_type.is_dir() {
            // Prune excluded directories (equivalent to Python's post-filter
            // on path components, but cheaper). Gitignore semantics: a muted
            // directory prunes its whole subtree.
            if EXCLUDE_DIRS.contains(&name.as_ref()) {
                continue;
            }
            if user_ignored(&path, root, suppressions) {
                continue;
            }
            collect_recursive(&path, root, suppressions, out);
        } else if file_type.is_file() {
            if !SOURCE_EXTENSIONS.contains(&suffix(&name)) {
                continue;
            }
            if is_test_filename(&name) {
                continue;
            }
            if user_ignored(&path, root, suppressions) {
                continue;
            }
            out.push(path);
        }
    }
}

/// Result of a `train` run.
pub struct TrainOutcome {
    pub source_file_count: usize,
}

/// Port of `train.py:main`: collect the repo corpus and write the generic
/// baseline. `repo_path` must contain a `.git`.
pub fn run_train(
    repo_path: &Path,
    repo_corpus_out: &Path,
    generic_baseline_out: &Path,
) -> Result<TrainOutcome> {
    let repo_path = fs::canonicalize(repo_path).unwrap_or_else(|_| repo_path.to_path_buf());
    if !repo_path.join(".git").exists() {
        bail!("not a git repository: {}", repo_path.display());
    }

    if let Some(parent) = repo_corpus_out.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = generic_baseline_out.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let files = collect_source_files(&repo_path);
    if files.is_empty() {
        bail!("no source files found in repository");
    }

    // "\n".join(str(p) for p in files) — no trailing newline.
    let listing = files
        .iter()
        .map(|p| p.to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(repo_corpus_out, listing)?;
    fs::write(generic_baseline_out, GENERIC_BASELINE_JSON)?;

    Ok(TrainOutcome {
        source_file_count: files.len(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suffix_is_case_sensitive() {
        // Python `.suffix` preserves case, so ".PY" != ".py".
        assert_eq!(suffix("a.py"), ".py");
        assert_eq!(suffix("a.PY"), ".PY");
        assert!(!SOURCE_EXTENSIONS.contains(&suffix("a.PY")));
    }

    #[test]
    fn filters_tests_and_excluded_dirs() {
        let tmp = std::env::temp_dir().join(format!("argot_train_test_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::create_dir_all(tmp.join("tests")).unwrap();
        fs::create_dir_all(tmp.join("node_modules/pkg")).unwrap();
        fs::write(tmp.join("src/app.py"), "x=1").unwrap();
        fs::write(tmp.join("src/app.test.ts"), "x").unwrap();
        fs::write(tmp.join("src/widget.tsx"), "x").unwrap();
        fs::write(tmp.join("tests/test_app.py"), "x").unwrap();
        fs::write(tmp.join("node_modules/pkg/index.ts"), "x").unwrap();
        fs::write(tmp.join("README.md"), "x").unwrap();

        let files = collect_source_files(&tmp);
        let names: Vec<String> = files
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(names.contains(&"app.py".to_string()));
        assert!(names.contains(&"widget.tsx".to_string()));
        assert!(!names.contains(&"app.test.ts".to_string()));
        assert!(
            !names.contains(&"test_app.py".to_string()),
            "tests dir excluded"
        );
        assert!(
            !names.contains(&"index.ts".to_string()),
            "node_modules excluded"
        );
        assert_eq!(files.len(), 2);
        let _ = fs::remove_dir_all(&tmp);
    }

    #[test]
    fn argotignore_user_patterns_prune_corpus() {
        let tmp = std::env::temp_dir().join(format!("argot_train_ignore_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::create_dir_all(tmp.join("repos/vendored")).unwrap();
        fs::create_dir_all(tmp.join("docs")).unwrap();
        fs::write(tmp.join("src/app.py"), "x=1").unwrap();
        fs::write(tmp.join("repos/vendored/lib.py"), "x=1").unwrap();
        fs::write(tmp.join("docs/example.py"), "x=1").unwrap();

        // Without .argotignore: repos/ and docs/ are in the corpus (train's
        // own exclude list never covered them).
        let before = collect_source_files(&tmp);
        assert_eq!(before.len(), 3);

        fs::write(tmp.join(".argotignore"), "repos/\n").unwrap();
        let after = collect_source_files(&tmp);
        let names: Vec<String> = after
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
            .collect();
        assert!(!names.contains(&"lib.py".to_string()), "repos/ pruned");
        // Recommended-set paths (docs/) are NOT pruned from the corpus —
        // only user patterns apply here.
        assert!(names.contains(&"example.py".to_string()));
        assert_eq!(after.len(), 2);
        let _ = fs::remove_dir_all(&tmp);
    }
}
