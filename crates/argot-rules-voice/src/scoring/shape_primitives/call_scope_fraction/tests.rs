use super::*;

#[test]
fn python_nested_calls_are_not_module_scope() {
    let src = "def f():\n    g()\n\nh()\n";
    let frac = fraction_module_scope(src, Language::Python).unwrap();
    assert!((frac - 0.5).abs() < 1e-12, "got {frac}");
}

#[test]
fn typescript_nested_calls_are_not_module_scope() {
    // Pre-fix, the Python boundary literal made every TS call look
    // module-scope (fraction constantly 1.0 → std 0 → permanent abstain).
    let src = "function f() {\n  g();\n}\nh();\n";
    let frac = fraction_module_scope(src, Language::Typescript).unwrap();
    assert!(frac < 1.0, "TS nested call still counted as module scope");
    assert!((frac - 0.5).abs() < 1e-12, "got {frac}");
}

#[test]
fn no_calls_abstains() {
    assert!(fraction_module_scope("const x = 1;\n", Language::Typescript).is_none());
}
