use super::*;
use crate::scoring::adapters::python::PythonAdapter;

#[test]
fn rarity_weight_math() {
    assert_eq!(RarityWeighting::Off.weight(1, 100), 1.0);
    assert_eq!(RarityWeighting::LinearDf.weight(50, 100), 0.5);
    assert_eq!(RarityWeighting::LinearDf.weight(0, 0), 1.0);
    assert_eq!(RarityWeighting::GatedDf { min_df: 3 }.weight(2, 100), 0.0);
    assert_eq!(RarityWeighting::GatedDf { min_df: 3 }.weight(3, 100), 1.0);
    let w = RarityWeighting::LogDf.weight(100, 100);
    assert!(w > 0.99 && w <= 1.0, "log weight near 1 for df=N, got {w}");
    let w1 = RarityWeighting::LogDf.weight(1, 100);
    assert!(w1 < 0.2, "log weight small for df=1, got {w1}");
}

fn toy_scorer(weighting: RarityWeighting) -> CallReceiverScorer {
    let adapter = PythonAdapter::new();
    // Two stylistically distinct groups so k-means separates them: the
    // "alpha" files call foo/bar everywhere, the "beta" files call
    // baz/qux. `shared()` appears in exactly one file (globally rare).
    let mut files: Vec<(PathBuf, String)> = Vec::new();
    for i in 0..6 {
        files.push((
            PathBuf::from(format!("a{i}.py")),
            "def f():\n    foo()\n    bar()\n".to_string(),
        ));
    }
    for i in 0..6 {
        files.push((
            PathBuf::from(format!("b{i}.py")),
            "def g():\n    baz()\n    qux()\n".to_string(),
        ));
    }
    files.push((
        PathBuf::from("rare.py"),
        "def h():\n    rare_helper()\n".to_string(),
    ));
    CallReceiverScorer::new(&files, Language::Python, 2.0, 5, &adapter, 4, 0, 0, 0, 0.65)
        .unwrap()
        .with_rarity_weighting(weighting)
}

#[test]
fn df_counts_are_per_file_presence() {
    let cr = toy_scorer(RarityWeighting::Off);
    assert_eq!(cr.n_corpus_files(), 13);
    assert_eq!(cr.callee_file_count("foo"), 6);
    assert_eq!(cr.callee_file_count("rare_helper"), 1);
    assert_eq!(cr.callee_file_count("never_seen"), 0);
}

#[test]
fn as_new_fires_alpha_on_singleton_df_callees() {
    // File-level LOO: a callee whose only corpus container is the held-out
    // file (df == 1) is unattested once that file is treated as newly added,
    // so it fires the global alpha branch; a widely-shared callee (df >= 2)
    // stays attested and, with cluster routing off for new files,
    // contributes nothing. This asymmetry lifts the new-file threshold above
    // the existing-file one (issue #92 new-file flooding).
    let cr = toy_scorer(RarityWeighting::Off);
    // rare_helper has df == 1 (only rare.py) → alpha fires as-new.
    let singleton = cr.weighted_contribution_as_new(
        "rare_helper()\n",
        Some(Path::new("rare.py")),
        2.0,
        2.0,
        5.0,
        5.0,
        None,
        &Default::default(),
    );
    // Alpha fires (>= 2.0), capped at 5.0. Root attestation stays global, so
    // a singleton bare callee resolves to UnattestedKnownRoot (alpha +
    // root_bonus = 4.0) — a bounded, conservative over-estimate that only
    // raises the new-file bar.
    assert!(
        (2.0..=5.0).contains(&singleton),
        "singleton-df callee fires alpha as-new, got {singleton}"
    );
    // foo has df == 6 → attested by other files → no alpha; cluster off → 0.
    let shared = cr.weighted_contribution_as_new(
        "foo()\n",
        Some(Path::new("a0.py")),
        2.0,
        2.0,
        5.0,
        5.0,
        None,
        &Default::default(),
    );
    assert_eq!(
        shared, 0.0,
        "widely-shared callee contributes nothing as-new"
    );
}

