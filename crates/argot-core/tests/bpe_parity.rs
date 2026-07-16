//! BPE encode regression test: the golden token-id encodes pin the expected
//! output of the embedded `microsoft/unixcoder-base` tokenizer.
//!
//! Golden set spans real source files (Python + TypeScript), unicode, and
//! whitespace edge cases — ~14.5k token ids total.

use argot_core::bpe::BpeTokenizer;
use serde::Deserialize;

#[derive(Deserialize)]
struct Sample {
    text: String,
    ids: Vec<u32>,
}

#[test]
fn encode_matches_python_golden() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/bpe/golden_encodes.json");
    let raw = std::fs::read_to_string(path).expect("read golden");
    let samples: Vec<Sample> = serde_json::from_str(&raw).expect("parse golden");

    let tok = BpeTokenizer::load();
    let mut total = 0usize;
    for (i, s) in samples.iter().enumerate() {
        let got = tok.encode(&s.text);
        assert_eq!(
            got,
            s.ids,
            "sample {i} diverges (text starts {:?})",
            &s.text.chars().take(40).collect::<String>()
        );
        total += s.ids.len();
    }
    assert!(
        total > 10_000,
        "expected a broad golden set, got {total} ids"
    );
}
