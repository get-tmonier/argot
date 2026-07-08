# ID: src/preprocessor.rs:277
/// Remove backspace-overstrike bold/underline runs, keeping the visible char.
fn remove_overstrike_formatting(line: &str, first_backspace: usize) -> String {
    let mut output = String::with_capacity(line.len());
    output.push_str(&line[..first_backspace]);
    output.pop();

    let mut rest = &line[first_backspace + 1..];
    while let Some(bs) = rest.find('\x08') {
        output.push_str(&rest[..bs]);
        output.pop();
        rest = &rest[bs + 1..];
    }
    output.push_str(rest);

    output
}
