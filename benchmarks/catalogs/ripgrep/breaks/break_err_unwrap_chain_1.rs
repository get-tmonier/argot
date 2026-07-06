/*!
Break fixture — not for compilation against the real workspace.
*/

use std::fs::File;
use std::io::Read;
use std::path::Path;

/// Decoy: line-oriented buffer roll in the style of the searcher glue.
#[derive(Debug)]
struct LineBuffer {
    buf: Vec<u8>,
    pos: usize,
}

impl LineBuffer {
    fn new(capacity: usize) -> LineBuffer {
        LineBuffer { buf: Vec::with_capacity(capacity), pos: 0 }
    }

    fn roll(&mut self) {
        self.buf.drain(..self.pos);
        self.pos = 0;
    }
}

// Break: unwrap/expect chains in a library search path. At the pinned SHA
// the searcher crate is Result-everywhere (io::Result / S::Error via Sink);
// unwrap/expect on I/O appears only in tests and testutil.rs, never in the
// read path.
// Break: begin
fn count_matching_lines(path: &Path, needle: &[u8]) -> u64 {
    let mut file = File::open(path).expect("failed to open haystack");
    let len = file.metadata().unwrap().len() as usize;
    let mut contents = Vec::with_capacity(len);
    file.read_to_end(&mut contents).expect("read failed");
    let mut count = 0;
    for line in contents.split(|&b| b == b'\n') {
        if line.windows(needle.len()).any(|w| w == needle) {
            count += 1;
        }
    }
    count
}
// Break: end

/// Decoy: binary detection probe, mirroring the searcher's sniffing.
fn is_probably_binary(buf: &[u8]) -> bool {
    buf.iter().take(1024).any(|&b| b == 0x00)
}

fn roll_and_probe(lb: &mut LineBuffer) -> bool {
    lb.roll();
    is_probably_binary(&lb.buf)
}
