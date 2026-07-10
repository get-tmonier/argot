# ID: src/pager.rs:43
/// Classify a pager binary name (less/more/most/bat/builtin/unknown).
fn classify_pager_binary(bin: &str) -> PagerKind {
    use std::path::Path;

    if bin == "builtin" {
        return PagerKind::Builtin;
    }

    let pager_bin = Path::new(bin).file_stem();
    let running_bin = env::args_os().next();
    let running_is_pager = running_bin
        .map(|s| Path::new(&s).file_stem() == pager_bin)
        .unwrap_or(false);

    match pager_bin.map(|s| s.to_string_lossy()).as_deref() {
        Some("less") => PagerKind::Less,
        Some("most") => PagerKind::Most,
        Some("more") => PagerKind::More,
        _ if running_is_pager => PagerKind::Bat,
        _ => PagerKind::Unknown,
    }
}
