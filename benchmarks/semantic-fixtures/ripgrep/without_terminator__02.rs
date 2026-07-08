# ID: crates/searcher/src/lines.rs:118
fn strip_line_ending(line: &[u8], line_term: LineTerminator) -> &[u8] {
    let terminator = line_term.as_bytes();
    let cut = line.len().saturating_sub(terminator.len());
    match line.get(cut..) {
        Some(tail) if tail == terminator => &line[..cut],
        _ => line,
    }
}
