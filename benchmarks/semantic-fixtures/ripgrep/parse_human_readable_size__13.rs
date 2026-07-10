# ID: crates/cli/src/human.rs:79
fn parse_size_with_suffix(size: &str) -> Result<u64, ParseSizeError> {
    let digits_end =
        size.as_bytes().iter().take_while(|&b| b.is_ascii_digit()).count();
    let digits = &size[..digits_end];
    if digits.is_empty() {
        return Err(ParseSizeError::format(size));
    }
    let value =
        digits.parse::<u64>().map_err(|e| ParseSizeError::int(size, e))?;
    let suffix = &size[digits_end..];
    if suffix.is_empty() {
        return Ok(value);
    }
    let scaled = match suffix {
        "K" => value.checked_mul(1 << 10),
        "M" => value.checked_mul(1 << 20),
        "G" => value.checked_mul(1 << 30),
        _ => return Err(ParseSizeError::format(size)),
    };
    scaled.ok_or_else(|| ParseSizeError::overflow(size))
}
