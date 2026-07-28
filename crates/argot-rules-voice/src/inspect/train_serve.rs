//! Is the voice learned from the code that actually changes?
//!
//! A model fitted on one part of a repository and applied to another is
//! mis-calibrated in the way nobody notices: it does not error, it just judges
//! unfamiliar code against a vocabulary it never saw. uos is the worked
//! example — `examples/` is 63 files / 46 344 lines against `src/` at 21 files
//! / 25 994, so **64 % of what it learned was demo code**, and 141 of the 145
//! false alarms in a replayed window landed in `src/`, the library the demos
//! merely call. It took a benchmark outlier and a manual investigation to find.
//! Nothing in argot said a word.
//!
//! The measurement needs no findings, so it runs at fit: compare each
//! top-level directory's share of the **corpus** (what shapes the voice)
//! against its share of recent **churn** (what gets written, and therefore
//! reviewed). A directory that dominates the corpus and barely changes is
//! teaching a voice nobody is judged against.

use std::collections::BTreeMap;
use std::ops::ControlFlow;
use std::path::Path;

/// One top-level directory's share of the voice against its share of the work.
#[derive(serde::Serialize, Debug, Clone, PartialEq)]
pub struct DirectoryMix {
    /// Top-level directory, or `"."` for files at the repo root.
    pub dir: String,
    pub corpus_files: usize,
    /// Share of the fit corpus, 0.0–1.0.
    pub corpus_share: f64,
    pub changed_files: usize,
    /// Share of recent changed-file events, 0.0–1.0.
    pub churn_share: f64,
}

/// How far back to read churn. Long enough to cover a quiet directory's normal
/// rhythm, short enough to describe how the repository is worked on now.
const CHURN_COMMITS: usize = 400;

/// A directory has to shape at least this much of the voice before its churn
/// matters — below it, a mismatch cannot move the model much either way.
const MIN_CORPUS_SHARE: f64 = 0.25;

/// …and it is only a mismatch when its share of the work is this much smaller
/// than its share of the voice. Three-to-one is deliberately conservative: a
/// stable, rarely-touched core is normal and must not be reported.
const MISMATCH_RATIO: f64 = 3.0;

/// The top-level directory of a repo-relative path (`"."` at the root).
fn top_level(path: &str) -> &str {
    match path.split_once('/') {
        Some((head, _)) if !head.is_empty() => head,
        _ => ".",
    }
}

/// Corpus and churn shares per top-level directory, largest corpus share
/// first. `corpus_rel` is the fit corpus as repo-relative `/`-separated paths.
///
/// Churn counts one event per changed file per commit, over the last
/// [`CHURN_COMMITS`] single-parent commits — the same population `check`
/// eventually scores. Returns an empty vec when the history yields nothing to
/// compare against, which is a young repo rather than a finding.
pub fn directory_mix(repo_dir: &Path, corpus_rel: &[String]) -> Vec<DirectoryMix> {
    let mut corpus: BTreeMap<String, usize> = BTreeMap::new();
    for path in corpus_rel {
        *corpus.entry(top_level(path).to_string()).or_default() += 1;
    }
    let corpus_total: usize = corpus.values().sum();
    if corpus_total == 0 {
        return Vec::new();
    }

    // Churn is counted over exactly the files the corpus holds today. Anything
    // else makes the two shares incomparable, and both failure modes are real:
    // measured on argot itself, an unrestricted walk put 44 % of the churn in
    // `benchmarks/` (excluded from the voice) and another 31 % in `engine/` and
    // `cli/` — **directories the Rust port deleted**. A 400-commit window
    // reaches into layouts that no longer exist, and `crates/`, the entire
    // source tree, reported itself as a mismatch at 91 % of the voice against
    // 22 % of the work.
    //
    // A file changed inside the window and since renamed away therefore does
    // not count, which is correct: the question is how the code that exists now
    // is worked on.
    let corpus_paths: std::collections::HashSet<&str> =
        corpus_rel.iter().map(String::as_str).collect();
    let mut churn: BTreeMap<String, usize> = BTreeMap::new();
    let mut seen_commits = 0usize;
    let mut last_commit = String::new();
    let _ = argot_engine::git_walk::walk_repo(&repo_dir.display().to_string(), |item| {
        if item.commit_id != last_commit {
            last_commit = item.commit_id.clone();
            seen_commits += 1;
            if seen_commits > CHURN_COMMITS {
                return Ok(ControlFlow::Break(()));
            }
        }
        if corpus_paths.contains(item.file_path.as_str()) {
            *churn
                .entry(top_level(&item.file_path).to_string())
                .or_default() += 1;
        }
        Ok(ControlFlow::Continue(()))
    });
    let churn_total: usize = churn.values().sum();
    if churn_total == 0 {
        return Vec::new();
    }

    let mut out: Vec<DirectoryMix> = corpus
        .into_iter()
        .map(|(dir, corpus_files)| {
            let changed_files = churn.get(&dir).copied().unwrap_or(0);
            DirectoryMix {
                corpus_share: corpus_files as f64 / corpus_total as f64,
                churn_share: changed_files as f64 / churn_total as f64,
                dir,
                corpus_files,
                changed_files,
            }
        })
        .collect();
    out.sort_by(|a, b| {
        b.corpus_share
            .partial_cmp(&a.corpus_share)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.dir.cmp(&b.dir))
    });
    out
}

/// The directories teaching a voice the repository is not judged against —
/// a large share of the corpus, a much smaller share of the work.
pub fn mismatched(mix: &[DirectoryMix]) -> Vec<&DirectoryMix> {
    mix.iter()
        .filter(|d| {
            d.corpus_share >= MIN_CORPUS_SHARE && d.churn_share * MISMATCH_RATIO < d.corpus_share
        })
        .collect()
}

/// Human-readable note for one mismatched directory.
pub fn describe(d: &DirectoryMix) -> String {
    format!(
        "{}/ shapes {:.0}% of the voice but takes {:.0}% of recent changes \
         ({} corpus files, {} changed). A model learned from code nobody edits \
         judges the code everybody does — if this tree is demos, vendored, or \
         generated, exclude it in argot.toml [exclude].paths.",
        d.dir,
        d.corpus_share * 100.0,
        d.churn_share * 100.0,
        d.corpus_files,
        d.changed_files,
    )
}

#[cfg(test)]
mod tests;
