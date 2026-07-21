use super::*;
use argot_lang::adapters::adapter_for;

fn commit(sha: &str, date: &str, files: &[(&str, &[&str], &[&str])]) -> CommitDelta {
    CommitDelta {
        sha: sha.to_string(),
        date: date.to_string(),
        files: files
            .iter()
            .map(|(path, removed, added)| FileSpecDelta {
                path: path.to_string(),
                removed: removed.iter().map(|s| s.to_string()).collect(),
                added: added.iter().map(|s| s.to_string()).collect(),
            })
            .collect(),
    }
}

fn migration_stream() -> Vec<CommitDelta> {
    vec![
        commit(
            "aaa1111",
            "2026-01-05",
            &[("src/a.py", &["oldlib"], &["newlib"])],
        ),
        commit(
            "bbb2222",
            "2026-01-12",
            &[("src/b.py", &["oldlib"], &["newlib"])],
        ),
        commit(
            "ccc3333",
            "2026-02-02",
            &[("src/c.py", &["oldlib"], &["newlib"])],
        ),
    ]
}

#[test]
fn true_migration_is_mined_with_evidence() {
    let mined = mine_pairs(&migration_stream());
    assert_eq!(mined.len(), 1);
    let p = &mined[0];
    assert_eq!((p.old.as_str(), p.new.as_str()), ("oldlib", "newlib"));
    assert_eq!((p.commits, p.files), (3, 3));
    assert_eq!(
        (p.first.as_str(), p.last.as_str()),
        ("2026-01-05", "2026-02-02")
    );
    assert_eq!(p.example_commit, "aaa1111");
}

#[test]
fn below_support_is_silent() {
    let mut commits = migration_stream();
    commits.pop();
    assert!(mine_pairs(&commits).is_empty());
}

#[test]
fn one_file_churn_is_not_a_convention() {
    let commits = vec![
        commit("a", "2026-01-01", &[("src/a.py", &["oldlib"], &["newlib"])]),
        commit("b", "2026-01-02", &[("src/a.py", &["oldlib"], &["newlib"])]),
        commit("c", "2026-01-03", &[("src/a.py", &["oldlib"], &["newlib"])]),
    ];
    assert!(mine_pairs(&commits).is_empty());
}

#[test]
fn symmetric_flapping_is_noise() {
    let mut commits = migration_stream();
    commits.push(commit(
        "d",
        "2026-02-10",
        &[("src/d.py", &["newlib"], &["oldlib"])],
    ));
    commits.push(commit(
        "e",
        "2026-02-11",
        &[("src/e.py", &["newlib"], &["oldlib"])],
    ));
    assert!(mine_pairs(&commits).is_empty());
}

#[test]
fn still_actively_added_old_is_not_superseded() {
    let mut commits = migration_stream();
    for (i, (path, date)) in [
        ("src/d.py", "2026-02-10"),
        ("src/e.py", "2026-02-11"),
        ("src/f.py", "2026-02-12"),
        ("src/g.py", "2026-02-13"),
    ]
    .iter()
    .enumerate()
    {
        commits.push(commit(&format!("x{i}"), date, &[(path, &[], &["oldlib"])]));
    }
    assert!(mine_pairs(&commits).is_empty());
}

#[test]
fn refactor_sink_absorbing_many_specs_is_dropped() {
    let mut commits = Vec::new();
    for (i, x) in ["liba", "libb", "libc"].iter().enumerate() {
        for j in 0..3 {
            commits.push(commit(
                &format!("s{i}{j}"),
                "2026-01-10",
                &[(&format!("src/{x}_{j}.py"), &[*x], &["sink"])],
            ));
        }
    }
    assert!(mine_pairs(&commits).is_empty());
}

#[test]
fn ambiguous_replacement_needs_a_dominant_target() {
    let mut commits = Vec::new();
    for j in 0..3 {
        commits.push(commit(
            &format!("y1{j}"),
            "2026-01-10",
            &[(&format!("src/a{j}.py"), &["oldlib"], &["newlib"])],
        ));
    }
    for j in 0..2 {
        commits.push(commit(
            &format!("y2{j}"),
            "2026-01-20",
            &[(&format!("src/b{j}.py"), &["oldlib"], &["otherlib"])],
        ));
    }
    assert!(mine_pairs(&commits).is_empty());

    for j in 2..6 {
        commits.push(commit(
            &format!("y1{j}"),
            "2026-02-01",
            &[(&format!("src/a{j}.py"), &["oldlib"], &["newlib"])],
        ));
    }
    let mined = mine_pairs(&commits);
    assert_eq!(mined.len(), 1);
    assert_eq!(mined[0].new, "newlib");
}

