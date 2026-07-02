//! Convention-rarity scorer — the era-15 signal for out-of-voice *phrasing
//! shape* rather than out-of-voice tokens.
//!
//! Two corpus-derived features, no domain knowledge:
//!
//! * **Syntax-kind surprisal** — the corpus's frequency of every named
//!   tree-sitter node kind. A hunk built from constructs the corpus never
//!   uses (`var` declarations in a `const` codebase, class lifecycle in a
//!   functional one, accessibility modifiers) carries a kind whose corpus
//!   rate is near zero; the feature is the max `-ln(rate)` over the hunk's
//!   present kinds, scaled up to full weight at 3 occurrences.
//! * **Identifier-shape surprisal** — the corpus's mix of abstract
//!   identifier morphologies (camel / pascal / snake / scream / flat /
//!   other; pure character classes, never words). A hunk dominated by a
//!   shape the corpus doesn't use (snake_case in a camelCase repo) scores
//!   the shape's `-ln(corpus fraction)` weighted by its share of the hunk.
//!
//! Both saturate exactly where token surprise fails: the tokens of
//! `total_sum = total_sum + values[item_index]` are all common — their
//! arrangement is what's foreign. Firing bars are calibrated as the max
//! feature value over the calibration sample (see `run_calibrate`), so the
//! stage stays silent on in-voice code and is suppressed on the calibration
//! side per the calibration contract.

use crate::scoring::adapters::Language;
use crate::scoring::call_receiver::has_root_error;
use crate::scoring::model::ConventionModel;
use crate::scoring::ts_parse::parse;
use std::collections::{BTreeMap, HashMap};

/// Occurrence count at which a rare node kind reaches full surprisal weight.
const KIND_COUNT_SATURATION: f64 = 3.0;
/// Minimum identifiers in a hunk for the shape feature to be meaningful.
const MIN_HUNK_IDENTS: usize = 5;
/// Minimum occurrences and hunk share for a shape class to be scored.
const MIN_SHAPE_COUNT: usize = 3;
const MIN_SHAPE_SHARE: f64 = 0.3;

/// Both convention features for one hunk.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ConventionScores {
    pub syntax_surprisal: f64,
    pub ident_surprisal: f64,
}

/// Abstract morphology class of one identifier (character classes only).
pub fn ident_shape(ident: &str) -> &'static str {
    let has_under = ident.contains('_');
    let has_upper = ident.chars().any(|c| c.is_ascii_uppercase());
    let has_lower = ident.chars().any(|c| c.is_ascii_lowercase());
    let starts_upper = ident
        .chars()
        .next()
        .map(|c| c.is_ascii_uppercase())
        .unwrap_or(false);
    match (has_under, has_upper, has_lower, starts_upper) {
        (true, _, true, _) => "snake",
        (true, true, false, _) => "scream",
        (false, true, true, true) => "pascal",
        (false, true, true, false) => "camel",
        (false, false, true, _) => "flat",
        _ => "other",
    }
}

/// Count identifier shapes in `source` (identifiers of length ≥ 3; shorter
/// ones carry no morphology). Returns the total counted.
fn count_ident_shapes(source: &str, counts: &mut BTreeMap<String, u64>) -> u64 {
    let bytes = source.as_bytes();
    let mut total = 0u64;
    let mut i = 0;
    while i < bytes.len() {
        let c = bytes[i];
        if c == b'_' || c.is_ascii_alphabetic() {
            let start = i;
            i += 1;
            while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                i += 1;
            }
            let ident = &source[start..i];
            if ident.len() >= 3 {
                *counts.entry(ident_shape(ident).to_string()).or_insert(0) += 1;
                total += 1;
            }
        } else {
            i += 1;
        }
    }
    total
}

/// Count named node kinds in `source`, skipping error/missing nodes (their
/// kinds reflect the fragment's parse, not the code's conventions). When
/// `region` is given, only nodes starting within the 1-indexed inclusive row
/// range count — the host-AST path for fragments that don't parse alone.
fn count_node_kinds(
    source: &str,
    language: Language,
    region: Option<(usize, usize)>,
    counts: &mut HashMap<String, u64>,
) -> u64 {
    let Some(tree) = parse(source, language) else {
        return 0;
    };
    let mut total = 0u64;
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.is_named() && !node.is_error() && !node.is_missing() {
            let in_region = match region {
                None => true,
                Some((start, end)) => {
                    let row = node.start_position().row + 1;
                    row >= start && row <= end
                }
            };
            if in_region {
                *counts.entry(node.kind().to_string()).or_insert(0) += 1;
                total += 1;
            }
        }
        for i in (0..node.child_count()).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
    total
}

