#[cfg(feature = "arch")]
use super::arch::arch_evidence;
#[cfg(feature = "integrity")]
use super::integrity::integrity_hits;
#[cfg(feature = "semantic")]
use super::semantic::{format_semantic_evidence, SemanticHitEvidence};
#[cfg(feature = "integrity")]
use crate::scoring::adapters::LanguageAdapter;
#[cfg(feature = "integrity")]
use argot_engine::check::CheckArgs;
#[cfg(feature = "integrity")]
use argot_engine::output::OutputFormat;
#[cfg(feature = "integrity")]
use std::collections::HashMap;

#[test]
#[cfg(feature = "arch")]
fn arch_evidence_names_the_broken_direction() {
    use crate::scoring::arch_graph::Violation;
    let edge = ("core".to_string(), "cli".to_string());
    assert_eq!(
        arch_evidence(&edge, Violation::Reversal),
        "cli → core is this repo's direction — this import reverses it"
    );
    assert!(arch_evidence(&edge, Violation::TransitiveReversal).contains("closes a cycle"));
    assert!(arch_evidence(&edge, Violation::SinkOut).contains("never imports out of"));
}

#[test]
#[cfg(feature = "integrity")]
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

#[cfg(feature = "semantic")]
#[test]
fn semantic_evidence_renders_nearest_code() {
    let redundant = SemanticHitEvidence::Redundant {
        nearest_symbol: "slugify".into(),
        nearest_path: "src/utils/text.py".into(),
        nearest_line: 1,
        similarity: 0.86,
    };
    let lines = format_semantic_evidence(&redundant, false);
    assert!(lines[0].contains("duplicates slugify (src/utils/text.py:1)"));
    assert!(lines[0].contains("0.86"));

    let misplaced = SemanticHitEvidence::Misplaced {
        neighbor_area: "src/db".into(),
        actual_area: "src/ui".into(),
        peers: vec![("load_row".into(), "src/db/models.py".into(), 12)],
    };
    let lines = format_semantic_evidence(&misplaced, false);
    assert!(lines[0].contains("looks like src/db code filed under src/ui"));
    assert!(lines[1].contains("load_row (src/db/models.py:12)"));
}
