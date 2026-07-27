//! Collector for [`BpeEvidence`] payloads.
//!
//! Reconstructs the offending identifiers from the surprising token spans and
//! pairs each with its repo-wide attestation count so the rendered line is
//! self-explanatory.

use std::collections::HashSet;

use super::bpe_reconstruction::surprising_identifiers;
use super::types::{BpeEvidence, CommonEntry, EvidenceCorpus};
use argot_lang::adapters::LanguageAdapter;
use argot_lang::bpe::BpeTokenizer;

// Generous cap at the collector layer; the formatter applies the user-visible
// top-3 + (+N more) truncation.
const MAX_SURPRISING_PIECES: usize = 8;
const MAX_IDENTIFIERS: usize = 8;

/// Build the [`BpeEvidence`] payload for a BPE-fired hit. Each surprising
/// identifier is paired with its repo-wide attestation count; tokens absent
/// from the map render as `count=0` (a genuinely novel identifier reads as
/// `proposed (0×)` rather than being dropped — the zero is the signal).
///
/// The language's noise words are dropped on the way in, exactly as the corpus
/// counts dropped them on the way out. Filtering one side only manufactures the
/// zero: a keyword the corpus never counted renders `(0×)` and the rarest-first
/// sort then leads the evidence with it (`Self (0×)`, `DIV (3×)` on Object
/// Pascal, where they are a keyword and an operator).
pub fn collect_bpe_evidence(
    hunk_source: &str,
    tokenizer: &BpeTokenizer,
    score_fn: &dyn Fn(u32) -> f64,
    is_meaningful: Option<&dyn Fn(u32) -> bool>,
    evidence_corpus: &EvidenceCorpus,
    adapter: &dyn LanguageAdapter,
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
    let noise = adapter.identifier_noise();
    let mut entries: Vec<CommonEntry> = names
        .into_iter()
        .filter(|n| !is_noise(n, noise, adapter))
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

/// Whether `ident` is one of the language's noise words, honouring an
/// identifier casing that carries no meaning (Object Pascal's `DIV` is `div`).
pub(crate) fn is_noise(
    ident: &str,
    noise: &HashSet<String>,
    adapter: &dyn LanguageAdapter,
) -> bool {
    noise.contains(ident)
        || (adapter.identifiers_are_case_insensitive() && noise.contains(&ident.to_lowercase()))
}

#[cfg(test)]
mod tests;
