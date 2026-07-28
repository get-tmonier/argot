use super::*;

#[test]
fn language_for_filename_ctx_resolves_dot_h_by_repo_majority() {
    // `.h` follows the repo decision; nothing else moves.
    assert_eq!(language_for_filename_ctx("x.h", true), Some(Language::Cpp));
    assert_eq!(language_for_filename_ctx("x.h", false), Some(Language::C));
    assert_eq!(language_for_filename_ctx("x.c", true), Some(Language::C));
    assert_eq!(
        language_for_filename_ctx("x.cpp", false),
        Some(Language::Cpp)
    );
    assert_eq!(
        language_for_filename_ctx("x.py", true),
        Some(Language::Python)
    );
}

#[test]
fn head_source_reads_committed_version_ignoring_working_tree_edits() {
    use std::process::Command;
    let dir = std::env::temp_dir().join(format!("argot_headsrc_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let git = |args: &[&str]| {
        Command::new("git")
            .args(args)
            .current_dir(&dir)
            .output()
            .expect("git");
    };
    git(&["init", "-q"]);
    git(&["config", "user.email", "t@t.co"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(dir.join("a.py"), "import json\n").unwrap();
    git(&["add", "-A"]);
    git(&["commit", "-qm", "init"]);
    // Working-tree edit to a tracked file + a brand-new untracked file.
    std::fs::write(dir.join("a.py"), "import json\nimport requests\n").unwrap();
    std::fs::write(dir.join("b.py"), "import os\n").unwrap();

    let head = HeadSource::new(&dir);
    // Tracked file → its committed HEAD version, NOT the uncommitted edit.
    assert_eq!(
        head.read(&dir.join("a.py")).as_deref(),
        Some("import json\n"),
        "a modified tracked file must fit from HEAD, not the working tree"
    );
    // Untracked file → read as-is from disk (nested/non-repo callers unchanged).
    assert_eq!(head.read(&dir.join("b.py")).as_deref(), Some("import os\n"));

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn slice_matches_by_prefix_and_exact_file() {
    let paths = vec!["frontend/".to_string(), "shared/util.ts".to_string()];
    assert!(slice_matches("frontend/app.ts", &paths));
    assert!(slice_matches("shared/util.ts", &paths)); // exact file
    assert!(!slice_matches("backend/api.py", &paths));
    assert!(!slice_matches("shared/other.ts", &paths));
}

#[test]
fn auto_slices_are_top_level_dirs_above_the_floor() {
    let mut files: Vec<String> = Vec::new();
    // frontend/ has enough files; docs/ does not.
    for i in 0..SLICE_AUTO_MIN_FILES {
        files.push(format!("frontend/f{i}.ts"));
    }
    files.push("docs/readme.ts".to_string());
    files.push("top.ts".to_string()); // no dir → ignored
    let slices = auto_slices(&files);
    let names: Vec<&str> = slices.iter().map(|s| s.name.as_str()).collect();
    assert!(names.contains(&"path:frontend/"));
    assert!(!names.iter().any(|n| n.contains("docs")));
}

#[test]
fn resolve_slices_parses_path_specs_and_ignores_unknown() {
    let slices = resolve_slices(
        Path::new("/nonexistent"),
        &[],
        &[
            "path:frontend/".to_string(),
            "bogus".to_string(),
            "path:".to_string(), // empty → dropped
        ],
    );
    assert_eq!(slices.len(), 1);
    assert_eq!(slices[0].name, "path:frontend/");
    assert_eq!(slices[0].paths, vec!["frontend/".to_string()]);
}

/// Build a `by_lang` map with the given per-language file counts.
fn by_lang_of(
    counts: &[(&'static str, Language, usize)],
) -> BTreeMap<&'static str, (Language, Vec<PathBuf>)> {
    counts
        .iter()
        .map(|(name, lang, n)| {
            let files = (0..*n)
                .map(|i| PathBuf::from(format!("{name}/f{i}")))
                .collect();
            (*name, (*lang, files))
        })
        .collect()
}

#[test]
fn a_language_the_repo_barely_writes_gets_no_scorer() {
    // rocksdb carries exactly one `.inc` (a CMake include the Pascal port's
    // extension table claims), and every one of the 27 new `.inc` files in the
    // replay window fired — 100%. There is no voice in one file to be foreign to.
    let mut by_lang = by_lang_of(&[
        ("cpp", Language::Cpp, 1200),
        ("python", Language::Python, 40),
        ("pascal", Language::Pascal, 1),
    ]);
    let dropped = drop_unlearnable_languages(&mut by_lang);
    assert_eq!(dropped, vec![("pascal".to_string(), 1)]);
    assert!(
        !by_lang.contains_key("pascal"),
        "no scorer, so check abstains"
    );
    assert!(by_lang.contains_key("cpp") && by_lang.contains_key("python"));
}

#[test]
fn a_small_single_language_repo_keeps_its_voice() {
    // The floor says "this language is incidental here", which only means
    // something when another language clears it. In a repo that is simply
    // small, the file count is its size — dropping it would leave nothing.
    let mut by_lang = by_lang_of(&[("rust", Language::Rust, 4)]);
    let dropped = drop_unlearnable_languages(&mut by_lang);
    assert!(dropped.is_empty(), "a small repo is still judged");
    assert!(by_lang.contains_key("rust"));
}

#[test]
fn the_floor_is_inclusive_at_its_boundary() {
    let mut by_lang = by_lang_of(&[
        ("go", Language::Go, 500),
        ("ruby", Language::Ruby, MIN_LANGUAGE_FILES),
        ("java", Language::Java, MIN_LANGUAGE_FILES - 1),
    ]);
    let dropped = drop_unlearnable_languages(&mut by_lang);
    assert_eq!(dropped, vec![("java".to_string(), MIN_LANGUAGE_FILES - 1)]);
    assert!(
        by_lang.contains_key("ruby"),
        "exactly at the floor still counts"
    );
}
