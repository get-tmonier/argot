use super::*;

#[test]
fn a_whole_file_rewrite_is_not_one_pattern_being_introduced() {
    // uos' "Comment reordered for all the functions" changed 2 564 lines of one
    // file in a single hunk. A hunk that size holds most of the file's
    // vocabulary, so something in it is always unfamiliar.
    assert!(is_oversized(false, 1, 2564));
    assert!(is_oversized(false, 268, 1375));
}

#[test]
fn a_new_file_is_never_oversized() {
    // There the whole file legitimately *is* the change, and it is already
    // judged against the new-file threshold rather than an edit distribution.
    // uos added a real 1 495-line decoder this way — it must stay catchable.
    assert!(!is_oversized(true, 1, 1495));
    assert!(!is_oversized(true, 1, 12798));
}

#[test]
fn every_catalogued_fixture_stays_under_the_cap() {
    // The largest fixture in the whole catalogue is 80 lines (n=977, median 13,
    // p99 59), so the cap costs no measurable recall. Pin the margin: if a
    // fixture ever grows past it, this fails rather than silently going unjudged.
    assert!(
        !is_oversized(false, 1, 80),
        "the largest fixture must still be judged"
    );
    assert!(!is_oversized(false, 1, MAX_SCORED_HUNK_LINES));
    assert!(is_oversized(false, 1, MAX_SCORED_HUNK_LINES + 1));
}
