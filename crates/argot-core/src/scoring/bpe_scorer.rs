//! BPE token-surprise scoring — the BPE half of `SequentialImportBpeScorer`.
//!
//! For a hunk, the score is the maximum per-token log-likelihood ratio of the
//! generic baseline vs. the repo corpus, over "meaningful" tokens. Faithful
//! port of `_token_surprise` / `_bpe_score` / `_is_meaningful_token`.

use crate::bpe::BpeTokenizer;
use crate::scoring::model::BpeStats;
use anyhow::{Context, Result};
use serde::Deserialize;
use std::collections::HashMap;

const EPSILON: f64 = 1e-7;

#[derive(Deserialize)]
struct GenericBaseline {
    token_counts: HashMap<String, u64>,
    total_tokens: u64,
}

/// `_is_meaningful_token`: at least 3 characters and at least one
/// alphanumeric. Operates on the byte-level vocab string (e.g. `"Ġdef"`).
pub fn is_meaningful_token(token_str: &str) -> bool {
    token_str.chars().count() >= 3 && token_str.chars().any(|c| c.is_alphanumeric())
}

/// The BPE scorer: holds the generic baseline and repo-corpus token
/// distributions plus the tokenizer, and scores hunks by max token surprise.
pub struct BpeScorer {
    tokenizer: BpeTokenizer,
    id_to_token: HashMap<u32, String>,
    generic_baseline: HashMap<u32, u64>,
    total_generic: f64,
    repo_corpus: HashMap<u32, u64>,
    total_repo: f64,
}

impl BpeScorer {
    /// Build from the generic baseline JSON and the repo-corpus source texts
    /// (already read with Python `read_text` semantics).
    pub fn new(
        tokenizer: BpeTokenizer,
        generic_baseline_json: &[u8],
        repo_sources: &[String],
    ) -> Result<Self> {
        let raw: GenericBaseline =
            serde_json::from_slice(generic_baseline_json).context("parse generic baseline json")?;
        let mut generic_baseline = HashMap::with_capacity(raw.token_counts.len());
        for (k, v) in &raw.token_counts {
            let id: u32 = k.parse().context("generic baseline token id")?;
            generic_baseline.insert(id, *v);
        }
        let total_generic = raw.total_tokens as f64;

        let id_to_token: HashMap<u32, String> =
            tokenizer.vocab().into_iter().map(|(k, v)| (v, k)).collect();

        let mut repo_corpus: HashMap<u32, u64> = HashMap::new();
        for src in repo_sources {
            for id in tokenizer.encode(src) {
                *repo_corpus.entry(id).or_insert(0) += 1;
            }
        }
        let total: u64 = repo_corpus.values().sum();
        // avoid division by zero — matches `sum(counts.values()) or 1`.
        let total_repo = if total == 0 { 1.0 } else { total as f64 };

        Ok(Self {
            tokenizer,
            id_to_token,
            generic_baseline,
            total_generic,
            repo_corpus,
            total_repo,
        })
    }

    /// Build from a fit-time [`BpeStats`] snapshot instead of re-tokenizing
    /// the corpus — the check-time path. The snapshot pins the repo token
    /// distribution to what it was at fit time, so new code cannot dilute its
    /// own surprise.
    pub fn from_stats(
        tokenizer: BpeTokenizer,
        generic_baseline_json: &[u8],
        stats: &BpeStats,
    ) -> Result<Self> {
        let mut scorer = Self::new(tokenizer, generic_baseline_json, &[])?;
        let mut repo_corpus = HashMap::with_capacity(stats.token_counts.len());
        for (k, v) in &stats.token_counts {
            let id: u32 = k.parse().context("model bpe token id")?;
            repo_corpus.insert(id, *v);
        }
        scorer.repo_corpus = repo_corpus;
        scorer.total_repo = if stats.total_tokens == 0 {
            1.0
        } else {
            stats.total_tokens as f64
        };
        Ok(scorer)
    }

    /// Export the fitted repo token distribution as a [`BpeStats`] snapshot.
    pub fn stats(&self) -> BpeStats {
        let token_counts = self
            .repo_corpus
            .iter()
            .map(|(id, count)| (id.to_string(), *count))
            .collect();
        BpeStats {
            token_counts,
            // Persist the raw corpus total; `from_stats` reapplies the
            // "or 1" guard, so a genuinely empty corpus roundtrips.
            total_tokens: self.repo_corpus.values().sum(),
        }
    }

    pub fn total_repo(&self) -> f64 {
        self.total_repo
    }
    pub fn total_generic(&self) -> f64 {
        self.total_generic
    }

    /// `_token_surprise`: log(p_generic + eps) − log(p_repo + eps).
    pub fn token_surprise(&self, token_id: u32) -> f64 {
        let g = *self.generic_baseline.get(&token_id).unwrap_or(&0) as f64;
        let r = *self.repo_corpus.get(&token_id).unwrap_or(&0) as f64;
        (g / self.total_generic + EPSILON).ln() - (r / self.total_repo + EPSILON).ln()
    }

    /// Token-id form of [`is_meaningful_token`] over the reverse vocab.
    pub fn is_meaningful_token_id(&self, token_id: u32) -> bool {
        match self.id_to_token.get(&token_id) {
            Some(s) => is_meaningful_token(s),
            None => false, // is_meaningful_token("") == false
        }
    }

    /// `_bpe_score`: max token surprise over meaningful tokens (falling back
    /// to all tokens if none are meaningful; 0.0 if empty).
    pub fn bpe_score(&self, hunk_source: &str) -> f64 {
        let ids = self.tokenizer.encode(hunk_source);
        let filtered: Vec<u32> = ids
            .iter()
            .copied()
            .filter(|&i| self.is_meaningful_token_id(i))
            .collect();
        let use_ids: &[u32] = if filtered.is_empty() { &ids } else { &filtered };
        if use_ids.is_empty() {
            return 0.0;
        }
        use_ids
            .iter()
            .map(|&i| self.token_surprise(i))
            .fold(f64::NEG_INFINITY, f64::max)
    }

    /// Access the underlying tokenizer (evidence collectors need it).
    pub fn tokenizer(&self) -> &BpeTokenizer {
        &self.tokenizer
    }

    /// The BPE stage's reachable ceiling for this repo: the maximum surprise
    /// any single meaningful token can score. A hunk's BPE score is a max
    /// over its tokens, so no hunk can exceed this — comparing it against the
    /// calibrated threshold tells whether phrasing detection (as opposed to
    /// the import tripwire) can fire at all.
    ///
    /// The max is taken over the generic baseline's meaningful tokens that
    /// are absent from the repo corpus (repo-seen tokens only lower the
    /// surprise), falling back to all meaningful tokens when the corpus has
    /// seen everything.
    pub fn max_token_surprise_ceiling(&self) -> f64 {
        let mut best = f64::NEG_INFINITY;
        let mut best_any = f64::NEG_INFINITY;
        for id in self.generic_baseline.keys() {
            if !self.is_meaningful_token_id(*id) {
                continue;
            }
            let s = self.token_surprise(*id);
            best_any = best_any.max(s);
            if !self.repo_corpus.contains_key(id) {
                best = best.max(s);
            }
        }
        let ceiling = if best.is_finite() { best } else { best_any };
        if ceiling.is_finite() {
            ceiling
        } else {
            0.0
        }
    }
}
