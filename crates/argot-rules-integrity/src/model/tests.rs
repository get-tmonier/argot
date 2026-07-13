use super::*;

fn change(path: &str, old: &str, new: &str) -> FileChange {
    FileChange {
        path: path.to_string(),
        old: Some(old.to_string()),
        new: Some(new.to_string()),
    }
}

const PROD_OLD: &str = "def parse(x):\n    return x.strip()\n";
const PROD_NEW: &str = "def parse(x):\n    y = x.strip()\n    return y.lower()\n";

const TEST_OLD: &str = r#"
def test_parse():
    assert parse(" A ") == "A"
    assert parse("") == ""
"#;

#[test]
fn tests_only_changeset_is_silent() {
    let files = [change(
        "tests/test_parse.py",
        TEST_OLD,
        "def test_parse():\n    pass\n",
    )];
    assert!(changeset_events(&files).is_empty());
}

#[test]
fn skip_added_alongside_prod_change_fires_test_disabled() {
    let new_test = r#"
import pytest

@pytest.mark.skip(reason="broken on new parser")
def test_parse():
    assert parse(" A ") == "A"
    assert parse("") == ""
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, new_test),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::SkipAdded);
    assert_eq!(events[0].kind.reason(), "test_disabled");
    assert_eq!(events[0].test_name, "test_parse");
    let ev = events[0].evidence();
    assert!(ev.contains("test_parse"), "{ev}");
    assert!(ev.contains("parser.py"), "{ev}");
}

#[test]
fn body_gutting_fires_test_disabled() {
    let new_test = "def test_parse():\n    pass\n";
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, new_test),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::BodyGutted);
}

#[test]
fn pure_excision_fires_test_weakened() {
    let new_test = r#"
def test_parse():
    assert parse(" A ") == "A"
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, new_test),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::AssertionsRemoved);
    assert_eq!(events[0].kind.reason(), "test_weakened");
}

#[test]
fn excision_with_other_edits_to_the_test_is_churn_not_weakening() {
    // The surviving assertion also changed (retarget) → not pure excision.
    let new_test = r#"
def test_parse():
    assert parse(" A ") == "a"
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, new_test),
    ];
    let events = changeset_events(&files);
    assert!(events
        .iter()
        .all(|e| e.kind != EventKind::AssertionsRemoved));
}

#[test]
fn deleted_test_with_surviving_subject_fires_test_deleted() {
    let old_test = r#"
def test_parse():
    assert parse(" A ") == "A"

def test_other():
    assert other() == 1
"#;
    let new_test = r#"
def test_other():
    assert other() == 1
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", old_test, new_test),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::TestDeleted);
    assert_eq!(events[0].test_name, "test_parse");
}

#[test]
fn deletion_with_the_feature_is_legitimate() {
    // parse() itself is removed → the test followed its subject out.
    let files = [
        change("parser.py", PROD_OLD, "def other_thing():\n    return 1\n"),
        change(
            "tests/test_parse.py",
            TEST_OLD,
            "def test_nothing():\n    assert other_thing() == 1\n",
        ),
    ];
    let events = changeset_events(&files);
    assert!(events.iter().all(|e| e.kind != EventKind::TestDeleted));
}

#[test]
fn moved_test_is_not_a_deletion() {
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, "# moved\n"),
        FileChange {
            path: "tests/test_parser_new.py".to_string(),
            old: None,
            new: Some(TEST_OLD.to_string()),
        },
    ];
    let events = changeset_events(&files);
    assert!(events
        .iter()
        .all(|e| { e.kind != EventKind::TestDeleted && e.kind != EventKind::TestFileDeleted }));
}

#[test]
fn tautology_fires_test_weakened() {
    let new_test = r#"
def test_parse():
    assert parse(" A ") == "A"
    assert parse("") == ""
    assert True
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, new_test),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::Tautology);
}

