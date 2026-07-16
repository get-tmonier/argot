//! Regression test for BPE token-surprise scoring: the golden pins the
//! per-token surprise output over a fixed corpus.

use argot_core::bpe::BpeTokenizer;
use argot_core::scoring::bpe_scorer::BpeScorer;
use argot_core::text::read_text_lossy;
use argot_core::train::GENERIC_BASELINE_JSON;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Deserialize)]
struct ScoreCase {
    hunk: String,
    score: f64,
}

#[derive(Deserialize)]
struct Golden {
    total_repo: f64,
    total_generic: f64,
    scores: Vec<ScoreCase>,
    surprise: std::collections::HashMap<String, f64>,
}

fn fixtures() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bpe_score")
}

fn build_scorer() -> BpeScorer {
    let corpus_dir = fixtures().join("corpus");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&corpus_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "py").unwrap_or(false))
        .collect();
    files.sort();
    let sources: Vec<String> = files.iter().map(|p| read_text_lossy(p).unwrap()).collect();
    BpeScorer::new(BpeTokenizer::load(), GENERIC_BASELINE_JSON, &sources).unwrap()
}

#[test]
fn bpe_score_matches_python_golden() {
    let golden: Golden =
        serde_json::from_str(&std::fs::read_to_string(fixtures().join("golden.json")).unwrap())
            .unwrap();
    let scorer = build_scorer();

    // Integer totals must match exactly.
    assert_eq!(scorer.total_repo(), golden.total_repo, "total_repo");
    assert_eq!(
        scorer.total_generic(),
        golden.total_generic,
        "total_generic"
    );

    // token_surprise probes — bit-level (same libm, same op order).
    for (id_str, want) in &golden.surprise {
        let id: u32 = id_str.parse().unwrap();
        let got = scorer.token_surprise(id);
        assert!(
            (got - want).abs() < 1e-12,
            "surprise[{id}] = {got} != {want}"
        );
    }

    // per-hunk bpe_score within a tight float epsilon.
    for case in &golden.scores {
        let got = scorer.bpe_score(&case.hunk);
        assert!(
            (got - case.score).abs() < 1e-9,
            "bpe_score({:?}) = {got} != {}",
            case.hunk,
            case.score
        );
    }
}
