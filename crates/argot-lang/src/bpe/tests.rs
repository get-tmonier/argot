use super::*;

#[test]
fn loads_and_reports_vocab_size() {
    let tok = BpeTokenizer::load();
    // Python reported vocab_size 51416 for microsoft/unixcoder-base.
    assert_eq!(tok.vocab().len(), 51416);
}

#[test]
fn empty_string_encodes_empty() {
    let tok = BpeTokenizer::load();
    assert!(tok.encode("").is_empty());
}

#[test]
fn matches_python_sample() {
    // Captured from Python: tok.encode("def foo(x):\n    return x + 1\n").
    let tok = BpeTokenizer::load();
    let ids = tok.encode("def foo(x):\n    return x + 1\n");
    assert_eq!(
        ids,
        vec![729, 5089, 126, 206, 953, 317, 377, 483, 868, 513, 524, 317]
    );
}