#[test]
fn isolated_retarget_is_detected_bulk_retarget_is_not() {
    let one_flip = r#"
def test_parse():
    assert parse(" A ") == "A!"
    assert parse("") == ""
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, one_flip),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::Retarget);

    // A flip that also adds a new test is a behaviour change, not gaming.
    let flip_plus_growth = r#"
def test_parse():
    assert parse(" A ") == "A!"
    assert parse("") == ""

def test_parse_lower():
    assert parse(" b ") == "b"
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, flip_plus_growth),
    ];
    let events = changeset_events(&files);
    assert!(events.iter().all(|e| e.kind != EventKind::Retarget));
}

#[test]
fn strengthening_never_fires() {
    let stronger = r#"
def test_parse():
    assert parse(" A ") == "A"
    assert parse("") == ""
    assert parse(" b ") == "b"
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_parse.py", TEST_OLD, stronger),
    ];
    assert!(changeset_events(&files).is_empty());
}

#[test]
fn java_asserttrue_true_tautology_fires() {
    let prod_old = "class StringUtils {\n    static String x() { return \"a\"; }\n}\n";
    let prod_new = "class StringUtils {\n    static String x() { return \"a\"; }\n    static String y() { return \"b\"; }\n}\n";
    let test_old = r#"
class StringUtilsTests {
    @Test
    void nonEmpty() {
        String[] array = {"a"};
        assertSame(array, nonEmptyArray(array));
    }
}
"#;
    let test_new = r#"
class StringUtilsTests {
    @Test
    void nonEmpty() {
        String[] array = {"a"};
        assertTrue(true);
    }
}
"#;
    let files = [
        change("src/main/java/StringUtils.java", prod_old, prod_new),
        change("src/test/java/StringUtilsTests.java", test_old, test_new),
    ];
    let events = changeset_events(&files);
    assert!(
        events.iter().any(|e| e.kind == EventKind::Tautology),
        "expected tautology, got {events:?}"
    );
}

#[test]
fn middle_occurrence_churn_move_is_silent() {
    // Three identical-shape assertions; the MIDDLE one moves to a sibling
    // test. Positional zip must not fabricate a retarget or an excision.
    let old_test = r#"
def test_ticker():
    assert ticker.read() == 10
    assert ticker.read() == 20
    assert ticker.read() == 30

def test_other():
    assert other() == 1
"#;
    let new_test = r#"
def test_ticker():
    assert ticker.read() == 10
    assert ticker.read() == 30

def test_other():
    assert other() == 1
    assert ticker.read() == 20
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_ticker.py", old_test, new_test),
    ];
    let events = changeset_events(&files);
    assert!(events.is_empty(), "churn move fired: {events:?}");
}

#[test]
fn extract_to_helper_refactor_is_silent() {
    // The assertion moves into a plain helper the inventory doesn't walk;
    // its raw text survives in the changeset, so nothing was lost.
    let old_test = r#"
def test_info():
    assert info.version == 3
    assert info.name == "redis"
"#;
    let new_test = r#"
def check_version(info):
    assert info.version == 3

def test_info():
    check_version(info)
    assert info.name == "redis"
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_info.py", old_test, new_test),
    ];
    let events = changeset_events(&files);
    assert!(
        events
            .iter()
            .all(|e| e.kind != EventKind::AssertionsRemoved),
        "helper extraction read as excision: {events:?}"
    );
}

#[test]
fn adapted_extract_to_helper_is_silent_but_duplicate_excision_fires() {
    // Moved-and-adapted: the assertion reappears in an added helper with
    // a renamed variable — a refactor, not a loss.
    let old_test = r#"
def test_info():
    assert parse(raw_value) == expected_value
    assert info.name == "redis"
"#;
    let adapted = r#"
def check_parse(value):
    assert parse(value) == expected_value

def test_info():
    check_parse(raw_value)
    assert info.name == "redis"
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_info.py", old_test, adapted),
    ];
    assert!(
        changeset_events(&files)
            .iter()
            .all(|e| e.kind != EventKind::AssertionsRemoved),
        "adapted helper extraction read as excision"
    );

    // Duplicate-shape excision: one of two IDENTICAL assertions removed,
    // nothing added — the surviving twin must NOT excuse the excision.
    let old_dup = r#"
