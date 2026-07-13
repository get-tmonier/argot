use super::*;
use crate::scoring::model::BpeStats;
use argot_lang::bpe::BpeTokenizer;

/// Build a scorer with an explicit generic baseline and repo-corpus
/// snapshot, bypassing corpus re-tokenization so each test can pin the
/// exact counts a given token id sees on either side.
fn scorer(
    generic_counts: &[(u32, u64)],
    total_generic: u64,
    repo_counts: &[(u32, u64)],
    total_repo: u64,
) -> BpeScorer {
    let baseline = serde_json::json!({
        "token_counts": generic_counts
            .iter()
            .map(|(id, c)| (id.to_string(), *c))
            .collect::<HashMap<_, _>>(),
        "total_tokens": total_generic,
    });
    let baseline_bytes = serde_json::to_vec(&baseline).unwrap();
    let stats = BpeStats {
        token_counts: repo_counts
            .iter()
            .map(|(id, c)| (id.to_string(), *c))
            .collect(),
        total_tokens: total_repo,
    };
    BpeScorer::from_stats(BpeTokenizer::load(), &baseline_bytes, &stats)
        .expect("well-formed baseline + stats")
}

#[test]
fn is_meaningful_token_requires_three_chars_and_an_alnum() {
    assert!(!is_meaningful_token(":"), "too short");
    assert!(!is_meaningful_token("=="), "no alnum, still too short");
    assert!(!is_meaningful_token("..."), "long enough but no alnum");
    assert!(is_meaningful_token("def"), "3 alnum chars");
    assert!(is_meaningful_token("Ġdef"), "leading-space marker + alnum");
}

#[test]
fn token_surprise_is_positive_when_generic_common_but_repo_unseen() {
    let tok = BpeTokenizer::load();
    let id = tok.encode("mongoose")[0];
    let s = scorer(&[(id, 1_000)], 1_000_000, &[], 0);
    assert!(s.token_surprise(id) > 0.0);
}

#[test]
fn token_surprise_is_negative_when_repo_common_but_generic_rare() {
    let tok = BpeTokenizer::load();
    let id = tok.encode("mongoose")[0];
    let s = scorer(&[], 1_000_000, &[(id, 1_000)], 1_000_000);
    assert!(s.token_surprise(id) < 0.0);
}

#[test]
fn token_surprise_is_zero_for_a_token_unseen_in_both_corpora() {
    let tok = BpeTokenizer::load();
    let id = tok.encode("mongoose")[0];
    // Neither side has seen `id`: both fractions collapse to the same
    // epsilon, so the log-ratio cancels exactly.
    let s = scorer(&[], 1_000, &[], 1_000);
    assert_eq!(s.token_surprise(id), 0.0);
}

#[test]
fn is_meaningful_token_id_matches_the_free_function_over_the_real_vocab() {
    let tok = BpeTokenizer::load();
    let meaningful_id = tok.encode("mongoose")[0]; // "mongo"
    let non_meaningful_id = tok.encode("+=")[0]; // "+="
    let s = scorer(&[], 1, &[], 0);
    assert!(s.is_meaningful_token_id(meaningful_id));
    assert!(!s.is_meaningful_token_id(non_meaningful_id));
}

#[test]
fn is_meaningful_token_id_is_false_for_an_id_outside_the_vocab() {
    let s = scorer(&[], 1, &[], 0);
    assert!(!s.is_meaningful_token_id(u32::MAX));
}

#[test]
fn bpe_score_is_zero_for_an_empty_hunk() {
    let s = scorer(&[], 1, &[], 0);
    assert_eq!(s.bpe_score(""), 0.0);
}

