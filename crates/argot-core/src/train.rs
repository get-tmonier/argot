//! `train` (a.k.a. `fit`) — port of `engine/argot/train.py`.
//!
//! Collects the repo's production source files into `repo-corpus.txt` and
//! emits the pre-baked BPE generic baseline. There is no model training here;
//! "train" is corpus collection + baseline copy.

use anyhow::{bail, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// The generic BPE baseline (`generic_tokens_bpe.json`), embedded so the
/// binary is self-contained (Python `train.py` copies this file).
pub const GENERIC_BASELINE_JSON: &[u8] = include_bytes!("../data/generic_tokens_bpe.json");

const SOURCE_EXTENSIONS: &[&str] = &[".py", ".ts", ".tsx"];

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
/// excluded directory component, drop test/spec files.
///
/// The result is sorted for reproducibility. Python's `rglob` order is
/// filesystem-dependent (non-deterministic) and downstream consumers only
/// build order-independent counters, so sorting is a justified, score-neutral
/// divergence.
pub fn collect_source_files(repo_path: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect_recursive(repo_path, &mut out);
    out.sort();
    out
}

fn collect_recursive(dir: &Path, out: &mut Vec<PathBuf>) {
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
            // on path components, but cheaper).
            if EXCLUDE_DIRS.contains(&name.as_ref()) {
                continue;
            }
            collect_recursive(&path, out);
        } else if file_type.is_file() {
            if !SOURCE_EXTENSIONS.contains(&suffix(&name)) {
                continue;
            }
            if is_test_filename(&name) {
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
}
