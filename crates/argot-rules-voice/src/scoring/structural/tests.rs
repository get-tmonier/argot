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
