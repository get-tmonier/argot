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
pub const SUPPORTED_EXTENSIONS: &[&str] = &[".ts", ".tsx", ".js", ".jsx", ".py", ".php"];

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
