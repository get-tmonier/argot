use super::*;

const UNIT: &str = r#"unit MyUnit;

interface

uses
  SysUtils, Classes, Generics.Collections;

type
  TWidget = class(TObject)
  public
    constructor Create;
    function Area: Integer;
  end;

const
  MAX = 10;

implementation

uses
  StrUtils;

constructor TWidget.Create;
begin
  inherited Create;
end;

function TWidget.Area: Integer;
var
  Tmp: Integer;
begin
  Tmp := MAX;
  Result := Tmp;
  WriteLn(Format('area=%d', [Tmp]));
end;

end.
"#;

#[test]
fn uses_clause_yields_top_segments() {
    let a = PascalAdapter::new();
    let imports = a.extract_imports(UNIT);
    // interface + implementation uses, dotted unit collapses to its top
    // segment, every unit reduced to its case-insensitive identity.
    assert!(imports.contains("sysutils"), "{imports:?}");
    assert!(imports.contains("classes"), "{imports:?}");
    assert!(
        imports.contains("generics"),
        "dotted → top segment: {imports:?}"
    );
    assert!(
        imports.contains("strutils"),
        "impl-section uses: {imports:?}"
    );
}

#[test]
fn unit_names_fold_to_one_identity() {
    let a = PascalAdapter::new();
    // Pascal is case-insensitive and MSEide/MSEgui writes all four spellings of
    // this unit; four spellings must not read as four dependencies.
    for spelling in ["SysUtils", "sysutils", "sysUtils", "Sysutils"] {
        let imports = a.extract_imports(&format!("unit u;\ninterface\nuses\n {spelling};\n"));
        assert!(imports.contains("sysutils"), "{spelling}: {imports:?}");
    }
    // …and a declaration compares equal to a `uses` of it, whatever the casing.
    assert_eq!(
        a.declared_module("unit MSEGui;\n"),
        Some("msegui".to_string())
    );
}

#[test]
fn conditional_directive_inside_uses_does_not_hide_a_unit() {
    let a = PascalAdapter::new();
    // Verbatim from lib/common/kernel/linux/msesetlocale.pas — the `{$if}`
    // branch has no trailing comma, so the grammar wraps `cwstring` in an ERROR
    // node. Losing it made every later `uses cwstring` read as a new dependency.
    let imports = a.extract_imports(
        "unit u;\ninterface\nuses\n{$if defined(openbsd) or defined(darwin)} cwstring \
         {$else} msecwstring {$endif} ,sysutils;\nimplementation\nend.",
    );
    assert!(imports.contains("cwstring"), "{imports:?}");
    assert!(imports.contains("msecwstring"), "{imports:?}");
    assert!(imports.contains("sysutils"), "{imports:?}");
    // The `{$if …}` condition parses as a `pp` leaf: none of its identifiers
    // may leak in as units.
    assert!(!imports.contains("defined"), "{imports:?}");
    assert!(!imports.contains("openbsd"), "{imports:?}");
}

#[test]
fn inline_comment_does_not_make_a_code_line_prose() {
    let a = PascalAdapter::new();
    // Verbatim shape from lib/common/kernel/sdl/msesystimer.pas: a unit
    // commented out in place. Blanking the whole line as prose deleted the
    // `uses` clause, after which the parser recovered the next constant as a
    // module and the import stage scored a dependency that does not exist.
    let src = "unit u;\ninterface\nuses\n msewinglob,mseevent,msesys,msedynload{,mseguiintf};\n";
    let prose = a.prose_line_ranges(src);
    assert!(!prose.contains(&4), "code line marked prose: {prose:?}");
    let imports = a.extract_imports(src);
    assert!(imports.contains("msedynload"), "{imports:?}");
    // A line that is nothing but a comment still counts as prose.
    let only = a.prose_line_ranges("unit u;\n// just a note\n");
    assert!(only.contains(&2), "{only:?}");
}

#[test]
fn internal_import_bindings_always_empty() {
    let a = PascalAdapter::new();
    // Pascal has no relative-import form; internal units are resolved via
    // resolve_repo_modules, not this hook.
    assert!(a.internal_import_bindings(UNIT).is_empty());
}

#[test]
fn import_spans_point_at_top_segment() {
    let a = PascalAdapter::new();
    let spans = a.extract_imports_with_spans("uses Winapi.Windows, System.SysUtils;\n");
    let specs: Vec<&str> = spans.iter().map(|(s, ..)| s.as_str()).collect();
    assert!(specs.contains(&"winapi"), "{specs:?}");
    assert!(specs.contains(&"system"), "{specs:?}");
    let (spec, line, col_start, col_end) = &spans[0];
    assert_eq!(*line, 1);
    // The caret spans the source spelling, which the folded identity matches
    // in length.
    assert_eq!(col_end - col_start, spec.len());
}

#[test]
fn callable_definitions_capture_procs_and_types() {
    let a = PascalAdapter::new();
    let defs = a.callable_definitions(UNIT);
    assert!(defs.contains("TWidget"), "type name: {defs:?}");
    assert!(defs.contains("Create"), "constructor: {defs:?}");
    assert!(defs.contains("Area"), "function: {defs:?}");
}

#[test]
fn callable_bodies_cover_impl_definitions() {
    let a = PascalAdapter::new();
    let names: Vec<String> = a
        .callable_bodies(UNIT)
        .into_iter()
        .map(|b| b.symbol)
        .collect();
    assert!(names.contains(&"Create".to_string()), "{names:?}");
    assert!(names.contains(&"Area".to_string()), "{names:?}");
}

