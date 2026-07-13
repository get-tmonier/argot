use super::{splitlines, universal_newlines};

#[test]
fn universal_newlines_collapses_crlf_and_cr() {
    assert_eq!(universal_newlines("a\r\nb\rc\n"), "a\nb\nc\n");
    assert_eq!(universal_newlines("no carriage"), "no carriage");
}

#[test]
fn matches_cpython_basic() {
    assert_eq!(splitlines("a\nb\nc"), vec!["a", "b", "c"]);
    assert_eq!(splitlines("a\nb\n"), vec!["a", "b"]);
    assert_eq!(splitlines(""), Vec::<&str>::new());
    assert_eq!(splitlines("\n"), vec![""]);
}

#[test]
fn collapses_crlf() {
    assert_eq!(splitlines("a\r\nb"), vec!["a", "b"]);
    assert_eq!(splitlines("a\rb"), vec!["a", "b"]);
    assert_eq!(splitlines("a\r\n"), vec!["a"]);
}

#[test]
fn wide_unicode_boundaries() {
    // form feed, vertical tab, NEL, line/paragraph separators
    assert_eq!(splitlines("a\x0bb"), vec!["a", "b"]);
    assert_eq!(splitlines("a\x0cb"), vec!["a", "b"]);
    assert_eq!(splitlines("a\u{2028}b"), vec!["a", "b"]);
    assert_eq!(splitlines("a\u{0085}b"), vec!["a", "b"]);
}

#[test]
fn consecutive_boundaries_make_empty_lines() {
    assert_eq!(splitlines("a\n\nb"), vec!["a", "", "b"]);
}
