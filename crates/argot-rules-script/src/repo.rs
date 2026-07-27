//! Read-only repository access for scripts — host API v2.
//!
//! Everything a rule could reach before host API v2 was the changed file
//! itself. That is enough for "this line is wrong" and structurally not enough
//! for the whole family of rules about *two* files: a contract and the
//! implementations that must answer it, a migration and the schema it belongs
//! to, an endpoint and its entry in the API description. Those rules were
//! written by inlining the other file as a copied constant — a snapshot that
//! goes stale silently.
//!
//! The sandbox keeps the guarantee that matters. A script may **read** files
//! inside the repository root and **list** what the repository contains. It
//! cannot write, cannot escape the root (`..`, absolute paths and symlinks
//! that leave it are refused), cannot reach the network, and pays for every
//! read out of a per-file budget that the engine's operation cap already
//! bounds. A rule can see nothing its author's own clone does not already
//! hold.

use std::path::{Path, PathBuf};

/// Largest single file a script can read. Matches the engine's max string
/// size — a larger file could not be handed to the script anyway.
pub const MAX_FILE_BYTES: usize = 1 << 20;
/// Reads per (rule, file) run.
pub const MAX_READS: usize = 64;
/// Total bytes read per (rule, file) run.
pub const MAX_TOTAL_READ_BYTES: usize = 4 << 20;
/// `repo_paths` calls per (rule, file) run.
pub const MAX_PATHS_CALLS: usize = 16;
/// Paths one `repo_paths` call returns.
pub const MAX_PATHS_RESULTS: usize = 5_000;

/// Read-only view of one repository, as a script sees it. A port so the
/// fixture harness can root a case at its own directory and so the sandbox
/// boundary has one implementation to audit.
pub trait RepoFiles {
    /// The file's text, or `None` when it does not exist, escapes the root,
    /// is not valid UTF-8, or exceeds [`MAX_FILE_BYTES`].
    fn read(&self, rel: &str) -> Option<String>;
    /// Repo-relative paths matching `glob` (the `[[mute]].path` dialect:
    /// `*` and `**` cross `/`), sorted, capped at [`MAX_PATHS_RESULTS`].
    fn paths(&self, glob: &str) -> Vec<String>;
}

/// The production implementation: a directory, plus git's view of it when the
/// directory is a repository root.
pub struct RepoRoot {
    /// Canonical root. `None` when the path does not resolve — then every
    /// read is refused rather than guessed at.
    root: Option<PathBuf>,
    listing: Vec<String>,
}

impl RepoRoot {
    pub fn open(root: &Path) -> Self {
        let canonical = std::fs::canonicalize(root).ok();
        let listing = canonical.as_deref().map(listing_of).unwrap_or_default();
        Self {
            root: canonical,
            listing,
        }
    }

    /// Resolve `rel` inside the root, or `None` if it escapes. Refuses
    /// absolute paths and any `..` component before touching the filesystem,
    /// then canonicalizes so a symlink cannot walk out either.
    fn resolve(&self, rel: &str) -> Option<PathBuf> {
        let root = self.root.as_ref()?;
        if rel.is_empty() {
            return None;
        }
        let candidate = Path::new(rel);
        if candidate.is_absolute() {
            return None;
        }
        if candidate
            .components()
            .any(|c| !matches!(c, std::path::Component::Normal(_)))
        {
            return None;
        }
        let joined = std::fs::canonicalize(root.join(candidate)).ok()?;
        joined.starts_with(root).then_some(joined)
    }
}

impl RepoFiles for RepoRoot {
    fn read(&self, rel: &str) -> Option<String> {
        let path = self.resolve(rel)?;
        let meta = std::fs::metadata(&path).ok()?;
        if !meta.is_file() || meta.len() as usize > MAX_FILE_BYTES {
            return None;
        }
        std::fs::read_to_string(&path).ok()
    }

    fn paths(&self, glob: &str) -> Vec<String> {
        if glob.is_empty() {
            return Vec::new();
        }
        self.listing
            .iter()
            .filter(|p| argot_engine::suppress::fnmatch(p, glob))
            .take(MAX_PATHS_RESULTS)
            .cloned()
            .collect()
    }
}

/// What the root contains, repo-relative and sorted. Git's index when the
/// root is a repository (no walk, and build output stays out); otherwise a
/// bounded filesystem walk, which is what a fixture directory needs.
fn listing_of(root: &Path) -> Vec<String> {
    if let Some(scope) = argot_engine::corpus::GitScope::open(root) {
        return scope.tracked().to_vec();
    }
    let mut out = Vec::new();
    walk(root, root, 0, &mut out);
    out.sort();
    out
}

/// Depth-bounded directory walk for a non-git root. `.git` is skipped so a
/// fixture nested inside a working repo never enumerates object files.
fn walk(root: &Path, dir: &Path, depth: usize, out: &mut Vec<String>) {
    const MAX_DEPTH: usize = 16;
    if depth > MAX_DEPTH || out.len() >= MAX_PATHS_RESULTS {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        if name == ".git" {
            continue;
        }
        match entry.file_type() {
            Ok(t) if t.is_dir() => walk(root, &path, depth + 1, out),
            Ok(t) if t.is_file() => {
                out.push(argot_engine::corpus::rel_to_repo(&path, root));
            }
            _ => {}
        }
        if out.len() >= MAX_PATHS_RESULTS {
            return;
        }
    }
}

#[cfg(test)]
mod tests;
