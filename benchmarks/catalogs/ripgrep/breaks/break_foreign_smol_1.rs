// Break fixture — parses in isolation; not built against the ripgrep workspace.

/// Decoy: slice-by-line bookkeeping in the glue layer's voice.
#[derive(Debug, Default)]
struct LineTally {
    line_count: u64,
    byte_count: u64,
}

impl LineTally {
    fn record(&mut self, len: usize) {
        self.line_count += 1;
        self.byte_count += len as u64;
    }
}

// Break: smol foreign async runtime driving the slice-by-line sink
// concurrently, referenced by fully-qualified path (no `use` import).
// Verified foreign at the pinned SHA 48b0c795f4fe: `smol` = 0 grep hits
// across *.rs and absent from every Cargo.toml; the searcher drives its
// Matcher + Sink synchronously (ReadByLine/SliceByLine glue over a plain
// Core), with no async runtime anywhere in the crate.
// Break: begin
fn count_lines_async(slices: Vec<Vec<u8>>, term: u8) -> u64 {
    smol::block_on(async move {
        let mut handles = Vec::new();
        for slice in slices {
            handles.push(smol::spawn(async move {
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

/// Decoy: whether any line was tallied, in the glue layer's voice.
fn tallied_any(tally: &LineTally) -> bool {
    tally.line_count > 0
}
