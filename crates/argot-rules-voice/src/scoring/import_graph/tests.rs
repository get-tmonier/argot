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

#[test]
fn a_module_the_changeset_declares_stops_reading_as_foreign() {
    // The fit-time snapshot cannot know about a module the change itself adds.
    // Without widening it, a port that introduces three units and wires them
    // into six files reads as six foreign dependencies — the opposite of the
    // signal, and precisely on the change a guardrail should be quiet about.
    let mut s = ImportGraphScorer::new();
    s.load_snapshot(["msetypes".to_string(), "sysutils".to_string()], []);

    assert!(
        s.is_foreign("mwayland"),
        "unknown before the changeset says so"
    );
    s.extend_known(["mwayland".to_string()]);
    assert!(!s.is_foreign("mwayland"), "declared by the changeset");

    // And the hole this must not reopen: a genuine new dependency is never
    // declared by a file in the diff, so it stays foreign.
    assert!(s.is_foreign("fphttpclient"));
}
