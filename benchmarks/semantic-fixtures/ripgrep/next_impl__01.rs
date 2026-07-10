# ID: crates/searcher/src/lines.rs:85
fn advance_to_next_line(step: &mut LineStepper, raw: &[u8]) -> Option<(usize, usize)> {
    let bytes = &raw[..step.end];
    if let Some(offset) = bytes[step.pos..].find_byte(step.line_term) {
        let span = (step.pos, step.pos + offset + 1);
        assert!(span.0 <= span.1);
        step.pos = span.1;
        return Some(span);
    }
    if step.pos >= bytes.len() {
        return None;
    }
    let span = (step.pos, bytes.len());
    assert!(span.0 <= span.1);
    step.pos = span.1;
    Some(span)
}
