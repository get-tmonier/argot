//! Throwaway `git worktree` checkouts — shared by `replay` (fit the voice as
//! of a past commit) and the background auto-refresh (fit at the accepted
//! anchor so unmerged or uncommitted work never trains the voice).

use std::path::{Path, PathBuf};
use std::process::Command;

/// `git -C <repo> <args>` — worktree management shells out to the git CLI
/// (the one git feature libgit2 makes harder than the porcelain).
pub fn git(repo: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("git not found on PATH ({e}) — this step needs the git CLI"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {} failed", args.join(" ")))
    }
}

/// A temp worktree that cleans up after itself (best-effort).
pub struct TempWorktree {
    repo: PathBuf,
    pub path: PathBuf,
}

impl TempWorktree {
    /// Detached checkout of `sha` under the OS temp dir, named
    /// `<prefix>-<pid>` so concurrent argot processes can't collide.
    pub fn create(repo: &Path, sha: &str, prefix: &str) -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        git(
            repo,
            &["worktree", "add", "--detach", &path.to_string_lossy(), sha],
        )?;
        Ok(Self {
            repo: repo.to_path_buf(),
            path,
        })
    }

    /// Copy today's config (`argot.toml`, `argot.local.toml`) into the
    /// worktree — the checkout has the historical ones, and current excludes
    /// and severities must judge the past. Seeds `.argot/semantic-index.json`
    /// from the repo's current fit (when present) so the worktree fit reuses
    /// embeddings for unchanged functions instead of re-embedding.
    pub fn adopt_current_config(&self, repo: &Path) {
        for name in [
            argot_core::config::CONFIG_FILE,
            argot_core::config::LOCAL_CONFIG_FILE,
        ] {
            let src = repo.join(name);
            if src.is_file() {
                let _ = std::fs::copy(&src, self.path.join(name));
            }
        }
        let seed = repo.join(".argot").join("semantic-index.json");
        if seed.is_file() {
            let dst_dir = self.path.join(".argot");
            let _ = std::fs::create_dir_all(&dst_dir);
            let _ = std::fs::copy(&seed, dst_dir.join("semantic-index.json"));
        }
    }
}

impl Drop for TempWorktree {
    fn drop(&mut self) {
        let _ = git(
            &self.repo,
            &[
                "worktree",
                "remove",
                "--force",
                &self.path.to_string_lossy(),
            ],
        );
        let _ = std::fs::remove_dir_all(&self.path);
        let _ = git(&self.repo, &["worktree", "prune"]);
    }
}
