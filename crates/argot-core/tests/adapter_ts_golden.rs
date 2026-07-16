//! Golden-fixture regression test for the TypeScript language adapter.
//!
//! The golden (`tests/fixtures/adapter_ts/golden.json`) pins the expected
//! output of every adapter method over a set of source samples;
//! `resolve_repo_modules` is computed against the fixture repo, and
//! `identifier_noise` is the full sorted noise set. The `TypeScriptAdapter`
//! must reproduce all of them exactly — the golden is authoritative, so a diff
//! here is a regression until the golden is deliberately re-blessed.

use std::collections::HashMap;
use std::path::PathBuf;

use argot_core::scoring::adapters::typescript::TypeScriptAdapter;
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
    extract_callees: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct GoldenModules {
    exact: Vec<String>,
    prefixes: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Golden {
    samples: HashMap<String, GoldenSample>,
    resolve_repo_modules: GoldenModules,
    identifier_noise: Vec<String>,
}

fn fixtures() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/adapter_ts")
}

fn load_golden() -> Golden {
    let bytes = std::fs::read(fixtures().join("golden.json")).expect("read golden.json");
    serde_json::from_slice(&bytes).expect("parse golden.json")
}

fn sorted<T: Ord>(mut v: Vec<T>) -> Vec<T> {
    v.sort();
    v
}

#[test]
fn typescript_adapter_matches_golden() {
    let adapter = TypeScriptAdapter::new();
    let golden = load_golden();
    assert!(!golden.samples.is_empty(), "golden fixture is empty");

    for (name, sample) in &golden.samples {
        // extract_imports — compare as sorted lists.
        let imports = sorted(adapter.extract_imports(&sample.src).into_iter().collect());
        assert_eq!(
            imports, sample.extract_imports,
            "[{name}] extract_imports mismatch"
        );

        // imports_with_spans — adapter already sorts; compare exactly.
        assert_eq!(
            adapter.extract_imports_with_spans(&sample.src),
            sample.imports_with_spans,
            "[{name}] imports_with_spans mismatch"
        );

        // prose_line_ranges — compare as sorted lists.
        let prose = sorted(adapter.prose_line_ranges(&sample.src).into_iter().collect());
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

        // extract_callees preserves document order — compare exactly.
        assert_eq!(
            adapter.extract_callees(&sample.src),
            sample.extract_callees,
            "[{name}] extract_callees mismatch"
        );
    }

    // resolve_repo_modules against the fixture repo.
    let modules = adapter.resolve_repo_modules(&fixtures().join("repo"));
    assert_eq!(
        sorted(modules.exact.into_iter().collect::<Vec<_>>()),
        golden.resolve_repo_modules.exact,
        "resolve_repo_modules.exact mismatch"
    );
    assert_eq!(
        sorted(modules.prefixes.into_iter().collect::<Vec<_>>()),
        golden.resolve_repo_modules.prefixes,
        "resolve_repo_modules.prefixes mismatch"
    );

    // identifier_noise — full set, sorted.
    let noise = sorted(
        adapter
            .identifier_noise()
            .iter()
            .cloned()
            .collect::<Vec<_>>(),
    );
    assert_eq!(noise, golden.identifier_noise, "identifier_noise mismatch");
}
