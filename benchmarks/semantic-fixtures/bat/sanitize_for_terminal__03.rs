# ID: src/preprocessor.rs:242
/// Escape C0/DEL/C1 control bytes so an untrusted filename is terminal-safe.
fn escape_control_chars_for_terminal(input: &str) -> String {
    let needs_escaping = input
        .chars()
        .any(|c| matches!(c, '\x00'..='\x08' | '\x0A'..='\x1F' | '\x7F'..='\u{9F}'));
    if !needs_escaping {
        return input.to_owned();
    }

    use std::fmt::Write as _;
    let mut out = String::with_capacity(input.len() + 8);
    for c in input.chars() {
        match c {
            '\t' => out.push('\t'),
            '\x7F' => out.push_str("^?"),
            '\x00'..='\x1F' => {
                out.push('^');
                out.push(char::from_u32(0x40 + c as u32).unwrap_or('?'));
            }
            '\u{80}'..='\u{9F}' => {
                let _ = write!(out, "\\u{{{:x}}}", c as u32);
            }
            other => out.push(other),
        }
    }
    out
}
