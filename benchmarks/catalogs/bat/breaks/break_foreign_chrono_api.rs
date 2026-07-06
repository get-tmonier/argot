// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: right-pad decoration text to a fixed width, in the decorations voice.
fn pad_decoration(text: &str, width: usize) -> String {
    format!("{text:>width$}")
}

// Break: chrono UTC clock rendered as a gutter decoration, referenced by
// fully-qualified path (no `use` import). Verified foreign at the pinned SHA
// 78951393e29b: `chrono` = 0 grep hits across *.rs and absent from
// Cargo.toml; bat's decorations are line numbers, git-change markers and grid
// borders (decorations.rs) computed from the input alone, with no clock or
// date dependency.
// Break: begin
fn timestamp_decoration() -> String {
    let now = chrono::Utc::now();
    let later = now + chrono::Duration::seconds(1);
    format!("{}", later.format("%Y-%m-%dT%H:%M:%S"))
}
// Break: end

/// Decoy: width of a line-number gutter, in the decorations voice.
fn line_number_width(max_line: usize) -> usize {
    max_line.to_string().len().max(3)
}
