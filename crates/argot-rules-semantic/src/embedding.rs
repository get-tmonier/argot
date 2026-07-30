//! The embedding contract the semantic layer is written against.
//!
//! The index, the machine cache and both rules only ever see this surface, so
//! which model produces the vectors is a composition decision rather than a
//! rewrite. Implementations must return **L2-normalised, f16-canonical**
//! vectors: identical bits whether a vector was computed now, reloaded from the
//! artifact, or served from the cache — that is what makes a cache hit and a
//! fresh embed produce the same finding.

use anyhow::Result;

/// A model that turns a function's source into a unit vector.
pub trait EmbeddingModel: Send + Sync {
    /// Embed each text into a unit vector of [`Self::dim`] components,
    /// order-preserving.
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>>;

    /// Dimensionality of this model's vectors.
    fn dim(&self) -> usize;

    /// Model name, recorded in the artifact so an index built by another model
    /// is rejected loudly rather than queried in the wrong space.
    fn name(&self) -> &str;

    /// Content fingerprint of the weights, recorded alongside [`Self::name`].
    fn fingerprint(&self) -> &str;
}
