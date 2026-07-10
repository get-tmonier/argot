# ID: crates/globset/src/pathutil.rs:44
fn trailing_extension<'a>(name: &Cow<'a, [u8]>) -> Option<Cow<'a, [u8]>> {
    if name.is_empty() {
        return None;
    }
    let dot = match name.rfind_byte(b'.') {
        Some(i) => i,
        None => return None,
    };
    Some(match *name {
        Cow::Borrowed(raw) => Cow::Borrowed(&raw[dot..]),
        Cow::Owned(ref raw) => {
            let mut owned = raw.clone();
            owned.drain_bytes(..dot);
            Cow::Owned(owned)
        }
    })
}
