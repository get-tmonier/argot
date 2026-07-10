# ID: crates/searcher/src/lines.rs:170
fn line_start_offset_before(
    bytes: &[u8],
    mut pos: usize,
    line_term: u8,
    mut remaining: usize,
) -> usize {
    if pos == 0 {
        return 0;
    }
    if bytes[pos - 1] == line_term {
        pos -= 1;
    }
    while let Some(idx) = bytes[..pos].rfind_byte(line_term) {
        if remaining == 0 {
            return idx + 1;
        }
        if idx == 0 {
            return 0;
        }
        remaining -= 1;
        pos = idx;
    }
    0
}
