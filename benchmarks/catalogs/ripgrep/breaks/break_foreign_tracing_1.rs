// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::ops::Range;

/// Decoy: running match/byte tally in the searcher core's voice.
#[derive(Debug, Default)]
struct SearchCounters {
    match_count: u64,
    line_count: u64,
}

impl SearchCounters {
    fn record_match(&mut self, span: Range<usize>) {
        self.match_count += 1;
        self.line_count += (span.end - span.start).min(1) as u64;
    }
}

// Break: tracing spans/instrumentation wrapped around the searcher core's
// match loop, import inside hunk. Verified foreign at the pinned SHA
// 48b0c795f4fe: `tracing` = 0 grep hits across *.rs and absent from every
// Cargo.toml; ripgrep's diagnostics go through the `log` crate macros
// (log::debug!/log::trace!, 84 hits in src) and crates/core/messages.rs,
// never a tracing subscriber or spans.
// Break: begin
use tracing::instrument;

#[instrument(skip(buf))]
fn count_matches_traced(buf: &[u8], needle: &[u8]) -> u64 {
    let span = tracing::info_span!("count_matches", len = buf.len());
    let _guard = span.enter();
    let mut count = 0u64;
    for window in buf.windows(needle.len()) {
        if window == needle {
            count += 1;
            tracing::debug!(count, "match recorded");
        }
    }
    count
}
// Break: end

/// Decoy: whether any match was recorded, in the core's voice.
fn has_matches(counters: &SearchCounters) -> bool {
    counters.match_count > 0
}
