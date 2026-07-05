// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::path::PathBuf;
use std::sync::Arc;

/// Decoy: a shared directory-visit counter in ignore/dir.rs's voice.
#[derive(Clone, Debug, Default)]
struct VisitState {
    dirs_seen: u64,
    files_seen: u64,
}

/// Decoy: whether any entry has been visited yet.
fn visited_any(state: &VisitState) -> bool {
    state.dirs_seen + state.files_seen > 0
}

// Break: parking_lot Mutex (aliased on import) guarding the shared walker
// visit state across worker threads. Verified foreign at the pinned SHA
// 48b0c795f4fe: `parking_lot` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; the parallel walker coordinates through crossbeam
// (its crossbeam-deque work-stealing stack behind WalkParallel), and where it
// needs a lock it uses std::sync::Mutex, never parking_lot.
// Break: begin
use parking_lot::Mutex as PlMutex;

fn record_visit(state: &Arc<PlMutex<VisitState>>, path: &PathBuf) {
    let mut guard = state.lock();
    if path.is_dir() {
        guard.dirs_seen += 1;
    } else {
        guard.files_seen += 1;
    }
}
// Break: end

/// Decoy: snapshot the total number of visited entries.
fn total_visited(state: &VisitState) -> u64 {
    state.dirs_seen + state.files_seen
}
