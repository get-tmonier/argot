//! `argot check --format json|sarif` against the deterministic check fixture.
//!
//! Structural (not byte-parity) assertions: the machine formats must emit a
//! single well-formed document on stdout — nothing else — with exit semantics
//! identical to the human path (0 clean, 1 when hits are visible).
//!
//! Requires `git` and `bash` on PATH (fixture build). Reuses the same fixture
//! repo + fit flow (train → calibrate) as `check_parity.rs`.

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, CheckArgs};
use argot_core::output::OutputFormat;
use argot_core::scoring::calibration::{run_calibrate, CalibrateOptions};
use serde_json::Value;

fn assert_check_json_v1(doc: &Value) {
    let schema: Value =
        serde_json::from_str(include_str!("../../../docs/schema/check-v1.schema.json"))
            .expect("valid check JSON schema");
    let validator = jsonschema::JSONSchema::compile(&schema).expect("compile check JSON schema");
    let errors = validator
        .validate(doc)
        .err()
        .map(|errors| errors.map(|error| error.to_string()).collect::<Vec<_>>());
    if let Some(errors) = errors {
        panic!(
            "check JSON v1 schema validation failed: {}",
            errors.join("; ")
        );
    }
}

#[test]
fn check_json_v1_accepts_unknown_consumer_fields() {
    let schema: Value =
        serde_json::from_str(include_str!("../../../docs/schema/check-v1.schema.json"))
            .expect("valid check JSON schema");
    let validator = jsonschema::JSONSchema::compile(&schema).expect("compile check JSON schema");
    let mut doc = serde_json::json!({
        "schema_version": 1,
        "tool": { "name": "argot", "version": "0.0.0-test" },
        "model": "test-model",
        "repo": "/tmp/repo",
        "scanned": "workdir",
        "hunks_scanned": 0,
        "files_scanned": [],
        "result": {
            "exit_code": 0,
            "unsuppressed_hits": 0,
            "visible_hits": 0,
            "hidden_hits": 0,
            "suppressed_hits": 0,
            "error_hits": 0,
            "warn_hits": 0,
            "gating_hits": 0
        },
        "hits": []
    });
    doc["consumer_extension"] = serde_json::json!({ "future": true });
    assert!(
        validator.is_valid(&doc),
        "v1 consumers must tolerate additive unknown fields"
    );
}

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    // Distinct dir per test: tests run in parallel.
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("check_format_repo_{suffix}"));
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

fn prepare_repo(suffix: &str) -> PathBuf {
    let repo = build_fixture_repo(suffix);
    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    argot_core::train::run_train(
        &repo,
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
        &repo,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &opts,
    )
    .expect("calibrate");
    repo
}

fn base_args(repo: &Path, format: OutputFormat) -> CheckArgs {
    CheckArgs {
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
        format,
        today: "2026-01-01".to_string(),
    }
}

#[test]
fn check_json_format_emits_a_single_json_document_with_hits() {
    let repo = prepare_repo("json_hits");
    let mut args = base_args(&repo, OutputFormat::Json);
    args.reference = "HEAD~2..HEAD".to_string();
    args.threshold = Some(-1000.0); // every scored hunk becomes a hit
    let out = run_check(args);

    assert_eq!(out.exit_code, 1, "hits present → exit 1");
    // Stdout must be exactly one JSON document — parse the whole string.
    let doc: Value = serde_json::from_str(&out.stdout).expect("stdout is pure JSON");
    assert_eq!(doc["tool"]["name"], "argot");
    assert_eq!(doc["schema_version"], 1);
    assert_eq!(doc["tool"]["version"], env!("CARGO_PKG_VERSION"));
    let hits = doc["hits"].as_array().expect("hits array");
    assert!(!hits.is_empty(), "threshold -1000 must surface hits");
    for h in hits {
        assert!(h["path"].is_string());
        assert!(h["line_start"].as_u64().unwrap() >= 1);
        assert!(h["line_end"].as_u64().unwrap() >= h["line_start"].as_u64().unwrap());
        assert!(h["score"].is_number());
        assert!(h["threshold"].is_number());
        assert!(
            ["unusual", "suspicious", "foreign"].contains(&h["confidence"].as_str().unwrap()),
            "confidence is a known tier"
        );
        assert!(
            ["error", "warn"].contains(&h["severity"].as_str().unwrap()),
            "severity is the rule's configured level"
        );
        assert!(h["rule"].is_string());
        assert_eq!(h["hash"].as_str().unwrap().len(), 12, "hit hash present");
        assert!(h["evidence"].is_array());
    }
    assert_check_json_v1(&doc);
}

