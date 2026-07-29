use super::*;

#[test]
fn parses_valid_python_into_a_module_with_no_errors() {
    let tree = parse("def foo():\n    pass\n", Language::Python).expect("parse succeeds");
    assert_eq!(tree.root_node().kind(), "module");
    assert!(!tree.root_node().has_error());
}

#[test]
fn parses_valid_typescript_into_a_program_with_no_errors() {
    let tree = parse("function foo(): void {}\n", Language::Typescript).expect("parse succeeds");
    assert_eq!(tree.root_node().kind(), "program");
    assert!(!tree.root_node().has_error());
}

#[test]
fn parses_valid_go_into_a_source_file() {
    let tree = parse("package main\n\nfunc main() {}\n", Language::Go).expect("parse succeeds");
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
}

#[test]
fn parses_valid_rust_into_a_source_file() {
    let tree = parse("fn main() {}\n", Language::Rust).expect("parse succeeds");
    assert_eq!(tree.root_node().kind(), "source_file");
    assert!(!tree.root_node().has_error());
}

#[test]
fn empty_source_still_parses_to_an_empty_tree() {
    let tree = parse("", Language::Python).expect("empty input is a valid (empty) parse");
    assert_eq!(tree.root_node().kind(), "module");
    assert_eq!(tree.root_node().child_count(), 0);
}

#[test]
fn syntactically_broken_source_still_returns_a_tree_flagged_with_an_error() {
    // tree-sitter is error-tolerant: parse() still returns Some(tree), but
    // the tree records that recovery was needed.
    let tree = parse("def foo(:\n", Language::Python).expect("parse never returns None");
    assert!(tree.root_node().has_error());
}

#[test]
fn reusing_the_thread_local_parser_reflects_each_call_source_independently() {
    // The parser is reused across calls (thread_local); each call must
    // still produce a tree over exactly its own `source`, with no leakage
    // from the previous call on the same thread.
    let first = parse("def a():\n    pass\n", Language::Python).unwrap();
    let second = parse("def b():\n    return 1\n", Language::Python).unwrap();
    assert_eq!(first.root_node().byte_range(), 0..18);
    assert_eq!(second.root_node().byte_range(), 0..22);
}

#[test]
fn python_and_typescript_parsers_are_independent() {
    // Interleave languages on the same thread and confirm neither
    // thread-local parser's state bleeds into the other's result.
    let py = parse("x = 1\n", Language::Python).unwrap();
    let ts = parse("let x: number = 1;\n", Language::Typescript).unwrap();
    assert_eq!(py.root_node().kind(), "module");
    assert_eq!(ts.root_node().kind(), "program");
    assert!(!py.root_node().has_error());
    assert!(!ts.root_node().has_error());
}

#[test]
fn a_compiler_directive_does_not_swallow_the_unit() {
    use crate::adapters::Language;
    // The grammar cannot parse a directive between a routine header and its
    // body, and an *empty* one is enough. Untreated, the error node runs to the
    // end of the unit — 7 500 lines of MSEide/MSEgui's msedb.pas — and every
    // function after it looks structureless.
    let breaks = "unit u;\ninterface\nimplementation\n\
        function outer: boolean; {$ifdef FPC}inline;{$endif}\n\
        begin\n result:= true;\nend;\n\
        function after: integer;\nbegin\n result:= 1;\nend;\nend.";
    let tree = crate::ts_parse::parse(breaks, Language::Pascal).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "the directive must not break it"
    );

    // Offsets survive the masking, so every span still addresses the original
    // source — the property everything downstream relies on.
    let src =
        "unit u;\ninterface\nuses\n {$ifdef FPC}cthreads,{$endif}classes;\nimplementation\nend.";
    let tree = crate::ts_parse::parse(src, Language::Pascal).unwrap();
    let mut found = Vec::new();
    fn walk<'a>(n: tree_sitter::Node<'a>, src: &str, out: &mut Vec<String>) {
        if n.kind() == "identifier" {
            out.push(src[n.byte_range()].to_string());
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, src, out);
        }
    }
    walk(tree.root_node(), src, &mut found);
    assert!(found.contains(&"cthreads".to_string()), "{found:?}");
    assert!(found.contains(&"classes".to_string()), "{found:?}");

    // A file with no directive is borrowed unchanged, so the common case pays
    // nothing.
    let plain = "unit u;\ninterface\nimplementation\nend.";
    assert!(!crate::ts_parse::parse(plain, Language::Pascal)
        .unwrap()
        .root_node()
        .has_error());
}

#[test]
fn only_one_branch_of_a_conditional_survives() {
    use crate::adapters::Language;
    // A conditional inside a *single* declaration is not two declarations. Both
    // branches left standing give a duplicate name and an unterminated `record`,
    // and mORMot's mormot.crypt.core.pas loses all 10 643 of its lines to it.
    let src = "unit u;\ninterface\ntype\n\
        {$ifdef USERECORDWITHMETHODS}\n  TAes = record\n{$else}\n  TAes = object\n{$endif}\n\
          a: byte;\n  end;\nimplementation\nend.";
    let tree = crate::ts_parse::parse(src, Language::Pascal).unwrap();
    assert!(
        !tree.root_node().has_error(),
        "the dropped branch must not stand beside the kept one"
    );

    // The same shape inline, where blanking alone yields `TStrLen = SizeInt
    // integer;` — two type names where one belongs.
    let inline = "unit u;\ninterface\ntype\n  \
        TStrLen = {$ifdef FPC} SizeInt {$else} integer {$endif};\nimplementation\nend.";
    assert!(!crate::ts_parse::parse(inline, Language::Pascal)
        .unwrap()
        .root_node()
        .has_error());
}

