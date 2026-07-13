use super::*;
use crate::index::IndexEntry;

fn unit(v: Vec<f32>) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    v.into_iter().map(|x| x / n).collect()
}

fn entry_c(
    symbol: &str,
    path: &str,
    vec: Vec<f32>,
    callees: &[&str],
    subtokens: &[&str],
) -> IndexEntry {
    IndexEntry {
        symbol: symbol.into(),
        path: path.into(),
        line: 1,
        vec: unit(vec),
        callees: callees.iter().map(|s| s.to_string()).collect(),
        subtokens: subtokens.iter().map(|s| s.to_string()).collect(),
        text_hash: String::new(),
    }
}

fn func_c(symbol: &str, path: &str, callees: &[&str], subtokens: &[&str]) -> FunctionRef {
    // A ≥6-line body so the substance filter (MIN_REINVENTION_BODY_LINES) treats
    // these as real functions; the tests exercise the structural fire logic, not
    // the wrapper filter (which has its own tests).
    let text = format!("fn {symbol}() {{\n    let a = 1;\n    let b = 2;\n    let c = a + b;\n    let d = c * 2;\n    d\n}}");
    FunctionRef {
        symbol: symbol.into(),
        path: path.into(),
        line: 10,
        end_line: 16,
        text: text.clone(),
        embed_text: text,
        callees: callees.iter().map(|s| s.to_string()).collect(),
        subtokens: subtokens.iter().map(|s| s.to_string()).collect(),
    }
}

/// `slugify` sits alone in one direction, a cluster of config code elsewhere.
fn index() -> SemanticIndex {
    SemanticIndex {
        dim: 3,
        entries: vec![
            entry_c(
                "slugify",
                "src/utils/text.py",
                vec![1.0, 0.0, 0.0],
                &[],
                &["slug", "normalize", "whitespace"],
            ),
            entry_c(
                "parse_config",
                "src/cfg.py",
                vec![0.0, 1.0, 0.0],
                &[],
                &["config", "parse", "yaml"],
            ),
            entry_c(
                "load_yaml",
                "src/cfg.py",
                vec![0.0, 0.9, 0.1],
                &[],
                &["yaml", "load", "stream"],
            ),
        ],
    }
}

#[test]
fn fires_on_near_duplicate_via_subtokens() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    // Very close to `slugify` (cos ≈ 0.98) and shares its rare subtokens.
    let q = unit(vec![0.98, 0.02, 0.0]);
    let f = scorer
        .evaluate(
            &func_c(
                "normalize_slug",
                "src/api/handlers.py",
                &[],
                &["slug", "normalize", "whitespace"],
            ),
            &q,
        )
        .expect("subtoken-confirmed near-duplicate fires");
    assert_eq!(f.nearest_symbol, "slugify");
    assert!(f.similarity > 0.9);
}

#[test]
fn does_not_fire_on_close_but_structurally_unrelated() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    // Close to `slugify` in embedding, but no shared callees and disjoint
    // subtokens → anisotropy near-match, not a reinvention.
    let q = unit(vec![0.98, 0.02, 0.0]);
    assert!(scorer
        .evaluate(
            &func_c(
                "draw_widget",
                "src/api/handlers.py",
                &[],
                &["draw", "widget", "canvas"]
            ),
            &q,
        )
        .is_none());
}

#[test]
fn same_file_match_is_excluded() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![0.98, 0.02, 0.0]);
    // Candidate lives in slugify's own file → its only near-dup is same-file.
    assert!(scorer
        .evaluate(
            &func_c(
                "slug2",
                "src/utils/text.py",
                &[],
                &["slug", "normalize", "whitespace"]
            ),
            &q,
        )
        .is_none());
}

#[test]
fn same_name_is_treated_as_move_not_reinvention() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![0.98, 0.02, 0.0]);
    // Same symbol name, different file → a move/rename, not a reinvention.
    assert!(scorer
        .evaluate(
            &func_c(
                "slugify",
                "src/api/handlers.py",
                &[],
                &["slug", "normalize", "whitespace"]
            ),
            &q,
        )
        .is_none());
}

#[test]
fn dunder_and_test_paths_are_gated() {
    assert!(!is_reinvention_candidate("__init__", "src/a.py"));
    assert!(!is_reinvention_candidate("helper", "tests/test_a.py"));
    assert!(!is_reinvention_candidate("helper", "src/a.test.ts"));
    assert!(!is_reinvention_candidate("helper", "spec/thing.py"));
    // Not a test file despite containing "test" in a component.
    assert!(is_reinvention_candidate("helper", "src/testing_utils.py"));
    assert!(is_reinvention_candidate("normalize", "src/utils/text.py"));
}

