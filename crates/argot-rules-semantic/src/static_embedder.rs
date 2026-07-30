//! The static embedder — a distilled token-embedding table, run in pure Rust.
//!
//! argot used to run a 161M-parameter transformer through llama.cpp for this;
//! the distilled table replaces it at ~350× the throughput, with no attention,
//! no C++ backend and no download — the weights live inside the binary.
//!
//! It is not a free lunch — a bag of token vectors cannot represent order or
//! structure. What makes it viable here is measured, not assumed: across 31
//! corpora / 581 authored fixtures / 11 languages it holds `redundant` recall
//! 0.936 against the transformer's 0.943 and fires 16% less, while `misplaced`
//! recall is 0.940 against 0.945
//! (`docs/research/evidence/static-embedder-P0-verdict.md`).
//!
//! The forty lines of inference live here rather than behind the `model2vec`
//! crate for three reasons: that crate can only load from a *directory*, which
//! would force writing ~130 MB to disk before the model can be embedded in the
//! binary; it pulls a second copy of `tokenizers`; and pooling order is part of
//! our determinism contract, so it should be ours to pin.

use anyhow::{bail, Context, Result};
use std::path::Path;
use tokenizers::Tokenizer;

use super::embedding::EmbeddingModel;

/// Env override pointing at a model directory (`model.safetensors`,
/// `tokenizer.json`). Present so a candidate model can be exercised before one
/// is embedded in the binary.
pub const STATIC_MODEL_ENV: &str = "ARGOT_STATIC_MODEL";

/// The tensor holding the token-embedding table. `model2vec` writes it under
/// this name; sentence-transformers' static models call it `embedding.weight`.
const TENSOR_NAMES: [&str; 2] = ["embeddings", "embedding.weight"];

/// Companion tensor carrying the int8 table's dequantisation step. Quantising
/// the table halves the weights shipped in the binary (31.3 MB → 15.6 MB) and
/// costs almost nothing: pooling sums ~100 rows, so the per-row error largely
/// cancels — measured cosine against the f16 table is 0.9995 mean / 0.9990
/// worst, an order of magnitude below one step of the index's own int8
/// quantisation.
const SCALE_TENSOR: &str = "embeddings_scale";

/// The embedded model — argot ships its own weights, so a fit needs no network,
/// no cache directory and no first-use download.
const EMBEDDED_WEIGHTS: &[u8] = include_bytes!("../model/embeddings.safetensors");
const EMBEDDED_TOKENIZER: &[u8] = include_bytes!("../model/tokenizer.json");
/// Name recorded in the artifact for the embedded model.
const EMBEDDED_NAME: &str = "jina-v2-code-static-256";

/// A loaded static embedding table.
pub struct StaticEmbedder {
    tokenizer: Tokenizer,
    /// Row-major `vocab × dim`.
    rows: Vec<f32>,
    dim: usize,
    name: String,
    fingerprint: String,
}

impl StaticEmbedder {
    /// Load from a model directory.
    pub fn load(dir: &Path) -> Result<Self> {
        let weights = dir.join("model.safetensors");
        let tok = dir.join("tokenizer.json");
        if !weights.exists() || !tok.exists() {
            bail!(
                "not a static model directory (need model.safetensors and tokenizer.json): {}",
                dir.display()
            );
        }
        let name = dir
            .file_name()
            .map(|s| s.to_string_lossy().to_string())
            .unwrap_or_else(|| "static".to_string());
        Self::from_bytes(
            &std::fs::read(&weights).with_context(|| format!("read {}", weights.display()))?,
            &std::fs::read(&tok).with_context(|| format!("read {}", tok.display()))?,
            name,
        )
    }

