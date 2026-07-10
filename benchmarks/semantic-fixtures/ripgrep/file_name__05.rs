# ID: crates/globset/src/pathutil.rs:9
fn base_component<'a>(path: &Cow<'a, [u8]>) -> Option<Cow<'a, [u8]>> {
    if path.is_empty() {
        return None;
    }
    let cut = path.rfind_byte(b'/').map(|i| i + 1).unwrap_or(0);
    let name = match *path {
        Cow::Borrowed(raw) => Cow::Borrowed(&raw[cut..]),
        Cow::Owned(ref raw) => {
            let mut owned = raw.clone();
            owned.drain_bytes(..cut);
            Cow::Owned(owned)
        }
    };
    if name == &b".."[..] {
        None
    } else {
        Some(name)
    }
}
