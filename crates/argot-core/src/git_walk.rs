//! Git history walk — port of `engine/argot/git_walk.py`.
//!
//! Uses `git2` (libgit2 bindings) — the same C library `pygit2` binds — so
//! the diff, hunk boundaries, and `find_similar` rename detection match the
//! Python engine exactly. The revwalk uses libgit2's topological sort, giving
//! the identical commit visitation order.

use anyhow::{Context, Result};
use git2::{DiffFindOptions, Oid, Patch, Repository, RepositoryOpenFlags, Sort};
use std::collections::HashSet;
use std::ffi::OsStr;
use std::ops::ControlFlow;
use std::path::Path;

/// Open a repository the way `pygit2.Repository(path)` does: default flags
/// (`git_repository_open_ext(path, 0, NULL)`), which searches parent
/// directories for a `.git`. git2's plain `Repository::open` does NOT search,
/// so this centralised opener is required for parity.
pub fn open_repo(path: &str) -> std::result::Result<Repository, git2::Error> {
    Repository::open_ext(
        path,
        RepositoryOpenFlags::empty(),
        std::iter::empty::<&OsStr>(),
    )
}

/// Extensions the extractor considers, matching `SUPPORTED_EXTENSIONS`.
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".go", ".rs"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".c", ".h"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".java"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".cs"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".php"];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[
    ".ts", ".tsx", ".js", ".jsx", ".py", ".cpp", ".cc", ".hpp", ".cxx",
];
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".rb"];

/// A hunk's post-image span, as reported by the diff (1-indexed start).
#[derive(Debug, Clone, Copy)]
pub struct HunkSpan {
    pub new_start: u32,
    pub new_lines: u32,
}

/// One `(commit, file, post-blob, hunks)` yield of the walk. Commit-level
/// fields are duplicated per changed file, mirroring the Python generator.
#[derive(Debug, Clone)]
pub struct WalkItem {
    pub commit_id: String,
    /// First parent id. Always `Some` here: the walk only visits commits with
    /// exactly one parent.
    pub parent_id: Option<String>,
    /// `commit.author.time` — unix seconds.
    pub author_time: i64,
    pub file_path: String,
    pub post_blob: Vec<u8>,
    pub hunks: Vec<HunkSpan>,
}

/// Whether `path` opens as a git repository — mirrors the guard
/// `pygit2.Repository(path)` raising `GitError` (with parent search).
pub fn repo_exists(path: &str) -> bool {
    open_repo(path).is_ok()
}

/// Current HEAD commit SHA, or `None` if unresolvable (mirrors the calibrate
/// `repo_sha` fallback to `"unknown"`).
pub fn head_sha(path: &str) -> Option<String> {
    let repo = open_repo(path).ok()?;
    let head = repo.head().ok()?;
    head.target().map(|o| o.to_string())
}

/// The working-tree root of the repository containing `path` (in-process via
/// libgit2, so it needs no external `git` binary on any platform). `None` for a
/// bare repo or when `path` isn't inside a repository. The trailing separator
/// libgit2 appends is stripped so the result matches `git rev-parse
/// --show-toplevel`.
pub fn repo_workdir(path: &str) -> Option<String> {
    let repo = open_repo(path).ok()?;
    let workdir = repo.workdir()?;
    let s = workdir.to_string_lossy();
    Some(s.trim_end_matches(['/', '\\']).to_string())
}

fn is_supported_ext(path: &str) -> bool {
    let name = match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    };
    match name.rfind('.') {
        Some(i) => {
            let ext = name[i..].to_ascii_lowercase();
            SUPPORTED_EXTENSIONS.contains(&ext.as_str())
        }
        None => false,
    }
}

fn resolve_start_oid(repo: &Repository) -> Result<Option<Oid>> {
    match repo.head() {
        Ok(head) => {
            if let Some(t) = head.target() {
                Ok(Some(t))
            } else {
                Ok(Some(head.peel_to_commit()?.id()))
            }
        }
        Err(_) => {
            // No resolvable HEAD (e.g. unborn): fall back to the first branch.
            for r in repo.references_glob("refs/heads/*")? {
                let r = r?;
                if let Some(t) = r.target() {
                    return Ok(Some(t));
                }
            }
            Ok(None)
        }
    }
}

