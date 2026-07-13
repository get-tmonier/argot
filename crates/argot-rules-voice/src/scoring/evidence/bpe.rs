//! Collector for [`BpeEvidence`] payloads.
//!
//! Port of `engine/argot/scoring/evidence/bpe.py`. Reconstructs the offending
//! identifiers from the surprising token spans and pairs each with its
//! repo-wide attestation count so the rendered line is self-explanatory.

use super::bpe_reconstruction::surprising_identifiers;
use super::types::{BpeEvidence, CommonEntry, EvidenceCorpus};
use argot_lang::bpe::BpeTokenizer;

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
    let mut entries: Vec<CommonEntry> = names
        .into_iter()
        .map(|n| {
            let count = counts.get(&n).copied().unwrap_or(0);
            CommonEntry { name: n, count }
        })
        .collect();
    // Lead with the genuinely rare names. Reconstruction orders by BPE surprise,
    // which correlates with rarity but not perfectly — a common word (`not`,
    // 285×) can top the list because its token was surprising *in this
    // sequence*, reading as a contradiction under a "rare tokens" heading. A
    // stable sort by ascending attestation puts the novel `(0×)` names first (so
    // the formatter's top-3 shows them) while preserving surprise order within a
    // tier.
    entries.sort_by_key(|e| e.count);
    BpeEvidence {
        surprising_identifiers: entries,
    }
}

#[cfg(test)]
mod tests;
