//! End-to-end behaviour of the scripted rule group (`--features script`):
//! discovery from `.argot/rules/`, findings through the full `run_check`
//! pipeline (rendering, severity, exit code), config/severity resolution,
//! suppression, sandbox containment, and vocabulary collision rejection.
//!
//! Requires `git` and `bash` on PATH (fixture build).
#![cfg(feature = "script")]

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::output::OutputFormat;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("script_rules_repo_{suffix}"));
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
        // CI runners carry no global git identity — supply one so commits in
        // the fixtures succeed (mirrors locked_rules.rs).
        .env("GIT_AUTHOR_NAME", "t")
        .env("GIT_AUTHOR_EMAIL", "t@t")
        .env("GIT_COMMITTER_NAME", "t")
        .env("GIT_COMMITTER_EMAIL", "t@t")
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// Fit at HEAD~1 with the BPE threshold pinned high, so the built-in voice
/// rules stay quiet except the deterministic foreign-import — scripted-rule
/// findings are then easy to isolate.
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

fn write_rule(repo: &Path, name: &str, manifest_extra: &str, script: &str) {
    let d = repo.join(".argot/rules").join(name);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        format!("[rule]\nschema = 1\nname = \"{name}\"\n{manifest_extra}"),
    )
    .unwrap();
    std::fs::write(d.join("check.rhai"), script).unwrap();
}

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

/// A rule that always fires on any changed Python file.
const ALWAYS_FIRE: &str = r#"
if file.language == "python" {
    report(1, "scripted finding on " + file.path);
}
"#;

#[test]
fn discovered_rule_reports_through_the_full_pipeline() {
    let repo = prepare_repo("fires");
    write_rule(&repo, "my-rule", "severity = \"error\"\n", ALWAYS_FIRE);
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 1, "error severity gates: {}", out.stderr);
    assert!(
        out.stdout.contains("my-rule"),
        "rule name rendered:\n{}",
        out.stdout
    );
    assert!(out.stdout.contains("scripted finding on"), "{}", out.stdout);
}

#[test]
fn default_severity_is_warn_and_config_can_silence_or_gate() {
    let repo = prepare_repo("severity");
    write_rule(&repo, "quiet-rule", "", ALWAYS_FIRE);
    // Default warn: reported, does not fail (the fixture's foreign-import is
    // muted off for isolation).
    std::fs::write(repo.join("argot.toml"), "[rules]\nvoice = \"off\"\n").unwrap();
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 0, "warn does not gate: {}", out.stderr);
    assert!(out.stdout.contains("quiet-rule"), "{}", out.stdout);
    // [rules] can turn the custom rule off by name…
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\nvoice = \"off\"\n\"quiet-rule\" = \"off\"\n",
    )
    .unwrap();
    let out = run_check(args(&repo));
    assert!(!out.stdout.contains("quiet-rule"), "{}", out.stdout);
    // …or promote the whole custom group to error.
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\nvoice = \"off\"\ncustom = \"error\"\n",
    )
    .unwrap();
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 1, "{}", out.stderr);
}

#[test]
fn json_format_carries_the_custom_rule_vocabulary() {
    let repo = prepare_repo("json");
    write_rule(
        &repo,
        "labelled",
        "label = \"my label\"\n",
        r#"report_span(1, 2, "msg", #{ evidence: ["why"], symbol: "sym" });"#,
    );
    let mut a = args(&repo);
    a.format = OutputFormat::Json;
    let out = run_check(a);
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("json stdout");
    let hit = doc["hits"]
        .as_array()
        .unwrap()
        .iter()
        .find(|h| h["rule"] == "labelled")
        .expect("custom finding in JSON");
    assert_eq!(hit["rule_label"], "my label");
    assert_eq!(hit["severity"], "warn");
    assert_eq!(hit["symbol"], "sym");
    assert!(hit["evidence"]
        .as_array()
        .unwrap()
        .iter()
        .any(|e| e.as_str().unwrap().contains("why")));
}