/// Fit the corpus-frequency side of a [`ConventionModel`] (bars start at
/// `f64::INFINITY` — never firing — until calibrated).
pub fn fit_convention_frequencies(
    corpus: &[(std::path::PathBuf, String)],
    language: Language,
) -> ConventionModel {
    let mut node_kinds: HashMap<String, u64> = HashMap::new();
    let mut total_nodes = 0u64;
    let mut ident_shapes: BTreeMap<String, u64> = BTreeMap::new();
    let mut total_idents = 0u64;
    for (_, src) in corpus {
        total_nodes += count_node_kinds(src, language, None, &mut node_kinds);
        total_idents += count_ident_shapes(src, &mut ident_shapes);
    }
    ConventionModel {
        node_kinds: node_kinds.into_iter().collect(),
        total_nodes,
        ident_shapes,
        total_idents,
        syntax_bar: f64::INFINITY,
        ident_bar: f64::INFINITY,
    }
}

/// The convention scorer: corpus frequencies + calibrated bars.
pub struct ConventionScorer {
    model: ConventionModel,
    language: Language,
}

impl ConventionScorer {
    pub fn new(model: ConventionModel, language: Language) -> Self {
        Self { model, language }
    }

    pub fn model(&self) -> &ConventionModel {
        &self.model
    }

    /// Set the calibrated firing bars.
    pub fn set_bars(&mut self, syntax_bar: f64, ident_bar: f64) {
        self.model.syntax_bar = syntax_bar;
        self.model.ident_bar = ident_bar;
    }

    fn kind_surprisal(&self, kind: &str, count: u64) -> f64 {
        let c = self.model.node_kinds.get(kind).copied().unwrap_or(0) as f64;
        let rate = (c + 1.0) / (self.model.total_nodes as f64 + 1.0);
        -rate.ln() * ((count as f64).min(KIND_COUNT_SATURATION) / KIND_COUNT_SATURATION)
    }

    /// Raw convention features for a hunk. `host_context` is
    /// `(host_source, start_line, end_line)`, consulted only when the bare
    /// hunk's parse has root errors (same discipline as the call-receiver's
    /// parse-error fallback).
    pub fn scores(
        &self,
        hunk: &str,
        host_context: Option<(&str, usize, usize)>,
    ) -> ConventionScores {
        let mut kinds: HashMap<String, u64> = HashMap::new();
        if has_root_error(hunk, self.language) {
            if let Some((host, start, end)) = host_context {
                count_node_kinds(host, self.language, Some((start, end)), &mut kinds);
            }
        } else {
            count_node_kinds(hunk, self.language, None, &mut kinds);
        }
        let syntax_surprisal = kinds
            .iter()
            .map(|(k, &n)| self.kind_surprisal(k, n))
            .fold(0.0f64, f64::max);

        let mut shapes: BTreeMap<String, u64> = BTreeMap::new();
        let total = count_ident_shapes(hunk, &mut shapes);
        let mut ident_surprisal = 0.0f64;
        if total as usize >= MIN_HUNK_IDENTS && self.model.total_idents > 0 {
            for (class, &n) in &shapes {
                let share = n as f64 / total as f64;
                if n as usize >= MIN_SHAPE_COUNT && share >= MIN_SHAPE_SHARE {
                    let cf = (self.model.ident_shapes.get(class).copied().unwrap_or(0) as f64
                        + 1.0)
                        / (self.model.total_idents as f64 + 1.0);
                    ident_surprisal = ident_surprisal.max(-cf.ln() * share);
                }
            }
        }
        ConventionScores {
            syntax_surprisal,
            ident_surprisal,
        }
    }