/// A near-duplicate cluster where the candidate shares the original's callees
/// but NOT its subtokens — the callee path must carry the fire on its own.
fn callee_index() -> SemanticIndex {
    SemanticIndex {
        dim: 3,
        entries: vec![
            entry_c(
                "format_price",
                "src/money.py",
                vec![1.0, 0.0, 0.0],
                &["round", "currency", "symbol"],
                &["price", "format"],
            ),
            entry_c(
                "format_amount",
                "src/money.py",
                vec![0.99, 0.02, 0.0],
                &["round", "currency"],
                &["amount", "format"],
            ),
        ],
    }
}

#[test]
fn callee_overlap_fires_without_subtoken_overlap() {
    let idx = callee_index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![0.995, 0.01, 0.0]); // cos ≈ 1.0 to format_price
    let f = scorer
        .evaluate(
            &func_c(
                "render_price",
                "src/ui/views.py",
                &["round", "currency", "symbol"], // full callee overlap
                &["render", "view"],              // disjoint subtokens
            ),
            &q,
        )
        .expect("callee overlap alone confirms the reinvention");
    assert_eq!(f.nearest_symbol, "format_price");
}

#[test]
fn no_structural_overlap_does_not_fire() {
    // Same close embedding, but disjoint callees AND disjoint subtokens.
    let idx = callee_index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![0.995, 0.01, 0.0]);
    assert!(scorer
        .evaluate(
            &func_c(
                "compute_layout",
                "src/ui/views.py",
                &["measure", "wrap", "clamp"],
                &["layout", "compute", "grid"],
            ),
            &q,
        )
        .is_none());
}

#[test]
fn strong_tier_rescues_distant_match_with_high_subtoken_overlap() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    // cos ≈ 0.72 to slugify — below the normal 0.78 floor — but identical
    // rare subtokens. The strong tier rescues it; the normal tier would not.
    let q = unit(vec![0.72, 0.69, 0.0]);
    let f = scorer
        .evaluate(
            &func_c(
                "make_slug",
                "src/api/handlers.py",
                &[],
                &["slug", "normalize", "whitespace"],
            ),
            &q,
        )
        .expect("strong tier rescues a distant but structurally-identical match");
    assert_eq!(f.nearest_symbol, "slugify");
    assert!(
        f.similarity < NORMAL_SIMILARITY,
        "sim {} below normal floor",
        f.similarity
    );
    assert!(f.similarity >= STRONG_SIMILARITY);
}

#[test]
fn distant_match_with_weak_overlap_does_not_fire() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    // Same cos ≈ 0.72, but only partial subtoken overlap (1 of 3 shared) →
    // below the strong bar, and too distant for the normal tier. No fire.
    let q = unit(vec![0.72, 0.69, 0.0]);
    assert!(scorer
        .evaluate(
            &func_c(
                "make_slug",
                "src/api/handlers.py",
                &[],
                &["slug", "trim", "pad"]
            ),
            &q,
        )
        .is_none());
}

/// A large family of near-identical functions (an interface implemented across
/// many files) all embed together. A new sibling that only *loosely* resembles
/// one of them is a family member, not a reinvention; one that shares a member's
/// *rare* helper is a genuine reimplementation of that member and still fires.
#[test]
fn sibling_in_a_dense_family_is_filtered_unless_it_shares_a_rare_helper() {
    let mut entries = Vec::new();
    for i in 0..8 {
        // One member owns a distinctive helper (`rotl64`, df=1 → rare); the
        // family's shared helpers (`base`, `emit`, df=8) are generic.
        let callees: &[&str] = if i == 0 {
            &["base", "emit", "rotl64"]
        } else {
            &["base", "emit"]
        };
        entries.push(entry_c(
            "handle",
            &format!("src/mod{i}/h.rs"),
            vec![1.0, 0.004 * i as f32, 0.0],
            callees,
            &["handle", "event"],
        ));
    }
    let idx = SemanticIndex { dim: 3, entries };
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    // Nearest member is mod0 — the one with the distinctive helper.
    let q = unit(vec![1.0, 0.0, 0.0]);
    // Loosely-resembling new sibling: fires the callee tier (shares `base`) but
    // weak overlap in a very dense neighbourhood → filtered as a family member.
    let weak = func_c(
        "route",
        "src/new/h.rs",
        &["base", "x", "y"],
        &["route", "path"],
    );
    assert!(scorer.evaluate(&weak, &q).is_none());
    // Full generic-callee overlap is NOT an exemption any more (per-locale
    // provider families overlap 100% on generic helpers) — still filtered.
    let generic = func_c(
        "route",
        "src/new/h.rs",
        &["base", "emit"],
        &["route", "path"],
    );
    assert!(scorer.evaluate(&generic, &q).is_none());
    // Sharing the one member's RARE helper is genuine reimplementation
    // evidence → exempt from the family filter, fires.
    let genuine = func_c(
        "route",
        "src/new/h.rs",
        &["base", "emit", "rotl64"],
        &["route", "path"],
    );
    assert!(scorer.evaluate(&genuine, &q).is_some());
}

