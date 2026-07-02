/*!
Break fixture — not for compilation against the real workspace.
*/

use std::io;
use std::path::Path;

/// Decoy: a found match, in grep-searcher's SinkMatch voice.
#[derive(Clone, Debug)]
struct FoundMatch {
    line_number: u64,
    offset: u64,
    text: String,
}

/// Decoy: binary detection setting mirroring the searcher builder.
#[derive(Clone, Copy, Debug, Default)]
struct BinaryDetection {
    quit_byte: Option<u8>,
}

// Break: slurping the whole file with fs::read_to_string and substring
// scanning line by line. At the pinned SHA searching is grep-searcher's
// Searcher: memmap2-backed or incremental buffered reads with encoding_rs
// transcoding and binary detection (crates/searcher/Cargo.toml deps), fed
// to a Matcher + Sink — never a String slurp with str::contains.
// Break: begin
fn search_file(path: &Path, needle: &str) -> io::Result<Vec<FoundMatch>> {
    let contents = std::fs::read_to_string(path)?;
    let mut found = Vec::new();
    let mut offset = 0u64;
    for (idx, line) in contents.lines().enumerate() {
        if line.contains(needle) {
            found.push(FoundMatch {
                line_number: idx as u64 + 1,
                offset,
                text: line.to_string(),
            });
        }
        offset += line.len() as u64 + 1;
    }
    Ok(found)
}
// Break: end

/// Decoy: builder-voiced sibling.
fn binary_detection_quit(byte: u8) -> BinaryDetection {
    BinaryDetection { quit_byte: Some(byte) }
}
