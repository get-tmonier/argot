use super::*;
use std::fs;
use std::path::PathBuf;

fn temp_repo(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argot_inspect_{name}_{}", std::process::id()));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn py_functions(n: usize) -> String {
    (0..n)
        .map(|i| {
            format!(
                "def compute_{i}(a, b):\n    x = a + b\n    y = x * 2\n    \
                 z = y - a\n    w = z + 1\n    v = w * 3\n    u = v - b\n    return u\n"
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

fn ts_functions(n: usize) -> String {
    (0..n)
        .map(|i| {
            format!(
                "export function compute{i}(a: number, b: number): number {{\n  \
                 const x = a + b;\n  const y = x * 2;\n  const z = y - a;\n  \
                 const w = z + 1;\n  const v = w * 3;\n  return v - b;\n}}\n"
            )
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn empty_repo_is_not_recommended() {
    let dir = temp_repo("empty");
    fs::write(dir.join("README.md"), "hello").unwrap();
    let report = inspect_repo(&dir).unwrap();
    assert_eq!(report.verdict, Verdict::NotRecommended);
    assert!(report
        .reasons
        .iter()
        .any(|r| r.signal == "no_supported_files" && r.level == ReasonLevel::Red));
    assert_eq!(report.corpus.supported_files, 0);
    assert_eq!(report.corpus.unsupported_files, 1);
    assert!(report.calibration.is_none());
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn tiny_repo_is_not_recommended_for_low_candidates() {
    let dir = temp_repo("tiny");
    fs::write(dir.join("app.py"), py_functions(3)).unwrap();
    let report = inspect_repo(&dir).unwrap();
    let py = &report.corpus.languages["python"];
    assert_eq!(py.candidate_hunks, 3);
    assert_eq!(report.verdict, Verdict::NotRecommended);
    let reason = report
        .reasons
        .iter()
        .find(|r| r.signal == "low_candidate_hunks")
        .expect("low_candidate_hunks reason");
    assert_eq!(reason.level, ReasonLevel::Red);
    assert!(reason.message.contains("3 calibration candidate hunks"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn mid_size_repo_is_ready_with_notes() {
    let dir = temp_repo("mid");
    fs::write(dir.join("app.py"), py_functions(80)).unwrap();
    let report = inspect_repo(&dir).unwrap();
    assert_eq!(report.corpus.languages["python"].candidate_hunks, 80);
    assert_eq!(report.verdict, Verdict::ReadyWithNotes);
    let reason = report
        .reasons
        .iter()
        .find(|r| r.signal == "low_candidate_hunks")
        .expect("low_candidate_hunks reason");
    assert_eq!(reason.level, ReasonLevel::Yellow);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn healthy_single_language_repo_is_ready() {
    let dir = temp_repo("healthy");
    for f in 0..5 {
        fs::write(dir.join(format!("mod_{f}.py")), py_functions(50)).unwrap();
    }
    let report = inspect_repo(&dir).unwrap();
    let py = &report.corpus.languages["python"];
    assert_eq!(py.files, 5);
    assert_eq!(py.included, 5);
    assert_eq!(py.candidate_hunks, 250);
    assert!((py.share_of_supported - 1.0).abs() < 1e-9);
    assert_eq!(report.verdict, Verdict::Ready);
    assert!(report.reasons.is_empty());
    assert!(!report.corpus.meaningfully_mixed);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn polyglot_mix_is_ready_with_notes_with_polyglot_reason() {
    let dir = temp_repo("polyglot");
    for f in 0..5 {
        fs::write(dir.join(format!("mod_{f}.py")), py_functions(50)).unwrap();
        fs::write(dir.join(format!("mod_{f}.ts")), ts_functions(50)).unwrap();
    }
    let report = inspect_repo(&dir).unwrap();
    assert!(report.corpus.meaningfully_mixed);
    assert_eq!(report.verdict, Verdict::ReadyWithNotes);
    let reason = report
        .reasons
        .iter()
        .find(|r| r.signal == "polyglot_mix")
        .expect("polyglot_mix reason");
    assert_eq!(reason.level, ReasonLevel::Yellow);
    assert!(reason.message.contains("50% python"));
    assert!(reason.message.contains("50% typescript"));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn excluded_only_language_does_not_tank_verdict() {
    // Healthy Python voice, plus TypeScript that exists ONLY under an
    // excluded benchmarks/ dir (0 included). The unmodeled TS must not
    // appear in the verdict — the repo is Ready on its Python voice alone.
    // Regression: a polyglot repo with fixture/example code in a second
    // language used to read "Not recommended".
    let dir = temp_repo("excluded_lang");
    for f in 0..5 {
        fs::write(dir.join(format!("mod_{f}.py")), py_functions(50)).unwrap();
    }
    fs::create_dir_all(dir.join("benchmarks")).unwrap();
    for f in 0..5 {
        fs::write(dir.join(format!("benchmarks/b_{f}.ts")), ts_functions(50)).unwrap();
    }
    let report = inspect_repo(&dir).unwrap();
    let ts = &report.corpus.languages["typescript"];
    assert_eq!(ts.included, 0, "all TS is under excluded benchmarks/");
    assert!(ts.excluded_path >= 5);
    assert!(
        !report.corpus.meaningfully_mixed,
        "an excluded-only language is not a meaningful mix"
    );
    assert_eq!(report.verdict, Verdict::Ready);
    assert!(
        !report
            .reasons
            .iter()
            .any(|r| r.message.contains("typescript")),
        "no verdict reason should cite the unmodeled language"
    );
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn exclusion_reasons_are_counted() {
    let dir = temp_repo("exclusions");
    fs::create_dir_all(dir.join("tests")).unwrap();
    fs::write(dir.join("app.py"), py_functions(2)).unwrap();
    fs::write(dir.join("tests/test_app.py"), py_functions(1)).unwrap();
    fs::write(
        dir.join("generated.py"),
        format!(
            "# This file is auto-generated. Do not edit.\n{}",
            py_functions(1)
        ),
    )
    .unwrap();
    fs::write(
        dir.join("data.py"),
        "TABLE = {\n    \"a\": 1,\n    \"b\": 2,\n    \"c\": 3,\n    \"d\": 4,\n    \"e\": 5,\n}\n",
    )
    .unwrap();
    let report = inspect_repo(&dir).unwrap();
    let py = &report.corpus.languages["python"];
    assert_eq!(py.files, 4);
    assert_eq!(py.included, 1);
    assert_eq!(py.excluded_path, 1, "tests/ file excluded by path");
    assert_eq!(py.auto_generated, 1);
    assert_eq!(py.data_dominant, 1);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn calibration_health_is_read_from_v3_config() {
    let dir = temp_repo("calibrated");
    fs::write(dir.join("app.py"), py_functions(10)).unwrap();
    fs::create_dir_all(dir.join(".argot")).unwrap();
    fs::write(
        dir.join(".argot/scorer-config.json"),
        r#"{
          "version": 3,
          "languages": {
            "python": {
              "threshold": 42.5,
              "call_receiver_cap": 5,
              "calibration": {
                "n_cal": 9,
                "seed": 0,
                "n_seeds": 7,
                "repo_sha": "deadbeef",
                "timestamp_utc": "1970-01-01T00:00:00+00:00"
              },
              "model": {
                "bpe": { "token_counts": {}, "total_tokens": 0 },
                "call_receiver": { "attested": [], "n_corpus_files": 0, "clusters": {} }
              }
            }
          }
        }"#,
    )
    .unwrap();
    let report = inspect_repo(&dir).unwrap();
    let cal = report.calibration.expect("calibration report");
    let py = &cal.languages["python"];
    assert_eq!(py.threshold, 42.5);
    assert_eq!(py.n_cal, 9);
    assert_eq!(py.n_seeds, 7);
    assert_eq!(py.seed, 0);
    assert_eq!(py.repo_sha, "deadbeef");
    assert_eq!(py.timestamp_utc, "1970-01-01T00:00:00+00:00");
    assert_eq!(py.candidate_hunks_now, 10, "live candidate pass");
    // Threshold 42.5 is far above any reachable token surprise → the
    // phrasing-dead red reason fires and the verdict is honest about it.
    assert!(py.bpe_ceiling > 0.0);
    assert!(py.phrasing_headroom < 0.0);
    assert!(report
        .reasons
        .iter()
        .any(|r| r.signal == "phrasing_detection_dead" && r.level == ReasonLevel::Red));
    assert_eq!(report.verdict, Verdict::NotRecommended);
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn healthy_threshold_has_phrasing_headroom() {
    let dir = temp_repo("headroom");
    fs::write(dir.join("app.py"), py_functions(10)).unwrap();
    fs::create_dir_all(dir.join(".argot")).unwrap();
    fs::write(
        dir.join(".argot/scorer-config.json"),
        r#"{
          "version": 3,
          "languages": {
            "python": {
              "threshold": 4.5,
              "call_receiver_cap": 5,
              "calibration": {},
              "model": {
                "bpe": { "token_counts": {}, "total_tokens": 0 },
                "call_receiver": { "attested": [], "n_corpus_files": 0, "clusters": {} }
              }
            }
          }
        }"#,
    )
    .unwrap();
    let report = inspect_repo(&dir).unwrap();
    let cal = report.calibration.expect("calibration report");
    let py = &cal.languages["python"];
    assert!(
        py.phrasing_headroom > 0.0,
        "headroom {}",
        py.phrasing_headroom
    );
    assert!(!report
        .reasons
        .iter()
        .any(|r| r.signal.starts_with("phrasing")));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn invalid_config_yields_yellow_reason_not_error() {
    let dir = temp_repo("badconfig");
    fs::write(dir.join("app.py"), py_functions(1)).unwrap();
    fs::create_dir_all(dir.join(".argot")).unwrap();
    fs::write(dir.join(".argot/scorer-config.json"), "{ not json").unwrap();
    let report = inspect_repo(&dir).unwrap();
    assert!(report.calibration.is_none());
    assert!(report
        .reasons
        .iter()
        .any(|r| r.signal == "scorer_config_invalid" && r.level == ReasonLevel::Yellow));
    let _ = fs::remove_dir_all(&dir);
}

#[test]
fn json_document_shape_is_stable() {
    let dir = temp_repo("jsonshape");
    fs::write(dir.join("app.py"), py_functions(1)).unwrap();
    let report = inspect_repo(&dir).unwrap();
    let json: Value = serde_json::from_str(&serde_json::to_string(&report).unwrap()).unwrap();
    assert_eq!(json["verdict"], "not_recommended");
    assert!(json["reasons"].as_array().unwrap().iter().all(|r| {
        r.get("level").is_some() && r.get("signal").is_some() && r.get("message").is_some()
    }));
    let py = &json["corpus"]["languages"]["python"];
    for key in [
        "files",
        "included",
        "excluded_path",
        "auto_generated",
        "data_dominant",
        "share_of_supported",
        "candidate_hunks",
    ] {
        assert!(py.get(key).is_some(), "missing corpus key {key}");
    }
    assert!(json.get("calibration").is_none(), "pre-fit: no calibration");
    let _ = fs::remove_dir_all(&dir);
}