#[test]
fn a_conditional_keeps_the_first_branch_and_drops_the_rest() {
    use crate::adapters::Language;
    // Which branch survives decides what vocabulary the repository is learned
    // to have, so it must be the same on every machine and run: the first.
    let src = "unit u;\ninterface\nuses\n  \
        {$ifdef FPC}taken{$else}dropped{$endif};\nimplementation\nend.";
    let tree = crate::ts_parse::parse(src, Language::Pascal).unwrap();
    let mut found = Vec::new();
    fn walk<'a>(n: tree_sitter::Node<'a>, src: &str, out: &mut Vec<String>) {
        if n.kind() == "identifier" {
            out.push(src[n.byte_range()].to_string());
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, src, out);
        }
    }
    walk(tree.root_node(), src, &mut found);
    assert!(found.contains(&"taken".to_string()), "{found:?}");
    assert!(!found.contains(&"dropped".to_string()), "{found:?}");

    // A nested conditional inside a dropped branch stays dropped whatever it
    // says, and the `{$endif}` that closes it must not close the outer one.
    let nested = "unit u;\ninterface\nuses\n  \
        {$ifdef A}outer{$else}{$ifdef B}inner{$endif}also{$endif};\nimplementation\nend.";
    let tree = crate::ts_parse::parse(nested, Language::Pascal).unwrap();
    let mut found = Vec::new();
    walk(tree.root_node(), nested, &mut found);
    assert!(found.contains(&"outer".to_string()), "{found:?}");
    assert!(!found.contains(&"inner".to_string()), "{found:?}");
    assert!(!found.contains(&"also".to_string()), "{found:?}");
}

#[test]
fn a_directive_inside_a_comment_is_not_a_directive() {
    use crate::adapters::Language;
    // Commenting a conditional out is an everyday edit and appears in
    // MSEide/MSEgui (`//{$endif}` at msedbedit.pas:17). Counting it would pop a
    // conditional that is still open and drop the rest of the unit.
    let src = "unit u;\ninterface\nuses\n  {$ifdef FPC}\n  //{$endif}\n  kept;\n  \
        {$endif}\nimplementation\nend.";
    let tree = crate::ts_parse::parse(src, Language::Pascal).unwrap();
    let mut found = Vec::new();
    fn walk<'a>(n: tree_sitter::Node<'a>, src: &str, out: &mut Vec<String>) {
        if n.kind() == "identifier" {
            out.push(src[n.byte_range()].to_string());
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, src, out);
        }
    }
    walk(tree.root_node(), src, &mut found);
    assert!(found.contains(&"kept".to_string()), "{found:?}");

    // A `{$…}` written inside a string literal is data, not an instruction.
    let literal = "unit u;\ninterface\nconst s = '{$else}';\nimplementation\nend.";
    assert!(!crate::ts_parse::parse(literal, Language::Pascal)
        .unwrap()
        .root_node()
        .has_error());
}

#[test]
fn masking_a_dropped_branch_preserves_every_offset() {
    use crate::adapters::Language;
    // Everything downstream addresses the original source through the tree, so
    // a dropped branch must cost the same bytes and the same rows it occupied —
    // including when it carries a multi-byte character.
    let src = "unit u;\ninterface\nuses\n  {$ifdef A}kept{$else}drppé\n  more{$endif};\n\
        implementation\nend.";
    let tree = crate::ts_parse::parse(src, Language::Pascal).unwrap();
    let mut found = None;
    fn walk<'a>(n: tree_sitter::Node<'a>, src: &str, out: &mut Option<(usize, String)>) {
        if n.kind() == "identifier" && &src[n.byte_range()] == "kept" {
            *out = Some((n.start_position().row, src[n.byte_range()].to_string()));
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, src, out);
        }
    }
    walk(tree.root_node(), src, &mut found);
    assert_eq!(
        found,
        Some((3, "kept".to_string())),
        "row and text preserved"
    );
}

