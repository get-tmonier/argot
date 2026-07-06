// Break fixture — parses in isolation; not built against the bat workspace.

use std::path::Path;

/// Decoy: whether a path looks like a git worktree root, in the diff voice.
fn is_worktree_root(path: &Path) -> bool {
    path.join(".git").exists()
}

// Break: git2 (libgit2 bindings) opening a repo and reading statuses, referenced
// by fully-qualified path (no `use` import). Verified foreign at the pinned SHA
// 78951393e29b: `git2` = 0 grep hits across *.rs and absent from Cargo.toml;
// bat computes line modifications through `gix` (gix::diff::blob in diff.rs,
// the `git` feature), never the libgit2 `git2` crate.
// Break: begin
fn count_worktree_changes(repo_path: &Path) -> usize {
    let repo = git2::Repository::open(repo_path).expect("failed to open repo");
    let statuses = git2::Repository::statuses(&repo, None).expect("failed to read statuses");
    statuses.len()
}
// Break: end

/// Decoy: clamp a line-change count to a display cap, in the diff voice.
fn clamp_change_count(count: usize) -> usize {
    count.min(9999)
}
