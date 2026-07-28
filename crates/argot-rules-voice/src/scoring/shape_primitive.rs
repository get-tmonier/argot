//! Shape-primitive interface + registry.
//!
//! A shape primitive is a swappable, per-cluster scalar AST-shape term that
//! rides alongside the call-receiver scorer. Each primitive fits a baseline on
//! its cluster's files, then scores a hunk against that baseline, returning a
//! single non-negative contribution clipped to `cluster_bonus_clip`.
//!
//! Design constraints (binding):
//! - one scalar per primitive; composition happens at the caller,
//! - swappable: same trait, same per-cluster baseline mechanism,
//! - language-agnostic: defined on tree-sitter node kinds,
//! - domain-blind: no framework/function/decorator/string literals,
//! - per-cluster baseline only,
//! - cluster-size floor: abstain (0.0) below `min_cluster_size`.
//!
//! Baseline payload shapes differ per primitive, so they are unified in the
//! [`Baseline`] enum (namespace histogram / mean-std / top10-mean-std). The
//! trait's `fit_cluster_baseline` returns `Option<Baseline>`; `None` means the
//! primitive permanently abstains on that cluster.

use crate::scoring::adapters::Language;
use crate::scoring::shape_primitives::{
    CallScopeFraction, CalleeDistributionUnderCoverage, ClusterStapleDeficit,
    ExceptReturnRaiseRatio, FallThroughGuards, NamespaceJsd, TypicalCallDensity,
};
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Unified per-cluster baseline payload. Each variant is produced (and
/// consumed) by exactly one primitive; the enum lets the trait stay uniform.
#[derive(Debug, Clone)]
pub enum Baseline {
    /// `namespace_jsd`: pooled namespace-prefix histogram. `alphabet` is the
    /// set of observed prefixes; `distribution` maps prefix → probability
    /// (values sum to 1.0). `language` is captured so `score` can parse the
    /// hunk without a separate parameter.
    Namespace {
        language: Language,
        alphabet: BTreeSet<String>,
        distribution: BTreeMap<String, f64>,
    },
    /// `call_scope_fraction`, `except_return_raise_ratio`, `fall_through_guards`:
    /// population mean/std of a per-file scalar.
    MeanStd { mean: f64, std: f64 },
    /// `typical_call_density`, `cluster_staple_deficit`: top-10 callee set +
    /// population mean/std of a per-file scalar.
    Top10MeanStd {
        top10_set: BTreeSet<String>,
        mean: f64,
        std: f64,
    },
    /// `callee_distribution_under_coverage`: pooled callee distribution +
    /// population mean/std of per-file one-sided divergence.
    DistributionMeanStd {
        distribution: BTreeMap<String, f64>,
        mean: f64,
        std: f64,
    },
}

/// Per-cluster scalar AST-shape primitive.
///
/// Lifecycle: `fit_cluster_baseline` once per
/// cluster, then `score` per hunk. Both take `&self`; primitives that need to
/// remember the fit-time language use interior mutability and expose it via
/// [`ShapePrimitive::set_language`].
pub trait ShapePrimitive: Sync {
    /// Unique identifier (e.g. `except_return_raise_ratio`).
    fn name(&self) -> &str;
    /// Below this cluster size the primitive abstains (returns 0.0).
    fn min_cluster_size(&self) -> usize;
    /// Per-primitive cap on the score contribution.
    fn cluster_bonus_clip(&self) -> f64;
    /// Fit the cluster baseline. `None` permanently abstains on this cluster.
    fn fit_cluster_baseline(
        &self,
        cluster_files: &[(PathBuf, String)],
        language: Language,
    ) -> Option<Baseline>;
    /// Score a hunk against `baseline`. Returns 0.0 when `baseline` is `None`
    /// or `cluster_size < min_cluster_size`, otherwise a non-negative
    /// contribution clipped to `cluster_bonus_clip`.
    fn score(&self, hunk: &str, baseline: Option<&Baseline>, cluster_size: usize) -> f64;
    /// Inject the language a language-stateful primitive would otherwise
    /// capture during `fit_cluster_baseline`. No-op for stateless primitives.
    /// Used when a baseline is supplied without a preceding fit (a stateful
    /// primitive normally captures its language during `fit_cluster_baseline`).
    fn set_language(&self, _language: Language) {}
}

/// Factory that builds a fresh primitive instance (fresh baseline state per
/// build).
pub type ShapePrimitiveFactory = fn() -> Box<dyn ShapePrimitive>;

/// Name → factory registry. `BTreeMap` keeps `known()` alphabetical for free.
pub struct ShapePrimitiveRegistry {
    factories: BTreeMap<String, ShapePrimitiveFactory>,
}

impl ShapePrimitiveRegistry {
    /// Empty registry.
    pub fn new() -> Self {
        Self {
            factories: BTreeMap::new(),
        }
    }

    /// Registry pre-populated with the five built-in primitives.
    pub fn with_builtins() -> Self {
        let mut r = Self::new();
        let call_scope: ShapePrimitiveFactory = || Box::new(CallScopeFraction::default());
        let except: ShapePrimitiveFactory = || Box::new(ExceptReturnRaiseRatio);
        let fall_through: ShapePrimitiveFactory = || Box::new(FallThroughGuards);
        let namespace: ShapePrimitiveFactory = || Box::new(NamespaceJsd);
        let typical: ShapePrimitiveFactory = || Box::new(TypicalCallDensity::default());
        let staple: ShapePrimitiveFactory = || Box::new(ClusterStapleDeficit::default());
        let under_cov: ShapePrimitiveFactory =
            || Box::new(CalleeDistributionUnderCoverage::default());
        r.register("call_scope_fraction", call_scope);
        r.register("cluster_staple_deficit", staple);
        r.register("callee_distribution_under_coverage", under_cov);
        r.register("except_return_raise_ratio", except);
        r.register("fall_through_guards", fall_through);
        r.register("namespace_jsd", namespace);
        r.register("typical_call_density", typical);
        r
    }

    /// Register `factory` under `name` (last registration wins).
    pub fn register(&mut self, name: &str, factory: ShapePrimitiveFactory) {
        self.factories.insert(name.to_string(), factory);
    }

    /// Registered primitive names, alphabetical.
    pub fn known(&self) -> Vec<String> {
        self.factories.keys().cloned().collect()
    }

    /// Translate names into freshly-built instances. Unknown names fail loudly
    /// (no silent skip).
    pub fn build(&self, names: &[String]) -> Result<Vec<Box<dyn ShapePrimitive>>, String> {
        let mut out = Vec::with_capacity(names.len());
        for name in names {
            match self.factories.get(name) {
                Some(factory) => out.push(factory()),
                None => {
                    let known = self.known().join(", ");
                    let known = if known.is_empty() {
                        "<none>".to_string()
                    } else {
                        known
                    };
                    return Err(format!("unknown shape primitive {name:?}; known: {known}"));
                }
            }
        }
        Ok(out)
    }
}

impl Default for ShapePrimitiveRegistry {
    fn default() -> Self {
        Self::with_builtins()
    }
}

#[cfg(test)]
mod tests;
