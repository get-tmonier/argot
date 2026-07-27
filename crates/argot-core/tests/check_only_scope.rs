//! `[exclude].check-only` end to end: a path that is checked but never shapes
//! the voice.
//!
//! The behaviour under test is an asymmetry, and all three halves of it matter:
//! a dependency established by these files is familiar *there*, still foreign
//! in production, and the model's style signals never judge them at all.
//!
//! Requires `git` and `bash` on PATH (fixture build).

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};

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

/// The fixture repo, plus a `tests/` tree whose only outside dependency is
/// `fixture_harness` — vocabulary that exists nowhere in production code.
fn prepare_repo(suffix: &str) -> PathBuf {
    let repo =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check_only_scope_{suffix}"));
    let script =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check/build_check_repo.sh");
    let status = Command::new(
        std::env::var_os("ARGOT_TEST_BASH")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "bash".into()),
    )
    .arg(&script)
    .arg(&repo)
    .status()
    .expect("run build_check_repo.sh");
    assert!(status.success(), "fixture build failed");

    std::fs::create_dir_all(repo.join("tests")).unwrap();
    std::fs::write(
        repo.join("tests/test_stats.py"),
        "import fixture_harness\n\nfrom stats import mean\n\n\ndef test_mean():\n    with fixture_harness.clock():\n        assert mean([1, 2, 3]) == 2\n",
    )
    .unwrap();
    // The repo checks its tests: the test patterns leave `recommended` and stay
    // in `check-only`.
    std::fs::write(
        repo.join("argot.toml"),
        "[exclude]\nrecommended = [\"docs/\", \"build/\", \"dist/\"]\npaths = []\n\
         check-only = [\"test/\", \"tests/\", \"__tests__/\", \"test_*\", \"*.test.*\", \"*.spec.*\"]\n",
    )
    .unwrap();
    git(&repo, &["add", "-A"]);
    git(&repo, &["commit", "-qm", "tests + argot config"]);

    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    argot_core::train::run_train(
        &repo,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )
    .expect("train");
    run_calibrate(
        &repo,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &CalibrateOptions {
            repo_sha: "fixture".to_string(),
            timestamp_utc: "1970-01-01T00:00:00+00:00".to_string(),
            ..Default::default()
        },
    )
    .expect("calibrate");
    repo
}

fn check_workdir(repo: &Path) -> argot_core::check::CheckOutcome {
    run_check(CheckArgs {
        repo_path: repo.to_str().unwrap().to_string(),
        reference: String::new(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: repo.join(".argot"),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_confidence: "unusual".to_string(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        format: OutputFormat::Human,
        use_color: false,
        today: "2026-07-27".to_string(),
    })
}

#[test]
fn check_only_paths_are_judged_on_dependencies_and_never_teach_style() {
    let repo = prepare_repo("asymmetry");

    // 1. Out of the corpus: the fit never read a test file.
    let corpus = std::fs::read_to_string(repo.join(".argot/repo-corpus.txt")).unwrap();
    assert!(
        !corpus.contains("test_stats.py"),
        "a check-only file entered the voice corpus:\n{corpus}"
    );

    // 2. In the vocabulary: the fit did record what those files import.
    let config: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(repo.join(".argot/scorer-config.json")).unwrap(),
    )
    .unwrap();
    let learned = config["languages"]["python"]["check_only_import_modules"]
        .as_array()
        .map(|a| {
            a.iter()
                .filter_map(|v| v.as_str())
                .any(|m| m == "fixture_harness")
        })
        .unwrap_or(false);
    assert!(learned, "the check-only vocabulary was not harvested");

    // 3. Familiar *there*: a new test reusing that vocabulary stays quiet.
    std::fs::write(
        repo.join("tests/test_text.py"),
        "import fixture_harness\n\nfrom text import normalize\n\n\ndef test_normalize():\n    with fixture_harness.clock():\n        assert normalize('  a  b ') == 'a b'\n",
    )
    .unwrap();
    let out = check_workdir(&repo);
    assert!(
        !out.stdout.contains("test_text.py"),
        "a test reusing vocabulary the tests established was flagged:\n{}",
        out.stdout
    );

    // 4. Still foreign in production: the same import from real source fires.
    std::fs::write(
        repo.join("timing.py"),
        "import fixture_harness\n\n\ndef now():\n    return fixture_harness.clock()\n",
    )
    .unwrap();
    let out = check_workdir(&repo);
    assert!(
        out.stdout.contains("timing.py"),
        "a test-only dependency leaked into production's familiar set:\n{}",
        out.stdout
    );
    std::fs::remove_file(repo.join("timing.py")).unwrap();

    // 5. A genuinely new dependency in a test still asks — the signal the
    //    blunt workaround (`foreign-import = {{ exclude = [tests] }}`) throws away.
    std::fs::write(
        repo.join("tests/test_net.py"),
        "import responses\n\nfrom text import normalize\n\n\ndef test_net():\n    responses.add()\n    assert normalize('a') == 'a'\n",
    )
    .unwrap();
    let out = check_workdir(&repo);
    assert!(
        out.stdout.contains("test_net.py") && out.stdout.contains("foreign-import"),
        "a test reaching for a brand-new dependency must still ask:\n{}",
        out.stdout
    );
}