#[test]
fn explicit_foreign_namespace_fires_only_on_unattested_qualified_callees() {
    use crate::scoring::adapters::php::PhpAdapter;
    let adapter = PhpAdapter::new();
    // The repo attests the `\Known\Ns` namespace.
    let files: Vec<(PathBuf, String)> = (0..3)
        .map(|i| {
            (
                PathBuf::from(format!("f{i}.php")),
                "<?php\nfunction f() {\n    return \\Known\\Ns\\Thing::make();\n}\n".to_string(),
            )
        })
        .collect();
    let cr =
        CallReceiverScorer::new(&files, Language::Php, 2.0, 5, &adapter, 4, 0, 0, 0, 0.65).unwrap();
    let local = LocalBindings::default();
    // Explicit foreign namespace the repo never uses → detected.
    assert!(cr.hunk_names_explicit_foreign_namespace(
        "<?php\n$v = \\Respect\\Validation\\Validator::key($a);\n",
        None,
        &local,
    ));
    // A namespace the repo attests → not foreign.
    assert!(!cr.hunk_names_explicit_foreign_namespace(
        "<?php\n$v = \\Known\\Ns\\Thing::make();\n",
        None,
        &local,
    ));
    // A bare call (no explicit namespace separator) never qualifies for the
    // threshold-independent fire, ambiguous global or not.
    assert!(!cr.hunk_names_explicit_foreign_namespace("<?php\nstrlen($x);\n", None, &local,));
}

#[test]
fn callee_extraction_covers_member_arrow_and_qualified() {
    // The C++ callee extractor lives here in `call_receiver`; assert its
    // output routes through the `Language::Cpp` arm end-to-end. Lives
    // here (rather than the `CppAdapter`'s own tests, in argot-lang)
    // because argot-lang is a leaf crate and cannot depend on argot-core.
    let src = "void f() {\n    obj.method();\n    ptr->run();\n    ns::make();\n    bare();\n}\n";
    let callees: Vec<String> = extract_callees(src, Language::Cpp)
        .into_iter()
        .flatten()
        .collect();
    assert!(callees.contains(&"obj.method".to_string()));
    assert!(callees.contains(&"ptr.run".to_string()));
    assert!(callees.contains(&"ns.make".to_string()));
    assert!(callees.contains(&"bare".to_string()));
}

#[test]
fn knows_file_tracks_fit_membership() {
    let cr = toy_scorer(RarityWeighting::Off);
    assert!(cr.knows_file(Path::new("a0.py")), "fit file is known");
    assert!(
        !cr.knows_file(Path::new("brand_new.py")),
        "unseen file is a new file"
    );
}

#[test]
fn gated_df_at_one_matches_off_behaviour() {
    // Every globally-attested callee has df >= 1, so GatedDf{min_df: 1}
    // must reproduce the baseline (rule-off) contributions exactly on any hunk.
    let mut off = toy_scorer(RarityWeighting::Off);
    let mut gated = toy_scorer(RarityWeighting::GatedDf { min_df: 1 });
    for hunk in [
        "rare_helper()\nfoo()\n",
        "baz()\n",
        "unknown_callee()\n",
        "foo()\nbar()\nbaz()\nqux()\n",
    ] {
        let a = off.weighted_contribution_for_file(
            hunk,
            Some(Path::new("a0.py")),
            2.0,
            2.0,
            5.0,
            5.0,
            None,
            None,
            &Default::default(),
        );
        let b = gated.weighted_contribution_for_file(
            hunk,
            Some(Path::new("a0.py")),
            2.0,
            2.0,
            5.0,
            5.0,
            None,
            None,
            &Default::default(),
        );
        assert_eq!(a, b, "hunk {hunk:?}");
    }
}

#[test]
fn linear_df_shrinks_rare_callee_cluster_contribution() {
    // `rare_helper` is globally attested (df=1) — in a file from the
    // foo/bar cluster it takes a cluster branch. Under LinearDf its
    // bonus is scaled by 1/13; alpha branches are untouched.
    let mut off = toy_scorer(RarityWeighting::Off);
    let mut lin = toy_scorer(RarityWeighting::LinearDf);
    let hunk = "rare_helper()\n";
    let a = off.weighted_contribution_for_file(
        hunk,
        Some(Path::new("a0.py")),
        0.0,
        0.0,
        5.0,
        10.0,
        None,
        None,
        &Default::default(),
    );
    let b = lin.weighted_contribution_for_file(
        hunk,
        Some(Path::new("a0.py")),
        0.0,
        0.0,
        5.0,
        10.0,
        None,
        None,
        &Default::default(),
    );
    if a > 0.0 {
        // Cluster branch fired: weighting must shrink it by df/N.
        let expected = a * (1.0 / 13.0);
        assert!(
            (b - expected).abs() < 1e-9,
            "expected {expected}, got {b} (off={a})"
        );
    } else {
        // Cluster assignment put rare.py with a0.py; nothing fired for
        // either mode — invariant still holds.
        assert_eq!(b, 0.0);
    }
}

