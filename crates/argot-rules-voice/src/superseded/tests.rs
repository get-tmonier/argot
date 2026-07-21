use super::*;
use argot_engine::config::ArgotConfig;
use argot_engine::git_walk::HunkSpan;
use argot_lang::adapters::adapter_for;

fn batch(path: &str, content: &str, hunks: Vec<HunkSpan>) -> PatchBatch {
    PatchBatch {
        file_path: path.to_string(),
        content: content.as_bytes().to_vec(),
        hunks,
        source: "workdir".to_string(),
        ignored_by_pattern: false,
    }
}

fn adapters() -> HashMap<String, Box<dyn LanguageAdapter>> {
    let mut m: HashMap<String, Box<dyn LanguageAdapter>> = HashMap::new();
    m.insert("python".to_string(), adapter_for("python").unwrap());
    m
}

fn python_supersession() -> HashMap<String, Vec<Supersession>> {
    let mut m = HashMap::new();
    m.insert(
        "python".to_string(),
        vec![Supersession {
            old: "oldlib".into(),
            new: "newlib".into(),
            kind: SupersessionKind::Import,
            commits: 4,
            files: 9,
            first: "2026-01-05".into(),
            last: "2026-03-02".into(),
            example_commit: "abc1234".into(),
            leftover_count: 2,
            leftovers: vec!["src/a.py".into(), "src/b.py".into()],
        }],
    );
    m
}

fn scan(
    batches: &[PatchBatch],
    supersessions: &HashMap<String, Vec<Supersession>>,
    migrations: &[MigrationRule],
) -> Vec<Finding> {
    let config = ArgotConfig::default();
    let settings = config.rule_settings(&Vec::new());
    superseded_findings(
        batches,
        supersessions,
        migrations,
        &adapters(),
        &[],
        Registry::builtin(),
        &settings,
        false,
    )
}

#[test]
fn mined_supersession_fires_on_added_import_with_evidence() {
    let batches = vec![batch(
        "src/new.py",
        "import oldlib\n\ndef f():\n    return oldlib.go()\n",
        vec![HunkSpan {
            new_start: 1,
            new_lines: 4,
        }],
    )];
    let findings = scan(&batches, &python_supersession(), &[]);
    assert_eq!(findings.len(), 1);
    let f = &findings[0];
    assert_eq!(f.reason, "superseded");
    assert_eq!(f.line, 1);
    assert!(f.flagged);
    let evidence = f.evidence.as_ref().unwrap().machine(1).join(" ");
    assert!(evidence.contains("replaced 'oldlib' with 'newlib'"));
    assert!(evidence.contains("4 commits across 9 files"));
    assert!(evidence.contains("abc1234"));
    assert_eq!(
        f.evidence.as_ref().unwrap().symbol().as_deref(),
        Some("oldlib")
    );
}

#[test]
fn one_finding_per_pattern_per_changeset() {
    let batches = vec![
        batch(
            "src/one.py",
            "import oldlib\n",
            vec![HunkSpan {
                new_start: 1,
                new_lines: 1,
            }],
        ),
        batch(
            "src/two.py",
            "import oldlib\n",
            vec![HunkSpan {
                new_start: 1,
                new_lines: 1,
            }],
        ),
    ];
    let findings = scan(&batches, &python_supersession(), &[]);
    assert_eq!(findings.len(), 1);
}

#[test]
fn untouched_lines_never_fire() {
    let batches = vec![batch(
        "src/new.py",
        "import oldlib\n\ndef f():\n    return 1\n",
        vec![HunkSpan {
            new_start: 3,
            new_lines: 2,
        }],
    )];
    assert!(scan(&batches, &python_supersession(), &[]).is_empty());
}

#[test]
fn declared_migration_fires_with_reason() {
    let migrations = vec![MigrationRule {
        from: "moment".into(),
        to: "datefns".into(),
        reason: "Q2 refactor".into(),
        kind: MigrationKind::Import,
    }];
    let batches = vec![batch(
        "src/new.py",
        "import moment\n",
        vec![HunkSpan {
            new_start: 1,
            new_lines: 1,
        }],
    )];
    let findings = scan(&batches, &HashMap::new(), &migrations);
    assert_eq!(findings.len(), 1);
    let evidence = findings[0].evidence.as_ref().unwrap().machine(1).join(" ");
    assert!(evidence.contains("argot.toml migration to 'datefns'"));
    assert!(evidence.contains("Q2 refactor"));
}

#[test]
fn callee_kind_matches_calls_not_imports() {
    let mut supersessions = HashMap::new();
    supersessions.insert(
        "python".to_string(),
        vec![Supersession {
            old: "legacy_render".into(),
            new: "render_v2".into(),
            kind: SupersessionKind::Callee,
            commits: 3,
            files: 5,
            first: "2026-01-05".into(),
            last: "2026-03-02".into(),
            example_commit: "abc1234".into(),
            leftover_count: 1,
            leftovers: vec!["src/a.py".into()],
        }],
    );
    let hit = vec![batch(
        "src/new.py",
        "def f():\n    return legacy_render(1)\n",
        vec![HunkSpan {
            new_start: 1,
            new_lines: 2,
        }],
    )];
    assert_eq!(scan(&hit, &supersessions, &[]).len(), 1);

    let mention_only = vec![batch(
        "src/other.py",
        "NAMES = [\"legacy_render\"]\n",
        vec![HunkSpan {
            new_start: 1,
            new_lines: 1,
        }],
    )];
    assert!(scan(&mention_only, &supersessions, &[]).is_empty());
}

#[test]
fn unrelated_code_is_silent() {
    let batches = vec![batch(
        "src/new.py",
        "import json\n\ndef f():\n    return json.dumps({})\n",
        vec![HunkSpan {
            new_start: 1,
            new_lines: 4,
        }],
    )];
    assert!(scan(&batches, &python_supersession(), &[]).is_empty());
}
