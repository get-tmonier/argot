use super::*;

fn tmp(case: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("argot_discover_{}_{case}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

#[test]
fn no_rules_dir_is_silent() {
    let argot_dir = tmp("silent");
    let mut warnings = Vec::new();
    assert!(discover(&argot_dir, &mut warnings).is_empty());
    assert!(warnings.is_empty());
    let _ = std::fs::remove_dir_all(&argot_dir);
}

#[test]
fn valid_rules_load_sorted_and_bad_ones_warn() {
    let argot_dir = tmp("sorted");
    for (name, manifest) in [
        ("zeta", "[rule]\nschema = 1\nname = \"zeta\"\n"),
        ("alpha", "[rule]\nschema = 1\nname = \"alpha\"\n"),
        ("broken", "[rule\nnot toml"),
    ] {
        let d = argot_dir.join(RULES_DIR).join(name);
        std::fs::create_dir_all(&d).unwrap();
        std::fs::write(d.join("rule.toml"), manifest).unwrap();
        std::fs::write(d.join("check.rhai"), "report(1, \"m\");").unwrap();
    }
    let mut warnings = Vec::new();
    let rules = discover(&argot_dir, &mut warnings);
    let names: Vec<&str> = rules.iter().map(|r| r.name.as_str()).collect();
    assert_eq!(names, vec!["alpha", "zeta"], "sorted, broken skipped");
    assert_eq!(warnings.len(), 1);
    assert!(warnings[0].contains("broken"), "{}", warnings[0]);
    let _ = std::fs::remove_dir_all(&argot_dir);
}
