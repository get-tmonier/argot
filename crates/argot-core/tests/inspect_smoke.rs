//! Smoke test for `inspect`: run it pre-fit and post-fit on the check fixture
//! repo and assert the report reflects the corpus and the emitted
//! scorer-config.json. Verdict thresholds are unit-tested in-module; this
//! covers the end-to-end wiring (walk → candidates → config read).

use argot_core::inspect::inspect_repo;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use argot_core::train::{run_train, GENERIC_BASELINE_JSON};
use std::path::PathBuf;
use std::process::Command;

fn build_repo() -> PathBuf {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("inspect_check_repo");
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
    assert!(status.success());
    out
}

#[test]
fn inspect_pre_and_post_fit() {
    let repo = build_repo();

    // Pre-fit: corpus composition only, no calibration block.
    let pre = inspect_repo(&repo).expect("inspect pre-fit");
    assert!(pre.calibration.is_none(), "no scorer-config yet");
    let py = pre
        .corpus
        .languages
        .get("python")
        .expect("python in fixture corpus");
    assert!(py.files > 0);
    assert!(
        py.candidate_hunks > 0,
        "fixture repo has sampleable functions"
    );

    // Fit, then inspect again: calibration block mirrors the config.
    let argot_dir = repo.join(".argot");
    let repo_corpus = argot_dir.join("repo-corpus.txt");
    let generic = argot_dir.join("generic-baseline.json");
    run_train(&repo, &repo_corpus, &generic).expect("train");
    let out = argot_dir.join("scorer-config.json");
    let opts = CalibrateOptions {
        timestamp_utc: "1970-01-01T00:00:00+00:00".to_string(),
        repo_sha: "deadbeef".to_string(),
        ..Default::default()
    };
    let thresholds =
        run_calibrate(&repo, &repo_corpus, GENERIC_BASELINE_JSON, &out, &opts).expect("calibrate");

    let post = inspect_repo(&repo).expect("inspect post-fit");
    let cal = post.calibration.expect("calibration block post-fit");
    let py_cal = cal.languages.get("python").expect("python calibrated");
    let (_, expected_threshold) = thresholds
        .iter()
        .find(|(l, _)| l == "python")
        .expect("python threshold");
    assert_eq!(py_cal.threshold, *expected_threshold);
    assert_eq!(py_cal.n_seeds, 7);
    assert_eq!(py_cal.repo_sha, "deadbeef");
    assert_eq!(py_cal.timestamp_utc, "1970-01-01T00:00:00+00:00");
    assert_eq!(
        py_cal.candidate_hunks_now, post.corpus.languages["python"].candidate_hunks,
        "live pass feeds the comparison"
    );
    // The fixture repo is tiny, so calibration used every candidate.
    assert_eq!(py_cal.n_cal, py_cal.candidate_hunks_now.min(500));
}

/// The twin of `corpus.rs::gitignored_untracked_paths_are_not_voice`.
///
/// `inspect` is what a human — or an agent following the setup skill — reads to
/// decide whether setup is healthy, and it used to walk the tree without git's
/// view. On a JS monorepo that meant a `node_modules/` tree outnumbering the
/// authored code, counted as corpus, driving a polyglot warning about a
/// language with zero tracked files. The fit was never polluted; only the
/// report was, which is worse: it invites an exclusion entry that changes
/// nothing.
#[test]
fn inspect_reports_the_corpus_the_fit_will_use_not_the_raw_scan() {
    let repo = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("inspect_gitignored_repo");
    let _ = std::fs::remove_dir_all(&repo);
    std::fs::create_dir_all(repo.join("src")).unwrap();
    std::fs::create_dir_all(repo.join("node_modules/leftpad")).unwrap();

    let git = |args: &[&str]| {
        let ok = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .env("GIT_AUTHOR_NAME", "t")
            .env("GIT_AUTHOR_EMAIL", "t@t")
            .env("GIT_COMMITTER_NAME", "t")
            .env("GIT_COMMITTER_EMAIL", "t@t")
            .status()
            .expect("run git")
            .success();
        assert!(ok, "git {args:?} failed");
    };
    git(&["init", "-q", "-b", "main"]);
    std::fs::write(repo.join(".gitignore"), "node_modules/\n").unwrap();
    for i in 0..5 {
        std::fs::write(
            repo.join(format!("src/m{i}.ts")),
            format!("export function f{i}(a: number) {{ return a + {i} }}\n"),
        )
        .unwrap();
    }
    // Enough gitignored JavaScript to dominate the raw scan outright.
    for i in 0..40 {
        std::fs::write(
            repo.join(format!("node_modules/leftpad/i{i}.js")),
            "module.exports = function pad(s) { return s }\n",
        )
        .unwrap();
    }
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init"]);

    let report = inspect_repo(&repo).expect("inspect");
    assert!(
        !report.corpus.languages.contains_key("javascript"),
        "gitignored dependency code was counted as corpus: {:?}",
        report.corpus.languages
    );
    assert_eq!(
        report.corpus.languages["typescript"].files, 5,
        "the tracked TypeScript is the whole corpus"
    );
    assert!(
        !report.corpus.meaningfully_mixed,
        "a polyglot verdict was derived from files the fit never reads"
    );
    assert!(
        report
            .corpus
            .gitignored_dirs
            .iter()
            .any(|d| d == "node_modules"),
        "the skipped tree is named rather than silently dropped: {:?}",
        report.corpus.gitignored_dirs
    );

    // The authoritative answer — what a fit actually collects — must agree.
    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    run_train(
        &repo,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )
    .expect("train");
    let corpus = std::fs::read_to_string(argot_dir.join("repo-corpus.txt")).unwrap();
    assert_eq!(
        corpus.lines().filter(|l| !l.trim().is_empty()).count(),
        report
            .corpus
            .languages
            .values()
            .map(|l| l.files)
            .sum::<usize>(),
        "inspect and the fit disagree on the corpus size:\n{corpus}"
    );
    let _ = std::fs::remove_dir_all(&repo);
}
