use super::*;

fn tmp(case: &str) -> std::path::PathBuf {
    let d = std::env::temp_dir().join(format!("argot_scriptdet_{}_{case}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

fn write_rule(argot_dir: &std::path::Path, name: &str, script: &str) {
    let d = argot_dir.join("rules").join(name);
    std::fs::create_dir_all(&d).unwrap();
    std::fs::write(
        d.join("rule.toml"),
        format!("[rule]\nschema = 1\nname = \"{name}\"\n"),
    )
    .unwrap();
    std::fs::write(d.join("check.rhai"), script).unwrap();
}

#[test]
fn vocabulary_contributes_discovered_rules() {
    let argot_dir = tmp("vocab");
    write_rule(&argot_dir, "my-rule", "report(1, \"m\");");
    let mut det = ScriptDetector::new();
    let mut warnings = Vec::new();
    let vocab = det.vocabulary(&argot_dir, &mut warnings);
    assert_eq!(vocab.len(), 1);
    assert_eq!(vocab[0].name, "my-rule");
    assert!(warnings.is_empty());
    let _ = std::fs::remove_dir_all(&argot_dir);
}

#[test]
fn compile_failure_disables_only_that_rule() {
    let argot_dir = tmp("disable");
    write_rule(&argot_dir, "bad", "fn {");
    write_rule(&argot_dir, "good", "report(1, \"m\");");
    let mut det = ScriptDetector::new();
    let mut warnings = Vec::new();
    let vocab = det.vocabulary(&argot_dir, &mut warnings);
    assert_eq!(vocab.len(), 2, "vocabulary keeps the name addressable");
    let detect = argot_engine::config::DetectConfig::default();
    let suppressions = argot_engine::suppress::PathSuppressions::recommended();
    det.load(&argot_engine::detector::LoadContext {
        argot_dir: &argot_dir,
        detect: &detect,
        path_suppressions: &suppressions,
    })
    .unwrap();
    assert!(det.disabled.contains("bad"));
    assert!(!det.disabled.contains("good"));
    let _ = std::fs::remove_dir_all(&argot_dir);
}
