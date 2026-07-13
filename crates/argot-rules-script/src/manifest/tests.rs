use super::*;
use argot_engine::rules::Severity;

fn write_rule(dir: &Path, name: &str, manifest: &str, script: &str) -> std::path::PathBuf {
    let d = dir.join(name);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(d.join("rule.toml"), manifest).unwrap();
    std::fs::write(d.join("check.rhai"), script).unwrap();
    d
}

fn tmp() -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!(
        "argot_manifest_{}_{}",
        std::process::id(),
        std::thread::current()
            .name()
            .unwrap_or("t")
            .replace(':', "_")
    ));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn full_manifest_parses_with_all_fields() {
    let root = tmp();
    let d = write_rule(
        &root,
        "no-raw-sql",
        r#"
[rule]
schema = 1
name = "no-raw-sql"
label = "raw SQL string"
description = "SQL assembled by hand"
severity = "error"
languages = ["python", "typescript"]

[engine]
api = 1
script = "check.rhai"
"#,
        r#"report(1, "x");"#,
    );
    let rule = load_rule_dir(&d).unwrap();
    assert_eq!(rule.name, "no-raw-sql");
    assert_eq!(rule.label, "raw SQL string");
    assert_eq!(rule.default_severity, Severity::Error);
    assert_eq!(rule.languages, vec!["python", "typescript"]);
    assert!(rule.covers_language("python"));
    assert!(!rule.covers_language("go"));
    assert_eq!(rule.script, r#"report(1, "x");"#);
    let custom = rule.custom_rule();
    assert_eq!(custom.name, "no-raw-sql");
    assert_eq!(custom.default_severity, Severity::Error);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn minimal_manifest_gets_the_defaults() {
    let root = tmp();
    let d = write_rule(
        &root,
        "tiny",
        "[rule]\nschema = 1\nname = \"tiny\"\n",
        "report(1, \"y\");",
    );
    let rule = load_rule_dir(&d).unwrap();
    // Defaults: label = name, severity = warn (report before gating),
    // languages = all.
    assert_eq!(rule.label, "tiny");
    assert_eq!(rule.default_severity, Severity::Warn);
    assert!(rule.covers_language("ruby"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn rejections_are_per_rule_and_explain_themselves() {
    let root = tmp();
    // schema from the future
    let d = write_rule(
        &root,
        "future",
        "[rule]\nschema = 9\nname = \"future\"\n",
        "",
    );
    assert!(load_rule_dir(&d).unwrap_err().contains("schema 9"));
    // api from the future
    let d = write_rule(
        &root,
        "api9",
        "[rule]\nschema = 1\nname = \"api9\"\n[engine]\napi = 9\n",
        "",
    );
    assert!(load_rule_dir(&d).unwrap_err().contains("host API 9"));
    // dir/name mismatch
    let d = write_rule(
        &root,
        "dirname",
        "[rule]\nschema = 1\nname = \"other\"\n",
        "",
    );
    assert!(load_rule_dir(&d).unwrap_err().contains("does not match"));
    // invalid severity
    let d = write_rule(
        &root,
        "loud",
        "[rule]\nschema = 1\nname = \"loud\"\nseverity = \"loud\"\n",
        "",
    );
    assert!(load_rule_dir(&d).unwrap_err().contains("invalid severity"));
    // missing script
    let d = root.join("noscript");
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        "[rule]\nschema = 1\nname = \"noscript\"\n",
    )
    .unwrap();
    assert!(load_rule_dir(&d).unwrap_err().contains("check.rhai"));
    // unknown manifest keys are rejected (deny_unknown_fields — typos surface)
    let d = write_rule(
        &root,
        "typo",
        "[rule]\nschema = 1\nname = \"typo\"\nseverety = \"warn\"\n",
        "",
    );
    assert!(load_rule_dir(&d).is_err());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn covers_file_defaults_to_supported_languages_only() {
    let root = tmp();
    let d = write_rule(&root, "plain", "[rule]\nschema = 1\nname = \"plain\"\n", "");
    let rule = load_rule_dir(&d).unwrap();
    assert!(rule.covers_file("src/app.py", Some("python")));
    assert!(
        !rule.covers_file(".env", None),
        "unscored files need `files` globs"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn include_globs_claim_any_extension_and_narrow_with_languages() {
    let root = tmp();
    let d = write_rule(
        &root,
        "envs",
        "[rule]\nschema = 1\nname = \"envs\"\ninclude = [\"*.env\", \".github/workflows/*.yml\"]\n",
        "",
    );
    let rule = load_rule_dir(&d).unwrap();
    assert!(rule.covers_file("deploy/prod.env", None));
    assert!(rule.covers_file(".github/workflows/ci.yml", None));
    assert!(
        !rule.covers_file("src/app.py", Some("python")),
        "globs replace the language gate"
    );
    // files + languages = intersection.
    let d = write_rule(
        &root,
        "narrow",
        "[rule]\nschema = 1\nname = \"narrow\"\ninclude = [\"src/api/**\"]\nlanguages = [\"typescript\"]\n",
        "",
    );
    let rule = load_rule_dir(&d).unwrap();
    assert!(rule.covers_file("src/api/routes.ts", Some("typescript")));
    assert!(
        !rule.covers_file("src/api/notes.env", None),
        "languages narrows"
    );
    assert!(
        !rule.covers_file("src/ui/x.ts", Some("typescript")),
        "outside the globs"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn exclude_subtracts_from_the_default_language_scope() {
    let root = tmp();
    let d = write_rule(
        &root,
        "no-tests",
        "[rule]\nschema = 1\nname = \"no-tests\"\nlanguages = [\"typescript\"]\nexclude = [\"**/*.test.ts\", \"**/__tests__/**\"]\n",
        "",
    );
    let rule = load_rule_dir(&d).unwrap();
    assert!(rule.covers_file("src/app.ts", Some("typescript")));
    assert!(
        !rule.covers_file("src/app.test.ts", Some("typescript")),
        "excluded"
    );
    assert!(
        !rule.covers_file("src/__tests__/app.ts", Some("typescript")),
        "excluded dir"
    );
    // exclude wins even over an explicit include.
    let d = write_rule(
        &root,
        "envs-not-example",
        "[rule]\nschema = 1\nname = \"envs-not-example\"\ninclude = [\"*.env\"]\nexclude = [\"*.example.env\"]\n",
        "",
    );
    let rule = load_rule_dir(&d).unwrap();
    assert!(rule.covers_file("prod.env", None));
    assert!(
        !rule.covers_file("prod.example.env", None),
        "exclude beats include"
    );
    let _ = std::fs::remove_dir_all(&root);
}
