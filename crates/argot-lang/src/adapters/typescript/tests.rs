use super::*;

#[test]
fn export_from_is_not_captured_but_import_from_is() {
    let adapter = TypeScriptAdapter::new();
    let src =
        "import a from \"react\";\nexport { b } from \"lodash\";\nimport c from \"./local\";\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("react"));
    assert!(!imports.contains("lodash"));
    assert!(!imports.contains("./local"));
}

#[test]
fn specifiers_are_not_split_on_dot() {
    let adapter = TypeScriptAdapter::new();
    let imports = adapter.extract_imports("import x from \"node:fs\";\n");
    assert!(imports.contains("node:fs"));
}

#[test]
fn identifier_noise_size_matches() {
    let adapter = TypeScriptAdapter::new();
    assert!(adapter.identifier_noise().contains("const"));
    assert!(adapter.identifier_noise().contains("arguments"));
    assert_eq!(adapter.identifier_noise().len(), NOISE.len());
}

#[test]
fn fnmatch_star_matches_within_segment() {
    assert!(fnmatch_segment("*", "packages"));
    assert!(fnmatch_segment("pkg-*", "pkg-core"));
    assert!(!fnmatch_segment("pkg-*", "other"));
    assert!(fnmatch_segment("[a-c]x", "bx"));
    assert!(!fnmatch_segment("[a-c]x", "dx"));
}

#[test]
fn subpath_imports_are_internal_not_foreign() {
    let adapter = TypeScriptAdapter::new();
    // Node.js subpath imports (`#…`) resolve inside the declaring package —
    // they must never enter the foreign-import surface.
    let src = "import { cmd } from \"#internal/commands\";\nimport z from \"zod\";\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("zod"));
    assert!(!imports.iter().any(|s| s.starts_with('#')), "{imports:?}");
    // …and their bound names count as repo-internal, like relative imports.
    let bindings = adapter.internal_import_bindings(src);
    assert!(bindings.contains("cmd"), "{bindings:?}");
}

#[test]
fn pnpm_workspace_packages_resolve_as_internal() {
    let dir = std::env::temp_dir().join(format!("argot_ts_pnpm_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("packages/core")).unwrap();
    // Root package.json has NO `workspaces` field — pnpm keeps that list in
    // its own file, the exact shape that used to hide internal packages.
    std::fs::write(dir.join("package.json"), "{\"name\": \"@acme/repo\"}\n").unwrap();
    std::fs::write(
        dir.join("pnpm-workspace.yaml"),
        "packages:\n  - \"packages/*\"\ncatalog:\n  react: 19.0.0\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("packages/core/package.json"),
        "{\"name\": \"@acme/core\"}\n",
    )
    .unwrap();

    let modules = TypeScriptAdapter::new().resolve_repo_modules(&dir);
    assert!(modules.exact.contains("@acme/core"), "{:?}", modules.exact);
    assert!(modules.exact.contains("@acme/repo"), "{:?}", modules.exact);
    let _ = std::fs::remove_dir_all(&dir);
}
