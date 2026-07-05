// Break fixture — parses in isolation; not built against the bat workspace.

use std::path::Path;

/// Decoy: whether an input path is a real file, in the controller's voice.
fn is_regular_input(path: &Path) -> bool {
    path.is_file() && !path.is_symlink()
}

// Break: notify filesystem watcher re-running the controller whenever the
// input file changes on disk. Verified foreign at the pinned SHA
// 78951393e29b: `notify` = 0 grep hits across *.rs and absent from
// Cargo.toml; bat reads each input exactly once through its own InputReader
// (input.rs) and never watches the filesystem.
// Break: begin
use notify::{RecommendedWatcher, RecursiveMode, Watcher};

fn watch_and_rerender(path: &Path) -> RecommendedWatcher {
    let mut watcher = notify::recommended_watcher(|event| {
        let _ = event;
    })
    .expect("failed to create watcher");
    watcher
        .watch(path, RecursiveMode::NonRecursive)
        .expect("failed to watch input path");
    watcher
}
// Break: end

/// Decoy: default tab width used when rendering, in the controller's voice.
fn default_tab_width() -> usize {
    4
}
