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

const SOURCE_EXTENSIONS: &[&str] = &[
    ".py", ".ts", ".tsx", ".js", ".jsx", ".go", ".rs", ".c", ".h", ".java", ".cs", ".php", ".cpp",
    ".cc", ".hpp", ".cxx", ".rb",
];

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
/// excluded directory component, drop test/spec files, drop paths the
/// user muted in `[exclude].paths` (user patterns only — the built-in
/// `argot:recommended` set governs calibration/check scope, not corpus
/// collection, so a repo without an `argot.toml` gets exactly the corpus
/// it always did), and drop gitignored-and-untracked paths (see [`GitScope`]).
/// Vendored trees (`repos/`, editor-history dirs, …) would otherwise attest
/// their own voice into the model.
///
/// The result is sorted for reproducibility. Python's `rglob` order is
/// filesystem-dependent (non-deterministic) and downstream consumers only
/// build order-independent counters, so sorting is a justified, score-neutral
/// divergence.
pub fn collect_source_files(repo_path: &Path) -> Vec<PathBuf> {
    let suppressions = crate::config::ArgotConfig::load(repo_path).path_suppressions();
    collect_source_files_with(repo_path, &suppressions)
}

/// [`collect_source_files`] against an already-resolved suppression set.
pub fn collect_source_files_with(
    repo_path: &Path,
    suppressions: &PathSuppressions,
) -> Vec<PathBuf> {
    let git_scope = GitScope::open(repo_path);
    let mut out = Vec::new();
    collect_recursive(
        repo_path,
        repo_path,
        suppressions,
        git_scope.as_ref(),
        &mut out,
    );
    out.sort();
    out
}

/// Git's own view of what belongs to the repo. A path that is gitignored and
/// not tracked — an editor's local-history tree, a `.worktrees/` checkout,
/// build output — is deliberately outside the repo and carries no authored
/// voice, so the corpus drops it. Tracked files always stay in, even when an
/// ignore pattern covers them (force-added wins, matching git's own rule).
/// On a plain directory with no `.git`, everything stays in.
struct GitScope {
    repo: git2::Repository,
    /// Sorted, `/`-separated, repo-relative tracked paths from the index.
    tracked: Vec<String>,
}

impl GitScope {
    fn open(root: &Path) -> Option<Self> {
        let repo = git2::Repository::open(root).ok()?;
        let index = repo.index().ok()?;
        let mut tracked: Vec<String> = index
            .iter()
            .map(|e| String::from_utf8_lossy(&e.path).into_owned())
            .collect();
        tracked.sort_unstable();
        Some(Self { repo, tracked })
    }

    fn ignored(&self, rel: &str) -> bool {
        self.repo.is_path_ignored(rel).unwrap_or(false)
    }

    /// File outside the repo per git: ignored and not in the index.
    fn excludes_file(&self, rel: &str) -> bool {
        self.ignored(rel)
            && self
                .tracked
                .binary_search_by(|p| p.as_str().cmp(rel))
                .is_err()
    }

    /// Directory prunable per git: ignored (checked with a trailing `/` so
    /// `dir/`-style patterns match) and holding no tracked file beneath it.
    fn excludes_dir(&self, rel: &str) -> bool {
        if !self.ignored(rel) && !self.ignored(&format!("{rel}/")) {
            return false;
        }
        let prefix = format!("{rel}/");
        let i = self
            .tracked
            .partition_point(|p| p.as_str() < prefix.as_str());
        !self.tracked.get(i).is_some_and(|p| p.starts_with(&prefix))
    }
}

