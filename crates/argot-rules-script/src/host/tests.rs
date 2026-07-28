use super::*;

fn file<'a>(source: &'a str, hunks: &'a [(usize, usize)]) -> FileInput<'a> {
    FileInput {
        path: "src/app.py",
        language: "python",
        new_text: source,
        old_text: None,
        hunks,
    }
}

fn run(script: &str, source: &str, hunks: &[(usize, usize)]) -> Vec<ScriptFinding> {
    let ast = compile(script).unwrap();
    run_on_file(
        &ast,
        &file(source, hunks),
        vec!["src/app.py".into()],
        None,
        None,
    )
    .unwrap()
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
        None,
    )
    .unwrap();
    assert_eq!(out[0].line, 7);
}

#[test]
fn old_side_reaches_the_script() {
    let ast = compile(
        r#"
// A test that existed before and vanished: present in the pre-image,
// absent from the post-image.
let before = ts_query_old("(function_definition name: (identifier) @fn)");
let after = ts_query("(function_definition name: (identifier) @fn)");
if before.len() > after.len() && file.old_text != () {
    report(1, "a function disappeared");
}
"#,
    )
    .unwrap();
    let out = run_on_file(
        &ast,
        &FileInput {
            path: "src/app.py",
            language: "python",
            new_text: "def keep():\n    pass\n",
            old_text: Some("def keep():\n    pass\n\ndef gone():\n    pass\n"),
            hunks: &[(1, 2)],
        },
        Vec::new(),
        None,
        None,
    )
    .unwrap();
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].message, "a function disappeared");
}

#[test]
fn infinite_loop_trips_the_sandbox() {
    let ast = compile("loop { }").unwrap();
    let err = run_on_file(&ast, &file("x\n", &[]), Vec::new(), None, None).unwrap_err();
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

/// A repo the script can read, without touching a real filesystem.
struct FakeRepo {
    files: Vec<(String, String)>,
    reads: std::cell::Cell<usize>,
}

impl FakeRepo {
    fn new(files: &[(&str, &str)]) -> Self {
        Self {
            files: files
                .iter()
                .map(|(p, b)| (p.to_string(), b.to_string()))
                .collect(),
            reads: std::cell::Cell::new(0),
        }
    }
}

impl crate::repo::RepoFiles for FakeRepo {
    fn read(&self, rel: &str) -> Option<String> {
        self.reads.set(self.reads.get() + 1);
        self.files
            .iter()
            .find(|(p, _)| p == rel)
            .map(|(_, b)| b.clone())
    }
    fn paths(&self, glob: &str) -> Vec<String> {
        self.files
            .iter()
            .map(|(p, _)| p.clone())
            .filter(|p| argot_engine::suppress::fnmatch(p, glob))
            .collect()
    }
}

fn run_with_repo(script: &str, repo: Rc<dyn crate::repo::RepoFiles>) -> Vec<ScriptFinding> {
    let ast = compile(script).unwrap();
    run_on_file(
        &ast,
        &file("x = 1\n", &[(1, 1)]),
        vec!["src/app.py".into()],
        None,
        Some(repo),
    )
    .unwrap()
}

#[test]
fn read_repo_file_reaches_the_script() {
    let out = run_with_repo(
        r#"
let contract = read_repo_file("kernel/gui.inc");
if contract != () && contract.contains("gui_init") {
    report(1, "contract has gui_init");
}
"#,
        Rc::new(FakeRepo::new(&[("kernel/gui.inc", "function gui_init;\n")])),
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].message, "contract has gui_init");
}

#[test]
fn a_missing_file_is_unit_so_a_rule_can_branch_on_it() {
    let out = run_with_repo(
        r#"
if read_repo_file("nope.inc") == () { report(1, "absent"); }
"#,
        Rc::new(FakeRepo::new(&[])),
    );
    assert_eq!(out.len(), 1);
    assert_eq!(out[0].message, "absent");
}

