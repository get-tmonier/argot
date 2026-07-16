//! Collector for [`ImportEvidence`] payloads.
//!
//! Names the foreign specifiers and shows the repo's typical top-level
//! imports, keeping the framing strictly factual.

use super::types::{CommonEntry, EvidenceCorpus, ImportEvidence, RarityStat, SourceSpan};

const COMMON_HERE_LIMIT: usize = 10;
const IMPORT_RARITY_NOUN: &str = "module specifiers";
const IMPORT_RARITY_WHERE: &str = "repo";

/// Build the [`ImportEvidence`] payload for an import-fired hit.
/// `foreign_specifiers` is the scorer's flagged set in hunk order;
/// `foreign_specifier_spans` is the insertion-ordered `(spec, span)` map used
/// for `(L7)` annotations and carets (omit a specifier to render it bare).
pub fn collect_import_evidence(
    foreign_specifiers: Vec<String>,
    foreign_specifier_spans: Vec<(String, SourceSpan)>,
    evidence_corpus: &EvidenceCorpus,
) -> ImportEvidence {
    let common_here: Vec<CommonEntry> = evidence_corpus
        .imports
        .iter()
        .take(COMMON_HERE_LIMIT)
        .cloned()
        .collect();
    ImportEvidence {
        foreign_specifiers,
        rarity: RarityStat {
            flagged_count: 0,
            attested_total: evidence_corpus.totals.import_specifiers_attested,
            noun: IMPORT_RARITY_NOUN.to_string(),
            where_: IMPORT_RARITY_WHERE.to_string(),
        },
        common_here,
        foreign_specifier_spans,
    }
}

#[cfg(test)]
mod tests;
