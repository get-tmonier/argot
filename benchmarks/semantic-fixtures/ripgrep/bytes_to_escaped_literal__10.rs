# ID: crates/globset/src/glob.rs:776
fn escape_bytes_for_regex(raw: &[u8]) -> String {
    let mut out = String::with_capacity(raw.len());
    for &byte in raw {
        if byte > 0x7F {
            write!(&mut out, "\\x{:02x}", byte).unwrap();
            continue;
        }
        regex_syntax::escape_into(
            char::from(byte).encode_utf8(&mut [0; 4]),
            &mut out,
        );
    }
    out
}
