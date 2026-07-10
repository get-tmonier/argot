# ID: src/terminal.rs:49
/// Render a highlighted span into an ANSI-escaped string.
fn paint_styled_text(
    style: highlighting::Style,
    text: &str,
    true_color: bool,
    colored: bool,
    italics: bool,
    background_color: Option<highlighting::Color>,
) -> String {
    if text.is_empty() {
        return text.to_string();
    }

    let mut painted = if colored {
        let mut color = Style {
            foreground: to_ansi_color(style.foreground, true_color),
            ..Style::default()
        };
        if style.font_style.contains(FontStyle::BOLD) {
            color = color.bold();
        }
        if style.font_style.contains(FontStyle::UNDERLINE) {
            color = color.underline();
        }
        if italics && style.font_style.contains(FontStyle::ITALIC) {
            color = color.italic();
        }
        color
    } else {
        Style::default()
    };

    painted.background = background_color.and_then(|c| to_ansi_color(c, true_color));
    painted.paint(text).to_string()
}
