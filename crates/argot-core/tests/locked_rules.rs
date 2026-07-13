//! Behaviour of locked rules end to end: the committed lock freezes severity
//! against every runtime layer, refuses every suppression surface, and the
//! `rule-tampered` self-protection fires when the change itself weakens a
//! lock. Reuses the deterministic check fixture (BPE threshold pinned high →
//! exactly one foreign-import finding).
//!
//! Requires `git` and `bash` on PATH (fixture build).

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::output::OutputFormat;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("locked_rules_repo_{suffix}"));
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check/build_check_repo.sh");
    let status = Command::new(
        std::env::var_os("ARGOT_TEST_BASH")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "bash".into()),
    )
    .arg(&script)
    .arg(&out)
    .status()
    .expect("run build_check_repo.sh");
    assert!(status.success(), "fixture build failed");
    out
}

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn prepare_repo(suffix: &str) -> PathBuf {
    let repo = build_fixture_repo(suffix);
    git(&repo, &["checkout", "-q", "HEAD~1"]);
    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    argot_core::train::run_train(
        &repo,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )
    .expect("train");
    let opts = CalibrateOptions {
        repo_sha: "fixture".to_string(),
        timestamp_utc: "1970-01-01T00:00:00+00:00".to_string(),
        ..Default::default()
    };
    run_calibrate(
        &repo,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &opts,
    )
    .expect("calibrate");
    git(&repo, &["checkout", "-q", "main"]);
    let config_path = argot_dir.join("scorer-config.json");
    let raw = std::fs::read_to_string(&config_path).expect("read config");
    let mut config: serde_json::Value = serde_json::from_str(&raw).expect("parse config");
    for (_, lang_cfg) in config["languages"].as_object_mut().expect("languages") {
        lang_cfg["threshold"] = serde_json::json!(1000.0);
    }
    std::fs::write(&config_path, serde_json::to_string_pretty(&config).unwrap()).unwrap();
    repo
}

const LOCKED_TOML: &str = "[rules]\n\"foreign-import\" = { severity = \"error\", locked = true }\n";

fn args(repo: &Path) -> CheckArgs {
    CheckArgs {
        repo_path: repo.to_str().unwrap().to_string(),
        reference: "HEAD~1..HEAD".to_string(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: repo.join(".argot"),
        hunk_lines: 6,
        verbose: false,
        min_confidence: "unusual".to_string(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: OutputFormat::Human,
        today: "2026-01-01".to_string(),
    }
}

#[test]
fn locked_severity_ignores_local_toml_and_cli() {
    let repo = prepare_repo("freeze");
    std::fs::write(repo.join("argot.toml"), LOCKED_TOML).unwrap();
    // argot.local.toml tries to soften it.
    std::fs::write(
        repo.join("argot.local.toml"),
        "[rules]\n\"foreign-import\" = \"off\"\n",
    )
    .unwrap();
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 1, "still gates: {}", out.stderr);
    assert!(out.stdout.contains("foreign-import"), "{}", out.stdout);
    assert!(
        out.stderr
            .contains("locked in argot.toml — runtime override ignored"),
        "refusal surfaces: {}",
        out.stderr
    );
    // CLI tries next.
    std::fs::remove_file(repo.join("argot.local.toml")).unwrap();
    let mut a = args(&repo);
    a.rule_overrides = vec![(
        "foreign-import".to_string(),
        argot_core::rules::Severity::Off,
    )];
    let out = run_check(a);
    assert_eq!(out.exit_code, 1, "{}", out.stderr);
    assert!(out.stdout.contains("foreign-import"), "{}", out.stdout);
}

#[test]
fn locked_findings_refuse_mutes_and_inline_ignores() {
    let repo = prepare_repo("suppress");
    // A blanket mute that would normally silence the finding.
    std::fs::write(
        repo.join("argot.toml"),
        format!("{LOCKED_TOML}\n[[mute]]\npath = \"**\"\nrule = \"foreign-import\"\nreason = \"testing\"\n"),
    )
    .unwrap();
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 1, "mute refused: {}", out.stderr);
    assert!(out.stdout.contains("foreign-import"), "{}", out.stdout);
}

