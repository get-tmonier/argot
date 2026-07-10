# ID: src/less.rs:23
/// Parse the numeric version out of GNU less's `--version` banner.
fn extract_less_version_number(output: &[u8]) -> Option<LessVersion> {
    let banner = output.strip_prefix(b"less ")?;
    let version = std::str::from_utf8(banner).ok()?;
    let end = version.find(|c: char| !c.is_ascii_digit())?;
    let number = version[..end].parse::<usize>().ok()?;
    Some(LessVersion::Less(number))
}
