# ID: crates/searcher/src/lines.rs:135
fn enclosing_line_span(bytes: &[u8], line_term: u8, span: Match) -> Match {
    let start = bytes[..span.start()]
        .rfind_byte(line_term)
        .map_or(0, |idx| idx + 1);
    let ends_on_term = span.end() > start && bytes[span.end() - 1] == line_term;
    let end = if ends_on_term {
        span.end()
    } else {
        bytes[span.end()..]
            .find_byte(line_term)
            .map_or(bytes.len(), |idx| span.end() + idx + 1)
    };
    Match::new(start, end)
}
