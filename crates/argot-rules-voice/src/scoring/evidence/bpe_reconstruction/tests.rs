use super::*;

#[test]
fn reconstruct_expands_to_identifier_boundaries() {
    let src = "def connect(url):";
    // A span landing inside `connect` (bytes 5..8 = "onn") expands to the
    // whole identifier; a punctuation-only span is dropped.
    let ids = reconstruct_identifiers(src, &[(5, 8), (11, 12)]);
    assert_eq!(ids, vec!["connect".to_string()]);
}

#[test]
fn reconstruct_dedups_in_order() {
    let src = "a bb a";
    let ids = reconstruct_identifiers(src, &[(0, 1), (2, 3), (5, 6)]);
    assert_eq!(ids, vec!["a".to_string(), "bb".to_string()]);
}
