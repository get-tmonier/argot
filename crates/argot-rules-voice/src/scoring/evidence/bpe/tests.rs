use super::*;
use crate::scoring::evidence::types::EvidenceCorpusTotals;
use argot_lang::adapters::{adapter_for, LanguageAdapter};
use argot_lang::bpe::BpeTokenizer;
use std::collections::HashMap;

fn py() -> Box<dyn LanguageAdapter> {
    adapter_for("python").unwrap()
}

fn corpus_with_identifiers(identifiers: &[(&str, i64)]) -> EvidenceCorpus {
    EvidenceCorpus {
        imports: Vec::new(),
        identifiers: identifiers
            .iter()
            .map(|(name, count)| (name.to_string(), *count))
            .collect(),
        callees_by_cluster: HashMap::new(),
        totals: EvidenceCorpusTotals {
            import_specifiers_attested: 0,
            callees_attested_by_cluster: HashMap::new(),
        },
    }
}

#[test]
fn collect_bpe_evidence_pairs_identifiers_with_corpus_counts_defaulting_to_zero() {
    let tok = BpeTokenizer::load();
    let source = "mongoose(url)";
    // Uniform score: both identifiers are within the top-8 cap, so
    // reconstruction and count-pairing are the only things under test.
    let corpus = corpus_with_identifiers(&[("mongoose", 5)]);
    let evidence = collect_bpe_evidence(source, &tok, &|_id| 0.0, None, &corpus, py().as_ref());
    assert_eq!(
        evidence.surprising_identifiers,
        vec![
            // Rarest-first: the novel `url (0×)` leads the attested
            // `mongoose (5×)`, though reconstruction saw mongoose first.
            CommonEntry {
                name: "url".to_string(),
                count: 0
            },
            CommonEntry {
                name: "mongoose".to_string(),
                count: 5
            },
        ],
        "an identifier absent from the corpus renders as count=0, not dropped"
    );
}

#[test]
fn collect_bpe_evidence_is_empty_when_no_token_passes_the_meaningfulness_filter() {
    let tok = BpeTokenizer::load();
    let corpus = corpus_with_identifiers(&[]);
    let evidence = collect_bpe_evidence(
        "mongoose(url)",
        &tok,
        &|_id| 0.0,
        Some(&|_id| false),
        &corpus,
        py().as_ref(),
    );
    assert!(evidence.surprising_identifiers.is_empty());
}

#[test]
fn collect_bpe_evidence_is_empty_for_an_empty_hunk() {
    let tok = BpeTokenizer::load();
    let corpus = corpus_with_identifiers(&[]);
    let evidence = collect_bpe_evidence("", &tok, &|_id| 0.0, None, &corpus, py().as_ref());
    assert!(evidence.surprising_identifiers.is_empty());
}

#[test]
fn collect_bpe_evidence_keeps_only_the_top_scoring_pieces_up_to_the_cap() {
    let tok = BpeTokenizer::load();
    // 10 single-letter identifiers; score them by descending source
    // position so the two lowest-scored ("i", "j") must be dropped once
    // the collector's internal 8-piece cap kicks in.
    let source = "a b c d e f g h i j";
    let ids = tok.encode(source);
    let scores: HashMap<u32, f64> = ids
        .iter()
        .enumerate()
        .map(|(idx, &id)| (id, (ids.len() - idx) as f64))
        .collect();
    let score_fn = move |id: u32| *scores.get(&id).unwrap_or(&0.0);

    let corpus = corpus_with_identifiers(&[]);
    let evidence = collect_bpe_evidence(source, &tok, &score_fn, None, &corpus, py().as_ref());
    let names: Vec<String> = evidence
        .surprising_identifiers
        .iter()
        .map(|e| e.name.clone())
        .collect();
    assert_eq!(
        names,
        vec!["a", "b", "c", "d", "e", "f", "g", "h"],
        "the two lowest-scored identifiers must not survive the cap"
    );
}

#[test]
fn a_language_keyword_is_not_evidence_of_rarity() {
    // The corpus counts drop the language's noise words, so leaving them in on
    // the hunk side manufactures a zero: the keyword renders `(0×)` and the
    // rarest-first sort leads the evidence with it. Object Pascal showed both
    // halves — `Self (0×)` (a keyword the corpus never counted) and `DIV (3×)`
    // (an operator the lowercase noise list did not match).
    let tok = BpeTokenizer::load();
    let pascal = adapter_for("pascal").unwrap();
    let corpus = corpus_with_identifiers(&[("count", 6)]);
    let evidence = collect_bpe_evidence(
        "if Self.count DIV 2 > 0 then thestrext(x);",
        &tok,
        &|_id| 0.0,
        None,
        &corpus,
        pascal.as_ref(),
    );
    let names: Vec<&str> = evidence
        .surprising_identifiers
        .iter()
        .map(|e| e.name.as_str())
        .collect();
    assert!(!names.contains(&"Self"), "{names:?}");
    assert!(!names.contains(&"DIV"), "{names:?}");
    assert!(
        names.contains(&"count"),
        "a real identifier survives: {names:?}"
    );
}
