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
    let status = Command::new("bash")
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
