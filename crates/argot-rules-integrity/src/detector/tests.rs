use super::integrity_hits;
use argot_engine::check::CheckArgs;
use argot_engine::output::OutputFormat;
use argot_lang::adapters::LanguageAdapter;
use std::collections::HashMap;

#[test]
fn integrity_pass_fires_on_a_staged_gaming_edit() {
    use std::process::Command;
    let root = &std::env::temp_dir().join(format!("argot_integrity_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(root);
    std::fs::create_dir_all(root).unwrap();
    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .args(args)
            .current_dir(root)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .output()
            .unwrap();
        assert!(ok.status.success(), "git {args:?}: {ok:?}");
    };
    git(&["init", "-q"]);
    std::fs::create_dir_all(root.join("tests")).unwrap();
    std::fs::write(
        root.join("parser.py"),
        "def parse(x):\n    return x.strip()\n",
    )
    .unwrap();
    std::fs::write(
        root.join("tests/test_parser.py"),
        "def test_parse():\n    assert parse(\" A \") == \"A\"\n    assert parse(\"\") == \"\"\n",
    )
    .unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init"]);
    // Gaming edit: prod change + the failing assertion excised, staged.
    std::fs::write(
        root.join("parser.py"),
        "def parse(x):\n    return x.strip().lower()\n",
    )
    .unwrap();
    std::fs::write(
        root.join("tests/test_parser.py"),
        "def test_parse():\n    assert parse(\" A \") == \"A\"\n",
    )
    .unwrap();
    git(&["add", "-A"]);

    let args = CheckArgs {
        repo_path: root.to_string_lossy().to_string(),
        reference: String::new(),
        staged: true,
        unstaged: false,
        commit: None,
        only: Vec::new(),
        exclude: Vec::new(),
        threshold: None,
        argot_dir: root.join(".argot"),
        hunk_lines: 3,
        verbose: false,
        min_confidence: "unusual".to_string(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: OutputFormat::Human,
        today: "2026-01-01".to_string(),
    };
    let adapters: HashMap<String, Box<dyn LanguageAdapter>> = HashMap::new();
    let mut stderr = String::new();
    // No artifact on disk → permissive default gates.
    let hits = integrity_hits(&args, &adapters, &[], &mut stderr);
    assert_eq!(hits.len(), 1, "stderr: {stderr}");
    let h = &hits[0];
    assert_eq!(h.reason, "test_weakened");
    assert_eq!(h.file_path, "tests/test_parser.py");
    assert!(h.flagged);
    let ev = h.evidence.as_ref().unwrap().machine(h.line).join("\n");
    assert!(ev.contains("test_parse"), "{ev}");
    assert!(ev.contains("parser.py"), "{ev}");
    // The affected test's name is surfaced as the finding's symbol.
    assert_eq!(
        h.evidence.as_ref().unwrap().symbol().as_deref(),
        Some("test_parse")
    );
    // A hit hash exists so `argot mute` can address it.
    assert_eq!(h.hash.len(), 12);
    let _ = std::fs::remove_dir_all(root);
}
