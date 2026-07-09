//! Structural-foreignness floor validation (`--mode structural`, feature-gated).
//!
//! Validates the irreducible floor from
//! `docs/research/evidence/foreign-structure-gate-floor.md` on REAL
//! infrastructure — closing the two gaps the Python proxy left:
//!   1. real multi-language tree-sitter extraction (11 languages, not Python
//!      `ast` only), via `argot_core::scoring::structural`;
//!   2. real temporal-holdout over-fire (each corpus's own clean commits after a
//!      HEAD~window fit), not a random 70/30 function split.
//!
//! It is self-contained and NON-GATING: it reuses argot-core's `structural`
//! primitives + argot-bench's clone/holdout helpers, but never touches the base
//! scoring path, `check`/`train`, or the base `dashboard.json`. The shipped
//! guardrail is byte-for-byte unchanged.
//!
//! Unit: a **clean-commit added hunk** (the added lines of one diff hunk on a
//! post-fit, non-merge commit). The same hunk set does double duty —
//!   over-fire(C) = fire-rate of C's hunks against C's OWN fit-SHA vocabulary,
//!   catch(A←B)  = fire-rate of B's hunks against A's vocabulary (A≠B, same
//!                 language) — the real-code analog of "a foreign idiom pasted
//!                 into this repo". Fire rule + background prior are argot-core's.

use std::collections::{BTreeMap, HashMap};
use std::path::Path;

use anyhow::{Context, Result};
use argot_core::scoring::adapters::Language;
use argot_core::scoring::structural::{hunk_foreignness, Bigram, StructuralPrior, StructuralVocab};
use serde::Serialize;

use crate::holdout::{ensure_full_history, fit_tree_files, plan_holdout};
use crate::production::git_stdout;
use crate::run::{ensure_clone, ensure_sha_checked_out};
use crate::targets::Target;

/// Operating point (from the sweep): fire on a globally-common (`bg_df ≥ TAU`),
/// repo-absent bigram; `k` = how many distinct such bigrams a hunk needs.
const TAU: f64 = 0.5;
const KS: &[usize] = &[1, 2, 3];
const DEFAULT_WINDOW: usize = 150;
const MAX_HUNKS: usize = 4000;

/// Source-file extensions → language. Bench-local (the base `ext_to_lang`
/// returns a name string; we need the `Language` enum). Kept in sync with the
/// languages that have pinned corpora.
fn ext_lang(path: &str) -> Option<Language> {
    let ext = path.rsplit('.').next()?;
    Some(match ext {
        "py" => Language::Python,
        "ts" | "tsx" => Language::Typescript,
        "js" | "jsx" | "mjs" | "cjs" => Language::Javascript,
        "go" => Language::Go,
        "rs" => Language::Rust,
        "c" | "h" => Language::C,
        "java" => Language::Java,
        "cs" => Language::CSharp,
        "php" => Language::Php,
        "cc" | "cpp" | "cxx" | "hpp" | "hh" => Language::Cpp,
        "rb" => Language::Ruby,
        _ => return None,
    })
}

/// Paths outside a repo's "voice" — vendored, generated, or test trees whose
/// structure would muddy the vocabulary. Domain-blind path heuristics only.
fn is_voice_path(path: &str) -> bool {
    const SKIP: &[&str] = &[
        "/test/",
        "/tests/",
        "test_",
        "_test.",
        ".test.",
        "/spec/",
        "/vendor/",
        "/node_modules/",
        "/migrations/",
        "/third_party/",
        "/generated/",
        ".min.",
        "/dist/",
        "/build/",
    ];
    !SKIP.iter().any(|s| path.contains(s))
}

struct CorpusData {
    name: String,
    language: String,
    vocab: StructuralVocab,
    /// Clean-commit added hunks: (language, text).
    hunks: Vec<(Language, String)>,
}

/// Parse `git show --unified=0` output into per-hunk added-line blocks.
fn added_hunks(diff: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut cur_path: Option<String> = None;
    let mut block = String::new();
    let flush = |path: &Option<String>, block: &mut String, out: &mut Vec<(String, String)>| {
        if let Some(p) = path {
            if !block.trim().is_empty() {
                out.push((p.clone(), std::mem::take(block)));
                return;
            }
        }
        block.clear();
    };
    for line in diff.lines() {
        if let Some(rest) = line.strip_prefix("+++ b/") {
            flush(&cur_path, &mut block, &mut out);
            cur_path = Some(rest.to_string());
        } else if line.starts_with("@@") {
            flush(&cur_path, &mut block, &mut out);
        } else if let Some(added) = line.strip_prefix('+') {
            if !line.starts_with("+++") {
                block.push_str(added);
                block.push('\n');
            }
        }
    }
    flush(&cur_path, &mut block, &mut out);
    out
}

