//! Golden-fixture parity test for the Python language adapter.
//!
//! The golden (`tests/fixtures/adapter_py/golden.json`) pins the expected
//! output of every adapter method over a set of source samples. The
//! `PythonAdapter` must reproduce all of them exactly — the golden is
//! authoritative, so a diff here is a regression until the golden is
//! deliberately re-blessed.

use std::collections::HashMap;
use std::path::PathBuf;

use argot_core::scoring::adapters::python::PythonAdapter;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct GoldenSample {
    src: String,
    extract_imports: Vec<String>,
    imports_with_spans: Vec<(String, usize, usize, usize)>,
    prose_line_ranges: Vec<usize>,
    is_data_dominant: bool,
    is_auto_generated: bool,
    enumerate_sampleable_ranges: Vec<(usize, usize)>,
}

fn load_golden() -> HashMap<String, GoldenSample> {
    let path =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adapter_py/golden.json");
    let bytes = std::fs::read(path).expect("read golden.json");
    serde_json::from_slice(&bytes).expect("parse golden.json")
}

#[test]
fn python_adapter_matches_golden() {
    let adapter = PythonAdapter::new();
    let golden = load_golden();
    assert!(!golden.is_empty(), "golden fixture is empty");

    for (name, sample) in &golden {
        // extract_imports — compare as sorted lists.
        let mut imports: Vec<String> = adapter.extract_imports(&sample.src).into_iter().collect();
        imports.sort();
        assert_eq!(
            imports, sample.extract_imports,
            "[{name}] extract_imports mismatch"
        );

        // imports_with_spans — already sorted by the adapter; compare exactly.
        assert_eq!(
            adapter.extract_imports_with_spans(&sample.src),
            sample.imports_with_spans,
            "[{name}] imports_with_spans mismatch"
        );

        // prose_line_ranges — compare as sorted lists.
        let mut prose: Vec<usize> = adapter.prose_line_ranges(&sample.src).into_iter().collect();
        prose.sort_unstable();
        assert_eq!(
            prose, sample.prose_line_ranges,
            "[{name}] prose_line_ranges mismatch"
        );

        assert_eq!(
            adapter.is_data_dominant(&sample.src, 0.65),
            sample.is_data_dominant,
            "[{name}] is_data_dominant mismatch"
        );

        assert_eq!(
            adapter.is_auto_generated(
                &sample.src,
                &argot_core::config::default_generated_markers()
            ),
            sample.is_auto_generated,
            "[{name}] is_auto_generated mismatch"
        );

        assert_eq!(
            adapter.enumerate_sampleable_ranges(&sample.src),
            sample.enumerate_sampleable_ranges,
            "[{name}] enumerate_sampleable_ranges mismatch"
        );
    }
}
