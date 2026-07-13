//! `train` (a.k.a. `fit`) — port of `engine/argot/train.py`.
//!
//! Collects the repo's production source files into `repo-corpus.txt` and
//! emits the pre-baked BPE generic baseline. There is no model training here;
//! "train" is corpus collection + baseline copy.
//!
//! The corpus-walk machinery itself (`collect_source_files` / `is_corpus_source`
//! and their private helpers) lives in `argot_engine::corpus` — shared with the
//! engine's check-time freshness scan — and is re-exported here at its
//! historical `argot_core::train::` path.

use anyhow::{bail, Result};
use std::fs;
use std::path::Path;

pub use argot_engine::corpus::{collect_source_files, collect_source_files_with, is_corpus_source};

/// The generic BPE baseline (`generic_tokens_bpe.json`), embedded so the
/// binary is self-contained (Python `train.py` copies this file).
pub const GENERIC_BASELINE_JSON: &[u8] = include_bytes!("../data/generic_tokens_bpe.json");

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
        bail!(
            "no source files found in repository — every candidate file is \
             unsupported, gitignored, or excluded by argot.toml [exclude] patterns"
        );
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
