use super::*;

#[test]
fn language_serialises_to_lowercase_literals() {
    assert_eq!(
        serde_json::to_string(&Language::Typescript).unwrap(),
        "\"typescript\""
    );
    assert_eq!(
        serde_json::to_string(&Language::Python).unwrap(),
        "\"python\""
    );
}

#[test]
fn none_parent_sha_serialises_as_null() {
    let rec = HunkRecord {
        commit_sha: "abc".into(),
        file_path: "a.py".into(),
        language: Language::Python,
        hunk_start_line: 0,
        hunk_end_line: 1,
        context_before: vec![],
        hunk_tokens: vec![],
        context_after: vec![],
        parent_sha: None,
        author_date_iso: "0".into(),
    };
    let s = serde_json::to_string(&rec).unwrap();
    assert!(s.contains("\"parent_sha\":null"), "got: {s}");
}
