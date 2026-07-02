//! Collector for [`CallReceiverEvidence`] payloads.
//!
//! Port of `engine/argot/scoring/evidence/call_receiver.py`. Cluster-scoped:
//! the `common here:` slice and the rarity denominator come from the hunk
//! file's MinHash cluster. Singleton / unknown clusters fall back to a
//! repo-empty framing rather than printing a wrong cluster's data.

use super::types::{CallReceiverEvidence, CommonEntry, EvidenceCorpus, RarityStat};

const COMMON_HERE_LIMIT: usize = 10;
const CALLEE_NOUN: &str = "callees";
const CLUSTER_WHERE: &str = "this cluster";
const REPO_WHERE: &str = "repo";

/// Build [`CallReceiverEvidence`] scoped to the hunk's cluster. When
/// `cluster_id` is `None` (or absent from the corpus), `common_here` is empty
/// and the denominator is 0, so the formatter prints "never seen in repo"
/// rather than a misleading cluster denominator.
pub fn collect_call_receiver_evidence(
    unattested_callees: Vec<String>,
    cluster_id: Option<usize>,
    evidence_corpus: &EvidenceCorpus,
) -> CallReceiverEvidence {
    let (common_here, denom, where_) = match cluster_id {
        Some(cid) if evidence_corpus.callees_by_cluster.contains_key(&cid) => {
            let common: Vec<CommonEntry> = evidence_corpus.callees_by_cluster[&cid]
                .iter()
                .take(COMMON_HERE_LIMIT)
                .cloned()
                .collect();
            let denom = evidence_corpus
                .totals
                .callees_attested_by_cluster
                .get(&cid)
                .copied()
                .unwrap_or(0);
            (common, denom, CLUSTER_WHERE)
        }
        _ => (Vec::new(), 0, REPO_WHERE),
    };
    CallReceiverEvidence {
        unfamiliar_callees: unattested_callees,
        rarity: RarityStat {
            flagged_count: 0,
            attested_total: denom,
            noun: CALLEE_NOUN.to_string(),
            where_: where_.to_string(),
        },
        common_here,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::evidence::types::EvidenceCorpusTotals;
    use std::collections::HashMap;

    fn corpus_with_cluster(cid: usize, callees: &[&str], attested_total: i64) -> EvidenceCorpus {
        let mut callees_by_cluster = HashMap::new();
        callees_by_cluster.insert(
            cid,
            callees
                .iter()
                .map(|name| CommonEntry {
                    name: name.to_string(),
                    count: 1,
                })
                .collect(),
        );
        let mut callees_attested_by_cluster = HashMap::new();
        callees_attested_by_cluster.insert(cid, attested_total);
        EvidenceCorpus {
            imports: Vec::new(),
            identifiers: HashMap::new(),
            callees_by_cluster,
            totals: EvidenceCorpusTotals {
                import_specifiers_attested: 0,
                callees_attested_by_cluster,
            },
        }
    }

    #[test]
    fn known_cluster_scopes_common_here_and_denominator_to_that_cluster() {
        let corpus = corpus_with_cluster(3, &["save", "load"], 42);
        let evidence =
            collect_call_receiver_evidence(vec!["mystery_call".to_string()], Some(3), &corpus);

        assert_eq!(
            evidence.unfamiliar_callees,
            vec!["mystery_call".to_string()]
        );
        assert_eq!(
            evidence.common_here,
            vec![
                CommonEntry {
                    name: "save".to_string(),
                    count: 1
                },
                CommonEntry {
                    name: "load".to_string(),
                    count: 1
                },
            ]
        );
        assert_eq!(evidence.rarity.attested_total, 42);
        assert_eq!(evidence.rarity.where_, "this cluster");
        assert_eq!(evidence.rarity.noun, "callees");
        assert_eq!(evidence.rarity.flagged_count, 0);
    }

    #[test]
    fn common_here_is_capped_at_ten_entries() {
        let members: Vec<&str> = vec![
            "c0", "c1", "c2", "c3", "c4", "c5", "c6", "c7", "c8", "c9", "c10", "c11",
        ];
        let corpus = corpus_with_cluster(1, &members, 100);
        let evidence = collect_call_receiver_evidence(vec![], Some(1), &corpus);
        assert_eq!(evidence.common_here.len(), COMMON_HERE_LIMIT);
        assert_eq!(evidence.common_here.first().unwrap().name, "c0");
        assert_eq!(evidence.common_here.last().unwrap().name, "c9");
    }

    #[test]
    fn no_cluster_id_falls_back_to_an_empty_repo_scoped_view() {
        let corpus = corpus_with_cluster(3, &["save", "load"], 42);
        let evidence = collect_call_receiver_evidence(vec!["x".to_string()], None, &corpus);

        assert!(evidence.common_here.is_empty());
        assert_eq!(evidence.rarity.attested_total, 0);
        assert_eq!(evidence.rarity.where_, "repo");
    }

    #[test]
    fn unknown_cluster_id_falls_back_the_same_way_as_no_cluster_id() {
        let corpus = corpus_with_cluster(3, &["save", "load"], 42);
        // Cluster 999 isn't in the corpus at all — must not leak cluster 3's data.
        let evidence = collect_call_receiver_evidence(vec![], Some(999), &corpus);

        assert!(evidence.common_here.is_empty());
        assert_eq!(evidence.rarity.attested_total, 0);
        assert_eq!(evidence.rarity.where_, "repo");
    }

    #[test]
    fn unattested_callees_pass_through_unchanged_including_order_and_duplicates() {
        let corpus = corpus_with_cluster(1, &[], 0);
        let input = vec!["b".to_string(), "a".to_string(), "b".to_string()];
        let evidence = collect_call_receiver_evidence(input.clone(), Some(1), &corpus);
        assert_eq!(evidence.unfamiliar_callees, input);
    }
}
