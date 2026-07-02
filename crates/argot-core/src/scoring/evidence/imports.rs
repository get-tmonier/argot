//! Collector for [`ImportEvidence`] payloads.
//!
//! Port of `engine/argot/scoring/evidence/imports.py`. Names the foreign
//! specifiers and shows the repo's typical top-level imports, keeping the
//! framing strictly factual.

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
mod tests {
    use super::*;
    use crate::scoring::evidence::types::EvidenceCorpusTotals;
    use std::collections::HashMap;

    fn corpus_with_imports(imports: &[&str], attested_total: i64) -> EvidenceCorpus {
        EvidenceCorpus {
            imports: imports
                .iter()
                .map(|name| CommonEntry {
                    name: name.to_string(),
                    count: 1,
                })
                .collect(),
            identifiers: HashMap::new(),
            callees_by_cluster: HashMap::new(),
            totals: EvidenceCorpusTotals {
                import_specifiers_attested: attested_total,
                callees_attested_by_cluster: HashMap::new(),
            },
        }
    }

    #[test]
    fn foreign_specifiers_and_spans_pass_through_unchanged() {
        let corpus = corpus_with_imports(&[], 0);
        let specifiers = vec!["left-pad".to_string(), "requests".to_string()];
        let spans = vec![(
            "left-pad".to_string(),
            SourceSpan {
                line: 1,
                col_start: 7,
                col_end: 15,
            },
        )];
        let evidence = collect_import_evidence(specifiers.clone(), spans.clone(), &corpus);
        assert_eq!(evidence.foreign_specifiers, specifiers);
        assert_eq!(evidence.foreign_specifier_spans, spans);
    }

    #[test]
    fn common_here_is_taken_from_the_corpus_and_capped_at_ten() {
        let members: Vec<&str> = vec![
            "m0", "m1", "m2", "m3", "m4", "m5", "m6", "m7", "m8", "m9", "m10",
        ];
        let corpus = corpus_with_imports(&members, 500);
        let evidence = collect_import_evidence(vec![], vec![], &corpus);
        assert_eq!(evidence.common_here.len(), 10);
        assert_eq!(evidence.common_here.first().unwrap().name, "m0");
        assert_eq!(evidence.common_here.last().unwrap().name, "m9");
    }

    #[test]
    fn rarity_stat_reports_repo_scope_and_the_corpus_denominator() {
        let corpus = corpus_with_imports(&["os"], 123);
        let evidence = collect_import_evidence(vec![], vec![], &corpus);
        assert_eq!(evidence.rarity.attested_total, 123);
        assert_eq!(evidence.rarity.flagged_count, 0);
        assert_eq!(evidence.rarity.noun, "module specifiers");
        assert_eq!(evidence.rarity.where_, "repo");
        assert_eq!(evidence.rarity.scope_label(), "module specifiers in repo");
    }

    #[test]
    fn empty_corpus_yields_empty_common_here_and_zero_denominator() {
        let corpus = corpus_with_imports(&[], 0);
        let evidence = collect_import_evidence(vec!["requests".to_string()], vec![], &corpus);
        assert!(evidence.common_here.is_empty());
        assert_eq!(evidence.rarity.attested_total, 0);
    }
}
