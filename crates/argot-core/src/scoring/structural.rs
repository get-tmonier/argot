//! Structural-foreignness sense — the shape analog of the foreign-vocabulary
//! gate. Feature-gated (`--features structural`), advisory / measurement-only:
//! it is **not** wired into the base gating path, so the shipped guardrail is
//! byte-for-byte unchanged with or without it.
//!
//! # What it is
//!
//! argot's base gate flags foreign **vocabulary** (an import/callee 0-usage in
//! the repo). This sense is the analog on **shape**: a repo's node-kind
//! parent→child bigrams are its *structural vocabulary*; a hunk is
//! structurally foreign to the extent it uses bigrams the repo has never
//! written. Domain-blind (tree-sitter node **kinds** only — never identifiers,
//! strings, or framework literals) and language-agnostic (any grammar argot
//! parses), matching `shape_primitive.rs`'s design and the no-hardcoded-domain
//! rule.
//!
//! # Why it is not a gate
//!
//! The research sweep (`docs/research/evidence/foreign-structure-gate-floor.md`)
//! proved an irreducible floor: at an over-fire budget ≤5% on every corpus the
//! recall of pasted foreign idioms tops out at ~8–13%, because (1) only ~13% of
//! foreign code is structurally distinct — the rest reuses the repo's own
//! node-kind bigrams — and (2) over-fire is corpus-size-dependent (a young repo
//! hasn't saturated its structural vocabulary). So this ships as a *measured*
//! sense, not a gate; the module exposes the fit + fire primitives the bench
//! drives to validate that floor on real corpora.
//!
//! # Fire rule (the portable analog of "one foreign import fires")
//!
//! `FIRE(hunk) := #{ bigrams 0-usage in the repo AND globally common
//! (bg_df ≥ τ) } ≥ k`. The background prior `bg_df` (fraction of a diverse repo
//! set that uses a bigram) is **injected**, not hardcoded — the bench supplies a
//! leave-one-out prior; a production build would embed a precomputed table
//! (mirroring the embedded BPE baseline). A 0-usage bigram that is globally
//! common is real foreign structure; a 0-usage bigram that is globally rare is
//! combinatorial noise — the IDF/rarity lever that made the signal separable.

use std::collections::{HashMap, HashSet};

use crate::scoring::adapters::Language;
use crate::scoring::ts_parse;

/// A parent→child node-kind bigram, the atom of the structural vocabulary.
/// Encoded as `"{parent}\u{1f}{child}"` so it is a cheap hashable key shared by
/// the vocab, the prior, and hunk extraction.
pub type Bigram = String;

fn bigram_key(parent: &str, child: &str) -> Bigram {
    let mut s = String::with_capacity(parent.len() + child.len() + 1);
    s.push_str(parent);
    s.push('\u{1f}');
    s.push_str(child);
    s
}

/// Domain-blind parent→child node-kind bigrams over the **named** nodes of
/// `source` (anonymous punctuation/keyword nodes are skipped — they are
/// syntactic noise, not structure). Returns every occurrence (with repeats);
/// callers dedup as needed. Empty on parse failure — a graceful no-op.
pub fn extract_bigrams(source: &str, language: Language) -> Vec<Bigram> {
    let Some(tree) = ts_parse::parse(source, language) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    // Explicit stack (matches typicality.rs) to avoid deep recursion on large
    // files. `kind()` is &'static str, so carrying the parent kind is free.
    let mut stack: Vec<(tree_sitter::Node, Option<&'static str>)> = vec![(tree.root_node(), None)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if let Some(pk) = parent {
            out.push(bigram_key(pk, kind));
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            stack.push((child, Some(kind)));
        }
    }
    out
}

/// The repo's attested structural vocabulary — the set of bigrams it has ever
/// written. Attestation floor is one occurrence (`df ≥ 1`): the sweep found the
/// widest vocab minimizes native self-novelty (the over-fire driver).
#[derive(Debug, Clone, Default)]
pub struct StructuralVocab {
    attested: HashSet<Bigram>,
}

impl StructuralVocab {
    /// Fit over an iterator of source strings (the repo's files at the pinned
    /// SHA). Language is per-file so a monorepo fits one vocab per language.
    pub fn fit<'a, I>(files: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, Language)>,
    {
        let mut attested = HashSet::new();
        for (source, language) in files {
            for bg in extract_bigrams(source, language) {
                attested.insert(bg);
            }
        }
        Self { attested }
    }

    pub fn contains(&self, bigram: &str) -> bool {
        self.attested.contains(bigram)
    }

    /// The attested bigram set — used to build a cross-repo background prior.
    pub fn attested(&self) -> &HashSet<Bigram> {
        &self.attested
    }

    pub fn len(&self) -> usize {
        self.attested.len()
    }

    pub fn is_empty(&self) -> bool {
        self.attested.is_empty()
    }
}

/// Background structural prior: bigram → fraction of a diverse repo set that
/// uses it (document-frequency in `[0, 1]`). Injected, never hardcoded.
pub type StructuralPrior = HashMap<Bigram, f64>;

/// How structurally foreign a hunk reads against a repo's vocabulary. The
/// primitive the fire rule and the bench both consume.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct HunkForeignness {
    /// Distinct bigrams in the hunk that are 0-usage in the repo AND globally
    /// common (`bg_df ≥ τ`) — the "conspicuously-avoided idiom" count.
    pub foreign_common: usize,
    /// The loudest such bigram's `bg_df` (0.0 if none) — how globally-idiomatic
    /// the hunk's most-foreign pattern is.
    pub loudness: f64,
    /// Total distinct bigrams in the hunk (denominator for a rate view).
    pub distinct: usize,
}

