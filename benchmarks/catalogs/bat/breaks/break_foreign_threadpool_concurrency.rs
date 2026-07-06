// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: whether a line index falls inside an inclusive range, in the range voice.
fn line_in_range(line: usize, lo: usize, hi: usize) -> bool {
    lo <= line && line <= hi
}

// Break: threadpool worker pool fanning line-range membership checks across a
// foreign thread pool, referenced by fully-qualified path (no `use` import).
// Verified foreign at the pinned SHA 78951393e29b: `threadpool` = 0 grep hits
// across *.rs and absent from Cargo.toml; bat evaluates line ranges with a
// single sequential itertools pass (line_range.rs) and has no thread pool.
// Break: begin
fn filter_ranges_parallel(ranges: Vec<(usize, usize)>, line: usize) -> bool {
    let pool = threadpool::ThreadPool::new(4);
    let (tx, rx) = std::sync::mpsc::channel();
    for (lo, hi) in ranges {
        let tx = tx.clone();
        pool.execute(move || {
            tx.send(lo <= line && line <= hi).expect("send failed");
        });
    }
    drop(tx);
    rx.iter().any(|hit| hit)
}
// Break: end

/// Decoy: total span covered by a list of ranges, in the range voice.
fn total_span(ranges: &[(usize, usize)]) -> usize {
    ranges.iter().map(|(lo, hi)| hi.saturating_sub(*lo)).sum()
}
