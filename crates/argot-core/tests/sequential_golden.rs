//! Integration golden for the sequential composite scorer's per-hunk scoring
//! with the call-receiver DISABLED (alpha=0) — isolates typicality + import +
//! BPE + multi-reason resolution, fully reproducible (no KMeans).

use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use argot_core::text::read_text_lossy;
use argot_core::train::GENERIC_BASELINE_JSON;
use serde::Deserialize;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Case {
    hunk: String,
    score: f64,
    #[allow(dead_code)]
    threshold: f64,
    flagged: bool,
    reason: String,
    import_score: f64,
    bpe_score: f64,
}

#[derive(Deserialize)]
struct Golden {
    bpe_threshold: f64,
    import_modules: Vec<String>,
    cases: Vec<Case>,
}

#[test]
fn score_hunk_matches_python_golden_no_cr() {
    let fixtures = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let golden: Golden = serde_json::from_str(
        &std::fs::read_to_string(fixtures.join("sequential/golden_no_cr.json")).unwrap(),
    )
    .unwrap();

    let corpus_dir = fixtures.join("bpe_score/corpus");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&corpus_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "py").unwrap_or(false))
        .collect();
    files.sort();
    let repo_files: Vec<(PathBuf, String)> = files
        .iter()
        .map(|p| (p.clone(), read_text_lossy(p).unwrap()))
        .collect();

    let cfg = SequentialConfig {
        bpe_threshold: golden.bpe_threshold,
        enable_typicality: true,
        exclude_data_dominant: false,
        call_receiver_alpha: 0.0,
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
        import_modules: golden.import_modules.clone(),
        import_module_prefixes: vec![],
        check_only_import_modules: vec![],
        check_only_patterns: vec![],
        evidence_corpus: None,
        detect: argot_core::config::DetectConfig::default(),
    };
    let scorer = SequentialImportBpeScorer::from_config(
        &repo_files,
        GENERIC_BASELINE_JSON,
        Box::new(PythonAdapter::new()),
        cfg,
    )
    .unwrap();

    for case in &golden.cases {
        // file_source == hunk, spanning line 1..=line_count; file_path=None.
        let line_count = case.hunk.matches('\n').count() + 1;
        let got = scorer.score_hunk(
            &case.hunk,
            Some(&case.hunk),
            Some(1),
            Some(line_count),
            None,
        );
        assert_eq!(
            got.reason.as_str(),
            case.reason,
            "reason for {:?}",
            case.hunk
        );
        assert_eq!(got.flagged, case.flagged, "flagged for {:?}", case.hunk);
        assert!(
            (got.stages.import_score - case.import_score).abs() < 1e-12,
            "import_score for {:?}: {} != {}",
            case.hunk,
            got.stages.import_score,
            case.import_score
        );
        assert!(
            (got.stages.bpe_score - case.bpe_score).abs() < 1e-9,
            "bpe_score for {:?}: {} != {}",
            case.hunk,
            got.stages.bpe_score,
            case.bpe_score
        );
        assert!(
            (got.score - case.score).abs() < 1e-9,
            "score for {:?}: {} != {}",
            case.hunk,
            got.score,
            case.score
        );
    }
}