#[test]
fn bpe_score_ignores_non_meaningful_tokens_when_meaningful_ones_exist() {
    let tok = BpeTokenizer::load();
    let source = "identifier = 1";
    let ids = tok.encode(source);
    let (meaningful, non_meaningful): (Vec<u32>, Vec<u32>) = ids.iter().copied().partition(|&id| {
        tok.id_to_token(id)
            .map(|piece| is_meaningful_token(&piece))
            .unwrap_or(false)
    });
    assert!(
        !meaningful.is_empty(),
        "fixture must contain \"identifier\""
    );
    assert!(
        !non_meaningful.is_empty(),
        "fixture must contain \"=\"/\"1\""
    );

    // Make the non-meaningful tokens look far more surprising than the
    // meaningful ones, so a correct implementation must still ignore them.
    let mut generic_counts: Vec<(u32, u64)> =
        non_meaningful.iter().map(|&id| (id, 1_000_000)).collect();
    generic_counts.extend(meaningful.iter().map(|&id| (id, 1)));
    let s = scorer(&generic_counts, 1_000_000, &[], 0);

    let meaningful_max = meaningful
        .iter()
        .map(|&id| s.token_surprise(id))
        .fold(f64::NEG_INFINITY, f64::max);
    let non_meaningful_max = non_meaningful
        .iter()
        .map(|&id| s.token_surprise(id))
        .fold(f64::NEG_INFINITY, f64::max);

    let score = s.bpe_score(source);
    assert!((score - meaningful_max).abs() < 1e-9);
    assert!(
        score < non_meaningful_max,
        "must not be dragged up by the inflated non-meaningful tokens"
    );
}

#[test]
fn bpe_score_falls_back_to_all_tokens_when_none_are_meaningful() {
    let tok = BpeTokenizer::load();
    let source = ". . . . .";
    let ids = tok.encode(source);
    assert!(
        ids.iter().all(|&id| !tok
            .id_to_token(id)
            .map(|piece| is_meaningful_token(&piece))
            .unwrap_or(false)),
        "fixture must be entirely punctuation"
    );

    let generic_counts: Vec<(u32, u64)> = ids.iter().map(|&id| (id, 1)).collect();
    let s = scorer(&generic_counts, 1_000, &[], 0);
    let expected = ids
        .iter()
        .map(|&id| s.token_surprise(id))
        .fold(f64::NEG_INFINITY, f64::max);
    assert_eq!(s.bpe_score(source), expected);
}

#[test]
fn max_token_surprise_ceiling_prefers_repo_unseen_meaningful_tokens() {
    let tok = BpeTokenizer::load();
    let mongo_id = tok.encode("mongoose")[0]; // repo-unseen, generic-rare
    let identifier_id = tok.encode("identifier")[0]; // repo-seen, generic-common

    // `identifier` is generically near-ubiquitous (so its *raw* surprise
    // would win a naive max), but it has also been seen in the repo
    // corpus; `mongo` is generically rare but never seen in the repo. The
    // ceiling must restrict itself to repo-unseen tokens when any exist.
    let s = scorer(
        &[(mongo_id, 10), (identifier_id, 1_000_000)],
        1_000_000,
        &[(identifier_id, 5)],
        1_000,
    );
    let ceiling = s.max_token_surprise_ceiling();
    assert_eq!(ceiling, s.token_surprise(mongo_id));
    assert!(ceiling < s.token_surprise(identifier_id));
}

#[test]
fn max_token_surprise_ceiling_falls_back_to_best_any_when_repo_has_seen_everything() {
    let tok = BpeTokenizer::load();
    let mongo_id = tok.encode("mongoose")[0];
    let identifier_id = tok.encode("identifier")[0];

    // Both meaningful generic tokens are repo-seen, so there is no
    // repo-unseen candidate: the ceiling must fall back to the best over
    // all meaningful tokens instead of being vacuously 0.
    let s = scorer(
        &[(mongo_id, 100), (identifier_id, 100)],
        1_000,
        &[(mongo_id, 1), (identifier_id, 1)],
        1_000,
    );
    let best_any = [mongo_id, identifier_id]
        .iter()
        .map(|&id| s.token_surprise(id))
        .fold(f64::NEG_INFINITY, f64::max);
    assert_eq!(s.max_token_surprise_ceiling(), best_any);
}