/// Same-directory near-matches need to stand out from the neighbourhood: a
/// co-located protocol-variant family member (tiny cos₁−cos₂ margin) is
/// filtered; the same candidate in another directory fires.
#[test]
fn same_dir_match_requires_margin() {
    let idx = SemanticIndex {
        dim: 3,
        entries: vec![
            entry_c(
                "h2_go_state",
                "lib/proxy.py",
                vec![1.0, 0.0, 0.0],
                &[],
                &["tunnel", "go", "state"],
            ),
            // A second neighbour almost as close → margin ≈ 0.
            entry_c(
                "h2_reset",
                "lib/other.py",
                vec![0.999, 0.03, 0.0],
                &[],
                &["tunnel", "reset"],
            ),
        ],
    };
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![1.0, 0.01, 0.0]);
    // Same dir as the match (lib/) and margin ~0 → filtered.
    let same_dir = func_c(
        "h3_go_state",
        "lib/proxy_h3.py",
        &[],
        &["tunnel", "go", "state"],
    );
    assert!(scorer.evaluate(&same_dir, &q).is_none());
    // Identical candidate in a different directory → fires.
    let elsewhere = func_c(
        "h3_go_state",
        "src/http/proxy_h3.py",
        &[],
        &["tunnel", "go", "state"],
    );
    assert!(scorer.evaluate(&elsewhere, &q).is_some());
}

/// The substance floor applies unconditionally: a 4-line stub never fires,
/// even with perfect structural overlap.
#[test]
fn short_stub_never_fires_even_with_strong_overlap() {
    let idx = index();
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![0.98, 0.02, 0.0]);
    let stub = FunctionRef {
        symbol: "normalize_slug".into(),
        path: "src/api/handlers.py".into(),
        line: 1,
        end_line: 4,
        text: "def normalize_slug(s):\n    a = s.strip()\n    b = a.lower()\n    return b".into(),
        embed_text: "def f(s):\n    a = s.strip()\n    b = a.lower()\n    return b".into(),
        callees: vec![],
        subtokens: vec!["slug".into(), "normalize".into(), "whitespace".into()],
    };
    assert!(scorer.evaluate(&stub, &q).is_none());
}

/// Both sides built around the same single helper: the identical-single-callee
/// path confirms at normal-tier cosine (util reimplementations wrapping one
/// primitive have exactly one callee and generic subtokens).
#[test]
fn identical_single_callee_confirms_at_normal_cosine() {
    let idx = SemanticIndex {
        dim: 3,
        entries: vec![
            entry_c(
                "point_rotate",
                "src/math/point.py",
                vec![1.0, 0.0, 0.0],
                &["rotate_rads"],
                &["point", "rotate"],
            ),
            entry_c(
                "unrelated",
                "src/cfg.py",
                vec![0.0, 1.0, 0.0],
                &["load"],
                &["config"],
            ),
        ],
    };
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![1.0, 0.05, 0.0]);
    let f = scorer
        .evaluate(
            &func_c(
                "rotate_point",
                "src/geometry/util.py",
                &["rotate_rads"],    // same single callee
                &["vector", "spin"], // disjoint subtokens
            ),
            &q,
        )
        .expect("identical single callee confirms");
    assert_eq!(f.nearest_symbol, "point_rotate");
}

/// A thin delegator/wrapper is too short to be a meaningful reinvention when it
/// only loosely resembles its match.
#[test]
fn short_wrapper_is_filtered_when_overlap_is_weak() {
    let idx = callee_index(); // format_price / format_amount
    let scorer = RedundantScorer::new(&idx, &ReinventionConfig::default());
    let q = unit(vec![0.995, 0.01, 0.0]); // ≈1.0 to format_price
    let short = FunctionRef {
        symbol: "show".into(),
        path: "src/ui.rs".into(),
        line: 1,
        end_line: 3,
        text: "fn show() {\n    round()\n}".into(), // 3 lines, below the substance floor
        embed_text: "fn f() {\n    round()\n}".into(),
        callees: vec!["round".into(), "x".into(), "y".into()], // callee_jac ≈0.20 (weak)
        subtokens: vec!["show".into()],
    };
    assert!(scorer.evaluate(&short, &q).is_none());
}
