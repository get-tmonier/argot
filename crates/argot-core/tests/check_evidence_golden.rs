//! Golden test for `argot check` **with evidence rendering**.
//!
//! Builds the deterministic `check` fixture repo (fixed authors/dates →
//! reproducible SHAs), fits it at HEAD~1 (checkout HEAD~1 → train → calibrate
//! → checkout back) so the last commit's `integration.py` is genuinely
//! post-fit code, then asserts `run_check`'s stdout — plus a trailing
//! `exit=<code>` line — is byte-identical to each committed golden.
//!
//! Two scenarios, both against the HEAD~1-fit v3 config (whose model snapshot
//! and import_modules exclude `sqlalchemy`):
//!
//! * BPE evidence (`golden_bpe_evidence.txt`): with the model fit at HEAD~1,
//!   `integration.py`'s tokens are unattested; the threshold is pinned to a
//!   fixed low value so the BPE stage wins over the import tripwire → a
//!   `↳ sessionmaker (0×), ...` line. (The honest LOO calibration otherwise
//!   sits above this tiny fixture's reachable BPE scores.)
//! * Import evidence (`golden_import_evidence.txt`): same fit, but with the
//!   BPE threshold pinned high in a test-local copy of the config so only the
//!   import stage can fire (the renderer under test) → a
//!   `↳ sqlalchemy (L2) — never seen in repo` line, a `common here:` line, and
//!   the `^^^^` caret underline under the offending import.
//!
//! Requires `git` and `bash` on PATH (build step).

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

fn fixture_check_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn fixture_evidence_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check_evidence")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("check_evidence_fixture_repo_{suffix}"));
    let script = fixture_check_dir().join("build_check_repo.sh");
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

/// Full fit (train → calibrate) against the current checkout; deterministic
/// v3 config with the fit-time model snapshot.
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
}

/// Pin the calibrated BPE threshold to a fixed value in the fitted config.
/// High (1000.0) leaves only the import stage able to fire — isolates the
/// import-evidence renderer. Low (4.0) lets the BPE stage win over the
/// import tripwire — isolates the BPE-evidence renderer (the honest LOO
/// calibration otherwise sits above this tiny fixture's reachable scores).
fn pin_threshold(repo: &Path, value: f64) {
    let config_path = repo.join(".argot/scorer-config.json");
    let raw = std::fs::read_to_string(&config_path).expect("read config");
    let mut config: serde_json::Value = serde_json::from_str(&raw).expect("parse config");
    for (_, lang_cfg) in config["languages"].as_object_mut().expect("languages") {
        lang_cfg["threshold"] = serde_json::json!(value);
        // integration.py is a new file relative to the HEAD~1 fit, so it is
        // judged against the new-file threshold — pin both to isolate the
        // evidence renderer (issue #92).
        lang_cfg["new_file_threshold"] = serde_json::json!(value);
    }
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).expect("write config");
}

fn base_args(repo: &Path) -> CheckArgs {
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
        format: argot_core::output::OutputFormat::Human,
        today: "2026-01-01".to_string(),
    }
}

/// The committed goldens embed the exit code as a trailing `exit=<code>` line,
/// so we compare `stdout + "exit=<code>\n"` byte-for-byte.
fn assert_golden(stdout: &str, exit_code: i32, golden_name: &str) {
    let actual = format!("{stdout}exit={exit_code}\n");
    // See check_golden.rs: deliberate rendering changes regenerate goldens.
    if std::env::var_os("ARGOT_UPDATE_GOLDENS").is_some() {
        std::fs::write(fixture_evidence_dir().join(golden_name), &actual).expect("update golden");
        return;
    }
    let golden = std::fs::read(fixture_evidence_dir().join(golden_name)).expect("read golden");
    let expected = String::from_utf8(golden).unwrap();
    let a: Vec<&str> = actual.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    for (i, (got, want)) in a.iter().zip(e.iter()).enumerate() {
        assert_eq!(got, want, "{golden_name}: line {i} diverges");
    }
    assert_eq!(a.len(), e.len(), "{golden_name}: line count differs");
    assert_eq!(actual, expected, "{golden_name}: not byte-identical");
}

#[test]
fn check_bpe_evidence() {
    // Fit at HEAD~1 (before integration.py existed) so the sqlalchemy tokens
    // are unattested in the model snapshot and the BPE stage wins.
    let repo = build_fixture_repo("bpe");
    git(&repo, &["checkout", "-q", "HEAD~1"]);
    fit(&repo);
    git(&repo, &["checkout", "-q", "main"]);
    pin_threshold(&repo, 4.0);

    let out = run_check(base_args(&repo));
    assert_eq!(out.exit_code, 1, "bpe evidence exit code");
    assert_golden(&out.stdout, out.exit_code, "golden_bpe_evidence.txt");
}

#[test]
fn check_import_evidence() {
    // Same HEAD~1 fit, but with the BPE threshold pinned high so only the
    // import stage can fire — exercising the import `↳` line, the
    // `common here:` line, and the caret underline.
    let repo = build_fixture_repo("import");
    git(&repo, &["checkout", "-q", "HEAD~1"]);
    fit(&repo);
    git(&repo, &["checkout", "-q", "main"]);
    pin_threshold(&repo, 1000.0);

    let out = run_check(base_args(&repo));
    assert_eq!(out.exit_code, 1, "import evidence exit code");
    assert_golden(&out.stdout, out.exit_code, "golden_import_evidence.txt");
}
