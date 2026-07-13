use super::*;

#[test]
fn callable_bodies_covers_methods_and_constructors() {
    let a = JavaAdapter::new();
    let src = "class Foo {\n    Foo(int x) {\n        this.x = x;\n    }\n    int add(int a, int b) {\n        return a + b;\n    }\n}\n";
    let names: Vec<String> = a
        .callable_bodies(src)
        .into_iter()
        .map(|b| b.symbol)
        .collect();
    assert!(names.contains(&"add".to_string()), "{names:?}");
    assert!(names.contains(&"Foo".to_string()), "constructor: {names:?}");
}

#[test]
fn resolve_repo_modules_derives_owned_package_prefixes() {
    let dir = std::env::temp_dir().join(format!("argot_java_pkg_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    std::fs::write(
        dir.join("src/Maps.java"),
        "// header comment\npackage com.google.common.collect;\n\nclass Maps {}\n",
    )
    .unwrap();

    let modules = JavaAdapter::new().resolve_repo_modules(&dir);
    assert!(modules.exact.contains("com.google.common.collect"));
    assert!(modules.prefixes.contains("com.google.common.collect."));
    // The prefix attests static member imports the corpus never used.
    assert!("com.google.common.collect.Maps.keyIterator".starts_with("com.google.common.collect."));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extract_imports_keeps_qualified_paths() {
    let adapter = JavaAdapter::new();
    let src = "package com.example.app;\n\
               import java.util.List;\n\
               import static org.junit.Assert.assertEquals;\n\
               import com.squareup.okhttp3.*;\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("java.util.List"));
    assert!(imports.contains("org.junit.Assert.assertEquals"));
    // Wildcard keeps the package path (the asterisk is a separate node).
    assert!(imports.contains("com.squareup.okhttp3"));
    // The package declaration is not an import.
    assert!(!imports.contains("com.example.app"));
}

#[test]
fn imports_with_spans_are_sorted_and_positioned() {
    let adapter = JavaAdapter::new();
    let src = "import java.util.List;\nimport java.util.Map;\n";
    let spans = adapter.extract_imports_with_spans(src);
    assert_eq!(spans.len(), 2);
    // `import ` is 7 chars, so the path starts at column 7 on each line.
    assert_eq!(spans[0], ("java.util.List".to_string(), 1, 7, 21));
    assert_eq!(spans[1], ("java.util.Map".to_string(), 2, 7, 20));
}

#[test]
fn callable_definitions_cover_types_and_members() {
    let adapter = JavaAdapter::new();
    let src = "public class Foo {\n\
               \x20 public Foo() {}\n\
               \x20 public int get(int y) { return y; }\n\
               }\n\
               interface Svc { void go(); }\n\
               enum Color { RED }\n\
               record Point(int x, int y) {}\n";
    let defs = adapter.callable_definitions(src);
    assert!(defs.contains("Foo")); // class + constructor share the name
    assert!(defs.contains("get"));
    assert!(defs.contains("Svc"));
    assert!(defs.contains("go"));
    assert!(defs.contains("Color"));
    assert!(defs.contains("Point"));
}

#[test]
fn value_bindings_cover_locals_fields_params_and_imports() {
    let adapter = JavaAdapter::new();
    let src = "import java.util.List;\n\
               class C {\n\
               \x20 private int field;\n\
               \x20 void m(String p) {\n\
               \x20   int local = 1;\n\
               \x20   try { work(); } catch (Exception e) { }\n\
               \x20 }\n\
               }\n";
    let binds = adapter.value_bindings(src);
    assert!(binds.contains("field"));
    assert!(binds.contains("p"));
    assert!(binds.contains("local"));
    assert!(binds.contains("e"));
    assert!(binds.contains("List")); // simple name of the import
}

#[test]
fn internal_import_bindings_are_empty() {
    let adapter = JavaAdapter::new();
    let src = "import java.util.List;\nclass C {}\n";
    assert!(adapter.internal_import_bindings(src).is_empty());
}

#[test]
fn static_final_array_tables_are_data_dominant() {
    let adapter = JavaAdapter::new();
    // Many single-line array-literal fields dwarf the two class-wrapper
    // lines, so the data-line ratio clears the 0.65 threshold.
    let src = "class Tables {\n\
               \x20 static final int[] A = {1,2,3,4};\n\
               \x20 static final int[] B = {5,6,7,8};\n\
               \x20 static final int[] C = {9,10,11,12};\n\
               \x20 static final int[] D = {13,14,15,16};\n\
               \x20 static final int[] E = {17,18,19,20};\n\
               \x20 static final int[] F = {21,22,23,24};\n\
               \x20 static final int[] G = {25,26,27,28};\n\
               \x20 static final int[] H = {29,30,31,32};\n\
               }\n";
    assert!(adapter.is_data_dominant(src, 0.65));
    let lines = adapter.data_literal_lines(src);
    assert!(lines.contains(&2));
    assert!(lines.contains(&9));
}

#[test]
fn code_is_not_data_dominant() {
    let adapter = JavaAdapter::new();
    let src = "class C {\n  int add(int a, int b) {\n    return a + b;\n  }\n}\n";
    assert!(!adapter.is_data_dominant(src, 0.65));
}

#[test]
fn generated_annotation_and_header_marker_detected() {
    let adapter = JavaAdapter::new();
    let annotated = "@Generated(\"tool\")\npublic class Foo {}\n";
    assert!(adapter.is_auto_generated(annotated, &crate::test_support::generated_markers()));
    let header = "// This file was automatically generated. Do not edit.\nclass Foo {}\n";
    assert!(adapter.is_auto_generated(header, &crate::test_support::generated_markers()));
    let hand_written = "// A normal class.\nclass Foo {}\n";
    assert!(!adapter.is_auto_generated(hand_written, &crate::test_support::generated_markers()));
}

#[test]
fn sampleable_ranges_span_methods_and_constructors() {
    let adapter = JavaAdapter::new();
    let src = "public class Foo {\n\
               \x20 public Foo(int a) {\n\
               \x20   this.a = a;\n\
               \x20 }\n\
               \x20 public int get() {\n\
               \x20   return a;\n\
               \x20 }\n\
               }\n";
    let ranges = adapter.enumerate_sampleable_ranges(src);
    assert_eq!(ranges.len(), 2);
    // Constructor spans lines 2..=4, method spans 5..=7 (1-indexed).
    assert!(ranges.contains(&(2, 4)));
    assert!(ranges.contains(&(5, 7)));
}

#[test]
fn extract_callees_builds_dotted_receivers() {
    let adapter = JavaAdapter::new();
    let src = "class C {\n\
               \x20 void m() {\n\
               \x20   obj.doThing(1);\n\
               \x20   helper();\n\
               \x20   new java.util.ArrayList<String>();\n\
               \x20   a.b.run();\n\
               \x20 }\n\
               }\n";
    let callees = adapter.extract_callees(src);
    assert!(callees.contains(&"obj.doThing".to_string()));
    assert!(callees.contains(&"helper".to_string()));
    assert!(callees.contains(&"ArrayList".to_string()));
    assert!(callees.contains(&"a.b.run".to_string()));
}

#[test]
fn identifier_noise_contains_keywords() {
    let adapter = JavaAdapter::new();
    assert!(adapter.identifier_noise().contains("class"));
    assert!(adapter.identifier_noise().contains("void"));
    assert_eq!(adapter.identifier_noise().len(), NOISE.len());
}