/// True if `rel_path` (repo-root-relative, `/`-separated) is a corpus source
/// file — the per-path form of [`collect_source_files`]'s filter: a source
/// extension, not a test/spec file, no excluded-dir component, and not muted by a
/// user `.argotignore`/`[exclude].paths` pattern. Lets a caller classify a single
/// path (e.g. a changed file in a diff) without re-walking the tree.
pub fn is_corpus_source(rel_path: &str, suppressions: &PathSuppressions) -> bool {
    let file = rel_path.rsplit('/').next().unwrap_or(rel_path);
    if !SOURCE_EXTENSIONS.contains(&suffix(file)) || is_test_filename(file) {
        return false;
    }
    let comps: Vec<&str> = rel_path.split('/').collect();
    // any excluded-dir component (all but the basename) prunes the path
    if comps[..comps.len().saturating_sub(1)]
        .iter()
        .any(|c| EXCLUDE_DIRS.contains(c))
    {
        return false;
    }
    !suppressions.matches_user_pattern(rel_path)
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
    git_scope: Option<&GitScope>,
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
            if let (Some(scope), Some(rel)) = (git_scope, crate::suppress::rel_string(&path, root))
            {
                if scope.excludes_dir(&rel) {
                    continue;
                }
            }
            collect_recursive(&path, root, suppressions, git_scope, out);
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
            if let (Some(scope), Some(rel)) = (git_scope, crate::suppress::rel_string(&path, root))
            {
                if scope.excludes_file(&rel) {
                    continue;
                }
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
    fn is_corpus_source_matches_collector_filters() {
        let s = PathSuppressions::recommended();
        assert!(is_corpus_source("app/models/user.py", &s));
        assert!(is_corpus_source("src/widget.tsx", &s));
        // test files / test dirs / vendored dirs are not voice
        assert!(!is_corpus_source("tests/test_app.py", &s));
        assert!(!is_corpus_source("app/test_helpers.py", &s));
        assert!(!is_corpus_source("node_modules/pkg/index.ts", &s));
        assert!(!is_corpus_source("app/foo.test.ts", &s));
        // non-source extensions
        assert!(!is_corpus_source("README.md", &s));
        assert!(!is_corpus_source("app/data.json", &s));
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
    fn exclude_paths_prune_corpus() {
        let tmp = std::env::temp_dir().join(format!("argot_train_ignore_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::create_dir_all(tmp.join("repos/vendored")).unwrap();
        fs::create_dir_all(tmp.join("docs")).unwrap();
        fs::write(tmp.join("src/app.py"), "x=1").unwrap();
        fs::write(tmp.join("repos/vendored/lib.py"), "x=1").unwrap();
        fs::write(tmp.join("docs/example.py"), "x=1").unwrap();

        // Without argot.toml: repos/ and docs/ are in the corpus (train's
        // own exclude list never covered them).
        let before = collect_source_files(&tmp);
        assert_eq!(before.len(), 3);

        fs::write(tmp.join("argot.toml"), "[exclude]\npaths = [\"repos/\"]\n").unwrap();
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

    #[test]
    fn gitignored_untracked_paths_are_not_voice() {
        let tmp = std::env::temp_dir().join(format!("argot_train_gitscope_{}", std::process::id()));
        let _ = fs::remove_dir_all(&tmp);
        fs::create_dir_all(tmp.join("src")).unwrap();
        fs::create_dir_all(tmp.join(".history/src")).unwrap();
        fs::create_dir_all(tmp.join("gen")).unwrap();
        fs::write(tmp.join(".gitignore"), ".history/\ngen/\nlocal_*.py\n").unwrap();
        fs::write(tmp.join("src/app.py"), "x=1").unwrap();
        fs::write(tmp.join("src/local_notes.py"), "x=1").unwrap();
        fs::write(tmp.join(".history/src/app_20260101.py"), "x=1").unwrap();
        fs::write(tmp.join("gen/stub.py"), "x=1").unwrap();
        fs::write(tmp.join("gen/forced.py"), "x=1").unwrap();

        let repo = git2::Repository::init(&tmp).unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("src/app.py")).unwrap();
        // Force-added despite the ignore pattern: git considers it tracked,
        // so the corpus keeps it.
        index
            .add_frombuffer(
                &git2::IndexEntry {
                    ctime: git2::IndexTime::new(0, 0),
                    mtime: git2::IndexTime::new(0, 0),
                    dev: 0,
                    ino: 0,
                    mode: 0o100644,
                    uid: 0,
                    gid: 0,
                    file_size: 3,
                    id: git2::Oid::zero(),
                    flags: 0,
                    flags_extended: 0,
                    path: b"gen/forced.py".to_vec(),
                },
                b"x=1",
            )
            .unwrap();
        index.write().unwrap();

        let names: Vec<String> = collect_source_files(&tmp)
            .iter()
            .map(|p| crate::suppress::rel_string(p, &tmp).unwrap())
            .collect();
        assert!(names.contains(&"src/app.py".to_string()));
        assert!(
            names.contains(&"gen/forced.py".to_string()),
            "tracked file survives an ignore pattern: {names:?}"
        );
        assert!(
            !names.iter().any(|n| n.starts_with(".history/")),
            "gitignored editor-history tree pruned: {names:?}"
        );
        assert!(
            !names.contains(&"gen/stub.py".to_string()),
            "ignored+untracked sibling of a tracked file dropped: {names:?}"
        );
        assert!(
            !names.contains(&"src/local_notes.py".to_string()),
            "ignored+untracked file dropped: {names:?}"
        );
        let _ = fs::remove_dir_all(&tmp);
    }
}