#[test]
fn inline_and_mute_suppression_cover_custom_rules() {
    let repo = prepare_repo("suppress");
    write_rule(&repo, "suppressible", "", ALWAYS_FIRE);
    std::fs::write(repo.join("argot.toml"), "[rules]\nvoice = \"off\"\n").unwrap();
    // Baseline: it fires.
    let out = run_check(args(&repo));
    assert!(out.stdout.contains("suppressible"), "{}", out.stdout);
    // A [[mute]] scoped to the custom rule name silences it (and the scope
    // validates — no unknown-rule warning).
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\nvoice = \"off\"\n\n[[mute]]\npath = \"**\"\nrule = \"suppressible\"\nreason = \"testing\"\n",
    )
    .unwrap();
    let out = run_check(args(&repo));
    assert!(!out.stdout.contains("suppressible"), "{}", out.stdout);
    assert!(
        !out.stderr.contains("unknown rule"),
        "custom name validates in [[mute]]: {}",
        out.stderr
    );
}

#[test]
fn runaway_rule_is_disabled_and_the_check_survives() {
    let repo = prepare_repo("runaway");
    write_rule(&repo, "spinner", "", "loop { }");
    write_rule(&repo, "steady", "severity = \"error\"\n", ALWAYS_FIRE);
    let out = run_check(args(&repo));
    assert!(
        out.stderr.contains("spinner") && out.stderr.contains("disabled"),
        "runaway diagnosed: {}",
        out.stderr
    );
    assert!(out.stdout.contains("steady"), "{}", out.stdout);
    assert_eq!(out.exit_code, 1);
}

#[test]
fn vocabulary_collisions_are_rejected_with_a_warning() {
    let repo = prepare_repo("collision");
    // Shadows a built-in rule name — skipped, warned, check unaffected.
    write_rule(&repo, "foreign-import", "", ALWAYS_FIRE);
    let out = run_check(args(&repo));
    assert!(
        out.stderr.contains("foreign-import") && out.stderr.contains("collides"),
        "{}",
        out.stderr
    );
    assert_eq!(out.exit_code, 1, "the built-in foreign-import still fires");
}

#[test]
fn unknown_rule_override_fails_fast_but_custom_names_resolve() {
    let repo = prepare_repo("overrides");
    write_rule(&repo, "tunable", "", ALWAYS_FIRE);
    // A custom rule is addressable by --rule …
    let mut a = args(&repo);
    a.rule_overrides = vec![("tunable".to_string(), argot_core::rules::Severity::Off)];
    let out = run_check(a);
    assert!(!out.stdout.contains("tunable"), "{}", out.stdout);
    // … and a typo still exits 2.
    let mut a = args(&repo);
    a.rule_overrides = vec![("not-a-rule".to_string(), argot_core::rules::Severity::Off)];
    let out = run_check(a);
    assert_eq!(out.exit_code, 2, "{}", out.stderr);
    assert!(
        out.stderr.contains("unknown rule 'not-a-rule'"),
        "{}",
        out.stderr
    );
}

#[test]
fn files_globs_run_rules_on_unscored_extensions() {
    let repo = prepare_repo("envfiles");
    let d = repo.join(".argot/rules/no-plaintext-secrets");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        "[rule]\nschema = 1\nname = \"no-plaintext-secrets\"\nseverity = \"error\"\ninclude = [\"*.env\"]\n",
    )
    .unwrap();
    std::fs::write(
        d.join("check.rhai"),
        r#"
for h in hunks {
    if h.text.contains("SECRET=") && file.language == "" {
        report(h.start, "plaintext secret in an env file — use the secret manager");
    }
}
"#,
    )
    .unwrap();
    // A committed .env, edited in the working tree (an unscored extension —
    // the voice model has never seen it).
    std::fs::write(repo.join("deploy.env"), "PORT=8080\n").unwrap();
    git(&repo, &["add", "deploy.env"]);
    git(&repo, &["commit", "-qm", "env file"]);
    std::fs::write(repo.join("deploy.env"), "PORT=8080\nSECRET=hunter2\n").unwrap();
    let mut a = args(&repo);
    a.reference = String::new(); // workdir mode
    let out = run_check(a);
    assert_eq!(out.exit_code, 1, "{}", out.stderr);
    assert!(
        out.stdout.contains("no-plaintext-secrets"),
        "{}",
        out.stdout
    );
    assert!(out.stdout.contains("deploy.env"), "{}", out.stdout);
}