fn collect_corpus(target: &Target, data_dir: &Path) -> Result<CorpusData> {
    let repo = ensure_clone(data_dir, &target.name, &target.url)?;
    let head = target.prs[0].sha.clone();
    ensure_full_history(&repo)?;
    ensure_sha_checked_out(&repo, &head)?;
    let window = target.holdout_window.unwrap_or(DEFAULT_WINDOW);
    let (fit_sha, replay) = plan_holdout(&repo, &head, window)?;
    eprintln!(
        "[{}] structural: fit @ {} (head~{}), {} clean commits",
        target.name,
        &fit_sha[..8],
        window,
        replay.len()
    );

    // Fit vocabulary from the fit-SHA source tree.
    ensure_sha_checked_out(&repo, &fit_sha)?;
    let files = fit_tree_files(&repo, &fit_sha)?;
    let mut sources: Vec<(String, Language)> = Vec::new();
    for path in &files {
        if !is_voice_path(path) {
            continue;
        }
        let Some(lang) = ext_lang(path) else { continue };
        if let Ok(src) = std::fs::read_to_string(repo.join(path)) {
            sources.push((src, lang));
        }
    }
    let vocab = StructuralVocab::fit(sources.iter().map(|(s, l)| (s.as_str(), *l)));

    // Clean-commit added hunks (post-fit commits, non-merge via plan_holdout).
    let mut hunks: Vec<(Language, String)> = Vec::new();
    for sha in &replay {
        if hunks.len() >= MAX_HUNKS {
            break;
        }
        let diff = git_stdout(
            &repo,
            &["show", "--unified=0", "--no-color", "--pretty=format:", sha],
        )
        .unwrap_or_default();
        for (path, block) in added_hunks(&diff) {
            if !is_voice_path(&path) {
                continue;
            }
            if let Some(lang) = ext_lang(&path) {
                hunks.push((lang, block));
            }
        }
    }
    Ok(CorpusData {
        name: target.name.clone(),
        language: target.language.clone(),
        vocab,
        hunks,
    })
}

/// Per-language background prior: bigram → fraction of the language's OTHER
/// corpora that attest it (leave-native-out document frequency).
fn language_prior(group: &[&CorpusData], native_idx: usize) -> StructuralPrior {
    let others: Vec<&&CorpusData> = group
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != native_idx)
        .map(|(_, c)| c)
        .collect();
    if others.is_empty() {
        return StructuralPrior::new();
    }
    let mut df: HashMap<Bigram, usize> = HashMap::new();
    for c in &others {
        for bg in c.vocab.attested() {
            *df.entry(bg.clone()).or_default() += 1;
        }
    }
    let n = others.len() as f64;
    df.into_iter().map(|(k, v)| (k, v as f64 / n)).collect()
}

#[derive(Serialize)]
struct CorpusResult {
    corpus: String,
    language: String,
    hunks: usize,
    /// over-fire per k (fraction of this corpus's own clean hunks that fire).
    over_fire: BTreeMap<usize, f64>,
    /// catch-any per k (fraction of same-language foreign hunks that fire).
    catch_any: BTreeMap<usize, f64>,
    foreign_hunks: usize,
}

fn fire_rate(
    hunks: &[(Language, String)],
    vocab: &StructuralVocab,
    prior: &StructuralPrior,
    k: usize,
) -> f64 {
    if hunks.is_empty() {
        return f64::NAN;
    }
    let fired = hunks
        .iter()
        .filter(|(lang, h)| hunk_foreignness(h, *lang, vocab, prior, TAU).foreign_common >= k)
        .count();
    fired as f64 / hunks.len() as f64
}