#[test]
fn check_sarif_format_emits_valid_sarif_with_hits() {
    let repo = prepare_repo("sarif_hits");
    let mut args = base_args(&repo, OutputFormat::Sarif);
    args.reference = "HEAD~2..HEAD".to_string();
    args.threshold = Some(-1000.0);
    let out = run_check(args);

    assert_eq!(out.exit_code, 1, "hits present → exit 1");
    let doc: Value = serde_json::from_str(&out.stdout).expect("stdout is pure JSON");
    assert_eq!(doc["version"], "2.1.0");
    assert!(doc["$schema"].as_str().unwrap().contains("2.1.0"));
    let run = &doc["runs"][0];
    assert_eq!(run["tool"]["driver"]["name"], "argot");
    let results = run["results"].as_array().expect("results array");
    assert!(!results.is_empty());
    let rules = run["tool"]["driver"]["rules"].as_array().expect("rules");
    for r in results {
        assert!(["note", "warning", "error"].contains(&r["level"].as_str().unwrap()));
        let idx = r["ruleIndex"].as_u64().unwrap() as usize;
        assert_eq!(rules[idx]["id"], r["ruleId"], "ruleIndex points at ruleId");
        let loc = &r["locations"][0]["physicalLocation"];
        assert!(loc["artifactLocation"]["uri"].is_string());
        assert!(loc["region"]["startLine"].as_u64().unwrap() >= 1);
        assert!(!r["message"]["text"].as_str().unwrap().is_empty());
    }
}

/// Strip CSI (`ESC [ … m`) sequences so a colored render can be compared to the
/// plain one byte-for-byte.
fn strip_ansi(s: &str) -> String {
    let mut out = String::new();
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Skip until the final byte of the CSI sequence (a letter).
            for e in chars.by_ref() {
                if e.is_ascii_alphabetic() {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

#[test]
fn check_human_color_is_purely_additive() {
    let repo = prepare_repo("human_color");

    let mut plain = base_args(&repo, OutputFormat::Human);
    plain.reference = "HEAD~2..HEAD".to_string();
    plain.threshold = Some(-1000.0); // force every scored hunk to a visible hit
    plain.use_color = false;
    let plain_out = run_check(plain);

    let mut colored = base_args(&repo, OutputFormat::Human);
    colored.reference = "HEAD~2..HEAD".to_string();
    colored.threshold = Some(-1000.0);
    colored.use_color = true;
    let colored_out = run_check(colored);

    assert_eq!(plain_out.exit_code, colored_out.exit_code);
    assert!(
        !plain_out.stdout.contains('\x1b'),
        "no-color path must be free of ANSI escapes"
    );
    assert!(
        colored_out.stdout.contains("\x1b[31m") || colored_out.stdout.contains("\x1b[1m"),
        "color path must emit ANSI accents"
    );
    // The whole point: color is additive, so stripping it recovers the exact
    // no-color render the parity fixtures pin.
    assert_eq!(
        strip_ansi(&colored_out.stdout),
        plain_out.stdout,
        "ANSI-stripped colored output must equal the plain output"
    );
}

#[test]
fn check_json_format_clean_run_exits_zero_with_empty_hits() {
    let repo = prepare_repo("json_clean");
    let mut args = base_args(&repo, OutputFormat::Json);
    args.reference = "HEAD~1..HEAD".to_string(); // clean range in the fixture
    let out = run_check(args);

    assert_eq!(out.exit_code, 0, "clean run → exit 0");
    let doc: Value = serde_json::from_str(&out.stdout).expect("stdout is pure JSON");
    assert_eq!(doc["hits"].as_array().unwrap().len(), 0);
    assert!(doc["hunks_scanned"].as_u64().unwrap() >= 1);
    assert_check_json_v1(&doc);
}
