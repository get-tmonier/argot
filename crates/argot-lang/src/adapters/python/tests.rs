use super::*;

#[test]
fn aliased_and_relative_imports_are_not_captured() {
    let adapter = PythonAdapter::new();
    let src = "import numpy as np\nfrom . import local\nfrom ..pkg import thing\nimport a.b.c\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("a"));
    assert!(!imports.contains("numpy"));
    assert!(!imports.contains("np"));
    assert!(!imports.contains("pkg"));
    assert!(!imports.contains("local"));
}

/// Error-recovery guard: a diff fragment starting mid-function whose only
/// import is relative (`from ._compat import v2`) must yield no module.
/// Tree-sitter re-parses the tail `import v2` as a standalone
/// `import_statement` mid-line; the phantom module `v2` used to leak
/// through and fire the import stage on the codebase's own relative import
/// (fastapi holdout false alarms).
#[test]
fn relative_import_in_error_fragment_yields_no_module() {
    let adapter = PythonAdapter::new();
    let frag = "    cloned_types: Optional[Mapping[str, str]] = None,\n) -> ModelField:\n    if PYDANTIC_V2:\n        from ._compat import v2\n\n        if isinstance(field, v2.ModelField):\n            return field\n    if cloned_types is None:";
    let imports = adapter.extract_imports(frag);
    assert!(
        !imports.contains("v2"),
        "imported symbol of a relative import leaked as a module: {imports:?}"
    );
    assert!(
        imports.is_empty(),
        "no top-level module in fragment: {imports:?}"
    );
    // A genuine top-level import in the same fragment still counts.
    let frag2 = format!("    x = 1\n{frag}\nimport requests\n");
    assert!(adapter.extract_imports(&frag2).contains("requests"));
}

#[test]
fn future_import_is_literal() {
    let adapter = PythonAdapter::new();
    let imports = adapter.extract_imports("from __future__ import annotations\n");
    assert!(imports.contains("__future__"));
}

#[test]
fn import_bindings_pairs_bound_names_with_modules() {
    let adapter = PythonAdapter::new();
    let src = "import numpy as np\nimport os.path\nfrom colorama import Fore, Back, Style\nfrom . import local\nfrom .pkg import thing as t\n";
    let b: std::collections::HashSet<(String, String)> =
        adapter.import_bindings(src).into_iter().collect();
    // Aliased top-level import: binding is the alias, module the real name.
    assert!(b.contains(&("np".to_string(), "numpy".to_string())));
    // Dotted import binds the crate root to itself.
    assert!(b.contains(&("os".to_string(), "os".to_string())));
    // `from m import a, b` binds each name to m.
    assert!(b.contains(&("Fore".to_string(), "colorama".to_string())));
    assert!(b.contains(&("Style".to_string(), "colorama".to_string())));
    // Relative imports are repo-internal — never surfaced as foreign bindings.
    assert!(!b
        .iter()
        .any(|(n, _)| n == "local" || n == "t" || n == "thing"));
}

#[test]
fn syntax_error_yields_empty() {
    let adapter = PythonAdapter::new();
    let src = "def broken(:\n    pass this is not python";
    assert!(adapter.extract_imports(src).is_empty());
    assert!(adapter.extract_imports_with_spans(src).is_empty());
    assert!(adapter.prose_line_ranges(src).is_empty());
    assert!(adapter.enumerate_sampleable_ranges(src).is_empty());
}

#[test]
fn identifier_noise_contains_keywords() {
    let adapter = PythonAdapter::new();
    assert!(adapter.identifier_noise().contains("self"));
    assert!(adapter.identifier_noise().contains("lambda"));
    assert_eq!(adapter.identifier_noise().len(), NOISE.len());
}
