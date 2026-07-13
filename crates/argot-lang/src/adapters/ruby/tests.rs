use super::*;

#[test]
fn callable_bodies_covers_methods_and_singletons() {
    let a = RubyAdapter::new();
    let src =
        "def slugify(s)\n  t = s.downcase\n  t.strip\nend\n\ndef self.build(x)\n  x * 2\nend\n";
    let names: Vec<String> = a
        .callable_bodies(src)
        .into_iter()
        .map(|b| b.symbol)
        .collect();
    assert!(names.contains(&"slugify".to_string()), "{names:?}");
    assert!(
        names.contains(&"build".to_string()),
        "singleton method: {names:?}"
    );
}

#[test]
fn require_family_yields_top_segment() {
    let adapter = RubyAdapter::new();
    let src = "require \"active_support/core_ext\"\nload \"rake\"\nautoload :Foo, \"foo/bar\"\n";
    let imports = adapter.extract_imports(src);
    assert!(imports.contains("active_support"));
    assert!(imports.contains("rake"));
    assert!(imports.contains("foo"));
}

#[test]
fn require_relative_is_internal_not_import() {
    let adapter = RubyAdapter::new();
    let src = "require_relative \"lib/widget\"\n";
    assert!(adapter.extract_imports(src).is_empty());
    assert!(adapter.internal_import_bindings(src).contains("widget"));
}

#[test]
fn include_extend_prepend_are_mixins_not_imports() {
    let adapter = RubyAdapter::new();
    // `include`/`extend`/`prepend` mix a module (internal helper, or a
    // stdlib module like `Comparable`) into a class — usage, not a
    // dependency load. The gem, if any, is signalled by its `require`.
    let src =
        "class C\n  include ExcludeLimitHelper\n  include Comparable\n  extend Forwardable\nend\n";
    assert!(adapter.extract_imports(src).is_empty());
    // `require` of the same gem is still captured as the dependency.
    let req =
        "require \"active_support/concern\"\nclass C\n  include ActiveSupport::Concern\nend\n";
    assert!(adapter.extract_imports(req).contains("active_support"));
}

#[test]
fn import_spans_point_at_leading_segment() {
    let adapter = RubyAdapter::new();
    let spans = adapter.extract_imports_with_spans("require \"faraday/http\"\n");
    assert_eq!(spans.len(), 1);
    let (spec, line, col_start, col_end) = &spans[0];
    assert_eq!(spec, "faraday");
    assert_eq!(*line, 1);
    // caret sits inside the string, under `faraday`.
    assert_eq!(col_end - col_start, "faraday".len());
}

#[test]
fn callable_definitions_capture_defs_classes_modules() {
    let adapter = RubyAdapter::new();
    let src =
        "module M\n  class Foo::Bar < Base\n    def hi; end\n    def self.build; end\n  end\nend\n";
    let defs = adapter.callable_definitions(src);
    assert!(defs.contains("M"));
    assert!(defs.contains("Bar")); // rightmost segment of Foo::Bar
    assert!(defs.contains("hi"));
    assert!(defs.contains("build"));
}

#[test]
fn value_bindings_capture_assignments_and_params() {
    let adapter = RubyAdapter::new();
    let src = "def run(a, b: 1, *rest)\n  x = a\n  @y = b\nend\n";
    let vals = adapter.value_bindings(src);
    assert!(vals.contains("a"));
    assert!(vals.contains("b"));
    assert!(vals.contains("rest"));
    assert!(vals.contains("x"));
}

#[test]
fn data_table_file_is_data_dominant() {
    let adapter = RubyAdapter::new();
    let src = "CITIES = [\"a\", \"b\", \"c\"]\nCODES = {a: 1, b: 2}\n";
    assert!(adapter.is_data_dominant(src, 0.65));
    assert!(!adapter.data_literal_lines(src).is_empty());
}

#[test]
fn code_file_is_not_data_dominant() {
    let adapter = RubyAdapter::new();
    assert!(!adapter.is_data_dominant("def f(x)\n  x + 1\nend\n", 0.65));
}

#[test]
fn auto_generated_header_is_detected() {
    let adapter = RubyAdapter::new();
    let src = "# This file is auto-generated from the current state of the database.\nActiveRecord::Schema.define do\nend\n";
    assert!(adapter.is_auto_generated(src, &crate::test_support::generated_markers()));
}

#[test]
fn sampleable_ranges_cover_method_bodies() {
    let adapter = RubyAdapter::new();
    let src = "class Widget\n  def build\n    a = 1\n    a + 2\n  end\nend\n";
    let ranges = adapter.enumerate_sampleable_ranges(src);
    // The `def build` (lines 2..5) is enumerated.
    assert!(ranges.iter().any(|&(s, e)| s == 2 && e == 5));
}

#[test]
fn dsl_file_falls_back_to_class_span() {
    let adapter = RubyAdapter::new();
    // No methods — top-level class span is the fallback.
    let src = "class Config\n  setting :a\n  setting :b\nend\n";
    let ranges = adapter.enumerate_sampleable_ranges(src);
    assert_eq!(ranges, vec![(1, 4)]);
}

#[test]
fn prose_ranges_cover_line_and_block_comments() {
    let adapter = RubyAdapter::new();
    let src = "# line one\n=begin\nblock\ncomment\n=end\ndef f; end\n";
    let rows = adapter.prose_line_ranges(src);
    assert!(rows.contains(&1)); // # line one
    assert!(rows.contains(&2)); // =begin
    assert!(rows.contains(&5)); // =end
    assert!(!rows.contains(&6)); // def f — not prose
}

#[test]
fn identifier_noise_contains_keywords() {
    let adapter = RubyAdapter::new();
    assert!(adapter.identifier_noise().contains("self"));
    assert!(adapter.identifier_noise().contains("def"));
    assert_eq!(adapter.identifier_noise().len(), NOISE.len());
}
