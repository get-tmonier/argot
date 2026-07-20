//! Two-sided changeset collection — both sides of every changed file
//! (renames resolved), **including deletions**, which the scoring
//! `PatchBatch` path never carries. Mirrors `collect_patches`' mode
//! dispatch; an explicit commit set yields one changeset per commit so
//! per-accepted-unit reasoning (the integrity pass) sees each unit
//! separately. Every changeset is labelled with its display source
//! (`workdir` / `staged` / short SHA). Consumers: the integrity pass and
//! the scripted rules' `old_text`/`ts_query_old` host calls.

use super::CheckArgs;
use crate::git_walk::{open_repo, resolve_shas};
use git2::DiffFindOptions;
use std::collections::HashSet;
use std::fs;
use std::path::Path;

/// One side-pair of a changed file (rename-resolved: a renamed file arrives
/// as one change with both sides, not delete + add).
#[derive(Debug, Clone)]
pub struct FileChange {
    /// Repo-relative path (post path; pre path when deleted).
    pub path: String,
    /// Pre-image text (`None` = added file).
    pub old: Option<String>,
    /// Post-image text (`None` = deleted file).
    pub new: Option<String>,
}

/// Two-sided changesets for a check invocation. `keep` scopes files by
/// repo-relative path BEFORE any blob read, so callers never pay for
/// content they would drop (the integrity pass keeps only paths whose
/// language has a test inventory).
pub fn collect_two_sided(
    args: &CheckArgs,
    keep: &dyn Fn(&str) -> bool,
) -> Vec<(String, Vec<FileChange>)> {
    collect_two_sided_impl(args, keep, false)
}

/// Like [`collect_two_sided`] but a `base..head` range yields one changeset per
/// commit (each diffed against its own parent), so per-accepted-unit reasoning
/// — the integrity pass's "did *this* commit also touch production?" gate —
/// never lets one commit's production edit satisfy the gate for another
/// commit's tests-only change. This is the audit-window false positive: the
/// aggregate range diff unions ~50 commits, so a production change anywhere in
/// the window looks co-changed with a test deletion anywhere else in it.
pub fn collect_two_sided_per_commit(
    args: &CheckArgs,
    keep: &dyn Fn(&str) -> bool,
) -> Vec<(String, Vec<FileChange>)> {
    collect_two_sided_impl(args, keep, true)
}

