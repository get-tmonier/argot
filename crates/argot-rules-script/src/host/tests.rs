use super::*;

fn file<'a>(source: &'a str, hunks: &'a [(usize, usize)]) -> FileInput<'a> {
    FileInput {
        path: "src/app.py",
        language: "python",
        new_text: source,
        hunks,
    }
}

fn run(script: &str, source: &str, hunks: &[(usize, usize)]) -> Vec<ScriptFinding> {
    let ast = compile(script).unwrap();
    run_on_file(&ast, &file(source, hunks), vec!["src/app.py".into()], None).unwrap()
}

#[test]
fn report_and_scope_variables() {
    let out = run(
        r#"
if file.language == "python" {
    report(2, "found in " + file.path);
}
"#,
        "x = 1\ny = 2\n",
        &[(1, 2)],
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].line, 2);
    assert_eq!(out[0].message, "found in src/app.py");
}

#[test]
fn hunks_carry_their_text() {
    let out = run(
        r#"
for h in hunks {
    if h.text.contains("secret") {
        report(h.start, "hunk " + h.start + ".." + h.end);
    }
}
"#,
        "a = 1\nsecret = 2\nc = 3\n",
        &[(2, 3)],
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].message, "hunk 2..3");
}

#[test]
fn report_span_carries_evidence_and_symbol() {
    let out = run(
        r#"report_span(3, 5, "m", #{ evidence: ["because", "reasons"], symbol: "handler" });"#,
        "x\n",
        &[],
    );
    assert_eq!(out[0].line, 3);
    assert_eq!(out[0].line_end, 5);
    assert_eq!(out[0].evidence, vec!["because", "reasons"]);
    assert_eq!(out[0].symbol.as_deref(), Some("handler"));
}

#[test]
fn ts_query_finds_python_calls() {
    let out = run(
        r#"
for m in ts_query("(call function: (attribute attribute: (identifier) @method))") {
    if m.capture == "method" && m.text == "execute" {
        report(m.line, "raw execute call");
    }
}
"#,
        "def f(db):\n    db.cursor().execute(\"select 1\")\n",
        &[(1, 2)],
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].line, 2);
}

#[test]
fn invalid_query_yields_no_matches_not_an_error() {
    let out = run(
        r#"report(1 + ts_query("(((").len(), "ok");"#,
        "x = 1\n",
        &[],
    );
    assert_eq!(out[0].line, 1);
}

#[test]
fn facts_default_to_unattested_without_a_model() {
    let out = run(
        r#"
if !import_attested("sqlalchemy") && !callee_attested("execute") {
    report(1, "nothing attested");
}
"#,
        "x = 1\n",
        &[],
    );
    assert_eq!(out.len(), 1);
}

#[test]
fn facts_route_through_the_model_facts_port() {
    struct Facts;
    impl argot_engine::detector::ModelFacts for Facts {
        fn import_attested(&self, language: &str, module: &str) -> bool {
            language == "python" && module == "requests"
        }
        fn callee_attested(&self, _language: &str, name: &str) -> bool {
            name == "get"
        }
    }
    let ast = compile(
        r#"
if import_attested("requests") && callee_attested("get") && !import_attested("httpx") {
    report(7, "attested");
}
"#,
    )
    .unwrap();
    let out = run_on_file(
        &ast,
        &file("x = 1\n", &[]),
        Vec::new(),
        Some(std::sync::Arc::new(Facts)),
    )
    .unwrap();
    assert_eq!(out[0].line, 7);
}

#[test]
fn infinite_loop_trips_the_sandbox() {
    let ast = compile("loop { }").unwrap();
    let err = run_on_file(&ast, &file("x\n", &[]), Vec::new(), None).unwrap_err();
    // Either the operation cap or the wall clock fires first — both are
    // termination, never a hang.
    assert!(
        err.contains("Script terminated") || err.contains("operations"),
        "{err}"
    );
}

#[test]
fn print_never_reaches_stdout_and_compile_errors_surface() {
    // `print` runs (captured, no panic, no stdout assertion possible here —
    // the engine hook swallows it).
    let out = run(r#"print("noise"); report(1, "after print");"#, "x\n", &[]);
    assert_eq!(out.len(), 1);
    assert!(compile("fn {").is_err());
}