/// Parse a git range (`A..B` or a bare ref) into a set of commit SHAs — port
/// of `_resolve_shas`.
///
/// `A..B` → commits reachable from B but not A. Bare `ref` → `ref^..ref`.
pub fn resolve_shas(repo: &Repository, reference: &str) -> Result<HashSet<String>> {
    let (start_ref, end_ref) = match reference.find("..") {
        Some(idx) => (
            reference[..idx].to_string(),
            reference[idx + 2..].to_string(),
        ),
        None => (format!("{reference}^"), reference.to_string()),
    };
    let end_oid = repo.revparse_single(&end_ref)?.id();
    let start_oid = repo.revparse_single(&start_ref).ok().map(|o| o.id());

    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL)?;
    walk.push(end_oid)?;

    let mut shas = HashSet::new();
    for oid in walk {
        let oid = oid?;
        if Some(oid) == start_oid {
            break;
        }
        shas.insert(oid.to_string());
    }
    Ok(shas)
}

fn walk_impl<F>(repo_path: &str, filter: Option<&HashSet<String>>, mut visit: F) -> Result<()>
where
    F: FnMut(WalkItem) -> Result<ControlFlow<()>>,
{
    let repo = open_repo(repo_path).with_context(|| format!("open repo {repo_path}"))?;
    if repo.is_empty()? {
        return Ok(());
    }
    let start = match resolve_start_oid(&repo)? {
        Some(o) => o,
        None => return Ok(()),
    };

    let mut walk = repo.revwalk()?;
    walk.set_sorting(Sort::TOPOLOGICAL)?;
    walk.push(start)?;

    for oid in walk {
        let oid = oid?;
        if let Some(f) = filter {
            if !f.contains(&oid.to_string()) {
                continue;
            }
        }
        let commit = repo.find_commit(oid)?;
        if commit.parent_count() != 1 {
            // Skip merge and root commits.
            continue;
        }
        let parent = commit.parent(0)?;
        let parent_tree = parent.tree()?;
        let commit_tree = commit.tree()?;

        let mut diff = repo.diff_tree_to_tree(Some(&parent_tree), Some(&commit_tree), None)?;
        // Default find options → GIT_DIFF_FIND_BY_CONFIG, matching pygit2's
        // `diff.find_similar()` with no args.
        let mut find_opts = DiffFindOptions::new();
        diff.find_similar(Some(&mut find_opts))?;

        let ndeltas = diff.deltas().len();
        for idx in 0..ndeltas {
            let delta = match diff.get_delta(idx) {
                Some(d) => d,
                None => continue,
            };
            let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
                Some(p) => p.to_string(),
                None => continue,
            };
            if !is_supported_ext(&file_path) {
                continue;
            }

            let patch = match Patch::from_diff(&diff, idx)? {
                Some(p) => p,
                None => continue,
            };
            let nhunks = patch.num_hunks();
            if nhunks == 0 {
                continue;
            }
            let mut hunks = Vec::with_capacity(nhunks);
            for h in 0..nhunks {
                let (hunk, _lines) = patch.hunk(h)?;
                hunks.push(HunkSpan {
                    new_start: hunk.new_start(),
                    new_lines: hunk.new_lines(),
                });
            }

            // Post-image blob from the commit's tree; skip if the path is
            // absent (deleted) or not a blob — matching Python's KeyError skip.
            let entry = match commit_tree.get_path(Path::new(&file_path)) {
                Ok(e) => e,
                Err(_) => continue,
            };
            let obj = entry.to_object(&repo)?;
            let blob = match obj.as_blob() {
                Some(b) => b,
                None => continue,
            };
            let post_blob = blob.content().to_vec();

            let item = WalkItem {
                commit_id: oid.to_string(),
                parent_id: Some(parent.id().to_string()),
                author_time: commit.author().when().seconds(),
                file_path,
                post_blob,
                hunks,
            };
            if let ControlFlow::Break(()) = visit(item)? {
                return Ok(());
            }
        }
    }
    Ok(())
}

/// Walk every single-parent commit reachable from HEAD (`walk_repo`), calling
/// `visit` per changed supported file. `visit` returns `ControlFlow::Break`
/// to stop early.
pub fn walk_repo<F>(repo_path: &str, visit: F) -> Result<()>
where
    F: FnMut(WalkItem) -> Result<ControlFlow<()>>,
{
    walk_impl(repo_path, None, visit)
}

