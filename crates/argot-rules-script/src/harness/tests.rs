use super::*;

fn tmp(case: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("argot_harness_{}_{case}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_rule_with_case(
    argot_dir: &Path,
    name: &str,
    script: &str,
    case: &str,
    input_name: &str,
    input: &str,
    expected: &str,
) {
    let d = argot_dir.join("rules").join(name);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        format!("[rule]\nschema = 1\nname = \"{name}\"\n"),
    )
    .unwrap();
    std::fs::write(d.join("check.rhai"), script).unwrap();
    let c = d.join("tests").join(case);
    std::fs::create_dir_all(&c).unwrap();
    std::fs::write(c.join(input_name), input).unwrap();
    std::fs::write(c.join("expected.json"), expected).unwrap();
}

const SECRET_RULE: &str = r#"
for h in hunks {
    if h.text.contains("secret") {
        report(h.start, "secret found");
    }
}
"#;

#[test]
fn passing_and_failing_cases_are_judged() {
    let argot_dir = tmp("judged");
    write_rule_with_case(
        &argot_dir,
        "find-secret",
        SECRET_RULE,
        "fires",
        "input.py",
        "x = 1\nsecret = 2\n",
        r#"[{"line": 1, "message": "secret found"}]"#,
    );
    // A silent case that wrongly expects a finding.
    let c = argot_dir.join("rules/find-secret/tests/wrong");
    std::fs::create_dir_all(&c).unwrap();
    std::fs::write(c.join("input.py"), "clean = 1\n").unwrap();
    std::fs::write(
        c.join("expected.json"),
        r#"[{"line": 1, "message": "secret found"}]"#,
    )
    .unwrap();

    let mut warnings = Vec::new();
    let results = run_rule_tests(&argot_dir, None, &mut warnings).unwrap();
    assert_eq!(results.len(), 2);
    let fires = results.iter().find(|r| r.case == "fires").unwrap();
    assert!(fires.failure.is_none(), "{:?}", fires.failure);
    let wrong = results.iter().find(|r| r.case == "wrong").unwrap();
    assert!(wrong
        .failure
        .as_deref()
        .unwrap()
        .contains("findings differ"));
    let _ = std::fs::remove_dir_all(&argot_dir);
}

#[test]
fn unknown_filter_and_missing_tests_dir_explain_themselves() {
    let argot_dir = tmp("missing");
    let d = argot_dir.join("rules/lonely");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        "[rule]\nschema = 1\nname = \"lonely\"\n",
    )
    .unwrap();
    std::fs::write(d.join("check.rhai"), "report(1, \"m\");").unwrap();

    let mut warnings = Vec::new();
    let err = run_rule_tests(&argot_dir, Some("nope"), &mut warnings).unwrap_err();
    assert!(err.contains("unknown custom rule 'nope'"));

    let results = run_rule_tests(&argot_dir, Some("lonely"), &mut warnings).unwrap();
    assert_eq!(results.len(), 1);
    assert!(results[0]
        .failure
        .as_deref()
        .unwrap()
        .contains("no tests/ directory"));
    let _ = std::fs::remove_dir_all(&argot_dir);
}
