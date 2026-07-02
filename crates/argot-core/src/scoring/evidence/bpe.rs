//! Collector for [`BpeEvidence`] payloads.
//!
//! Port of `engine/argot/scoring/evidence/bpe.py`. Reconstructs the offending
//! identifiers from the surprising token spans and pairs each with its
//! repo-wide attestation count so the rendered line is self-explanatory.

use super::bpe_reconstruction::surprising_identifiers;
use super::types::{BpeEvidence, CommonEntry, EvidenceCorpus};
use crate::bpe::BpeTokenizer;

// Generous cap at the collector layer; the formatter applies the user-visible
// top-3 + (+N more) truncation.
const MAX_SURPRISING_PIECES: usize = 8;
const MAX_IDENTIFIERS: usize = 8;

/// Build the [`BpeEvidence`] payload for a BPE-fired hit. Each surprising
/// identifier is paired with its repo-wide attestation count; tokens absent
/// from the map render as `count=0` (a genuinely novel identifier reads as
/// `proposed (0×)` rather than being dropped — the zero is the signal).
pub fn collect_bpe_evidence(
    hunk_source: &str,
    tokenizer: &BpeTokenizer,
    score_fn: &dyn Fn(u32) -> f64,
    is_meaningful: Option<&dyn Fn(u32) -> bool>,
    evidence_corpus: &EvidenceCorpus,
) -> BpeEvidence {
    let names = surprising_identifiers(
        hunk_source,
        tokenizer,
        score_fn,
        MAX_SURPRISING_PIECES,
        MAX_IDENTIFIERS,
        is_meaningful,
    );
    let counts = &evidence_corpus.identifiers;
    BpeEvidence {
        surprising_identifiers: names
            .into_iter()
            .map(|n| {
                let count = counts.get(&n).copied().unwrap_or(0);
                CommonEntry { name: n, count }
            })
            .collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bpe::BpeTokenizer;
    use crate::scoring::evidence::types::EvidenceCorpusTotals;
    use std::collections::HashMap;

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
        let evidence = collect_bpe_evidence(source, &tok, &|_id| 0.0, None, &corpus);
        assert_eq!(
            evidence.surprising_identifiers,
            vec![
                CommonEntry {
                    name: "mongoose".to_string(),
                    count: 5
                },
                CommonEntry {
                    name: "url".to_string(),
                    count: 0
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
        );
        assert!(evidence.surprising_identifiers.is_empty());
    }

    #[test]
    fn collect_bpe_evidence_is_empty_for_an_empty_hunk() {
        let tok = BpeTokenizer::load();
        let corpus = corpus_with_identifiers(&[]);
        let evidence = collect_bpe_evidence("", &tok, &|_id| 0.0, None, &corpus);
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
        let evidence = collect_bpe_evidence(source, &tok, &score_fn, None, &corpus);
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
}
