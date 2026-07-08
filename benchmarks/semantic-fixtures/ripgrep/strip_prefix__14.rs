# ID: crates/ignore/src/pathutil.rs:63
fn remove_path_prefix<'a, P: AsRef<Path> + ?Sized>(
    prefix: &'a P,
    path: &'a Path,
) -> Option<&'a Path> {
    use std::os::unix::ffi::OsStrExt;

    let prefix = prefix.as_ref().as_os_str().as_bytes();
    let path = path.as_os_str().as_bytes();
    if prefix.len() > path.len() {
        return None;
    }
    if prefix != &path[0..prefix.len()] {
        return None;
    }
    let rest = OsStr::from_bytes(&path[prefix.len()..]);
    Some(Path::new(rest))
}
