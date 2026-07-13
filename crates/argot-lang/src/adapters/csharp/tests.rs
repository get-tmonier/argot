use super::*;

#[test]
fn callable_bodies_covers_methods_and_constructors() {
    let a = CSharpAdapter::new();
    let src = "class Foo {\n    public Foo(int x) { this.x = x; }\n    public int Add(int a, int b) {\n        return a + b;\n    }\n}\n";
    let names: Vec<String> = a
        .callable_bodies(src)
        .into_iter()
        .map(|b| b.symbol)
        .collect();
    assert!(names.contains(&"Add".to_string()), "{names:?}");
    assert!(names.contains(&"Foo".to_string()), "constructor: {names:?}");
}

#[test]
fn resolve_repo_modules_derives_owned_namespace_prefixes() {
    let dir = std::env::temp_dir().join(format!("argot_cs_ns_test_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    // Block-scoped and file-scoped namespace forms.
    std::fs::write(
        dir.join("src/A.cs"),
        "namespace System.Management.Automation\n{\n    class A {}\n}\n",
    )
    .unwrap();
    std::fs::write(
        dir.join("src/B.cs"),
        "namespace Acme.Tool.Core;\n\nclass B {}\n",
    )
    .unwrap();

    let modules = CSharpAdapter::new().resolve_repo_modules(&dir);
    assert!(modules.exact.contains("System.Management.Automation"));
    assert!(modules.prefixes.contains("System.Management.Automation."));
    assert!(modules.exact.contains("Acme.Tool.Core"));
    assert!(modules.prefixes.contains("Acme.Tool.Core."));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn extract_imports_keeps_full_namespace() {
    let adapter = CSharpAdapter::new();
    let src = "using System;\nusing System.Net.Http;\nusing Newtonsoft.Json;\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("System"));
    assert!(imports.contains("System.Net.Http"));
    assert!(imports.contains("Newtonsoft.Json"));
}

#[test]
fn global_and_static_and_alias_usings() {
    let adapter = CSharpAdapter::new();
    let src = "global using System.Text;\nusing static System.Math;\nusing Foo = System.Collections.Generic.List;\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("System.Text"));
    assert!(imports.contains("System.Math"));
    // Alias target namespace is imported; the alias LHS `Foo` is not.
    assert!(imports.contains("System.Collections.Generic.List"));
    assert!(!imports.contains("Foo"));
}

#[test]
fn import_spans_are_sorted_and_cover_namespace() {
    let adapter = CSharpAdapter::new();
    let spans = adapter.extract_imports_with_spans("using System.Net.Http;\n");
    assert_eq!(spans.len(), 1);
    let (spec, line, col_start, col_end) = &spans[0];
    assert_eq!(spec, "System.Net.Http");
    assert_eq!(*line, 1);
    assert_eq!(*col_end - *col_start, "System.Net.Http".len());
}

#[test]
fn callable_definitions_cover_all_declaration_kinds() {
    let adapter = CSharpAdapter::new();
    let src = "namespace App {\n  public class Widget {\n    public Widget() {}\n    public int Add(int a, int b) { return a + b; }\n    public int Count { get; set; }\n  }\n  interface IThing {}\n  struct Point {}\n  record Rec(int X);\n  enum Color { Red }\n}\n";
    let defs = adapter.callable_definitions(src);
    assert!(defs.contains("Widget")); // class + constructor
    assert!(defs.contains("Add"));
    assert!(defs.contains("Count"));
    assert!(defs.contains("IThing"));
    assert!(defs.contains("Point"));
    assert!(defs.contains("Rec"));
    assert!(defs.contains("Color"));
}

#[test]
fn value_bindings_cover_locals_and_parameters() {
    let adapter = CSharpAdapter::new();
    let src = "class C { void M(int a, string b) { var y = 2; int z = 3; } }\n";
    let vals = adapter.value_bindings(src);
    assert!(vals.contains("a"));
    assert!(vals.contains("b"));
    assert!(vals.contains("y"));
    assert!(vals.contains("z"));
}

#[test]
fn extract_callees_handles_members_new_and_this() {
    let adapter = CSharpAdapter::new();
    let src =
        "class C { void M() { obj.Do(); Helper.Run(x); new Thing(1); this.Go(); base.Init(); } }\n";
    let callees = adapter.extract_callees(src);
    assert!(callees.contains(&"obj.Do".to_string()));
    assert!(callees.contains(&"Helper.Run".to_string()));
    assert!(callees.contains(&"Thing".to_string()));
    assert!(callees.contains(&"this.Go".to_string()));
    assert!(callees.contains(&"base.Init".to_string()));
}

#[test]
fn internal_import_bindings_is_empty() {
    let adapter = CSharpAdapter::new();
    assert!(adapter
        .internal_import_bindings("using App.Internal;\n")
        .is_empty());
}

#[test]
fn data_dominant_table_vs_code() {
    let adapter = CSharpAdapter::new();
    let table = "class Data {\n  static readonly int[] T = new int[] {\n    1, 2, 3, 4, 5,\n    6, 7, 8, 9, 10,\n    11, 12, 13, 14, 15,\n  };\n}\n";
    assert!(adapter.is_data_dominant(table, 0.65));
    assert!(!adapter.data_literal_lines(table).is_empty());

    let code = "class C {\n  public int Add(int a, int b) {\n    var sum = a + b;\n    return sum;\n  }\n}\n";
    assert!(!adapter.is_data_dominant(code, 0.65));
}

#[test]
fn auto_generated_markers_detected() {
    let adapter = CSharpAdapter::new();
    assert!(adapter.is_auto_generated(
        "// <auto-generated>\nclass C {}\n",
        &crate::test_support::generated_markers()
    ));
    assert!(adapter.is_auto_generated(
        "// DO NOT EDIT\nclass C {}\n",
        &crate::test_support::generated_markers()
    ));
    assert!(adapter.is_auto_generated(
        "[GeneratedCode(\"tool\", \"1\")]\nclass C {}\n",
        &crate::test_support::generated_markers()
    ));
    assert!(!adapter.is_auto_generated(
        "// a normal file\nclass C {}\n",
        &crate::test_support::generated_markers()
    ));
}

#[test]
fn sampleable_ranges_are_methods_and_constructors() {
    let adapter = CSharpAdapter::new();
    let src = "class C {\n  public C() {\n    x = 1;\n  }\n  public int Add(int a, int b) {\n    return a + b;\n  }\n}\n";
    let ranges = adapter.enumerate_sampleable_ranges(src);
    // constructor (lines 2-4) and method (lines 5-7).
    assert_eq!(ranges.len(), 2);
    assert!(ranges.contains(&(2, 4)));
    assert!(ranges.contains(&(5, 7)));
}

#[test]
fn prose_line_ranges_cover_comments() {
    let adapter = CSharpAdapter::new();
    let src = "// line comment\nclass C {\n  /* block\n     comment */\n  void M() {}\n}\n";
    let rows = adapter.prose_line_ranges(src);
    assert!(rows.contains(&1));
    assert!(rows.contains(&3));
    assert!(rows.contains(&4));
}

#[test]
fn identifier_noise_contains_keywords() {
    let adapter = CSharpAdapter::new();
    assert!(adapter.identifier_noise().contains("class"));
    assert!(adapter.identifier_noise().contains("var"));
    assert!(adapter.identifier_noise().contains("this"));
    assert_eq!(adapter.identifier_noise().len(), NOISE.len());
}

#[test]
fn line_comment_prefix_is_slashes() {
    let adapter = CSharpAdapter::new();
    assert_eq!(
        <CSharpAdapter as LanguageAdapter>::line_comment_prefix(&adapter),
        "//"
    );
}
