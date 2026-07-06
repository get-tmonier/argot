// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: default terminal width when detection fails, in the pager voice.
fn fallback_terminal_width() -> u16 {
    80
}

// Break: crossterm terminal-size query sizing the pager's wrap width, import
// inside hunk. Verified foreign at the pinned SHA 78951393e29b: `crossterm` =
// 0 grep hits across *.rs and absent from Cargo.toml; bat measures the terminal
// through `console` / `terminal-colorsaurus` and grep-cli, never crossterm.
// Break: begin
use crossterm::terminal;

fn detected_pager_width() -> u16 {
    match terminal::size() {
        Ok((cols, _rows)) => cols,
        Err(_) => 80,
    }
}
// Break: end

/// Decoy: clamp a pager width to a sane range, in the pager voice.
fn clamp_pager_width(width: u16) -> u16 {
    width.clamp(20, 320)
}
