//! Golden test for the full `argot check` pipeline.
//!
//! Builds the deterministic fixture repo (fixed authors/dates → reproducible
//! SHAs), fits it end-to-end (`train` → `calibrate`, fixed seeds/metadata →
//! deterministic v3 config with the fit-time model snapshot), then asserts
//! `run_check`'s stdout is byte-identical to each committed golden and the
//! exit code matches.
//!
//! Historically this compared against the Python engine's output with a
//! committed Python-generated config; the v3 model artifact (era 15) is a
//! deliberate divergence, so these are now self-contained pipeline goldens.
//!
//! Requires `git` and `bash` on PATH (build step).

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    // Distinct dir per test: tests run in parallel and the workdir case mutates
    // a tracked file, which would corrupt a shared repo.
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check_fixture_repo_{suffix}"));
    let script = fixture_dir().join("build_check_repo.sh");
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

/// Build repo + `.argot/` artifacts (full fit: train → calibrate) and return
/// the repo path. Calibrate metadata is pinned so the emitted v3 config is
/// deterministic.
fn prepare_repo(suffix: &str) -> PathBuf {
    let repo = build_fixture_repo(suffix);
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

    repo
}

fn base_args(repo: &Path) -> CheckArgs {
    CheckArgs {
        repo_path: repo.to_str().unwrap().to_string(),
        reference: String::new(),
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

fn assert_golden(stdout: &str, golden_name: &str) {
    // Deliberate rendering changes (e.g. the suppression hit-hash on the hit
    // header) regenerate the goldens via ARGOT_UPDATE_GOLDENS=1; the committed
    // fixture repo is deterministic, so the refreshed goldens are too.
    if std::env::var_os("ARGOT_UPDATE_GOLDENS").is_some() {
        std::fs::write(fixture_dir().join(golden_name), stdout).expect("update golden");
        return;
    }
    let golden = std::fs::read(fixture_dir().join(golden_name)).expect("read golden");
    let expected = String::from_utf8(golden).unwrap();
    // Line-by-line first for a readable diff, then full byte equality.
    let a: Vec<&str> = stdout.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    for (i, (got, want)) in a.iter().zip(e.iter()).enumerate() {
        assert_eq!(got, want, "{golden_name}: line {i} diverges");
    }
    assert_eq!(a.len(), e.len(), "{golden_name}: line count differs");
    assert_eq!(stdout, expected, "{golden_name}: not byte-identical");
}

#[test]
fn check_head1_clean() {
    let repo = prepare_repo("head1");
    let mut args = base_args(&repo);
    args.reference = "HEAD~1..HEAD".to_string();
    let out = run_check(args);
    assert_eq!(out.exit_code, 0, "head1 exit code");
    assert_golden(&out.stdout, "golden_head1.txt");
}

#[test]
fn check_render_range() {
    let repo = prepare_repo("render");
    let mut args = base_args(&repo);
    args.reference = "HEAD~2..HEAD".to_string();
    args.threshold = Some(-1000.0);
    let out = run_check(args);
    assert_eq!(out.exit_code, 1, "render exit code");
    assert_golden(&out.stdout, "golden_render.txt");
}

#[test]
fn check_workdir() {
    let repo = prepare_repo("workdir");

    // Reproduce the workdir change the golden was captured with: append a new
    // helper to a tracked file so it shows as an unstaged (workdir) diff.
    let text_py = repo.join("text.py");
    let mut content = std::fs::read_to_string(&text_py).unwrap();
    content.push_str(
        "\n\ndef extra_helper(items):\n    total = 0\n    for it in items:\n        total += len(it)\n    return total\n",
    );
    std::fs::write(&text_py, content).unwrap();

    let mut args = base_args(&repo);
    args.threshold = Some(-1000.0);
    let out = run_check(args);
    assert_eq!(out.exit_code, 1, "workdir exit code");
    assert_golden(&out.stdout, "golden_workdir.txt");
}