#[test]
fn bulk_rewrites_do_not_pair() {
    let many: Vec<String> = (0..8).map(|i| format!("lib{i}")).collect();
    let many_refs: Vec<&str> = many.iter().map(String::as_str).collect();
    let commits: Vec<CommitDelta> = (0..3)
        .map(|i| {
            commit(
                &format!("m{i}"),
                "2026-01-10",
                &[(&format!("src/f{i}.py"), &many_refs[..], &["newlib"])],
            )
        })
        .collect();
    assert!(mine_pairs(&commits).is_empty());
}

#[test]
fn callee_pairs_below_the_distinctiveness_bar_are_dropped() {
    let deltas = LanguageDeltas {
        imports: Vec::new(),
        callees: vec![
            commit("a", "2026-01-01", &[("src/a.py", &["run"], &["run_async"])]),
            commit("b", "2026-01-02", &[("src/b.py", &["run"], &["run_async"])]),
            commit("c", "2026-01-03", &[("src/c.py", &["run"], &["run_async"])]),
        ],
    };
    assert!(mine_language(&deltas).is_empty());
}

#[test]
fn attach_drops_completed_migrations_and_lists_leftovers() {
    let adapter = adapter_for("python").unwrap();
    let pairs = vec![(
        SupersessionKind::Import,
        MinedPair {
            old: "oldlib".into(),
            new: "newlib".into(),
            commits: 3,
            files: 3,
            first: "2026-01-05".into(),
            last: "2026-02-02".into(),
            example_commit: "aaa1111".into(),
        },
    )];
    let corpus_done: Vec<(String, String)> = vec![
        ("src/a.py".into(), "import newlib\n".into()),
        ("src/b.py".into(), "import newlib\n".into()),
    ];
    let done = attach_leftovers(
        pairs.clone(),
        &corpus_done,
        adapter.as_ref(),
        Language::Python,
    );
    assert!(done.is_empty());

    let corpus_left: Vec<(String, String)> = vec![
        ("src/a.py".into(), "import newlib\n".into()),
        ("src/legacy.py".into(), "import oldlib\n".into()),
    ];
    let left = attach_leftovers(pairs, &corpus_left, adapter.as_ref(), Language::Python);
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].leftover_count, 1);
    assert_eq!(left[0].leftovers, vec!["src/legacy.py".to_string()]);
}

#[test]
fn attach_drops_ubiquitous_callee_old_sides() {
    let adapter = adapter_for("python").unwrap();
    let pairs = vec![(
        SupersessionKind::Callee,
        MinedPair {
            old: "process".into(),
            new: "process_async".into(),
            commits: 3,
            files: 3,
            first: "2026-01-05".into(),
            last: "2026-02-02".into(),
            example_commit: "aaa1111".into(),
        },
    )];
    let corpus: Vec<(String, String)> = (0..4)
        .map(|i| (format!("src/f{i}.py"), "process()\n".to_string()))
        .collect();
    assert!(attach_leftovers(pairs, &corpus, adapter.as_ref(), Language::Python).is_empty());
}

#[test]
fn iso_date_converts_unix_seconds() {
    assert_eq!(iso_date(0), "1970-01-01");
    assert_eq!(iso_date(1_735_689_600), "2025-01-01");
}

#[test]
fn supersession_round_trips_through_json() {
    let s = Supersession {
        old: "oldlib".into(),
        new: "newlib".into(),
        kind: SupersessionKind::Import,
        commits: 3,
        files: 3,
        first: "2026-01-05".into(),
        last: "2026-02-02".into(),
        example_commit: "aaa1111".into(),
        leftover_count: 1,
        leftovers: vec!["src/legacy.py".into()],
    };
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains("\"kind\":\"import\""));
    let back: Supersession = serde_json::from_str(&json).unwrap();
    assert_eq!(back, s);
}