def test_same():
    assert parse(x) == parse(y)
    assert parse(x) == parse(y)
    assert other() == 1
"#;
    let new_dup = r#"
def test_same():
    assert parse(x) == parse(y)
    assert other() == 1
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_same.py", old_dup, new_dup),
    ];
    let events = changeset_events(&files);
    assert!(
        events
            .iter()
            .any(|e| e.kind == EventKind::AssertionsRemoved),
        "duplicate excision was wrongly excused: {events:?}"
    );
}

#[test]
fn prod_side_vocabulary_overlap_does_not_excuse_excision() {
    // The prod edit ADDS lines sharing the assertion's domain words —
    // that must not read as "the assertion moved".
    let old_test = r#"
def test_label():
    assert label.name == "bug"
    assert label.description == "Something broken"
"#;
    let new_test = r#"
def test_label():
    assert label.name == "bug"
"#;
    let prod_new = "def parse(x):\n    return x.strip()\n\ndef label_description(label):\n    assert_valid(label.description)\n    return label.description\n";
    let files = [
        change("parser.py", PROD_OLD, prod_new),
        change("tests/test_label.py", old_test, new_test),
    ];
    let events = changeset_events(&files);
    assert!(
        events
            .iter()
            .any(|e| e.kind == EventKind::AssertionsRemoved),
        "prod-side vocabulary excused the excision: {events:?}"
    );
}

#[test]
fn rust_tests_only_ignore_is_silent() {
    // In-file unit tests: adding #[ignore] is test text, not production
    // text — the scope guard must hold even though the attribute is a
    // sibling of the function item.
    let old_src = r#"
pub fn escape(s: &str) -> String { s.to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        assert_eq!(escape("x"), "x");
    }
}
"#;
    let new_src = r#"
pub fn escape(s: &str) -> String { s.to_string() }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "flaky on CI"]
    fn roundtrip() {
        assert_eq!(escape("x"), "x");
    }
}
"#;
    let files = [change("src/escape.rs", old_src, new_src)];
    let events = changeset_events(&files);
    assert!(events.is_empty(), "tests-only ignore fired: {events:?}");
}

#[test]
fn custom_assert_helper_with_literal_arg_is_not_a_tautology() {
    // `assertRunFAIL("cmd")`-style custom helpers embed their subject
    // internally; a literal-only argument is not a can-never-fail check.
    let old_test = "def test_cmds():\n    assertRunFAIL(\"checkconsistency\")\n";
    let new_test =
        "def test_cmds():\n    assertRunFAIL(\"checkconsistency\")\n    assertRunFAIL(\"scan\")\n";
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_cmds.py", old_test, new_test),
    ];
    let events = changeset_events(&files);
    assert!(
        events.iter().all(|e| e.kind != EventKind::Tautology),
        "custom helper flagged as tautology: {events:?}"
    );
}

#[test]
fn bulk_sweep_is_a_migration_not_gaming() {
    // Five tests gutted at once alongside a prod change: a migration.
    let mut old_test = String::new();
    let mut new_test = String::new();
    for i in 0..5 {
        old_test.push_str(&format!(
            "def test_case_{i}():\n    r = run({i})\n    assert r == {i}\n\n"
        ));
        new_test.push_str(&format!("def test_case_{i}():\n    r = run({i})\n\n"));
    }
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_bulk.py", old_test.as_str(), new_test.as_str()),
    ];
    let events = changeset_events(&files);
    assert!(events.is_empty(), "bulk sweep fired: {events:?}");

    // One test gutted: surgical — still fires.
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change(
            "tests/test_one.py",
            "def test_case():\n    r = run(1)\n    assert r == 1\n",
            "def test_case():\n    r = run(1)\n",
        ),
    ];
    let events = changeset_events(&files);
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, EventKind::BodyGutted);
}

