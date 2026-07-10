//! Behaviour tests for the rule-severity gate: `[rules]` in argot.toml, CLI
//! `--rule` overrides, `--error-on-warnings`, and the exit-code contract
//! (`error` findings fail the check, `warn` findings don't, `off` rules
//! don't run).
//!
//! Reuses the deterministic check fixture fitted at HEAD~1 with the BPE
//! threshold pinned high, so exactly one finding fires: a `foreign-import`
//! on `integration.py`'s sqlalchemy import.
//!
//! Requires `git` and `bash` on PATH (fixture build).

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::output::OutputFormat;
use argot_core::rules::Severity;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("rules_gating_repo_{suffix}"));
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
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

/// Fit at HEAD~1 and pin the BPE threshold high so only the import stage can
/// fire — one deterministic `foreign-import` finding at HEAD.
fn prepare_import_repo(suffix: &str) -> PathBuf {
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
        lang_cfg["new_file_threshold"] = serde_json::json!(1000.0);
    }
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).expect("write config");
    repo
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

#[test]
fn error_severity_finding_fails_the_check() {
    let repo = prepare_import_repo("error");
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 1, "default severity is error → exit 1");
    assert!(out.stdout.contains("foreign-import"));
}

#[test]
fn warn_severity_finding_is_reported_but_does_not_fail() {
    let repo = prepare_import_repo("warn");
    let mut a = args(&repo);
    a.rule_overrides = vec![("foreign-import".to_string(), Severity::Warn)];
    let out = run_check(a);
    assert_eq!(out.exit_code, 0, "warn finding must not fail the check");
    assert!(
        out.stdout.contains("foreign-import"),
        "warn finding still reported"
    );
}

#[test]
fn error_on_warnings_promotes_warn_findings() {
    let repo = prepare_import_repo("promote");
    let mut a = args(&repo);
    a.rule_overrides = vec![("foreign-import".to_string(), Severity::Warn)];
    a.error_on_warnings = true;
    let out = run_check(a);
    assert_eq!(out.exit_code, 1, "--error-on-warnings restores the gate");
}

#[test]
fn off_rule_emits_nothing() {
    let repo = prepare_import_repo("off");
    let mut a = args(&repo);
    a.rule_overrides = vec![("foreign-import".to_string(), Severity::Off)];
    let out = run_check(a);
    assert_eq!(out.exit_code, 0);
    assert!(
        !out.stdout.contains("foreign-import"),
        "off rule must not report"
    );
    assert!(out.stdout.contains("looks clean"), "clean summary shown");
}

#[test]
fn group_selector_covers_its_rules() {
    let repo = prepare_import_repo("group");
    let mut a = args(&repo);
    a.rule_overrides = vec![("voice".to_string(), Severity::Off)];
    let out = run_check(a);
    assert_eq!(out.exit_code, 0, "voice=off disables foreign-import too");
    assert!(!out.stdout.contains("foreign-import"));
}

#[test]
fn config_rules_section_sets_severity_and_cli_overrides_it() {
    let repo = prepare_import_repo("config");
    std::fs::write(
        repo.join("argot.toml"),
        "[rules]\n\"foreign-import\" = \"warn\"\n",
    )
    .unwrap();

    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 0, "argot.toml warn → exit 0");
    assert!(out.stdout.contains("foreign-import"), "still reported");

    // CLI wins over config.
    let mut a = args(&repo);
    a.rule_overrides = vec![("foreign-import".to_string(), Severity::Error)];
    let out = run_check(a);
    assert_eq!(out.exit_code, 1, "CLI --rule beats [rules]");
}

#[test]
fn unknown_rules_key_warns_and_is_ignored() {
    let repo = prepare_import_repo("unknown");
    std::fs::write(repo.join("argot.toml"), "[rules]\nquantum = \"off\"\n").unwrap();
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 1, "unknown key changes nothing");
    assert!(out.stderr.contains("unknown rule 'quantum'"));
}

#[test]
fn add_ignores_baselines_workdir_findings() {
    let repo = prepare_import_repo("add_ignores");
    // A workdir edit reaching for the never-used dependency.
    let target = repo.join("pipeline.py");
    let src = std::fs::read_to_string(&target).unwrap();
    std::fs::write(
        &target,
        format!(
            "import sqlalchemy
{src}"
        ),
    )
    .unwrap();

    // Workdir mode (empty reference) — the mode --add-ignores edits.
    let workdir = |repo: &std::path::Path| {
        let mut a = args(repo);
        a.reference = String::new();
        a
    };
    let out = run_check(workdir(&repo));
    assert_eq!(out.exit_code, 1, "workdir foreign-import fires");

    let mut a = workdir(&repo);
    a.add_ignores = true;
    let out = run_check(a);
    assert_eq!(out.exit_code, 0, "--add-ignores itself succeeds");
    assert!(
        out.stdout.contains("Added 1 ignore comment(s)"),
        "{}",
        out.stdout
    );
    let updated = std::fs::read_to_string(&target).unwrap();
    assert!(
        updated.contains("# argot: ignore-next-line rule=foreign-import — baselined"),
        "{updated}"
    );

    // The baselined finding no longer fails the check.
    let out = run_check(workdir(&repo));
    assert_eq!(out.exit_code, 0, "baselined → clean: {}", out.stdout);
}

#[test]
fn add_ignores_refuses_ref_ranges() {
    let repo = prepare_import_repo("add_ignores_ref");
    let mut a = args(&repo);
    a.reference = "HEAD~1..HEAD".to_string();
    a.add_ignores = true;
    let out = run_check(a);
    assert_eq!(out.exit_code, 2);
    assert!(out.stderr.contains("edits the working tree"));
}

#[test]
fn config_change_since_fit_is_noted_at_check() {
    let repo = prepare_import_repo("cfg_drift");
    // The fit persisted its config fingerprint (defaults). Change a
    // fit-relevant knob afterwards: check must say the fit is out of sync.
    std::fs::write(repo.join("argot.toml"), "[detect]\ndata-threshold = 0.5\n").unwrap();
    let out = run_check(args(&repo));
    assert!(
        out.stderr.contains("argot.toml changed since the last fit"),
        "{}",
        out.stderr
    );
}

#[test]
fn unfitted_language_in_the_diff_is_noted() {
    let repo = prepare_import_repo("newlang");
    let go_src = "package main\n\nimport \"fmt\"\n\nfunc main() {\n\tfmt.Println(\"hi\")\n}\n";
    std::fs::write(repo.join("main.go"), go_src).unwrap();
    let mut a = args(&repo);
    a.reference = String::new(); // workdir mode sees the untracked file
    let out = run_check(a);
    assert!(
        out.stderr
            .contains("go file(s) — no model in the current fit"),
        "{}",
        out.stderr
    );
}

#[test]
fn json_carries_rule_and_severity_fields() {
    let repo = prepare_import_repo("json");
    let mut a = args(&repo);
    a.rule_overrides = vec![("foreign-import".to_string(), Severity::Warn)];
    a.format = OutputFormat::Json;
    let out = run_check(a);
    assert_eq!(out.exit_code, 0, "warn finding in machine format → exit 0");
    let doc: serde_json::Value = serde_json::from_str(&out.stdout).expect("valid JSON");
    let h = &doc["hits"][0];
    assert_eq!(h["rule"], "foreign-import");
    assert_eq!(h["rule_label"], "foreign import");
    assert_eq!(h["severity"], "warn");
    assert_eq!(h["confidence"], "foreign");
}
