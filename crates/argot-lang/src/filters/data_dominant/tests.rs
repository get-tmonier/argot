use super::*;

#[test]
fn module_level_tables_are_data_dominant() {
    let src = "CITIES = [\"a\", \"b\"]\nNAMES = (\"x\", \"y\")\n";
    assert!(is_data_dominant(src, 0.65));
}

#[test]
fn class_body_tables_are_data_dominant() {
    let src = "class Provider:\n    cities = [\"a\", \"b\", \"c\"]\n    codes = {\"a\": 1}\n";
    assert!(is_data_dominant(src, 0.65));
}

#[test]
fn code_is_not_data_dominant() {
    let src = "def f(x):\n    return x + 1\n";
    assert!(!is_data_dominant(src, 0.65));
}

#[test]
fn empty_is_not_data_dominant() {
    assert!(!is_data_dominant("", 0.65));
    assert!(!is_data_dominant("   \n  \n", 0.65));
}