    /// Load from bytes — the path an embedded model takes, with no filesystem
    /// round-trip.
    pub fn from_bytes(weights: &[u8], tokenizer: &[u8], name: String) -> Result<Self> {
        let tokenizer =
            Tokenizer::from_bytes(tokenizer).map_err(|e| anyhow::anyhow!("load tokenizer: {e}"))?;
        let st = safetensors::SafeTensors::deserialize(weights).context("parse safetensors")?;
        let (_, view) = TENSOR_NAMES
            .iter()
            .find_map(|n| st.tensor(n).ok().map(|v| (*n, v)))
            .context("no token-embedding tensor in the model")?;
        let shape = view.shape();
        if shape.len() != 2 {
            bail!(
                "token-embedding tensor is {}-dimensional, expected 2",
                shape.len()
            );
        }
        let dim = shape[1];
        let scale = st.tensor(SCALE_TENSOR).ok().and_then(|v| {
            v.data()
                .get(..4)
                .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        });
        let rows = decode_rows(&view, scale)?;
        if rows.len() != shape[0] * dim {
            bail!(
                "token-embedding tensor holds {} values, expected {}",
                rows.len(),
                shape[0] * dim
            );
        }
        let fingerprint = fingerprint_of(weights);
        Ok(Self {
            tokenizer,
            rows,
            dim,
            name,
            fingerprint,
        })
    }

    /// The model argot ships. Never fails for want of a file, a cache or a
    /// network — the weights are in the binary.
    pub fn embedded() -> Result<Self> {
        Self::from_bytes(
            EMBEDDED_WEIGHTS,
            EMBEDDED_TOKENIZER,
            EMBEDDED_NAME.to_string(),
        )
    }

    /// The static embedder for this run: the embedded model, unless
    /// [`STATIC_MODEL_ENV`] points at another one (bench sweeps, evaluating a
    /// candidate before it is shipped).
    pub fn ready() -> Result<Option<Self>> {
        match std::env::var(STATIC_MODEL_ENV) {
            Ok(p) if !p.is_empty() => Self::load(Path::new(&p)).map(Some),
            _ => Self::embedded().map(Some),
        }
    }

    /// Mean of the text's token rows, L2-normalised. Summation runs in token
    /// order over a fixed row layout, so the result is reproducible.
    fn pool(&self, text: &str) -> Result<Vec<f32>> {
        let enc = self
            .tokenizer
            .encode_fast(text, false)
            .map_err(|e| anyhow::anyhow!("tokenize: {e}"))?;
        let mut acc = vec![0.0f32; self.dim];
        let vocab = self.rows.len() / self.dim;
        for &id in enc.get_ids() {
            let id = id as usize;
            if id >= vocab {
                continue;
            }
            let row = &self.rows[id * self.dim..(id + 1) * self.dim];
            for (a, &v) in acc.iter_mut().zip(row) {
                *a += v;
            }
        }
        l2_normalize(&mut acc);
        canonicalize_f16(&mut acc);
        Ok(acc)
    }
}

impl EmbeddingModel for StaticEmbedder {
    fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        texts.iter().map(|t| self.pool(t)).collect()
    }

    fn dim(&self) -> usize {
        self.dim
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn fingerprint(&self) -> &str {
        &self.fingerprint
    }
}

/// Widen a tensor of any width the distillation may emit into f32. An int8
/// table needs its dequantisation step; without one it cannot be read, because
/// silently treating the codes as values would produce a wrong space.
fn decode_rows(view: &safetensors::tensor::TensorView<'_>, scale: Option<f32>) -> Result<Vec<f32>> {
    use safetensors::Dtype;
    let raw = view.data();
    Ok(match view.dtype() {
        Dtype::I8 => {
            let scale = scale.context("int8 token-embedding table without a scale tensor")?;
            raw.iter().map(|&b| b as i8 as f32 * scale).collect()
        }
        Dtype::F32 => raw
            .chunks_exact(4)
            .map(|c| f32::from_le_bytes([c[0], c[1], c[2], c[3]]))
            .collect(),
        Dtype::F16 => raw
            .chunks_exact(2)
            .map(|c| f32::from(half::f16::from_le_bytes([c[0], c[1]])))
            .collect(),
        Dtype::BF16 => raw
            .chunks_exact(2)
            .map(|c| f32::from(half::bf16::from_le_bytes([c[0], c[1]])))
            .collect(),
        other => bail!("unsupported token-embedding dtype {other:?}"),
    })
}

fn fingerprint_of(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    h.update(bytes);
    h.finalize().iter().map(|b| format!("{b:02x}")).collect()
}

fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

fn canonicalize_f16(v: &mut [f32]) {
    for x in v.iter_mut() {
        *x = f32::from(half::f16::from_f32(*x));
    }
}

#[cfg(test)]
mod tests;