#[test]
fn parse_error_hunk_falls_back_to_host_region_callees() {
    // A bare `elif` fragment has root-level parse errors in Python.
    let hunk = "elif validate_payload(data):\n    send_alert(data)";
    assert!(has_root_error(hunk, Language::Python));
    let host = "def handler(data):\n    if data is None:\n        return\n    elif validate_payload(data):\n        send_alert(data)\n";
    let callees = callees_in_source_region(host, Language::Python, 4, 5);
    assert_eq!(callees, vec!["validate_payload", "send_alert"]);

    let cr = toy_scorer(RarityWeighting::Off);
    // Without host context: parse error blocks everything (baseline behaviour).
    assert!(cr
        .contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            None,
            &Default::default()
        )
        .is_empty());
    // With host context: both (unattested) callees produce events.
    let events = cr.contribution_events_for_file(
        hunk,
        Some(Path::new("a0.py")),
        None,
        Some((host, 4, 5)),
        &Default::default(),
    );
    assert_eq!(events.len(), 2);
    assert!(events
        .iter()
        .all(|e| matches!(e.branch, ContributionBranch::Unattested)));
}

#[test]
fn model_roundtrip_preserves_contribution_decisions() {
    // Export → import must reproduce every contribution decision, both
    // for path-routed hunks (file in a fit-time cluster) and for
    // source-routed hunks (unknown file → Jaccard-nearest cluster).
    let original = toy_scorer(RarityWeighting::Off);
    let model = original.export_model(Path::new(""));
    assert_eq!(model.n_corpus_files, 13);
    assert!(model.attested.contains(&"foo".to_string()));
    let restored = CallReceiverScorer::from_model(&model, Language::Python, 2.0, 5, 0, 0).unwrap();

    let unknown_file_source = "def h():\n    baz()\n    qux()\n";
    for hunk in [
        "rare_helper()\nfoo()\n",
        "unknown_callee()\nbaz()\n",
        "foo()\nbar()\nbaz()\nqux()\n",
    ] {
        let a = original.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            None,
            &Default::default(),
        );
        let b = restored.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            None,
            &Default::default(),
        );
        assert_eq!(a.len(), b.len(), "path-routed event count for {hunk:?}");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.callee, y.callee);
            assert_eq!(x.branch, y.branch);
        }
        let a = original.contribution_events_for_file(
            hunk,
            Some(Path::new("never_seen.py")),
            Some(unknown_file_source),
            None,
            &Default::default(),
        );
        let b = restored.contribution_events_for_file(
            hunk,
            Some(Path::new("never_seen.py")),
            Some(unknown_file_source),
            None,
            &Default::default(),
        );
        assert_eq!(a.len(), b.len(), "source-routed event count for {hunk:?}");
        for (x, y) in a.iter().zip(b.iter()) {
            assert_eq!(x.callee, y.callee);
            assert_eq!(x.branch, y.branch);
        }
    }

    // Document frequencies are recovered from the cluster sums.
    assert_eq!(restored.callee_file_count("foo"), 6);
    assert_eq!(restored.callee_file_count("rare_helper"), 1);
    assert_eq!(restored.n_corpus_files(), 13);
}

#[test]
fn host_context_is_ignored_when_bare_hunk_parses() {
    // G4.d invariant: a cleanly-parsing hunk scores identically with and
    // without host context — the fallback is purely additive on the
    // parse-error path.
    let cr = toy_scorer(RarityWeighting::Off);
    let hunk = "foo()\nbar()\n";
    // Deliberately contradictory host region (would yield zero callees).
    let host = "x = 1\ny = 2\n";
    let without = cr.contribution_events_for_file(
        hunk,
        Some(Path::new("a0.py")),
        None,
        None,
        &Default::default(),
    );
    let with_host = cr.contribution_events_for_file(
        hunk,
        Some(Path::new("a0.py")),
        None,
        Some((host, 1, 2)),
        &Default::default(),
    );
    assert_eq!(without.len(), with_host.len());
    for (a, b) in without.iter().zip(with_host.iter()) {
        assert_eq!(a.callee, b.callee);
        assert_eq!(a.branch, b.branch);
    }
}