pub fn run_structural(targets: &[Target], data_dir: &Path, results_dir: &Path) -> Result<String> {
    // Collect every corpus (skip failures loudly — a floor validation tolerates
    // the odd unclonable/too-shallow repo).
    let mut corpora: Vec<CorpusData> = Vec::new();
    for t in targets {
        if t.language == "multi" {
            continue; // no single-language vocab / prior
        }
        match collect_corpus(t, data_dir) {
            Ok(c) => corpora.push(c),
            Err(e) => eprintln!("[{}] structural: skipped ({e:#})", t.name),
        }
    }

    // Group by language; catch needs ≥2 same-language corpora.
    let mut by_lang: BTreeMap<String, Vec<usize>> = BTreeMap::new();
    for (i, c) in corpora.iter().enumerate() {
        by_lang.entry(c.language.clone()).or_default().push(i);
    }

    let mut results: Vec<CorpusResult> = Vec::new();
    for idxs in by_lang.values() {
        let group: Vec<&CorpusData> = idxs.iter().map(|&i| &corpora[i]).collect();
        for (gi, native) in group.iter().enumerate() {
            let prior = language_prior(&group, gi);
            // foreign hunks = all same-language OTHER corpora's clean hunks.
            let mut foreign: Vec<(Language, String)> = Vec::new();
            for (gj, other) in group.iter().enumerate() {
                if gj != gi {
                    foreign.extend(other.hunks.iter().cloned());
                }
            }
            let mut over_fire = BTreeMap::new();
            let mut catch_any = BTreeMap::new();
            for &k in KS {
                over_fire.insert(k, fire_rate(&native.hunks, &native.vocab, &prior, k));
                catch_any.insert(k, fire_rate(&foreign, &native.vocab, &prior, k));
            }
            results.push(CorpusResult {
                corpus: native.name.clone(),
                language: native.language.clone(),
                hunks: native.hunks.len(),
                over_fire,
                catch_any,
                foreign_hunks: foreign.len(),
            });
        }
    }

    let md = render(&results);
    std::fs::create_dir_all(results_dir).ok();
    std::fs::write(
        results_dir.join("structural.json"),
        serde_json::to_string_pretty(&results)?,
    )
    .context("writing structural.json")?;
    std::fs::write(results_dir.join("structural.md"), &md).ok();
    Ok(md)
}

fn pct(x: f64) -> String {
    if x.is_nan() {
        "  n/a".to_string()
    } else {
        format!("{:4.1}%", 100.0 * x)
    }
}

fn render(results: &[CorpusResult]) -> String {
    let mut s = String::new();
    s.push_str("# Structural-foreignness floor — real-infra validation\n\n");
    s.push_str("Fire rule: ≥k repo-absent bigrams with bg_df≥0.5. Unit: clean-commit hunk.\n");
    s.push_str("over-fire = corpus's own clean hunks; catch = same-language foreign hunks.\n\n");
    s.push_str("| corpus | lang | hunks | OF@1 | OF@2 | OF@3 | catch@1 | catch@2 | catch@3 |\n");
    s.push_str("|---|---|--:|--:|--:|--:|--:|--:|--:|\n");
    for r in results {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.corpus,
            r.language,
            r.hunks,
            pct(r.over_fire[&1]),
            pct(r.over_fire[&2]),
            pct(r.over_fire[&3]),
            pct(r.catch_any[&1]),
            pct(r.catch_any[&2]),
            pct(r.catch_any[&3]),
        ));
    }
    // aggregate over corpora with a foreign set (catch measurable)
    let with_catch: Vec<&CorpusResult> = results.iter().filter(|r| r.foreign_hunks > 0).collect();
    let mean = |f: &dyn Fn(&CorpusResult) -> f64| -> f64 {
        let vs: Vec<f64> = with_catch
            .iter()
            .map(|r| f(r))
            .filter(|x| !x.is_nan())
            .collect();
        if vs.is_empty() {
            f64::NAN
        } else {
            vs.iter().sum::<f64>() / vs.len() as f64
        }
    };
    let worst_of1 = with_catch
        .iter()
        .map(|r| r.over_fire[&1])
        .filter(|x| !x.is_nan())
        .fold(0.0f64, f64::max);
    let worst_of2 = with_catch
        .iter()
        .map(|r| r.over_fire[&2])
        .filter(|x| !x.is_nan())
        .fold(0.0f64, f64::max);
    s.push_str(&format!(
        "\n**MEAN** OF@1 {} · OF@2 {} · catch@1 {} · catch@2 {}\n",
        pct(mean(&|r| r.over_fire[&1])),
        pct(mean(&|r| r.over_fire[&2])),
        pct(mean(&|r| r.catch_any[&1])),
        pct(mean(&|r| r.catch_any[&2])),
    ));
    s.push_str(&format!(
        "**WORST over-fire** k=1 {} · k=2 {}  (gatable ⇒ ≤5% on every corpus)\n",
        pct(worst_of1),
        pct(worst_of2),
    ));
    s
}
