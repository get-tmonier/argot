//! The shipped example rules (`examples/rules/`) must stay green.
//!
//! They are what a rule author copies and what the docs point at, so a host-API
//! change that breaks one has to break the build too — an example that no
//! longer works teaches the wrong thing to everyone who reads it.

use argot_rules_script::harness::run_rule_tests;
use std::path::PathBuf;

/// The examples directory is laid out like an `.argot/` dir (`rules/<name>/`),
/// so the harness discovers it unchanged.
fn examples_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../examples")
        .canonicalize()
        .expect("examples/ next to the workspace root")
}

#[test]
fn every_example_rule_passes_its_own_fixtures() {
    let mut warnings = Vec::new();
    let results =
        run_rule_tests(&examples_dir(), None, &mut warnings).expect("examples discovered");
    assert!(
        warnings.is_empty(),
        "an example rule failed to load: {warnings:?}"
    );
    assert!(
        !results.is_empty(),
        "no example cases ran — did examples/rules/ move?"
    );
    let failures: Vec<String> = results
        .iter()
        .filter_map(|r| {
            r.failure
                .as_ref()
                .map(|f| format!("{}::{} — {f}", r.rule, r.case))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Each example must carry both a firing and a silent case: a rule proven only
/// to fire has never been checked for false alarms, and that is precisely the
/// habit the examples are there to teach.
#[test]
fn every_example_rule_has_a_firing_and_a_silent_case() {
    let mut warnings = Vec::new();
    let results = run_rule_tests(&examples_dir(), None, &mut warnings).unwrap();
    let mut by_rule: std::collections::BTreeMap<&str, (bool, bool)> =
        std::collections::BTreeMap::new();
    for r in &results {
        let entry = by_rule.entry(r.rule.as_str()).or_default();
        if r.case.starts_with("fires-") {
            entry.0 = true;
        }
        if r.case.starts_with("silent-") {
            entry.1 = true;
        }
    }
    for (rule, (fires, silent)) in by_rule {
        assert!(fires, "{rule} has no `fires-…` case");
        assert!(silent, "{rule} has no `silent-…` case");
    }
}
