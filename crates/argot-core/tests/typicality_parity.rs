//! Golden-fixture parity test for the typicality filter.
//!
//! The golden (`tests/fixtures/typicality/golden.json`) was captured from the
//! Python engine (`engine/argot/scoring/filters/typicality.py`). It maps
//! language → sample name → the source and the expected features/verdicts. The
//! Rust port must reproduce every one exactly — this is the objective
//! acceptance test for the port.

use std::collections::HashMap;
use std::path::PathBuf;

use argot_core::scoring::adapters::Language;
use argot_core::scoring::typicality::{compute_features, TypicalityModel};
use serde::Deserialize;

/// Float tolerance for the four continuous features.
const EPS: f64 = 1e-9;

#[derive(Debug, Deserialize)]
struct GoldenSample {
    src: String,
    literal_leaf_ratio: f64,
    named_leaf_count: usize,
    control_node_density: f64,
    ast_type_entropy: f64,
    unique_token_ratio: f64,
    is_atypical: bool,
    is_atypical_file: bool,
}

fn load_golden() -> HashMap<String, HashMap<String, GoldenSample>> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/typicality/golden.json");
    let bytes = std::fs::read(path).expect("read golden.json");
    serde_json::from_slice(&bytes).expect("parse golden.json")
}

fn language_for(tag: &str) -> Language {
    match tag {
        "python" => Language::Python,
        "typescript" => Language::Typescript,
        other => panic!("unknown language tag in golden: {other}"),
    }
}

#[test]
fn typicality_matches_golden() {
    let golden = load_golden();
    assert!(!golden.is_empty(), "golden fixture is empty");

    for (lang_tag, samples) in &golden {
        let language = language_for(lang_tag);
        let model = TypicalityModel::new(language);
        assert!(!samples.is_empty(), "[{lang_tag}] no samples");

        for (name, sample) in samples {
            let label = format!("{lang_tag}/{name}");
            let features = compute_features(&sample.src, language);

            assert_eq!(
                features.named_leaf_count, sample.named_leaf_count,
                "[{label}] named_leaf_count mismatch"
            );

            assert!(
                (features.literal_leaf_ratio - sample.literal_leaf_ratio).abs() <= EPS,
                "[{label}] literal_leaf_ratio: got {}, want {}",
                features.literal_leaf_ratio,
                sample.literal_leaf_ratio
            );
            assert!(
                (features.control_node_density - sample.control_node_density).abs() <= EPS,
                "[{label}] control_node_density: got {}, want {}",
                features.control_node_density,
                sample.control_node_density
            );
            assert!(
                (features.ast_type_entropy - sample.ast_type_entropy).abs() <= EPS,
                "[{label}] ast_type_entropy: got {}, want {}",
                features.ast_type_entropy,
                sample.ast_type_entropy
            );
            assert!(
                (features.unique_token_ratio - sample.unique_token_ratio).abs() <= EPS,
                "[{label}] unique_token_ratio: got {}, want {}",
                features.unique_token_ratio,
                sample.unique_token_ratio
            );

            assert_eq!(
                model.is_atypical(&sample.src).0,
                sample.is_atypical,
                "[{label}] is_atypical mismatch"
            );
            assert_eq!(
                model.is_atypical_file(&sample.src).0,
                sample.is_atypical_file,
                "[{label}] is_atypical_file mismatch"
            );
        }
    }
}
