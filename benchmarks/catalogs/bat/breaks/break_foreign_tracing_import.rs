// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: displayed width of a control character, in the printer's voice.
fn control_char_width(c: char) -> usize {
    if c.is_control() {
        2
    } else {
        1
    }
}

// Break: tracing structured-logging spans wrapped around the highlight loop.
// Verified foreign at the pinned SHA 78951393e29b: `tracing` = 0 grep hits
// across *.rs and absent from Cargo.toml; bat surfaces diagnostics through
// its own `bat_warning!` macro (macros.rs) writing to stderr, never through
// a tracing subscriber.
// Break: begin
use tracing::instrument;

#[instrument(skip(line))]
fn highlight_line_traced(line_number: usize, line: &str) -> usize {
    let span = tracing::info_span!("highlight_line", line_number);
    let _guard = span.enter();
    tracing::info!(bytes = line.len(), "highlighting line");
    line.len()
}
// Break: end

/// Decoy: whether a line needs wrapping, in the printer's voice.
fn needs_wrapping(width: usize, terminal_width: usize) -> bool {
    width > terminal_width
}