    /// Whether either convention clears its calibrated bar.
    pub fn fires(&self, scores: ConventionScores) -> bool {
        scores.syntax_surprisal > self.model.syntax_bar
            || scores.ident_surprisal > self.model.ident_bar
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn camel_corpus() -> Vec<(PathBuf, String)> {
        // Two templates × 10 files: arrow/const/typed-object style, no `var`,
        // no classes — a plausible modern-TS voice in miniature.
        (0..20)
            .map(|i| {
                let src = if i % 2 == 0 {
                    "export function computeTotal(itemValues: number[]): number {\n  \
                     const runningTotal = itemValues.reduce((acc, item) => acc + item, 0)\n  \
                     return runningTotal\n}\n"
                } else {
                    "export const describeBox = (box: { value: number }): string => {\n  \
                     const label = `box-${box.value}`\n  if (box.value > 0) {\n    \
                     return label\n  }\n  return label.trim()\n}\n"
                };
                (PathBuf::from(format!("m{i}.ts")), src.to_string())
            })
            .collect()
    }

    #[test]
    fn shape_classification() {
        assert_eq!(ident_shape("computeTotal"), "camel");
        assert_eq!(ident_shape("ComputeTotal"), "pascal");
        assert_eq!(ident_shape("compute_total"), "snake");
        assert_eq!(ident_shape("COMPUTE_TOTAL"), "scream");
        assert_eq!(ident_shape("compute"), "flat");
    }

    #[test]
    fn snake_case_in_camel_corpus_scores_high_ident_surprisal() {
        let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
        let scorer = ConventionScorer::new(model, Language::Typescript);
        let alien = "function compute_weighted_sum(input_values: number[]) {\n  \
                     let total_sum = 0\n  let weight_sum = 0\n  return total_sum + weight_sum\n}";
        let native = "function computeWeightedSum(inputValues: number[]) {\n  \
                      let totalSum = 0\n  let weightSum = 0\n  return totalSum + weightSum\n}";
        let a = scorer.scores(alien, None);
        let n = scorer.scores(native, None);
        assert!(
            a.ident_surprisal > n.ident_surprisal + 1.0,
            "snake {a:?} vs camel {n:?}"
        );
    }

    #[test]
    fn unused_construct_scores_high_syntax_surprisal() {
        let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
        let scorer = ConventionScorer::new(model, Language::Typescript);
        // The corpus has no `var` declarations and no classes.
        let alien = "export class LegacyBox {\n  private value = 0\n  \
                     public getValue(): number {\n    var copy = this.value\n    \
                     var doubled = copy * 2\n    var label = String(doubled)\n    \
                     return label.length\n  }\n}";
        let native = "export function getValue(box: { value: number }): number {\n  \
                      const copy = box.value\n  return copy\n}";
        let a = scorer.scores(alien, None);
        let n = scorer.scores(native, None);
        assert!(
            a.syntax_surprisal > n.syntax_surprisal + 2.0,
            "class/var {a:?} vs function/const {n:?}"
        );
    }

    #[test]
    fn parse_error_fragment_uses_host_region() {
        let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
        let scorer = ConventionScorer::new(model, Language::Typescript);
        // A fragment starting with a stray `}` has root errors; its kinds must
        // come from the host region, not the broken bare parse.
        let fragment = "}\n\nexport class LegacyBox {\n  public getValue(): number {\n    var copy = 1\n    return copy\n  }\n}";
        let host = "export function ok() {\n  return 1\n}\n\nexport class LegacyBox {\n  public getValue(): number {\n    var copy = 1\n    return copy\n  }\n}\n";
        let with_host = scorer.scores(fragment, Some((host, 3, 10)));
        let bare_ok = scorer.scores(
            "export class LegacyBox {\n  public getValue(): number {\n    var copy = 1\n    return copy\n  }\n}",
            None,
        );
        assert!(
            (with_host.syntax_surprisal - bare_ok.syntax_surprisal).abs() < 1.0,
            "host-region kinds ≈ clean-fragment kinds: {with_host:?} vs {bare_ok:?}"
        );
    }

    #[test]
    fn bars_gate_firing() {
        let model = fit_convention_frequencies(&camel_corpus(), Language::Typescript);
        let mut scorer = ConventionScorer::new(model, Language::Typescript);
        let alien = "function compute_weighted_sum(input_values: number[]) {\n  \
                     let total_sum = 0\n  let weight_sum = 0\n  return total_sum + weight_sum\n}";
        let s = scorer.scores(alien, None);
        assert!(!scorer.fires(s), "infinite bars never fire");
        scorer.set_bars(1000.0, s.ident_surprisal - 0.1);
        assert!(scorer.fires(s), "bar below the score fires");
    }
}
