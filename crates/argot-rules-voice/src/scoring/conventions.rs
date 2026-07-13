//! Convention-rarity scorer — the signal for out-of-voice *phrasing
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

/// Both convention features for one hunk. The identifier feature is per
/// morphology class (shape → surprisal contribution) so each shape can be judged
/// against its own calibrated bar.
#[derive(Debug, Clone, PartialEq)]
pub struct ConventionScores {
    pub syntax_surprisal: f64,
    pub ident_surprisals: BTreeMap<String, f64>,
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
        ident_bars: BTreeMap::new(),
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
    pub fn set_bars(&mut self, syntax_bar: f64, ident_bars: BTreeMap<String, f64>) {
        self.model.syntax_bar = syntax_bar;
        self.model.ident_bars = ident_bars;
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

        ConventionScores {
            syntax_surprisal,
            ident_surprisals: self.ident_surprisals(hunk),
        }
    }

    /// Per-morphology identifier-shape surprisal for a hunk in isolation — a
    /// pure byte scan (no parse), cheap enough to evaluate over many sliding
    /// windows during bar calibration. Returns one entry per shape class that
    /// dominates the hunk (count ≥ `MIN_SHAPE_COUNT`, share ≥ `MIN_SHAPE_SHARE`),
    /// mapping it to `-ln(corpus fraction) · share`. Always reads the bare hunk
    /// (the shape feature never consults a host region), so this is exactly the
    /// `ident_surprisals` component of [`Self::scores`].
    pub fn ident_surprisals(&self, hunk: &str) -> BTreeMap<String, f64> {
        let mut shapes: BTreeMap<String, u64> = BTreeMap::new();
        let total = count_ident_shapes(hunk, &mut shapes);
        let mut out = BTreeMap::new();
        if total as usize >= MIN_HUNK_IDENTS && self.model.total_idents > 0 {
            for (class, &n) in &shapes {
                let share = n as f64 / total as f64;
                if n as usize >= MIN_SHAPE_COUNT && share >= MIN_SHAPE_SHARE {
                    let cf = (self.model.ident_shapes.get(class).copied().unwrap_or(0) as f64
                        + 1.0)
                        / (self.model.total_idents as f64 + 1.0);
                    out.insert(class.clone(), -cf.ln() * share);
                }
            }
        }
        out
    }

    /// Scalar identifier-shape surprisal — the max over dominating shapes.
    /// Retained for reporting and tests; firing uses the per-shape map.
    pub fn ident_surprisal(&self, hunk: &str) -> f64 {
        self.ident_surprisals(hunk)
            .values()
            .copied()
            .fold(0.0f64, f64::max)
    }

    /// Whether either convention clears its calibrated bar. The syntax feature
    /// uses a single bar; the identifier feature is judged per morphology — a
    /// shape fires when its surprisal exceeds the bar calibrated for *that*
    /// shape. In a calibrated model a shape with no bar was never concentrated
    /// in-voice, so any dominant occurrence fires (bar defaults to 0). An empty
    /// `ident_bars` means the model was never calibrated (bars are set by
    /// `run_calibrate`), so the identifier feature stays silent — the analogue
    /// of the old `f64::INFINITY` uncalibrated bar.
    pub fn fires(&self, scores: &ConventionScores) -> bool {
        if scores.syntax_surprisal > self.model.syntax_bar {
            return true;
        }
        if self.model.ident_bars.is_empty() {
            return false;
        }
        scores
            .ident_surprisals
            .iter()
            .any(|(shape, &surp)| surp > self.model.ident_bars.get(shape).copied().unwrap_or(0.0))
    }
}

#[cfg(test)]
mod tests;
