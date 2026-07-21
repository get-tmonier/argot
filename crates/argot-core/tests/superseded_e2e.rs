//! End-to-end behaviour of the supersession pipeline: a synthetic repo whose
//! accepted history migrates `oldlib` → `newlib` file by file, fitted with
//! the real pipeline, then checked.
//!
//! Covers: mining lands in the artifact with evidence; new code using the
//! superseded side raises `superseded` (warn — reported, exit 0;
//! `--error-on-warnings` gates); the replacement side never reads as foreign;
//! declared `[[migration]]` entries enforce and exempt without a refit.
//!
//! Requires `git` on PATH.

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::output::OutputFormat;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

fn git(repo: &Path, args: &[&str]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args([
            "-c",
            "user.email=fixture@example.com",
            "-c",
            "user.name=fixture",
        ])
        .args(args)
        .status()
        .expect("run git");
    assert!(status.success(), "git {args:?} failed");
}

fn write(repo: &Path, rel: &str, content: &str) {
    std::fs::write(repo.join(rel), content).unwrap();
}

fn module_source(name: &str, dep: &str) -> String {
    format!(
        "import {dep}\n\n\ndef load_{name}(payload):\n    parsed = {dep}.parse(payload)\n    \
         return {dep}.render(parsed)\n\n\ndef save_{name}(record):\n    return {dep}.store(record)\n"
    )
}

/// A repo whose history migrates `oldlib` → `newlib` across four accepted
/// commits, with `legacy.py` left unmigrated at HEAD.
fn build_migration_repo(suffix: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("superseded_repo_{suffix}"));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);

    for name in ["alpha", "beta", "gamma", "delta", "legacy"] {
        write(&repo, &format!("{name}.py"), &module_source(name, "oldlib"));
    }
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-q", "-m", "initial modules"]);

    for name in ["alpha", "beta", "gamma", "delta"] {
        write(&repo, &format!("{name}.py"), &module_source(name, "newlib"));
        git(&repo, &["add", "-A"]);
        git(
            &repo,
            &["commit", "-q", "-m", &format!("migrate {name} to newlib")],
        );
    }
    repo
}

/// Fit the repo at HEAD and pin the statistical thresholds high so only
/// deterministic stages (imports, supersessions) can fire.
fn fit(repo: &Path) {
    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    argot_core::train::run_train(
        repo,
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
        repo,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &opts,
    )
    .expect("calibrate");

    let config_path = argot_dir.join("scorer-config.json");
    let raw = std::fs::read_to_string(&config_path).expect("read config");
    let mut config: serde_json::Value = serde_json::from_str(&raw).expect("parse config");
    for (_, lang_cfg) in config["languages"].as_object_mut().expect("languages") {
        lang_cfg["threshold"] = serde_json::json!(1000.0);
        lang_cfg["new_file_threshold"] = serde_json::json!(1000.0);
    }
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).expect("write config");
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

fn commit_file(repo: &Path, rel: &str, content: &str, message: &str) {
    write(repo, rel, content);
    git(repo, &["add", "-A"]);
    git(repo, &["commit", "-q", "-m", message]);
}

#[test]
fn mining_lands_in_the_artifact_with_evidence() {
    let repo = build_migration_repo("artifact");
    fit(&repo);
    let raw = std::fs::read_to_string(repo.join(".argot/scorer-config.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&raw).unwrap();
    let sup = &config["languages"]["python"]["supersessions"];
    let list = sup.as_array().expect("supersessions mined");
    let found = list
        .iter()
        .find(|s| s["old"] == "oldlib")
        .expect("oldlib pair mined");
    assert_eq!(found["new"], "newlib");
    assert_eq!(found["kind"], "import");
    assert_eq!(found["commits"], 4);
    assert_eq!(found["leftover_count"], 1);
    assert_eq!(found["leftovers"][0], "legacy.py");
    assert!(found["example_commit"].as_str().unwrap().len() >= 7);
}

#[test]
fn new_code_using_the_superseded_side_warns_but_does_not_gate() {
    let repo = build_migration_repo("warns");
    fit(&repo);
    commit_file(
        &repo,
        "feature.py",
        &module_source("feature", "oldlib"),
        "new feature on the old pattern",
    );

    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 0, "warn by default: {}", out.stdout);
    assert!(out.stdout.contains("superseded"), "{}", out.stdout);
    assert!(
        out.stdout.contains("replaced 'oldlib' with 'newlib'"),
        "{}",
        out.stdout
    );

    let mut gated = args(&repo);
    gated.error_on_warnings = true;
    assert_eq!(run_check(gated).exit_code, 1, "--error-on-warnings gates");

    let mut off = args(&repo);
    off.rule_overrides = vec![("superseded".to_string(), argot_core::rules::Severity::Off)];
    let out = run_check(off);
    assert!(!out.stdout.contains("superseded"), "off rule is silent");
}

#[test]
fn replacement_side_and_unrelated_code_stay_clean() {
    let repo = build_migration_repo("clean");
    fit(&repo);
    commit_file(
        &repo,
        "feature.py",
        &module_source("feature", "newlib"),
        "new feature on the new pattern",
    );
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 0);
    assert!(
        !out.stdout.contains("superseded") && !out.stdout.contains("foreign-import"),
        "replacement side is in-voice: {}",
        out.stdout
    );
}

/// A repo with a steady history (no replacement pairs to mine).
fn build_steady_repo(suffix: &str) -> PathBuf {
    let repo = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("steady_repo_{suffix}"));
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(&repo).unwrap();
    git(&repo, &["init", "-q", "-b", "main"]);
    for name in ["alpha", "beta", "gamma", "delta"] {
        write(&repo, &format!("{name}.py"), &module_source(name, "oldlib"));
        git(&repo, &["add", "-A"]);
        git(&repo, &["commit", "-q", "-m", &format!("add {name}")]);
    }
    repo
}

#[test]
fn declared_migration_enforces_and_exempts_without_a_refit() {
    let repo = build_steady_repo("declared");
    fit(&repo);
    // Declared AFTER the fit — must apply immediately.
    write(
        &repo,
        "argot.toml",
        "[[migration]]\nfrom = \"oldlib\"\nto = \"freshlib\"\nreason = \"platform rewrite\"\n",
    );

    // The declared target was never in the corpus: without the migration it
    // would be a foreign import; with it, it is the sanctioned replacement.
    commit_file(
        &repo,
        "fresh.py",
        &module_source("fresh", "freshlib"),
        "adopt the declared target",
    );
    let out = run_check(args(&repo));
    assert!(
        !out.stdout.contains("foreign-import"),
        "declared target must not read as foreign: {}",
        out.stdout
    );

    // And the declared source warns with the declared reason as evidence.
    commit_file(
        &repo,
        "stale.py",
        &module_source("stale", "oldlib"),
        "new code on the declared-away pattern",
    );
    let out = run_check(args(&repo));
    assert_eq!(out.exit_code, 0);
    assert!(
        out.stdout.contains("argot.toml migration to 'freshlib'")
            && out.stdout.contains("platform rewrite"),
        "{}",
        out.stdout
    );
}

#[test]
fn repo_without_replacement_history_mines_nothing() {
    let repo = build_steady_repo("control");
    fit(&repo);
    let raw = std::fs::read_to_string(repo.join(".argot/scorer-config.json")).unwrap();
    let config: serde_json::Value = serde_json::from_str(&raw).unwrap();
    assert!(
        config["languages"]["python"]["supersessions"].is_null(),
        "steady history must mine nothing"
    );
}