#[test]
fn repo_paths_lists_siblings_for_a_cross_file_rule() {
    let out = run_with_repo(
        r#"
let backends = repo_paths("kernel/*/gui.pas");
report(1, "" + backends.len() + " backends: " + backends[0]);
"#,
        Rc::new(FakeRepo::new(&[
            ("kernel/linux/gui.pas", ""),
            ("kernel/windows/gui.pas", ""),
            ("kernel/gui.inc", ""),
        ])),
    );
    assert_eq!(out[0].message, "2 backends: kernel/linux/gui.pas");
}

#[test]
fn without_a_repo_the_primitives_degrade_instead_of_failing() {
    let out = run(
        r#"
if read_repo_file("anything") == () && repo_paths("*").is_empty() {
    report(1, "degraded");
}
"#,
        "x = 1\n",
        &[(1, 1)],
    );
    assert_eq!(out.len(), 1);
}

#[test]
fn the_read_budget_caps_a_looping_rule() {
    let repo = Rc::new(FakeRepo::new(&[("a.txt", "body")]));
    let out = run_with_repo(
        r#"
let got = 0;
let i = 0;
while i < 500 {
    if read_repo_file("a.txt") != () { got += 1; }
    i += 1;
}
report(1, "" + got);
"#,
        repo.clone(),
    );
    // The rule keeps running; only the reads stop.
    assert_eq!(out[0].message, crate::repo::MAX_READS.to_string());
    assert_eq!(repo.reads.get(), crate::repo::MAX_READS);
}

#[test]
fn the_listing_budget_caps_a_looping_rule() {
    let out = run_with_repo(
        r#"
let got = 0;
let i = 0;
while i < 100 {
    if !repo_paths("*.txt").is_empty() { got += 1; }
    i += 1;
}
report(1, "" + got);
"#,
        Rc::new(FakeRepo::new(&[("a.txt", "")])),
    );
    assert_eq!(out[0].message, crate::repo::MAX_PATHS_CALLS.to_string());
}

#[test]
fn nesting_a_rule_actually_needs_compiles_in_any_build_profile() {
    // Rhai lowers its expression-depth defaults under `debug_assertions`, so an
    // unpinned engine accepts this in a release binary and rejects it in a
    // debug one — the shape below is the real `contract-answered` example.
    compile(
        r#"
fn members(text) {
    let out = [];
    if text == () { return out; }
    for line in text.split("\n") {
        let t = line.to_lower();
        t.trim();
        if t.starts_with("function gui_") || t.starts_with("procedure gui_") {
            let rest = t.sub_string(t.index_of("gui_"));
            let name = "";
            for ch in rest.split("") {
                if ch == "(" || ch == ":" || ch == ";" || ch == " " { break; }
                name += ch;
            }
            if name != "" { out.push(name); }
        }
    }
    out
}
let before = members(file.old_text);
let added = [];
for m in members(file.new_text) {
    if !before.contains(m) && !added.contains(m) { added.push(m); }
}
for path in repo_paths("*/impl.pas") {
    let body = read_repo_file(path);
    if body == () { continue; }
    let low = body.to_lower();
    for name in added {
        if !low.contains("function " + name) && !low.contains("procedure " + name) {
            report(1, path + " does not answer " + name);
        }
    }
}
"#,
    )
    .expect("a rule of this shape must compile in every profile");
}

#[test]
fn the_operation_budget_scales_with_the_file() {
    // A constant cap is a cap on the input, not on the rule: at a flat
    // 1 000 000 a 9 439-line unit leaves ~106 operations per line, and
    // MSEide/MSEgui's `c-abi-managed-type` ran out on `msedb.pas` — a sane
    // rule defeated by a large file.
    let small = operation_budget(100);
    let large = operation_budget(9_439);
    assert!(large > small, "{large} vs {small}");
    assert!(
        large >= 10 * BASE_OPERATIONS,
        "a 9 439-line file must get real room, not ~106 ops a line: {large}"
    );
    // …but not unbounded: a generated monster cannot buy arbitrary time.
    assert_eq!(operation_budget(usize::MAX), MAX_OPERATIONS_CEILING);
}
