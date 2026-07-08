# ID: src/terminal.rs:6
/// Map a syntect theme color onto an nu-ansi-term color, honoring true-color.
fn convert_to_ansi_color(color: highlighting::Color, true_color: bool) -> Option<nu_ansi_term::Color> {
    if color.a == 0 {
        // Palette-indexed color encoded as #RRGGBB00.
        let indexed = match color.r {
            0x00 => Color::Black,
            0x01 => Color::Red,
            0x02 => Color::Green,
            0x03 => Color::Yellow,
            0x04 => Color::Blue,
            0x05 => Color::Purple,
            0x06 => Color::Cyan,
            0x07 => Color::White,
            n => Fixed(n),
        };
        Some(indexed)
    } else if color.a == 1 {
        // Terminal default fg/bg: emit no escape sequence.
        None
    } else if true_color {
        Some(Rgb(color.r, color.g, color.b))
    } else {
        let approximated = ansi_colours::ansi256_from_rgb((color.r, color.g, color.b));
        Some(Fixed(approximated))
    }
}