/// Walk only the commits whose SHA is in `shas` (`walk_commits`).
pub fn walk_commits<F>(repo_path: &str, shas: &HashSet<String>, visit: F) -> Result<()>
where
    F: FnMut(WalkItem) -> Result<ControlFlow<()>>,
{
    walk_impl(repo_path, Some(shas), visit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::process::Command;

    /// A throwaway git repo for exercising the walker end-to-end. Cleans up
    /// its scratch directory on drop (including on test panic).
    struct TempRepo {
        dir: PathBuf,
    }

    impl TempRepo {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir()
                .join(format!("argot_git_walk_test_{}_{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).expect("create scratch dir");
            let repo = Self { dir };
            repo.git(&["init", "-q"]);
            repo.git(&["config", "user.email", "test@example.com"]);
            repo.git(&["config", "user.name", "Test"]);
            repo
        }

        fn git(&self, args: &[&str]) -> String {
            let out = Command::new("git")
                .args(args)
                .current_dir(&self.dir)
                .output()
                .expect("run git");
            assert!(
                out.status.success(),
                "git {:?} failed: {}",
                args,
                String::from_utf8_lossy(&out.stderr)
            );
            String::from_utf8(out.stdout).unwrap().trim().to_string()
        }

        fn write(&self, rel: &str, contents: &str) {
            let p = self.dir.join(rel);
            if let Some(parent) = p.parent() {
                std::fs::create_dir_all(parent).unwrap();
            }
            std::fs::write(p, contents).unwrap();
        }

        fn commit_all(&self, message: &str) -> String {
            self.git(&["add", "-A"]);
            self.git(&["commit", "-q", "-m", message]);
            self.git(&["rev-parse", "HEAD"])
        }

        fn path(&self) -> &str {
            self.dir.to_str().unwrap()
        }
    }

    impl Drop for TempRepo {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.dir);
        }
    }

    #[test]
    fn is_supported_ext_matches_the_documented_extensions_case_insensitively() {
        assert!(is_supported_ext("src/app.py"));
        assert!(is_supported_ext("app.PY"));
        assert!(is_supported_ext("component.tsx"));
        assert!(!is_supported_ext("README.md"));
        assert!(!is_supported_ext("Makefile"));
        assert!(
            !is_supported_ext("dir.py/no_ext"),
            "extension check is basename-only"
        );
    }

    #[test]
    fn repo_workdir_is_the_worktree_root_without_a_trailing_separator() {
        let repo = TempRepo::new("workdir");
        let workdir = repo_workdir(repo.path()).expect("inside a repo");
        // libgit2 appends a trailing separator; repo_workdir must strip it so
        // the result matches `git rev-parse --show-toplevel`.
        assert!(!workdir.ends_with('/') && !workdir.ends_with('\\'));
        // Same canonical directory as the repo (compare via metadata identity
        // to sidestep symlinked temp roots, e.g. macOS /var → /private/var).
        let a = std::fs::canonicalize(&workdir).unwrap();
        let b = std::fs::canonicalize(repo.path()).unwrap();
        assert_eq!(a, b);
        // Not a repo → None.
        let plain = std::env::temp_dir();
        // temp_dir itself may sit inside a repo on some CI images; assert the
        // positive case above and that a non-repo path also resolves cleanly.
        let _ = repo_workdir(plain.to_str().unwrap());
    }

    #[test]
    fn repo_exists_is_true_for_a_git_repo_and_false_for_a_plain_directory() {
        let repo = TempRepo::new("repo_exists");
        assert!(repo_exists(repo.path()));

        let not_a_repo = std::env::temp_dir().join(format!(
            "argot_git_walk_test_{}_not_a_repo",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&not_a_repo);
        std::fs::create_dir_all(&not_a_repo).unwrap();
        assert!(!repo_exists(not_a_repo.to_str().unwrap()));
        let _ = std::fs::remove_dir_all(&not_a_repo);
    }

    #[test]
    fn head_sha_is_none_before_the_first_commit_and_some_after() {
        let repo = TempRepo::new("head_sha");
        assert_eq!(head_sha(repo.path()), None, "unborn HEAD has no target");

        repo.write("a.py", "x = 1\n");
        let sha = repo.commit_all("first");
        assert_eq!(head_sha(repo.path()), Some(sha));
    }

    #[test]
    fn resolve_shas_returns_commits_reachable_from_end_excluding_start() {
        let repo = TempRepo::new("resolve_shas_range");
        repo.write("a.py", "x = 1\n");
        let c1 = repo.commit_all("c1");
        repo.write("a.py", "x = 2\n");
        let c2 = repo.commit_all("c2");
        repo.write("a.py", "x = 3\n");
        let c3 = repo.commit_all("c3");

        let git_repo = open_repo(repo.path()).unwrap();
        let shas = resolve_shas(&git_repo, &format!("{c1}..{c3}")).unwrap();
        assert_eq!(shas, HashSet::from([c2, c3]));
        assert!(!shas.contains(&c1), "the start ref itself is excluded");
    }

    #[test]
    fn resolve_shas_bare_ref_means_just_that_commit() {
        let repo = TempRepo::new("resolve_shas_bare");
        repo.write("a.py", "x = 1\n");
        repo.commit_all("c1");
        repo.write("a.py", "x = 2\n");
        let c2 = repo.commit_all("c2");

        let git_repo = open_repo(repo.path()).unwrap();
        let shas = resolve_shas(&git_repo, "HEAD").unwrap();
        assert_eq!(shas, HashSet::from([c2]));
    }

    #[test]
    fn walk_repo_skips_the_root_commit_and_visits_only_single_parent_commits() {
        let repo = TempRepo::new("walk_repo_root_skip");
        repo.write("a.py", "x = 1\n");
        repo.commit_all("root"); // parent_count == 0 -> must be skipped
        repo.write("a.py", "x = 2\n");
        let c2 = repo.commit_all("second");

        let mut visited_commits = HashSet::new();
        walk_repo(repo.path(), |item| {
            visited_commits.insert(item.commit_id.clone());
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();

        assert_eq!(visited_commits, HashSet::from([c2]));
    }

    #[test]
    fn walk_repo_only_reports_hunks_for_supported_extensions() {
        let repo = TempRepo::new("walk_repo_ext_filter");
        repo.write("a.py", "x = 1\n");
        repo.write("README.md", "hello\n");
        repo.commit_all("root");
        repo.write("a.py", "x = 2\n");
        repo.write("README.md", "hello world\n");
        repo.commit_all("second");

        let mut files = HashSet::new();
        walk_repo(repo.path(), |item| {
            files.insert(item.file_path.clone());
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();

        assert_eq!(
            files,
            HashSet::from(["a.py".to_string()]),
            "README.md's extension is not tracked"
        );
    }

    #[test]
    fn walk_repo_reports_the_post_image_blob_and_a_plausible_hunk_span() {
        let repo = TempRepo::new("walk_repo_hunk_shape");
        repo.write("a.py", "line1\nline2\nline3\n");
        repo.commit_all("root");
        repo.write("a.py", "line1\nline2 changed\nline3\n");
        repo.commit_all("second");

        let mut seen = Vec::new();
        walk_repo(repo.path(), |item| {
            seen.push(item);
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();

        assert_eq!(seen.len(), 1);
        let item = &seen[0];
        assert_eq!(item.file_path, "a.py");
        assert_eq!(
            String::from_utf8_lossy(&item.post_blob),
            "line1\nline2 changed\nline3\n"
        );
        assert_eq!(item.hunks.len(), 1);
        assert!(item.hunks[0].new_lines >= 1);
        assert!(item.hunks[0].new_start >= 1);
        assert!(item.parent_id.is_some());
    }

    #[test]
    fn walk_repo_visit_can_stop_the_walk_early() {
        let repo = TempRepo::new("walk_repo_early_stop");
        repo.write("a.py", "x = 1\n");
        repo.commit_all("root");
        repo.write("a.py", "x = 2\n");
        repo.commit_all("second");
        repo.write("a.py", "x = 3\n");
        repo.commit_all("third");

        let mut count = 0;
        walk_repo(repo.path(), |_item| {
            count += 1;
            Ok(ControlFlow::Break(()))
        })
        .unwrap();

        assert_eq!(count, 1, "must stop after the first visited item");
    }

    #[test]
    fn walk_commits_only_visits_commits_in_the_given_set() {
        let repo = TempRepo::new("walk_commits_filter");
        repo.write("a.py", "x = 1\n");
        repo.commit_all("root");
        repo.write("a.py", "x = 2\n");
        let c2 = repo.commit_all("second");
        repo.write("a.py", "x = 3\n");
        repo.commit_all("third");

        let filter: HashSet<String> = HashSet::from([c2.clone()]);
        let mut visited = HashSet::new();
        walk_commits(repo.path(), &filter, |item| {
            visited.insert(item.commit_id.clone());
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();

        assert_eq!(visited, HashSet::from([c2]));
    }

    #[test]
    fn walk_repo_on_an_empty_repo_visits_nothing() {
        let repo = TempRepo::new("walk_repo_empty");
        let mut count = 0;
        walk_repo(repo.path(), |_item| {
            count += 1;
            Ok(ControlFlow::Continue(()))
        })
        .unwrap();
        assert_eq!(count, 0);
    }
}