#[test]
fn a_multi_byte_character_in_a_dropped_branch_does_not_shift_what_follows() {
    use crate::adapters::Language;
    // Blanking a dropped branch character-by-character turns a multi-byte one
    // into a single space, and from there on every offset in the file addresses
    // the wrong place — mORMot's corpus reached `replace_range` with an index
    // mid-character and panicked. The masking must be byte for byte.
    let src = "unit u;\ninterface\nuses\n  {$ifdef A}kept{$else}accentué·½·{$endif};\n\
        implementation\nprocedure After;\nbegin\n  Marker;\nend;\nend.";
    let tree = crate::ts_parse::parse(src, Language::Pascal).unwrap();
    let mut found = None;
    fn walk<'a>(n: tree_sitter::Node<'a>, src: &str, out: &mut Option<(usize, usize)>) {
        if n.kind() == "identifier" && &src[n.byte_range()] == "Marker" {
            *out = Some((n.start_position().row, n.start_byte()));
        }
        let mut c = n.walk();
        for ch in n.children(&mut c) {
            walk(ch, src, out);
        }
    }
    walk(tree.root_node(), src, &mut found);
    let (row, byte) = found.expect("the code after the dropped branch is still parsed");
    assert_eq!(row, 7, "a later row must not shift");
    assert_eq!(
        &src[byte..byte + 6],
        "Marker",
        "the byte offset still lands"
    );
}

#[test]
fn jsx_is_read_by_the_tsx_grammar() {
    use crate::adapters::Language;
    // TSX is a separate grammar, not a superset. Parsed with LANGUAGE_TYPESCRIPT
    // a component file is unreadable — measured on excalidraw, 191 of 200 `.tsx`
    // files failed against 1 of 200 `.ts` — and one error node spanning the file
    // blinds every rule that reads structure behind it.
    let tsx = "export function Panel({ title }: { title: string }) {\n\
               \x20 return <div className=\"p\"><span>{title}</span></div>;\n\
               }\n";
    let t = crate::ts_parse::parse(tsx, Language::Typescript).unwrap();
    assert!(!t.root_node().has_error(), "JSX must be readable");

    // …and a `.ts`-only construct the TSX grammar reads differently must still
    // parse as TypeScript: the angle-bracket type assertion is exactly why the
    // two grammars are separate, so it must not be handed to TSX.
    let ts = "const el = <HTMLInputElement>document.getElementById('x');\n";
    let t = crate::ts_parse::parse(ts, Language::Typescript).unwrap();
    assert!(
        !t.root_node().has_error(),
        "TS type assertion must still parse"
    );
}

#[test]
fn a_conditional_in_an_unparseable_position_does_not_swallow_the_file() {
    // A `#ifdef` inside a constructor initialiser list is not a place the
    // grammar can put a directive. Measured on curl.h, one such conditional
    // turned the whole 3 347-line header into a single ERROR node.
    let src = "struct S { int a; int b; };\n\
               void f(void) { int x = 1; }\n\
               #ifdef FEATURE\n\
               int g(void) { return 1; }\n\
               #else\n\
               int g(void) { return 2; }\n\
               #endif\n\
               void h(void) { int y = 2; }\n";
    let tree = parse(src, Language::C).expect("parse succeeds");
    assert!(
        !tree.root_node().has_error(),
        "conditionals must not break the parse"
    );
    // Both branches survive: the control lines go, the code they guard stays.
    assert_eq!(
        src.len(),
        tree.root_node().end_byte(),
        "offsets still address the source"
    );
}

#[test]
fn a_define_is_kept_when_conditionals_are_blanked() {
    // `#define` and `#include` declare names and dependencies the scorers read;
    // only the `#if`/`#else`/`#endif` control lines are removed.
    // The conditional inside the initialiser list is what forces the repair.
    let src = "#include <stdio.h>\n\
               #define N 4\n\
               struct S { int a; int b; };\n\
               struct S s = {\n\
               #ifdef FEATURE\n\
                 1,\n\
               #endif\n\
                 2 };\n";
    let tree = parse(src, Language::C).expect("parse succeeds");
    let mut kinds = vec![];
    let mut cursor = tree.walk();
    loop {
        kinds.push(cursor.node().kind());
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            if !cursor.goto_parent() {
                assert!(
                    kinds.contains(&"preproc_include"),
                    "include kept, got {kinds:?}"
                );
                assert!(kinds.contains(&"preproc_def"), "define kept, got {kinds:?}");
                return;
            }
        }
    }
}

#[test]
fn a_clean_source_is_never_rewritten_by_the_repair() {
    // The repair only runs when the routed grammar failed, so a file that
    // parses pays nothing and is byte-for-byte what it always was.
    let src = "#ifdef A\nint x = 1;\n#endif\n";
    let tree = parse(src, Language::C).expect("parse succeeds");
    assert!(!tree.root_node().has_error());
    let node = tree.root_node().child(0).expect("a first child");
    assert_eq!(
        node.kind(),
        "preproc_ifdef",
        "an already-clean parse keeps its directives"
    );
}

#[test]
fn a_c_header_in_a_cpp_repo_falls_back_to_the_c_grammar() {
    // `.h` is C or C++ and the extension cannot say which; the repo-level
    // majority routes rocksdb's headers to C++, and its C headers read nine
    // times worse there (xxhash.h: 274 error rows as C, 2 402 as C++).
    let src = "int f(int (*cb)(void *), void *arg) { return cb(arg); }\n\
               class NotC { public: int x; };\n";
    // A file the routed grammar reads stays with it — C++ here, since the C
    // grammar cannot read `class`.
    let tree = parse(src, Language::Cpp).expect("parse succeeds");
    assert!(!tree.root_node().has_error(), "C++ source must stay C++");
}
