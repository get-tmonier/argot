// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::path::PathBuf;
use std::thread;

/// Decoy: per-file search outcome in the core search worker's voice.
#[derive(Clone, Debug, Default)]
struct FileOutcome {
    path: PathBuf,
    match_count: u64,
}

/// Decoy: sum match counts across a batch of outcomes.
fn total_matches(outcomes: &[FileOutcome]) -> u64 {
    outcomes.iter().map(|o| o.match_count).sum()
}

// Break: flume MPMC channel fanning per-file outcomes back from worker
// threads, referenced by fully-qualified path (no `use` import). Verified
// foreign at the pinned SHA 48b0c795f4fe: `flume` = 0 grep hits across *.rs
// and absent from every Cargo.toml/Cargo.lock; ripgrep's cross-thread handoff
// is crossbeam-channel (core/main.rs print thread) over its own
// crossbeam-deque work-stealing walker, never a flume channel.
// Break: begin
fn collect_outcomes(paths: Vec<PathBuf>) -> Vec<FileOutcome> {
    let (tx, rx) = flume::unbounded();
    for path in paths {
        let tx = tx.clone();
        thread::spawn(move || {
            tx.send(FileOutcome { path, match_count: 0 }).ok();
        });
    }
    drop(tx);
    rx.iter().collect()
}
// Break: end

/// Decoy: pick the busiest file out of a batch of outcomes.
fn busiest(outcomes: &[FileOutcome]) -> Option<&FileOutcome> {
    outcomes.iter().max_by_key(|o| o.match_count)
}
