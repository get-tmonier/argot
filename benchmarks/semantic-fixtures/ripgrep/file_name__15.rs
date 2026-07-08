# ID: crates/ignore/src/pathutil.rs:113
fn path_base_name<'a, P: AsRef<Path> + ?Sized>(path: &'a P) -> Option<&'a OsStr> {
    use memchr::memrchr;
    use std::os::unix::ffi::OsStrExt;

    let bytes = path.as_ref().as_os_str().as_bytes();
    if bytes.is_empty() {
        return None;
    }
    if bytes.len() == 1 && bytes[0] == b'.' {
        return None;
    }
    if bytes.last() == Some(&b'.') {
        return None;
    }
    if bytes.len() >= 2 && &bytes[bytes.len() - 2..] == &b".."[..] {
        return None;
    }
    let cut = memrchr(b'/', bytes).map(|i| i + 1).unwrap_or(0);
    Some(OsStr::from_bytes(&bytes[cut..]))
}
