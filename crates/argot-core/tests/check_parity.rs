//! Byte-parity test for `argot check`.
//!
//! Builds the deterministic fixture repo (fixed authors/dates → reproducible
//! SHAs), runs `train` to emit `repo-corpus.txt` + `generic-baseline.json`,
//! drops in the committed Python-generated `scorer-config.json`, then asserts
//! `run_check`'s stdout is byte-identical to each committed golden and the exit
//! code matches.
//!
//! Requires `git` and `bash` on PATH (build step). No Python at test time.

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    // Distinct dir per test: tests run in parallel and the workdir case mutates
    // a tracked file, which would corrupt a shared repo.
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check_fixture_repo_{suffix}"));
    let script = fixture_dir().join("build_check_repo.sh");
    let status = Command::new("bash")
        .arg(&script)
        .arg(&out)
        .status()
        .expect("run build_check_repo.sh");
    assert!(status.success(), "fixture build failed");
    out
}

/// Build repo + `.argot/` artifacts and return the repo path.
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

    // Drop in the committed, Python-generated v2 scorer-config.json.
    std::fs::copy(
        fixture_dir().join("scorer-config.json"),
        argot_dir.join("scorer-config.json"),
    )
    .expect("copy scorer-config.json");

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
        min_severity: "unusual".to_string(),
        use_color: false,
    }
}

fn assert_golden(stdout: &str, golden_name: &str) {
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