#[test]
fn max_token_surprise_ceiling_is_zero_when_no_generic_token_is_meaningful() {
    let tok = BpeTokenizer::load();
    let punctuation_id = tok.encode("+=")[0];
    let s = scorer(&[(punctuation_id, 100)], 1_000, &[], 0);
    assert_eq!(s.max_token_surprise_ceiling(), 0.0);
}

#[test]
fn max_token_surprise_ceiling_is_zero_for_an_empty_generic_baseline() {
    let s = scorer(&[], 1, &[], 0);
    assert_eq!(s.max_token_surprise_ceiling(), 0.0);
}

#[test]
fn total_repo_defaults_to_one_to_avoid_division_by_zero_on_an_empty_corpus() {
    let s = scorer(&[], 1, &[], 0);
    assert_eq!(s.total_repo(), 1.0);
}

#[test]
fn total_generic_reflects_the_baseline_total_tokens() {
    let s = scorer(&[], 12_345, &[], 0);
    assert_eq!(s.total_generic(), 12_345.0);
}

#[test]
fn bpe_score_excluding_equals_a_scorer_trained_without_the_file() {
    let file_a = "def alpha():\n    return mongoose_client(identifier)\n";
    let file_b = "def beta(values):\n    return sum(values)\n";
    let hunk = "result = mongoose_client(identifier)";

    let with_both = BpeScorer::new(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &[file_a.to_string(), file_b.to_string()],
    )
    .unwrap();
    let without_a = BpeScorer::new(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &[file_b.to_string()],
    )
    .unwrap();

    let counts_a = with_both.token_counts(file_a);
    // Exact LOO: subtracting file A's counts reproduces the corpus that
    // never contained it.
    assert_eq!(
        with_both.bpe_score_excluding(hunk, &counts_a),
        without_a.bpe_score(hunk)
    );
    // Held-out scoring can only read the hunk as at-least-as-surprising:
    // subtracting counts never raises p_repo. (Whether it is strictly
    // higher depends on which token attains the max.)
    assert!(with_both.bpe_score_excluding(hunk, &counts_a) >= with_both.bpe_score(hunk));
}

#[test]
fn bpe_score_excluding_nothing_is_the_plain_score() {
    let src = "def alpha():\n    return 1\n";
    let s = BpeScorer::new(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &[src.to_string()],
    )
    .unwrap();
    assert_eq!(
        s.bpe_score_excluding(src, &HashMap::new()),
        s.bpe_score(src)
    );
}

#[test]
fn stats_roundtrips_through_from_stats_preserving_bpe_score() {
    let source = "identifier = 1";
    let scorer_a = BpeScorer::new(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &[source.to_string()],
    )
    .expect("construct scorer from live corpus");
    let snapshot = scorer_a.stats();

    let scorer_b = BpeScorer::from_stats(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &snapshot,
    )
    .expect("rebuild scorer from snapshot");

    assert_eq!(scorer_a.bpe_score(source), scorer_b.bpe_score(source));
}

#[test]
fn stats_on_an_empty_corpus_roundtrips_to_the_same_empty_state() {
    let scorer_a = BpeScorer::new(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &[],
    )
    .expect("empty repo_sources is a valid corpus");
    let snapshot = scorer_a.stats();
    assert!(snapshot.token_counts.is_empty());
    assert_eq!(snapshot.total_tokens, 0);

    let scorer_b = BpeScorer::from_stats(
        BpeTokenizer::load(),
        crate::train::GENERIC_BASELINE_JSON,
        &snapshot,
    )
    .expect("rebuild scorer from empty snapshot");
    assert_eq!(
        scorer_b.total_repo(),
        1.0,
        "empty snapshot still guards against division by zero"
    );
}