/// Score a hunk: count its distinct native-absent, globally-common bigrams.
pub fn hunk_foreignness(
    hunk: &str,
    language: Language,
    vocab: &StructuralVocab,
    prior: &StructuralPrior,
    tau: f64,
) -> HunkForeignness {
    let mut seen = HashSet::new();
    let mut foreign_common = 0usize;
    let mut loudness = 0.0f64;
    for bg in extract_bigrams(hunk, language) {
        if !seen.insert(bg.clone()) {
            continue; // distinct only
        }
        if !vocab.contains(&bg) {
            let df = prior.get(&bg).copied().unwrap_or(0.0);
            if df >= tau {
                foreign_common += 1;
                if df > loudness {
                    loudness = df;
                }
            }
        }
    }
    HunkForeignness {
        foreign_common,
        loudness,
        distinct: seen.len(),
    }
}

/// The advisory fire decision: `≥ k` native-absent, globally-common bigrams.
/// Non-gating — the caller reports it, never blocks a commit on it.
pub fn fires(
    hunk: &str,
    language: Language,
    vocab: &StructuralVocab,
    prior: &StructuralPrior,
    tau: f64,
    k: usize,
) -> bool {
    hunk_foreignness(hunk, language, vocab, prior, tau).foreign_common >= k
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prior_of(pairs: &[(&str, &str, f64)]) -> StructuralPrior {
        pairs
            .iter()
            .map(|(p, c, d)| (bigram_key(p, c), *d))
            .collect()
    }

    #[test]
    fn extract_is_domain_blind_and_nonempty() {
        // Two functions with totally different identifiers/strings but the same
        // shape must yield the same bigram multiset — proof it is domain-blind.
        let a = "def f(x):\n    return x + 1\n";
        let b = "def wildly_different_name(zzz):\n    return zzz + 1\n";
        let mut ba = extract_bigrams(a, Language::Python);
        let mut bb = extract_bigrams(b, Language::Python);
        ba.sort();
        bb.sort();
        assert_eq!(
            ba, bb,
            "identical shape, different names → identical bigrams"
        );
        assert!(!ba.is_empty());
    }

    #[test]
    fn unparseable_is_graceful_noop() {
        // Empty input has no named children → no bigrams. Malformed input is
        // error-tolerant in tree-sitter (it yields ERROR-node bigrams), so the
        // guarantee is "never panics", not "empty".
        assert!(extract_bigrams("", Language::Python).is_empty());
        let _ = extract_bigrams("(((", Language::Python); // must not panic
    }

    #[test]
    fn vocab_attests_what_the_repo_wrote() {
        let vocab = StructuralVocab::fit([("def f():\n    return 1\n", Language::Python)]);
        assert!(!vocab.is_empty());
        // A bigram present in the fitted source is attested.
        let bgs = extract_bigrams("def g():\n    return 2\n", Language::Python);
        assert!(
            bgs.iter().all(|b| vocab.contains(b)),
            "same shape is attested"
        );
    }

    #[test]
    fn fires_only_on_native_absent_globally_common_bigram() {
        // Repo only writes simple return-constant functions.
        let vocab = StructuralVocab::fit([("def f():\n    return 1\n", Language::Python)]);
        // A hunk introducing a while-loop: `function_definition→while_statement`
        // and `while_statement→...` are 0-usage in the repo.
        let hunk = "def g():\n    while True:\n        break\n";
        let foreign_bg = {
            // find a bigram in the hunk that the vocab lacks, to build the prior
            let hb = extract_bigrams(hunk, Language::Python);
            hb.into_iter()
                .find(|b| !vocab.contains(b))
                .expect("a novel bigram exists")
        };
        // Prior says that novel bigram is globally common.
        let mut prior = StructuralPrior::new();
        prior.insert(foreign_bg, 0.9);
        let f = hunk_foreignness(hunk, Language::Python, &vocab, &prior, 0.5);
        assert!(f.foreign_common >= 1, "novel globally-common bigram counts");
        assert!(f.loudness >= 0.9 - 1e-9);
        assert!(fires(hunk, Language::Python, &vocab, &prior, 0.5, 1));

        // A hunk of the repo's own shape must not fire (no native-absent bigram).
        let native = "def h():\n    return 7\n";
        assert!(!fires(native, Language::Python, &vocab, &prior, 0.5, 1));
    }

    #[test]
    fn rare_novel_bigram_does_not_fire_below_tau() {
        let vocab = StructuralVocab::fit([("def f():\n    return 1\n", Language::Python)]);
        let hunk = "def g():\n    while True:\n        break\n";
        // Prior marks the novel bigrams as globally RARE (bg_df 0.1) → below τ.
        let novel: Vec<_> = extract_bigrams(hunk, Language::Python)
            .into_iter()
            .filter(|b| !vocab.contains(b))
            .collect();
        let prior = novel.iter().map(|b| (b.clone(), 0.1)).collect();
        assert!(
            !fires(hunk, Language::Python, &vocab, &prior, 0.5, 1),
            "globally-rare novelty is combinatorial noise, not foreign structure"
        );
    }

    #[test]
    fn language_agnostic_smoke() {
        // The same primitive works on a non-Python grammar with no special-casing.
        let bgs = extract_bigrams("fn main() { let x = 1; }", Language::Rust);
        assert!(!bgs.is_empty());
        let _ = prior_of(&[("function_item", "block", 0.5)]);
    }
}
