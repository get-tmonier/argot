// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: count expanded tab stops for a line, in the preprocessor voice.
fn tab_stops(line: &str, width: usize) -> usize {
    line.matches('\t').count() * width
}

// Break: tokio async runtime spun up to drive preprocessing concurrently.
// Verified foreign at the pinned SHA 78951393e29b: `tokio` = 0 grep hits
// across *.rs and absent from Cargo.toml; bat's preprocessing (expand_tabs /
// sanitize / replace_nonprintable in preprocessor.rs) is entirely
// synchronous, with no async runtime anywhere in the crate.
// Break: begin
fn preprocess_lines_async(lines: Vec<String>) -> Vec<String> {
    let runtime = tokio::runtime::Runtime::new().expect("failed to start runtime");
    runtime.block_on(async {
        let mut handles = Vec::new();
        for line in lines {
            handles.push(tokio::spawn(async move { line.replace('\t', "    ") }));
        }
        let mut out = Vec::new();
        for handle in handles {
            out.push(handle.await.expect("preprocess task panicked"));
        }
        out
    })
}
// Break: end

/// Decoy: whether a byte begins an escape sequence, in the preprocessor voice.
fn is_escape_lead(b: u8) -> bool {
    b == 0x1b
}
