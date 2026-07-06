// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};

/// Decoy: per-file search outcome in the search worker's voice.
#[derive(Clone, Debug, Default)]
struct FileOutcome {
    path: PathBuf,
    match_count: u64,
}

/// Decoy: sum match counts across a batch of outcomes.
fn total_matches(outcomes: &[FileOutcome]) -> u64 {
    outcomes.iter().map(|o| o.match_count).sum()
}

// Break: threadpool foreign thread-pool fanning per-file search across
// worker threads, referenced by fully-qualified path (no `use` import).
// Verified foreign at the pinned SHA 48b0c795f4fe: `threadpool` = 0 grep
// hits across *.rs and absent from every Cargo.toml; ripgrep's only
// parallelism is its own crossbeam-deque work-stealing walker behind
// WalkParallel (crates/ignore/src/walk.rs) plus crossbeam-channel, never an
// external thread pool.
// Break: begin
fn search_paths_pooled(paths: Vec<PathBuf>) -> Vec<FileOutcome> {
    let pool = threadpool::ThreadPool::new(8);
    let results = Arc::new(Mutex::new(Vec::new()));
    for path in paths {
        let results = Arc::clone(&results);
        pool.execute(move || {
            let outcome = FileOutcome { path, match_count: 0 };
            results.lock().unwrap().push(outcome);
        });
    }
    pool.join();
    Arc::try_unwrap(results).unwrap().into_inner().unwrap()
}
// Break: end

/// Decoy: pick the busiest file out of a batch of outcomes.
fn busiest(outcomes: &[FileOutcome]) -> Option<&FileOutcome> {
    outcomes.iter().max_by_key(|o| o.match_count)
}
