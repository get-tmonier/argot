use super::*;

#[test]
fn empty_and_whitespace_are_neutral() {
    let model = TypicalityModel::new(Language::Python);
    for src in ["", "   \n\t  \n"] {
        let (atyp, features) = model.is_atypical(src);
        assert!(!atyp);
        assert_eq!(features, NEUTRAL);
    }
}

#[test]
fn code_hunk_is_not_atypical() {
    let model = TypicalityModel::new(Language::Python);
    let (atyp, features) = model.is_atypical("def f(x):\n    return x + 1\n");
    assert!(!atyp);
    assert!(features.named_leaf_count > 0);
}

#[test]
fn small_data_table_trips_hunk_gate() {
    let model = TypicalityModel::new(Language::Python);
    let src = "CITIES = [\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\"]\n";
    let (hunk, features) = model.is_atypical(src);
    assert!(hunk);
    assert!(features.literal_leaf_ratio > LITERAL_RATIO_CUTOFF);
}
