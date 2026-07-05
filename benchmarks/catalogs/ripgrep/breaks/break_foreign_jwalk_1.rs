// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::path::PathBuf;

/// Decoy: a discovered path in ignore/walk.rs's voice.
#[derive(Clone, Debug, Default)]
struct DiscoveredPath {
    path: PathBuf,
    depth: usize,
}

/// Decoy: whether a discovered path is at the walk root.
fn is_root(entry: &DiscoveredPath) -> bool {
    entry.depth == 0
}

// Break: jwalk parallel directory walker (submodule import) replacing the
// repo-owned ignore::WalkBuilder traversal. Verified foreign at the pinned SHA
// 48b0c795f4fe: `jwalk` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; ripgrep's recursive traversal is its own
// crossbeam-deque work-stealing WalkParallel over the `ignore` crate (which
// owns gitignore/hidden filtering), never an external walker.
// Break: begin
use jwalk::WalkDir;
use jwalk::rayon::prelude::*;

fn walk_paths(root: &PathBuf) -> Vec<DiscoveredPath> {
    WalkDir::new(root)
        .into_iter()
        .par_bridge()
        .filter_map(|entry| entry.ok())
        .map(|entry| DiscoveredPath {
            path: entry.path(),
            depth: entry.depth,
        })
        .collect()
}
// Break: end

/// Decoy: deepest discovered path in a batch.
fn max_depth(entries: &[DiscoveredPath]) -> usize {
    entries.iter().map(|e| e.depth).max().unwrap_or(0)
}
