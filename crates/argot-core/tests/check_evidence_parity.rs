//! Byte-parity test for `argot check` **with evidence rendering**.
//!
//! Builds the deterministic `check` fixture repo (fixed authors/dates →
//! reproducible SHAs), runs `train` to emit `repo-corpus.txt` +
//! `generic-baseline.json`, drops in the committed Python-generated
//! `scorer-config-head1fit.json` (which carries an `evidence_corpus` block),
//! then asserts `run_check`'s stdout — plus a trailing `exit=<code>` line —
//! is byte-identical to each committed golden.
//!
//! Two scenarios, both keyed off the HEAD~1-fit config (import_modules exclude
//! `sqlalchemy`, so it is foreign):
//!
//! * BPE evidence (`golden_bpe_evidence.txt`): the BPE repo corpus is fit at
//!   HEAD~1 (checkout HEAD~1 → train → checkout back), so `integration.py`'s
//!   tokens are unattested and the BPE stage wins with score 8.17 → a
//!   `↳ sessionmaker (0×), ...` line.
//! * Import evidence (`golden_import_evidence.txt`): the BPE corpus is fit at
//!   HEAD (train on the working tree, `integration.py` included), so the BPE
//!   stage stays below threshold and the import stage wins → a
//!   `↳ sqlalchemy (L2) — never seen in repo` line, a `common here:` line, and
//!   the `^^^^` caret underline under the offending import.
//!
//! Requires `git` and `bash` on PATH (build step). No Python at test time.

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};

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
    let status = Command::new("bash")
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

fn train(repo: &Path) {
    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    argot_core::train::run_train(
        repo,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )
    .expect("train");
}

/// Copy the committed HEAD~1-fit config (it carries the `evidence_corpus`
/// block) into `.argot/scorer-config.json`.
fn install_head1_config(repo: &Path) {
    std::fs::copy(
        fixture_evidence_dir().join("scorer-config-head1fit.json"),
        repo.join(".argot/scorer-config.json"),
    )
    .expect("copy scorer-config-head1fit.json");
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
        min_severity: "unusual".to_string(),
        use_color: false,
        format: argot_core::output::OutputFormat::Human,
        today: "2026-01-01".to_string(),
    }
}

/// The committed goldens embed the exit code as a trailing `exit=<code>` line,
/// so we compare `stdout + "exit=<code>\n"` byte-for-byte.
fn assert_golden(stdout: &str, exit_code: i32, golden_name: &str) {
    let actual = format!("{stdout}exit={exit_code}\n");
    // See check_parity.rs: deliberate rendering changes regenerate goldens.
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
    // Fit the BPE repo corpus at HEAD~1 (before integration.py existed) so the
    // sqlalchemy tokens are unattested and the BPE stage wins.
    let repo = build_fixture_repo("bpe");
    git(&repo, &["checkout", "-q", "HEAD~1"]);
    train(&repo);
    git(&repo, &["checkout", "-q", "main"]);
    install_head1_config(&repo);

    let out = run_check(base_args(&repo));
    assert_eq!(out.exit_code, 1, "bpe evidence exit code");
    assert_golden(&out.stdout, out.exit_code, "golden_bpe_evidence.txt");
}

#[test]
fn check_import_evidence() {
    // Fit the BPE repo corpus at HEAD (integration.py included) so the BPE
    // stage stays below threshold and the import stage wins — exercising the
    // import `↳` line, the `common here:` line, and the caret underline.
    let repo = build_fixture_repo("import");
    train(&repo);
    install_head1_config(&repo);

    let out = run_check(base_args(&repo));
    assert_eq!(out.exit_code, 1, "import evidence exit code");
    assert_golden(&out.stdout, out.exit_code, "golden_import_evidence.txt");
}
