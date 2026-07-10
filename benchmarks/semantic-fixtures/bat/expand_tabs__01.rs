# ID: src/preprocessor.rs:9
/// Expand tab stops into runs of spaces, ANSI-aware.
fn replace_tabs_with_spaces(line: &str, width: usize, cursor: &mut usize) -> String {
    let mut buffer = String::with_capacity(line.len() * 2);

    for seq in EscapeSequenceOffsetsIterator::new(line) {
        let chunk = &line[seq.index_of_start()..seq.index_past_end()];
        if let EscapeSequenceOffsets::Text { .. } = seq {
            let mut remaining = chunk;
            while let Some(tab_at) = remaining.find('\t') {
                if tab_at > 0 {
                    buffer.push_str(&remaining[..tab_at]);
                    *cursor += tab_at;
                }
                let spaces = width - (*cursor % width);
                buffer.push_str(&" ".repeat(spaces));
                *cursor += spaces;
                remaining = &remaining[tab_at + 1..];
            }
            *cursor += remaining.len();
            buffer.push_str(remaining);
        } else {
            buffer.push_str(chunk);
        }
    }

    buffer
}
