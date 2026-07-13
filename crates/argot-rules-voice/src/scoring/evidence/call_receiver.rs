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
mod tests;