#[test]
fn value_bindings_capture_vars_args_and_assignments() {
    let a = PascalAdapter::new();
    let vals = a.value_bindings(UNIT);
    assert!(vals.contains("Tmp"), "local var: {vals:?}");
    assert!(vals.contains("MAX"), "const name: {vals:?}");
    assert!(vals.contains("Result"), "assignment lhs: {vals:?}");
}

#[test]
fn code_file_is_not_data_dominant() {
    let a = PascalAdapter::new();
    assert!(!a.is_data_dominant(UNIT, 0.65));
}

#[test]
fn const_table_is_data_dominant() {
    let a = PascalAdapter::new();
    let src = "const\n  A: array[0..2] of Integer = (1, 2, 3);\n  B: array[0..2] of Integer = (4, 5, 6);\n  C: array[0..2] of Integer = (7, 8, 9);\n";
    assert!(!a.data_literal_lines(src).is_empty(), "table rows detected");
    assert!(a.is_data_dominant(src, 0.5));
}

#[test]
fn auto_generated_header_is_detected() {
    let a = PascalAdapter::new();
    let src = "{ This file is auto-generated. Do not edit. }\nunit Gen;\ninterface\nimplementation\nend.\n";
    assert!(a.is_auto_generated(src, &crate::test_support::generated_markers()));
}

#[test]
fn prose_ranges_cover_all_comment_forms() {
    let a = PascalAdapter::new();
    let src = "// line comment\n{ brace block }\n(* paren block *)\nprocedure P; begin end;\n";
    let rows = a.prose_line_ranges(src);
    assert!(rows.contains(&1), "// : {rows:?}");
    assert!(rows.contains(&2), "{{}} : {rows:?}");
    assert!(rows.contains(&3), "(* *) : {rows:?}");
    assert!(!rows.contains(&4), "code line is not prose: {rows:?}");
}

#[test]
fn sampleable_ranges_cover_proc_bodies() {
    let a = PascalAdapter::new();
    let ranges = a.enumerate_sampleable_ranges(UNIT);
    // The two implementation bodies (Create, Area) are enumerated as defProc spans.
    assert!(ranges.len() >= 2, "{ranges:?}");
}

#[test]
fn interface_only_unit_falls_back_to_type_spans() {
    let a = PascalAdapter::new();
    // No implementation bodies — the class type declaration is the fallback.
    let src = "unit U;\ninterface\ntype\n  TFoo = class\n    procedure Bar;\n  end;\nimplementation\nend.\n";
    let ranges = a.enumerate_sampleable_ranges(src);
    assert!(!ranges.is_empty(), "type span fallback: {ranges:?}");
}

#[test]
fn identifier_noise_contains_keywords() {
    let a = PascalAdapter::new();
    assert!(a.identifier_noise().contains("begin"));
    assert!(a.identifier_noise().contains("procedure"));
    assert!(a.identifier_noise().contains("Result"));
}

#[test]
fn module_declaration_name_reads_unit_program_library() {
    assert_eq!(
        module_declaration_name("unit SysFoo;\ninterface\n").as_deref(),
        Some("SysFoo")
    );
    assert_eq!(
        module_declaration_name("program Hello;\nbegin\nend.\n").as_deref(),
        Some("Hello")
    );
    // Dotted unit name kept verbatim (top-segment split happens at registration).
    assert_eq!(
        module_declaration_name("unit mormot.core.base;\n").as_deref(),
        Some("mormot.core.base")
    );
    // License header before the declaration is skipped.
    assert_eq!(
        module_declaration_name("{ (c) 2026 license\n  multi-line }\nunit Later;\n").as_deref(),
        Some("Later")
    );
    // An include fragment with no module header yields nothing.
    assert_eq!(module_declaration_name("  x := x + 1;\n"), None);
}

#[test]
fn name_top_segment_splits_on_dot() {
    assert_eq!(name_top_segment("mormot.core.json"), "mormot");
    assert_eq!(name_top_segment("SysUtils"), "SysUtils");
    assert_eq!(unit_identity("SysUtils"), "sysutils");
}

#[test]
fn declared_module_reports_the_unit_this_file_defines() {
    let a = PascalAdapter::new();
    assert_eq!(
        a.declared_module("unit mwayland;\ninterface\nimplementation\nend.\n"),
        Some("mwayland".to_string())
    );
    // The licence header every MSEgui unit carries must not hide the name.
    assert_eq!(
        a.declared_module("{ Copyright (c) 1999\n  see COPYING }\nunit msegui;\n"),
        Some("msegui".to_string())
    );
    // Reduced to the same top segment `extract_imports` produces, so a
    // declaration and a `uses` entry compare equal.
    assert_eq!(
        a.declared_module("unit mormot.core.json;\n"),
        Some("mormot".to_string())
    );
    assert_eq!(
        a.declared_module("program demo;\n"),
        Some("demo".to_string())
    );
    assert_eq!(a.declared_module("// nothing declared here\n"), None);
}

#[test]
fn a_declared_unit_matches_how_that_unit_is_imported() {
    // The property that makes the changeset-attestation work: whatever a file
    // declares is exactly what a `uses` of it resolves to.
    let a = PascalAdapter::new();
    let declared = a
        .declared_module("unit sdl4msegui;\ninterface\nend.\n")
        .unwrap();
    let imported = a.extract_imports("unit user;\ninterface\nuses\n sdl4msegui,msetypes;\n");
    assert!(
        imported.contains(&declared),
        "declared {declared:?} not found in {imported:?}"
    );
}
