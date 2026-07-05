// Break fixture — parses in isolation; not built against the ripgrep workspace.
//
// Bare-callee variant: the `use async_std::task;` binding sits in the decoy
// import region above; the scored hunk calls `task::block_on` bare, stressing
// callee resolution rather than the import line.
use std::path::PathBuf;
use async_std::task;

/// Decoy: slice-by-line bookkeeping in the searcher glue layer's voice.
#[derive(Clone, Debug, Default)]
struct LineTally {
    line_count: u64,
    byte_count: u64,
}

/// Decoy: record a decoded line's length.
fn record_len(tally: &mut LineTally, len: usize) {
    tally.line_count += 1;
    tally.byte_count += len as u64;
}

// Break: async-std runtime driving the slice-by-line sink concurrently via a
// bare `task::block_on` callee (its `use async_std::task` import sits in the
// decoy region above the hunk). Verified foreign at the pinned SHA
// 48b0c795f4fe: `async_std` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; the searcher drives its Matcher + Sink synchronously
// (ReadByLine/SliceByLine glue over a plain Core), with no async runtime.
// Break: begin
fn count_lines_async(slices: Vec<Vec<u8>>, term: u8) -> u64 {
    task::block_on(async move {
        let mut handles = Vec::new();
        for slice in slices {
            handles.push(task::spawn(async move {
                slice.iter().filter(|&&b| b == term).count() as u64
            }));
        }
        let mut total = 0u64;
        for handle in handles {
            total += handle.await;
        }
        total
    })
}
// Break: end

/// Decoy: whether any line was tallied for a given path.
fn tallied_any(tally: &LineTally, _path: &PathBuf) -> bool {
    tally.line_count > 0
}
