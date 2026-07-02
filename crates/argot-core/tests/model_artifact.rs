//! The model artifact is versioned, hashed, and deterministic (#63).
//!
//! `argot fit` writes `.argot/manifest.json` next to `scorer-config.json`. With
//! a fixed seed, timestamp, and repo sha, two fits of the same corpus produce
//! byte-identical artifacts — the reproducibility guarantee the manifest exists
//! to prove.
//!
//! Requires `git` and `bash` on PATH (fixture build).

use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("model_artifact_repo_{suffix}"));
    let script = fixture_dir().join("build_check_repo.sh");
    let status = Command::new("bash")
        .arg(&script)
        .arg(&out)
        .status()
        .expect("run build_check_repo.sh");
    assert!(status.success(), "fixture build failed");
    out
}

/// Fit into a fresh `.argot` subdir with deterministic options; returns its path.
fn fit_into(repo: &Path, name: &str) -> PathBuf {
    let argot_dir = repo.join(name);
    std::fs::create_dir_all(&argot_dir).unwrap();
    let corpus = argot_dir.join("repo-corpus.txt");
    argot_core::train::run_train(repo, &corpus, &argot_dir.join("generic-baseline.json"))
        .expect("train");
    let opts = CalibrateOptions {
        repo_sha: "deadbeef".to_string(),
        timestamp_utc: "1970-01-01T00:00:00+00:00".to_string(),
        ..Default::default()
    };
    run_calibrate(
        repo,
        &corpus,
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &opts,
    )
    .expect("calibrate");
    argot_dir
}

#[test]
fn fit_writes_a_manifest_with_the_expected_fields() {
    let repo = build_fixture_repo("manifest_fields");
    let argot = fit_into(&repo, ".argot");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(argot.join("manifest.json")).expect("manifest exists"),
    )
    .expect("manifest is JSON");

    assert_eq!(manifest["manifest_version"], 1);
    assert_eq!(manifest["config_version"], 3);
    assert_eq!(manifest["fit_commit_sha"], "deadbeef");
    // 12-char short hashes for the artifact-level fingerprints.
    assert_eq!(manifest["model_hash"].as_str().unwrap().len(), 12);
    assert_eq!(manifest["scorer_config_hash"].as_str().unwrap().len(), 12);
    assert!(manifest["corpus"]["files"].as_u64().unwrap() >= 1);
    assert!(manifest["corpus"]["lines"].as_u64().unwrap() >= 1);
    assert!(!manifest["languages"].as_array().unwrap().is_empty());
}

#[test]
fn two_fits_of_the_same_corpus_are_byte_identical() {
    let repo = build_fixture_repo("determinism");
    let a = fit_into(&repo, ".argot_a");
    let b = fit_into(&repo, ".argot_b");

    for file in ["scorer-config.json", "manifest.json"] {
        let bytes_a = std::fs::read(a.join(file)).unwrap();
        let bytes_b = std::fs::read(b.join(file)).unwrap();
        assert_eq!(
            bytes_a, bytes_b,
            "{file} differs between two deterministic fits"
        );
    }
}

#[test]
fn the_manifest_model_hash_matches_the_hash_check_would_compute() {
    // The manifest's combined model_hash is derived from the per-language
    // model_hash fields the same way `check` derives the hash it prints — so
    // the two can never silently disagree.
    let repo = build_fixture_repo("hash_agreement");
    let argot = fit_into(&repo, ".argot");
    let config: Value =
        serde_json::from_slice(&std::fs::read(argot.join("scorer-config.json")).unwrap()).unwrap();
    let manifest: Value =
        serde_json::from_slice(&std::fs::read(argot.join("manifest.json")).unwrap()).unwrap();

    let per_lang: std::collections::BTreeMap<String, String> = config["languages"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(lang, lc)| (lang.clone(), lc["model_hash"].as_str().unwrap().to_string()))
        .collect();
    let expected = argot_core::scoring::calibration::combined_model_hash(&per_lang);
    assert_eq!(manifest["model_hash"].as_str().unwrap(), expected);
}
