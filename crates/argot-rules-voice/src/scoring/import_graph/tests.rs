use super::*;

#[test]
fn foreign_imports_are_counted() {
    let adapter = PythonAdapter::new();
    let mut scorer = ImportGraphScorer::new();
    scorer.fit(&["import os\nimport json\n".to_string()], &adapter);

    // os is known, requests is foreign.
    assert!(!scorer.is_foreign("os"));
    assert!(scorer.is_foreign("requests"));
    assert_eq!(scorer.score_hunk("import os\n", &adapter), 0.0);
    assert_eq!(scorer.score_hunk("import requests\n", &adapter), 1.0);
    assert_eq!(
        scorer.score_hunk("import requests\nimport flask\n", &adapter),
        2.0
    );
}

#[test]
fn snapshot_and_prefixes() {
    let mut scorer = ImportGraphScorer::new();
    scorer.load_snapshot(["numpy".to_string()], ["myapp".to_string()]);
    assert!(!scorer.is_foreign("numpy"));
    assert!(!scorer.is_foreign("myapp.sub")); // prefix match
    assert!(scorer.is_foreign("pandas"));
}

#[test]
fn future_is_never_foreign() {
    // `from __future__ import annotations` is a compiler directive, not a
    // dependency — a repo that never used it must not flag it as foreign.
    let scorer = ImportGraphScorer::new(); // empty repo_modules
    assert!(!scorer.is_foreign("__future__"));
    assert!(scorer.is_foreign("requests"));
}