fn collect_two_sided_impl(
    args: &CheckArgs,
    keep: &dyn Fn(&str) -> bool,
    split_ranges: bool,
) -> Vec<(String, Vec<FileChange>)> {
    const MAX_BLOB: usize = 400_000;

    fn tree_text(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
        let entry = tree.get_path(Path::new(path)).ok()?;
        let blob = repo.find_blob(entry.id()).ok()?;
        (blob.size() <= MAX_BLOB).then(|| String::from_utf8_lossy(blob.content()).to_string())
    }
    fn workdir_text(repo: &git2::Repository, path: &str) -> Option<String> {
        let full = repo.workdir()?.join(path);
        let data = fs::read(&full).ok()?;
        (data.len() <= MAX_BLOB).then(|| String::from_utf8_lossy(&data).to_string())
    }
    fn index_text(repo: &git2::Repository, path: &str) -> Option<String> {
        let index = repo.index().ok()?;
        let entry = index.get_path(Path::new(path), 0)?;
        let blob = repo.find_blob(entry.id).ok()?;
        (blob.size() <= MAX_BLOB).then(|| String::from_utf8_lossy(blob.content()).to_string())
    }
    fn changes_from_diff(
        diff: &mut git2::Diff,
        keep: &dyn Fn(&str) -> bool,
        old_side: &dyn Fn(&str) -> Option<String>,
        new_side: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<FileChange> {
        let _ = diff.find_similar(Some(&mut DiffFindOptions::new()));
        let mut out = Vec::new();
        for d in diff.deltas() {
            let new_path = d
                .new_file()
                .path()
                .and_then(|p| p.to_str())
                .map(str::to_string);
            let old_path = d
                .old_file()
                .path()
                .and_then(|p| p.to_str())
                .map(str::to_string);
            let path = new_path
                .clone()
                .or_else(|| old_path.clone())
                .unwrap_or_default();
            if !keep(&path) {
                continue;
            }
            let old = match d.status() {
                git2::Delta::Added | git2::Delta::Untracked => None,
                _ => old_path.as_deref().and_then(old_side),
            };
            let new = match d.status() {
                git2::Delta::Deleted => None,
                _ => new_path.as_deref().and_then(new_side),
            };
            if old.is_none() && new.is_none() {
                continue;
            }
            out.push(FileChange { path, old, new });
        }
        out
    }
    fn one(source: &str, cs: Vec<FileChange>) -> Vec<(String, Vec<FileChange>)> {
        if cs.is_empty() {
            Vec::new()
        } else {
            vec![(source.to_string(), cs)]
        }
    }

    let repo_path = args.repo_path.as_str();
    let Ok(repo) = open_repo(repo_path) else {
        return Vec::new();
    };
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    let ref_nonempty = !args.reference.is_empty();

    let per_commit = |shas: &HashSet<String>| -> Vec<(String, Vec<FileChange>)> {
        let mut out = Vec::new();
        for sha in shas {
            let Ok(oid) = git2::Oid::from_str(sha) else {
                continue;
            };
            let Ok(commit) = repo.find_commit(oid) else {
                continue;
            };
            if commit.parent_count() != 1 {
                continue;
            }
            let Ok(parent_tree) = commit.parent(0).and_then(|p| p.tree()) else {
                continue;
            };
            let Ok(tree) = commit.tree() else {
                continue;
            };
            let Ok(mut diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) else {
                continue;
            };
            let cs = changes_from_diff(
                &mut diff,
                keep,
                &|p| tree_text(&repo, &parent_tree, p),
                &|p| tree_text(&repo, &tree, p),
            );
            if !cs.is_empty() {
                let short: String = sha.chars().take(7).collect();
                out.push((short, cs));
            }
        }
        out
    };

    if commit_set {
        let Ok(shas) = resolve_shas(&repo, args.commit.as_deref().unwrap_or_default()) else {
            return Vec::new();
        };
        return per_commit(&shas);
    }
    if ref_nonempty {
        let reference = args.reference.as_str();
        if let Some((base_raw, head_raw)) = reference.split_once("..") {
            let base = if base_raw.is_empty() {
                "HEAD"
            } else {
                base_raw
            };
            let head_trimmed = head_raw.trim_start_matches('.');
            let head = if head_trimmed.is_empty() {
                "HEAD"
            } else {
                head_trimmed
            };
            let Ok(base_c) = repo.revparse_single(base).and_then(|o| o.peel_to_commit()) else {
                return Vec::new();
            };
            let Ok(head_c) = repo.revparse_single(head).and_then(|o| o.peel_to_commit()) else {
                return Vec::new();
            };
            let base_id = repo
                .merge_base(base_c.id(), head_c.id())
                .unwrap_or_else(|_| base_c.id());
            if split_ranges {
                // Per accepted unit: enumerate the range's commits and diff each
                // against its own parent (via `per_commit`), instead of one
                // aggregate diff over the whole window.
                let mut shas: HashSet<String> = HashSet::new();
                if let Ok(mut walk) = repo.revwalk() {
                    if walk.push(head_c.id()).is_ok() {
                        let _ = walk.hide(base_id);
                        for oid in walk.flatten() {
                            shas.insert(oid.to_string());
                        }
                    }
                }
                return per_commit(&shas);
            }
            let Ok(base_tree) = repo.find_commit(base_id).and_then(|c| c.tree()) else {
                return Vec::new();
            };
            let Ok(head_tree) = head_c.tree() else {
                return Vec::new();
            };
            let Ok(mut diff) = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
            else {
                return Vec::new();
            };
            let short: String = head_c.id().to_string().chars().take(7).collect();
            let cs = changes_from_diff(
                &mut diff,
                keep,
                &|p| tree_text(&repo, &base_tree, p),
                &|p| tree_text(&repo, &head_tree, p),
            );
            return one(&short, cs);
        }
        // Bare ref: the net view merge-base(ref, HEAD) → working tree.
        let Ok(base_c) = repo
            .revparse_single(reference)
            .and_then(|o| o.peel_to_commit())
        else {
            return Vec::new();
        };
        let base_id = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|h| repo.merge_base(base_c.id(), h).ok())
            .unwrap_or_else(|| base_c.id());
        let Ok(base_tree) = repo.find_commit(base_id).and_then(|c| c.tree()) else {
            return Vec::new();
        };
        let mut opts = git2::DiffOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut opts))
        else {
            return Vec::new();
        };
        let cs = changes_from_diff(
            &mut diff,
            keep,
            &|p| tree_text(&repo, &base_tree, p),
            &|p| workdir_text(&repo, p),
        );
        return one("workdir", cs);
    }
    if args.staged {
        let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) else {
            return Vec::new();
        };
        let Ok(index) = repo.index() else {
            return Vec::new();
        };
        let Ok(mut diff) = repo.diff_tree_to_index(Some(&head_tree), Some(&index), None) else {
            return Vec::new();
        };
        let cs = changes_from_diff(
            &mut diff,
            keep,
            &|p| tree_text(&repo, &head_tree, p),
            &|p| index_text(&repo, p),
        );
        return one("staged", cs);
    }
    if args.unstaged {
        let Ok(index) = repo.index() else {
            return Vec::new();
        };
        let Ok(mut diff) = repo.diff_index_to_workdir(Some(&index), None) else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, keep, &|p| index_text(&repo, p), &|p| {
            workdir_text(&repo, p)
        });
        return one("workdir", cs);
    }
    let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) else {
        return Vec::new();
    };
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))
    else {
        return Vec::new();
    };
    let cs = changes_from_diff(
        &mut diff,
        keep,
        &|p| tree_text(&repo, &head_tree, p),
        &|p| workdir_text(&repo, p),
    );
    one("workdir", cs)
}
