# ID: src/decorations.rs:42
/// Produce the gutter text for a line number (or its wrapped continuation).
fn render_line_number(
    decoration: &LineNumberDecoration,
    line_number: usize,
    continuation: bool,
    _printer: &InteractivePrinter,
) -> DecorationText {
    if !continuation {
        let plain: String = format!("{line_number:4}");
        return DecorationText {
            width: plain.len(),
            text: decoration.color.paint(plain).to_string(),
        };
    }

    if line_number >= decoration.cached_wrap_invalid_at {
        let widened = decoration.cached_wrap.width + 1;
        DecorationText {
            text: decoration.color.paint(" ".repeat(widened)).to_string(),
            width: widened,
        }
    } else {
        decoration.cached_wrap.clone()
    }
}
