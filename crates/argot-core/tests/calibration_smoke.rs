//! Smoke test for calibration: run the full pipeline (train → calibrate) on
//! the check fixture repo and assert a schema-valid, check-consumable v3
//! scorer-config.json is emitted — including the fit-time model snapshot.
//! The threshold VALUE is a documented divergence from numpy (see
//! PORTING-NOTES), so this asserts structure, not exact values.

use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use argot_core::train::{run_train, GENERIC_BASELINE_JSON};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn build_repo() -> PathBuf {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("calib_check_repo");
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
fn calibrate_emits_valid_v3_config() {
    let repo = build_repo();
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
    assert!(!thresholds.is_empty(), "expected at least one language");

    let cfg: Value = serde_json::from_str(&std::fs::read_to_string(&out).unwrap()).unwrap();
    assert_eq!(cfg["version"], 3);
    let py = &cfg["languages"]["python"];
    assert!(py.is_object(), "python language block present");
    assert!(py["threshold"].is_number(), "threshold is a number");
    assert_eq!(py["call_receiver_n_clusters"], 8);
    assert_eq!(py["call_receiver_cluster_bonus"], 5.0);
    assert!(py["import_modules"].is_array());
    assert!(py["import_module_prefixes"].is_array());
    let ev = &py["evidence_corpus"];
    assert!(ev["imports"].is_array(), "evidence imports");
    assert!(ev["identifiers"].is_object(), "evidence identifiers");
    assert!(ev["callees_by_cluster"].is_object(), "callees_by_cluster");
    assert!(ev["totals"]["import_specifiers_attested"].is_number());
    // Sanity: import_modules should include the stdlib modules the corpus imports.
    let mods: Vec<String> = py["import_modules"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        mods.contains(&"math".to_string()),
        "math imported, got {mods:?}"
    );
    // v3 model snapshot: BPE stats + callee attestation + clusters.
    let model = &py["model"];
    assert!(
        model["bpe"]["token_counts"].is_object(),
        "model bpe token counts"
    );
    assert!(model["bpe"]["total_tokens"].as_u64().unwrap() > 0);
    let attested: Vec<String> = model["call_receiver"]["attested"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| v.as_str().unwrap().to_string())
        .collect();
    assert!(
        attested.contains(&"math.fsum".to_string()),
        "corpus callee attested, got {attested:?}"
    );
    let clusters = model["call_receiver"]["clusters"].as_object().unwrap();
    assert!(!clusters.is_empty(), "cluster partition present");
    // Cluster file keys are repo-relative (no absolute prefixes).
    for (_, cluster) in clusters {
        for f in cluster["files"].as_array().unwrap() {
            assert!(
                !f.as_str().unwrap().starts_with('/'),
                "cluster file paths are repo-relative, got {f}"
            );
        }
    }
    assert_eq!(
        py["model_hash"].as_str().unwrap().len(),
        32,
        "model hash is an md5 hex digest"
    );
}
