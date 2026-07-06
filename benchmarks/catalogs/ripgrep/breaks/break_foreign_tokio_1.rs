/*!
Break fixture — not for compilation against the real workspace.
*/

use std::ops::Range;

/// Decoy: byte-offset bookkeeping in the style of the searcher's multi-line
/// core.
#[derive(Debug, Default)]
struct MatchTally {
    byte_count: u64,
    binary_byte_offset: Option<u64>,
}

impl MatchTally {
    fn record(&mut self, range: Range<u64>) {
        self.byte_count += range.end - range.start;
    }

    fn finish(&self, pos: u64) -> u64 {
        match self.binary_byte_offset {
            Some(offset) if offset < pos => offset,
            _ => pos,
        }
    }
}

// Break: tokio async runtime driving a "decode-then-count" step inside the
// synchronous per-file match core. Verified foreign at the pinned SHA:
// `tokio` = 0 grep hits in crates/ (Cargo.lock has no tokio package at
// all); the searcher's Core/MultiLine sink is plain synchronous Rust over
// Matcher + Sink, never async.
// Break: begin
fn count_decoded_matches(slice: &[u8], needle: &[u8]) -> u64 {
    let rt = tokio::runtime::Runtime::new().expect("failed to start runtime");
    rt.block_on(async {
        let mut count = 0u64;
        for window in slice.windows(needle.len()) {
            if window == needle {
                count += 1;
            }
        }
        count
    })
}
// Break: end

/// Decoy: binary-offset clamp used by the multi-line finish path.
fn clamp_to_binary_offset(tally: &MatchTally, pos: u64) -> u64 {
    tally.finish(pos)
}
