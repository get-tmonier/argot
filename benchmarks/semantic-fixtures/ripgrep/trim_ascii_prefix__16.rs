# ID: crates/printer/src/util.rs:452
fn skip_leading_whitespace(
    line_term: LineTerminator,
    slice: &[u8],
    range: Match,
) -> Match {
    fn is_space(b: u8) -> bool {
        matches!(b, b'\t' | b'\n' | b'\x0B' | b'\x0C' | b'\r' | b' ')
    }

    let terminator = line_term.as_bytes();
    let skipped = slice[range]
        .iter()
        .take_while(|&&b| is_space(b) && !terminator.contains(&b))
        .count();
    range.with_start(range.start() + skipped)
}