#[test]
fn removing_a_lock_in_the_diff_is_rule_tampered() {
    let repo = prepare_repo("tamper");
    // The lock is committed history…
    std::fs::write(repo.join("argot.toml"), LOCKED_TOML).unwrap();
    git(&repo, &["add", "argot.toml"]);
    git(&repo, &["commit", "-qm", "lock foreign-import"]);
    // …and the working diff removes it.
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\n\"foreign-import\" = \"off\"\n",
    )
    .unwrap();
    let mut a = args(&repo);
    a.reference = String::new(); // workdir mode: HEAD → working tree
    let out = run_check(a);
    assert_eq!(out.exit_code, 1, "tamper gates: {}", out.stderr);
    assert!(out.stdout.contains("rule-tampered"), "{}", out.stdout);
    assert!(out.stdout.contains("lock removed"), "{}", out.stdout);
    assert!(
        out.stderr.contains("weakens a locked guardrail"),
        "loud run-level warning: {}",
        out.stderr
    );
}

#[test]
fn weakening_a_locked_severity_is_rule_tampered_and_unsuppressable() {
    let repo = prepare_repo("weaken");
    std::fs::write(repo.join("argot.toml"), LOCKED_TOML).unwrap();
    git(&repo, &["add", "argot.toml"]);
    git(&repo, &["commit", "-qm", "lock"]);
    // Weaken in place AND try to mute the alarm itself in the same edit.
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\n\"foreign-import\" = { severity = \"warn\", locked = true }\n\n[[mute]]\npath = \"**\"\nrule = \"rule-tampered\"\nreason = \"nothing to see\"\n",
    )
    .unwrap();
    let mut a = args(&repo);
    a.reference = String::new();
    let out = run_check(a);
    assert_eq!(out.exit_code, 1, "{}", out.stderr);
    assert!(
        out.stdout.contains("error → warn") || out.stdout.contains("severity error"),
        "{}",
        out.stdout
    );
}

#[cfg(feature = "script")]
fn write_custom_rule(repo: &Path, name: &str, always_fire: bool) {
    let d = repo.join(".argot/rules").join(name);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        format!("[rule]\nschema = 1\nname = \"{name}\"\nseverity = \"error\"\n"),
    )
    .unwrap();
    let body = if always_fire {
        "report(1, \"custom fired\");"
    } else {
        "// no-op"
    };
    std::fs::write(d.join("check.rhai"), body).unwrap();
}

#[test]
#[cfg(feature = "script")]
fn locking_the_custom_group_freezes_and_protects_scripted_rules() {
    let repo = prepare_repo("customlock");
    write_custom_rule(&repo, "house-style", true);
    // Lock the whole custom group in the committed config.
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\ncustom = { severity = \"error\", locked = true }\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "lock custom rules"]);

    // A source change for the language-gated custom rule to fire on.
    std::fs::write(repo.join("src_change.py"), "def added():\n    return 1\n").unwrap();

    // A mute targeting the locked custom rule is refused — it still fires.
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\ncustom = { severity = \"error\", locked = true }\n\n[[mute]]\npath = \"**\"\nrule = \"house-style\"\nreason = \"nope\"\n",
    )
    .unwrap();
    let mut a = args(&repo);
    a.reference = String::new();
    let out = run_check(a);
    assert!(
        out.stdout.contains("house-style"),
        "locked custom still fires: {}",
        out.stdout
    );
    assert!(
        out.stdout.contains("rule-tampered"),
        "adding the mute is tamper: {}",
        out.stdout
    );

    // Editing the locked custom rule's script is itself rule-tampered.
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\ncustom = { severity = \"error\", locked = true }\n",
    )
    .unwrap();
    std::fs::write(
        repo.join(".argot/rules/house-style/check.rhai"),
        "// gutted so it never fires",
    )
    .unwrap();
    let mut a = args(&repo);
    a.reference = String::new();
    let out = run_check(a);
    assert!(
        out.stdout.contains("rule-tampered"),
        "editing the script is tamper: {}",
        out.stdout
    );
    assert!(
        out.stdout.contains("house-style"),
        "names the rule: {}",
        out.stdout
    );
    assert_eq!(out.exit_code, 1);
}
