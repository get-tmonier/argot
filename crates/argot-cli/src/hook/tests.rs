use super::*;

#[test]
fn only_write_family_tools_are_scored() {
    assert!(is_write_tool("Write"));
    assert!(is_write_tool("Edit"));
    assert!(is_write_tool("MultiEdit"));
    assert!(!is_write_tool("Read"));
    assert!(!is_write_tool("Bash"));
}

#[test]
fn proposed_content_reads_write_and_edit_shapes() {
    assert_eq!(
        proposed_content(&json!({"content": "import x\n"})),
        "import x\n"
    );
    let edit = json!({"edits": [
        {"old_string": "a", "new_string": "import y"},
        {"old_string": "b", "new_string": "z"}
    ]});
    assert_eq!(proposed_content(&edit), "import y\nz");
    assert_eq!(
        proposed_content(&json!({"new_string": "import q"})),
        "import q"
    );
    assert_eq!(proposed_content(&json!({})), "");
}

fn config_with_rules(rules: Vec<(&str, Severity)>) -> ArgotConfig {
    let mut config = ArgotConfig::default();
    config.rules_committed = vec![rules
        .into_iter()
        .map(|(name, severity)| (name.to_string(), severity))
        .collect()];
    config
}

#[test]
fn foreign_import_off_disables_the_hook_but_warn_and_error_still_ask() {
    for (selector, severity, expected) in [
        ("foreign-import", Severity::Off, false),
        ("voice", Severity::Off, false),
        ("foreign-import", Severity::Warn, true),
        ("foreign-import", Severity::Error, true),
    ] {
        let config = config_with_rules(vec![(selector, severity)]);
        assert_eq!(
            can_assess(&config, "src/app.py"),
            expected,
            "{selector} = {}",
            severity.as_str()
        );
    }
}

#[test]
fn path_excludes_and_rule_scopes_skip_pre_write_assessment() {
    let mut excluded = ArgotConfig::default();
    excluded.exclude.paths.push("generated/**".to_string());
    assert!(!can_assess(&excluded, "generated/client.py"));
    let mut scoped = ArgotConfig::default();
    scoped.rule_scopes.push((
        "foreign-import".to_string(),
        argot_core::rules::RuleScope {
            include: vec!["src/**".to_string()],
            exclude: vec!["src/legacy/**".to_string()],
        },
    ));
    assert!(can_assess(&scoped, "src/app.py"));
    assert!(!can_assess(&scoped, "src/legacy/app.py"));
    assert!(!can_assess(&scoped, "lib/app.py"));
}

#[test]
fn declared_import_replacements_do_not_prompt_and_other_mutes_stay_unsupported() {
    let config = config_from_toml(
        r#"
        [[migration]]
        from = "legacy_dependency"
        to = "approved_dependency"
        reason = "migration fixture"

        [[mute]]
        path = "src/app.py"
        hash = "a1b2c3d4e5f6"
        reason = "a full-check hash mute cannot apply before write"
    "#,
    );
    assert!(can_assess(&config, "src/app.py"));
    assert!(only_declared_replacements(
        &config,
        &["approved_dependency".to_string()]
    ));
    assert!(!only_declared_replacements(
        &config,
        &[
            "approved_dependency".to_string(),
            "different_dependency".to_string()
        ]
    ));
}

#[test]
fn malformed_config_fails_open() {
    let config = config_from_toml("[rules\nforeign-import = \"off\"");
    assert!(!can_assess(&config, "src/app.py"));
}

#[test]
fn only_repo_relative_paths_are_assessed() {
    let repo = std::env::temp_dir().join("argot-hook-path-repo");
    let inside = repo.join("src/app.py");
    let outside = repo
        .parent()
        .expect("temporary directory has a parent")
        .join("argot-hook-path-elsewhere/app.py");
    assert_eq!(
        repo_relative_path(&repo, &inside.to_string_lossy()),
        Some("src/app.py".to_string())
    );
    assert_eq!(
        repo_relative_path(&repo, "src/app.py"),
        Some("src/app.py".to_string())
    );
    assert_eq!(repo_relative_path(&repo, &outside.to_string_lossy()), None);
    assert_eq!(repo_relative_path(&repo, "../elsewhere/app.py"), None);
}

#[test]
fn assessment_failures_are_silent_allows() {
    let repo = temporary_repo("fail-open");
    assert_eq!(assess(&repo, "src/app.py", "import new_dependency\n"), None);
    std::fs::write(repo.join("argot.toml"), "[rules\nforeign-import = \"off\"").unwrap();
    assert_eq!(assess(&repo, "src/app.py", "import new_dependency\n"), None);
    assert_eq!(assess(&repo, "README.md", "new dependency\n"), None);
    std::fs::remove_dir_all(repo).unwrap();
}

fn config_from_toml(body: &str) -> ArgotConfig {
    let dir = temporary_repo("config");
    std::fs::write(dir.join("argot.toml"), body).unwrap();
    let config = ArgotConfig::load(&dir);
    std::fs::remove_dir_all(dir).unwrap();
    config
}

fn temporary_repo(name: &str) -> std::path::PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "argot-hook-test-{name}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    dir
}