#[test]
fn colliding_key_widening_emits_one_event_and_survives_bulk_guard() {
    // Several structurally identical exact assertions; ONE is replaced by
    // a weaker predicate. Must yield exactly one Widened event (duplicate
    // per-occurrence events would read as a bulk sweep and be dropped).
    let t_old = r#"
def test_advance():
    t = ticker()
    assert t.read() == 0
    assert t.read() == 1000000
    assert t.read() == 2000010
    assert t.read() == 3000010
"#;
    let t_new = r#"
def test_advance():
    t = ticker()
    assert t.read() == 0
    assert t.read() == 1000000
    assert t.read() == 2000010
    assert t.read() is not None
"#;
    let files = [
        change("parser.py", PROD_OLD, PROD_NEW),
        change("tests/test_ticker.py", t_old, t_new),
    ];
    let events = changeset_events(&files);
    let widened: Vec<_> = events
        .iter()
        .filter(|e| e.kind == EventKind::Widened)
        .collect();
    assert_eq!(widened.len(), 1, "events: {events:?}");
}

#[test]
fn tautology_capable_strips_generics() {
    use crate::test_inventory::tautology_capable;
    assert!(tautology_capable("Equal<uint>"));
    assert!(tautology_capable("assertEquals"));
    assert!(!tautology_capable("assertRunFAIL"));
}

#[test]
fn same_named_tests_pair_positionally() {
    // JUnit @Nested-style: many tests share a method name in one file.
    // Editing ONE of them must not read as mass gutting of the others.
    let mk = |bodies: &[&str]| -> String {
        bodies
            .iter()
            .enumerate()
            .map(|(i, b)| format!("class C{i} {{\n  @Test\n  void check() {{\n    {b}\n  }}\n}}\n"))
            .collect()
    };
    let t_old = mk(&[
        "assertEquals(1, f(1));",
        "assertEquals(2, f(2));",
        "assertEquals(3, f(3));",
    ]);
    let t_new = mk(&["assertEquals(1, f(1));", "assertEquals(2, f(2));", "g(3);"]);
    let files = [
        change(
            "src/main/java/F.java",
            "class F { int f(int x) { return x; } }",
            "class F { int f(int x) { return x; } int g(int x) { return x; } }",
        ),
        change("src/test/java/FTest.java", t_old.as_str(), t_new.as_str()),
    ];
    let events = changeset_events(&files);
    let gutted = events
        .iter()
        .filter(|e| e.kind == EventKind::BodyGutted)
        .count();
    assert_eq!(gutted, 1, "expected exactly one gutting event: {events:?}");
}

#[test]
fn model_gates_apply_defaults_and_learned_state() {
    let m = IntegrityModel::permissive();
    assert!(m.enabled(EventKind::SkipAdded));
    assert!(
        !m.enabled(EventKind::Retarget),
        "retarget is off by default"
    );

    let mut gates = BTreeMap::new();
    gates.insert(
        EventKind::TestDeleted.key().to_string(),
        EventGate {
            rate: 0.05,
            enabled: false,
        },
    );
    gates.insert(
        EventKind::Retarget.key().to_string(),
        EventGate {
            rate: 0.0,
            enabled: true,
        },
    );
    let m = IntegrityModel {
        version: 1,
        repo_sha: "abc".into(),
        observed_commits: 100,
        gates,
    };
    assert!(!m.enabled(EventKind::TestDeleted));
    assert!(m.enabled(EventKind::Retarget));
}

#[test]
fn artifact_roundtrips_and_rejects_other_versions() {
    let m = IntegrityModel::permissive();
    let json = m.to_json();
    assert!(IntegrityModel::from_json(&json).is_some());
    let bumped = json.replace("\"version\": 1", "\"version\": 99");
    assert!(IntegrityModel::from_json(&bumped).is_none());
}
