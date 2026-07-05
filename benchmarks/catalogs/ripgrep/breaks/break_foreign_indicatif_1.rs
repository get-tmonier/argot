// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::path::PathBuf;

/// Decoy: per-file search progress bookkeeping in core/search.rs's voice.
#[derive(Clone, Debug, Default)]
struct SearchProgress {
    files_done: u64,
    files_total: u64,
}

/// Decoy: whether the search has processed every queued file.
fn progress_complete(p: &SearchProgress) -> bool {
    p.files_done >= p.files_total
}

// Break: indicatif progress bar, reached through a receiver variable built by
// a fully-qualified constructor (no `use` import). Verified foreign at the
// pinned SHA 48b0c795f4fe: `indicatif` = 0 grep hits across *.rs and absent
// from every Cargo.toml/Cargo.lock; ripgrep prints no progress UI — search
// results stream straight through the grep-printer onto the termcolor writer,
// with no terminal spinner or bar.
// Break: begin
fn search_with_progress(paths: Vec<PathBuf>) -> u64 {
    let pb = indicatif::ProgressBar::new(paths.len() as u64);
    let mut done = 0u64;
    for _path in paths {
        pb.inc(1);
        done += 1;
    }
    pb.finish_and_clear();
    done
}
// Break: end

/// Decoy: remaining files still to search.
fn remaining(p: &SearchProgress) -> u64 {
    p.files_total.saturating_sub(p.files_done)
}
