//! The fit-time model snapshot pins scoring to the voice as learned (#79).
//!
//! Regression suite for the era-15 model artifact: check-time scoring must
//! come entirely from the v3 config's `model` block, never the live tree.
//! Before the snapshot, `check` rebuilt callee attestation and BPE token
//! counts from the corpus files on disk at check time — so brand-new code
//! attested its own callees and the unattested-callee branches never fired
//! on exactly the code `check` exists to judge.
//!
//! Requires `git` and `bash` on PATH (fixture build).

use argot_core::check::{run_check, CheckArgs};
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("model_snapshot_repo_{suffix}"));
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

fn check_workdir_json(repo: &Path) -> Value {
    let out = run_check(CheckArgs {
        repo_path: repo.to_str().unwrap().to_string(),
        reference: String::new(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        // Surface every scored hunk so the assertion sees scores, not just
        // flag decisions.
        threshold: Some(-1000.0),
        argot_dir: repo.join(".argot"),
        hunk_lines: 6,
        verbose: false,
        min_confidence: "unusual".to_string(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: argot_core::output::OutputFormat::Json,
        today: "2026-01-01".to_string(),
    });
    serde_json::from_str(&out.stdout).expect("check emits JSON")
}

/// The #79 scenario: after fit, a corpus file gains callees the corpus has
/// never seen. The file on disk now *contains* those callees, so an
/// attestation rebuilt from the live tree attests them (contribution 0) —
/// while the fit-time snapshot must keep firing the unattested-callee
/// branches. Both sides are asserted so the contrast is pinned.
#[test]
fn new_code_cannot_attest_its_own_callees() {
    use argot_core::scoring::adapters::python::PythonAdapter;
    use argot_core::scoring::model::LanguageModel;
    use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
    use argot_core::text::read_text_lossy;

    let repo = build_fixture_repo("selfattest");
    fit(&repo);

    // Post-fit change: unattested callees, no foreign import. `graph.py` is a
    // corpus file, so its on-disk content self-attests under a live rebuild.
    let graph_py = repo.join("graph.py");
    let mut content = std::fs::read_to_string(&graph_py).unwrap();
    let appended =
        "\n\ndef degree_summary(adj):\n    counts = tally_degrees(adj)\n    return normalize_histogram(counts)\n";
    content.push_str(appended);
    std::fs::write(&graph_py, content.clone()).unwrap();

    let hunk = "def degree_summary(adj):\n    counts = tally_degrees(adj)\n    return normalize_histogram(counts)";
    let lines = content.lines().count();
    let (hs, he) = (lines - 2, lines);

    let config: Value = serde_json::from_str(
        &std::fs::read_to_string(repo.join(".argot/scorer-config.json")).unwrap(),
    )
    .unwrap();
    let lang_cfg = &config["languages"]["python"];
    let model: LanguageModel = serde_json::from_value(lang_cfg["model"].clone()).unwrap();
    let cfg = || SequentialConfig {
        bpe_threshold: lang_cfg["threshold"].as_f64().unwrap(),
        enable_typicality: true,
        exclude_data_dominant: true,
        call_receiver_alpha: 2.0,
        call_receiver_cap: 5,
        call_receiver_root_bonus: 2.0,
        call_receiver_n_clusters: 8,
        call_receiver_cluster_seed: 0,
        call_receiver_cluster_bonus: 5.0,
        call_receiver_cluster_rare_threshold: 0,
        call_receiver_cluster_size_min: 0,
        call_receiver_rarity_weighting: argot_core::scoring::call_receiver::RarityWeighting::Off,
        call_receiver_shape_primitive_names: Vec::new(),
        call_receiver_parse_error_host_fallback: false,
        conventions: None,
        convention_bonus: 0.0,
        import_modules: Vec::new(),
        import_module_prefixes: Vec::new(),
        evidence_corpus: None,
        detect: argot_core::config::DetectConfig::default(),
    };
    let baseline = std::fs::read(repo.join(".argot/generic-baseline.json")).unwrap();

    // Snapshot path (what check does now): the new callees are unattested.
    let mut from_snapshot = SequentialImportBpeScorer::from_model(
        &model,
        &baseline,
        Box::new(PythonAdapter::new()),
        cfg(),
    )
    .unwrap();
    let scored = from_snapshot.score_hunk(
        hunk,
        Some(&content),
        Some(hs),
        Some(he),
        Some(Path::new("graph.py")),
    );
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "fit-time attestation must treat post-fit callees as unattested; got {:?}",
        scored.stages
    );

    // Live-tree path (the pre-#79 behaviour): the same hunk self-attests.
    let corpus_txt = std::fs::read_to_string(repo.join(".argot/repo-corpus.txt")).unwrap();
    let repo_files: Vec<(PathBuf, String)> = corpus_txt
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .filter_map(|p| read_text_lossy(&p).ok().map(|s| (p, s)))
        .collect();
    let mut from_disk = SequentialImportBpeScorer::from_config(
        &repo_files,
        &baseline,
        Box::new(PythonAdapter::new()),
        cfg(),
    )
    .unwrap();
    let scored_disk = from_disk.score_hunk(hunk, Some(&content), Some(hs), Some(he), None);
    assert_eq!(
        scored_disk.stages.call_receiver_contribution, 0.0,
        "live-tree attestation self-attests (the bug this suite pins)"
    );

    // End-to-end: the workdir hunk is scored and flagged through run_check.
    let doc = check_workdir_json(&repo);
    let hits: Vec<&Value> = doc["hits"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|h| h["path"] == "graph.py")
        .collect();
    assert!(!hits.is_empty(), "graph.py hunk was scored");
}

/// Same fit twice → byte-identical artifact (the #63 reproducibility slice).
#[test]
fn fit_is_deterministic() {
    let repo = build_fixture_repo("determinism");
    fit(&repo);
    let first = std::fs::read(repo.join(".argot/scorer-config.json")).unwrap();
    fit(&repo);
    let second = std::fs::read(repo.join(".argot/scorer-config.json")).unwrap();
    assert_eq!(first, second, "same corpus + config → same artifact bytes");
}

/// Check no longer needs `repo-corpus.txt` — the model block carries the
/// fitted state. Deleting the corpus listing must not break check.
#[test]
fn check_runs_without_repo_corpus_listing() {
    let repo = build_fixture_repo("nocorpus");
    fit(&repo);
    std::fs::remove_file(repo.join(".argot/repo-corpus.txt")).unwrap();
    let doc = check_workdir_json(&repo);
    assert!(doc["hits"].is_array(), "check ran from the model alone");
}

/// A v2 (pre-model) config is refused with the regeneration hint.
#[test]
fn v2_config_is_refused() {
    let repo = build_fixture_repo("v2refused");
    fit(&repo);
    let config_path = repo.join(".argot/scorer-config.json");
    let mut config: Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).unwrap()).unwrap();
    config["version"] = serde_json::json!(2);
    std::fs::write(&config_path, serde_json::to_string(&config).unwrap()).unwrap();

    let out = run_check(CheckArgs {
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
    });
    assert_eq!(out.exit_code, 2);
    assert!(
        out.stderr.contains("config version 2") && out.stderr.contains("argot fit"),
        "stderr explains the version mismatch: {}",
        out.stderr
    );
}
