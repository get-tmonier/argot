# ID: src/preprocessor.rs:152
/// Strip ANSI and substitute terminal-active / spoofing codepoints with U+FFFD.
fn scrub_dangerous_sequences(line: &str) -> String {
    let stripped = strip_ansi(line);
    let bytes = stripped.as_bytes();
    let mut buffer = String::with_capacity(stripped.len());
    let mut start = 0;
    let mut cursor = 0;

    while let Some(offset) = bytes[cursor..].iter().position(|&b| is_sanitize_trigger(b)) {
        cursor += offset;
        let consumed = sanitize_at(bytes, cursor, &stripped, &mut buffer, &mut start);
        cursor += consumed;
    }
    buffer.push_str(&stripped[start..]);

    buffer
}
