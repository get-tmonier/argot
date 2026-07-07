//! The embedder — llama.cpp (statically linked via `llama-cpp-2`) running the
//! pinned jina-code Q4 GGUF in-process to turn a function's source into a
//! 768-d unit vector.
//!
//! Design notes (validated in the P0 spike, `.scratch/semantic-layer/P0-*`):
//! - Pooling is llama.cpp's own `Mean` — it masks pad tokens internally, so the
//!   E1 pad-token pooling bug cannot recur. Parity vs the reference engine was
//!   cosine 1.0.
//! - Metal on macOS (~20 ms/fn warm); CPU elsewhere (2–4×, still sub-second).
//! - The GGUF is **not** embedded in the binary. It is fetched-on-first-use from
//!   a pinned argot-owned asset, verified by sha256, and cached. This keeps the
//!   binary small while the semantic layer stays always-on: if the model can't
//!   be obtained (offline/air-gapped), the caller degrades to the base guardrail
//!   rather than failing — semantic is standard behaviour, not an opt-in, but it
//!   is never load-bearing for argot's core correctness.

use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use llama_cpp_2::context::params::{LlamaContextParams, LlamaPoolingType};
use llama_cpp_2::llama_backend::LlamaBackend;
use llama_cpp_2::llama_batch::LlamaBatch;
use llama_cpp_2::model::params::LlamaModelParams;
use llama_cpp_2::model::{AddBos, LlamaModel};

/// Embedding dimensionality of jina-embeddings-v2-base-code.
pub const EMBED_DIM: usize = 768;

/// jina-code's trained context length; longer functions are truncated by the
/// tokenizer at this bound (rare for a single function).
const N_CTX: u32 = 8192;

/// The pinned model — `jina-embeddings-v2-base-code`, Q4_K_M GGUF. These exact
/// bytes cleared parity 1.0 in the P0 spike; the sha256 is the ollama blob name.
const MODEL_FILENAME: &str = "jina-embeddings-v2-base-code-Q4_K_M.gguf";
const MODEL_SHA256: &str = "1cea691a59c9aeb48f5a95d631f51a8f67850eb6638398c88343de8a6815b496";

/// Pinned download URL: an argot-owned release asset mirroring the exact bytes
/// (chosen over the community `gandolfi/` HF repo so we own availability +
/// integrity). The release process must upload this file under this tag.
const MODEL_URL: &str = "https://github.com/get-tmonier/argot/releases/download/semantic-model-v1/jina-embeddings-v2-base-code-Q4_K_M.gguf";

/// Env override for the model path — used by tests, offline installs, and CI to
/// point at a local GGUF instead of downloading. Highest-priority source.
const MODEL_ENV: &str = "ARGOT_SEMANTIC_MODEL";

/// The process-global llama.cpp backend. `LlamaBackend::init` must run exactly
/// once per process. We suppress *all* llama.cpp **and** ggml (Metal) logging
/// before anything initialises — `void_logs` only silences the llama channel and
/// leaks the ggml-metal device banner, whereas `send_logs_to_tracing(disabled)`
/// registers a dropping callback on both channels — so no backend chatter ever
/// reaches argot's stderr.
fn backend() -> Result<&'static LlamaBackend> {
    static BACKEND: OnceLock<LlamaBackend> = OnceLock::new();
    if let Some(b) = BACKEND.get() {
        return Ok(b);
    }
    llama_cpp_2::send_logs_to_tracing(llama_cpp_2::LogOptions::default().with_logs_enabled(false));
    let b = LlamaBackend::init().context("initialise llama.cpp backend")?;
    // If another thread raced us, the loser's backend is dropped — harmless.
    Ok(BACKEND.get_or_init(|| b))
}

/// A loaded embedder. Holds the model resident; a fresh inference context is
/// created per [`Self::embed`] call and reused across that call's batch (the
/// ~21 MiB compute buffer amortises over every function in one fit/check).
pub struct Embedder {
    model: LlamaModel,
}

impl Embedder {
    /// Load an embedder from an explicit GGUF path (already present on disk).
    pub fn load(model_path: &Path) -> Result<Self> {
        if !model_path.exists() {
            bail!("model file not found: {}", model_path.display());
        }
        let backend = backend()?;
        // Offload all layers to the GPU when a GPU backend is compiled in
        // (Metal on macOS); a no-op on CPU-only builds.
        let params = LlamaModelParams::default().with_n_gpu_layers(999);
        let model = LlamaModel::load_from_file(backend, model_path, &params)
            .with_context(|| format!("load GGUF model: {}", model_path.display()))?;
        Ok(Self { model })
    }

    /// Resolve the pinned model (env override → cache → fetch-on-first-use with
    /// sha256 verification) and load it. `Ok(None)` means the model is genuinely
    /// unavailable (e.g. offline and not yet cached) — the caller should degrade
    /// to the base guardrail. `Err` is reserved for real faults (corrupt cache
    /// that also can't be re-fetched, load failure on a verified file).
    pub fn ready() -> Result<Option<Self>> {
        match resolve_model_path()? {
            Some(path) => Ok(Some(Self::load(&path)?)),
            None => Ok(None),
        }
    }

    /// Embed each text into an L2-normalised 768-d vector, order-preserving.
    pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let backend = backend()?;
        // jina-code is an *encoder*: llama.cpp processes the whole sequence in a
        // single ubatch and asserts `n_ubatch >= n_tokens`, so n_batch/n_ubatch
        // must cover the longest function we embed (the context length). The
        // default 512 crashes on any function longer than that.
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(N_CTX))
            .with_n_batch(N_CTX)
            .with_n_ubatch(N_CTX)
            .with_embeddings(true)
            .with_pooling_type(LlamaPoolingType::Mean);
        let mut ctx = self
            .model
            .new_context(backend, ctx_params)
            .context("create embedding context")?;

        let mut out = Vec::with_capacity(texts.len());
        for text in texts {
            // A single-sequence batch per text: no padding, so the mean pool is
            // over exactly the real tokens.
            let mut tokens = self
                .model
                .str_to_token(text, AddBos::Always)
                .context("tokenize")?;
            // Defensive: never exceed the context window (a pathologically long
            // function is truncated rather than crashing the encoder assert).
            tokens.truncate(N_CTX as usize);
            let n = tokens.len().max(1);
            let mut batch = LlamaBatch::new(n, 1);
            batch.add_sequence(&tokens, 0, false)?;
            ctx.clear_kv_cache();
            ctx.decode(&mut batch).context("decode")?;
            let mut vec = ctx
                .embeddings_seq_ith(0)
                .context("read pooled embedding")?
                .to_vec();
            l2_normalize(&mut vec);
            out.push(vec);
        }
        Ok(out)
    }

    /// Embed a single text (convenience over [`Self::embed`]).
    pub fn embed_one(&self, text: &str) -> Result<Vec<f32>> {
        Ok(self.embed(&[text])?.pop().expect("one input, one output"))
    }
}

/// Normalise a vector to unit L2 length in place (cosine == dot after this).
fn l2_normalize(v: &mut [f32]) {
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in v.iter_mut() {
            *x /= norm;
        }
    }
}

/// Where the fetched GGUF is cached, matching argot's existing `~/.cache/argot`
/// convention (XDG on Linux, `%LOCALAPPDATA%` on Windows).
fn cache_dir() -> Result<PathBuf> {
    if let Ok(x) = std::env::var("XDG_CACHE_HOME") {
        if !x.is_empty() {
            return Ok(PathBuf::from(x).join("argot"));
        }
    }
    #[cfg(target_os = "windows")]
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.is_empty() {
            return Ok(PathBuf::from(local).join("argot"));
        }
    }
    let home = std::env::var("HOME").context("neither XDG_CACHE_HOME nor HOME is set")?;
    Ok(PathBuf::from(home).join(".cache").join("argot"))
}

/// Resolve the model path: env override → verified cache → download. Returns
/// `Ok(None)` when the model isn't present and can't be fetched (offline).
fn resolve_model_path() -> Result<Option<PathBuf>> {
    // 1. Explicit override (tests / offline / CI): trust the path as given.
    if let Ok(p) = std::env::var(MODEL_ENV) {
        if !p.is_empty() {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(Some(path));
            }
            bail!("{MODEL_ENV} points at a missing file: {}", path.display());
        }
    }

    // 2. Cache hit (verify integrity before trusting it).
    let cached = cache_dir()?.join("models").join(MODEL_FILENAME);
    if cached.exists() {
        if sha256_file(&cached)? == MODEL_SHA256 {
            return Ok(Some(cached));
        }
        // Corrupt/partial: remove and fall through to re-fetch.
        let _ = std::fs::remove_file(&cached);
    }

    // 3. Fetch-on-first-use. A network failure is a graceful degrade (None),
    // not an error — the base guardrail still runs.
    match download_model(&cached) {
        Ok(()) => Ok(Some(cached)),
        Err(_) => Ok(None),
    }
}

/// Download the pinned GGUF to `dest`, streaming to a temp file, verifying the
/// sha256, then atomically renaming into place.
fn download_model(dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).context("create model cache dir")?;
    }
    let tmp = dest.with_extension("gguf.partial");

    eprintln!("argot: downloading semantic model (~90 MB, one-time)…");
    let resp = ureq::get(MODEL_URL).call().context("fetch model")?;
    let mut reader = resp.into_reader();
    {
        let mut file = std::fs::File::create(&tmp).context("create temp model file")?;
        std::io::copy(&mut reader, &mut file).context("stream model to disk")?;
    }

    let got = sha256_file(&tmp)?;
    if got != MODEL_SHA256 {
        let _ = std::fs::remove_file(&tmp);
        bail!("downloaded model sha256 mismatch: got {got}, expected {MODEL_SHA256}");
    }
    std::fs::rename(&tmp, dest).context("install model into cache")?;
    Ok(())
}

/// Hex sha256 of a file.
fn sha256_file(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let mut file = std::fs::File::open(path).context("open file for hashing")?;
    let mut hasher = Sha256::new();
    std::io::copy(&mut file, &mut hasher).context("hash file")?;
    Ok(hasher
        .finalize()
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Cosine of two equal-length vectors.
    fn cosine(a: &[f32], b: &[f32]) -> f32 {
        a.iter().zip(b).map(|(x, y)| x * y).sum()
    }

    /// Load an embedder if a model is locally available (env or verified cache);
    /// otherwise `None` so the test skips instead of downloading 90 MB in CI.
    fn local_embedder() -> Option<Embedder> {
        resolve_model_path().ok().flatten().and_then(|p| {
            // Only use a real, verified local file — never trigger a download.
            Embedder::load(&p).ok()
        })
    }

    #[test]
    fn l2_normalize_makes_unit_vectors() {
        let mut v = vec![3.0f32, 4.0];
        l2_normalize(&mut v);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-6);
    }

    #[test]
    fn embed_shape_and_semantics() {
        let Some(emb) = local_embedder() else {
            eprintln!("skipping: no local model (set {MODEL_ENV})");
            return;
        };
        let dup = "def add(a, b):\n    return a + b\n";
        let same = "def add(a, b):\n    return a + b\n";
        let diff = "class HttpRetryPolicy:\n    def backoff(self, n):\n        return 2 ** n\n";

        let vecs = emb.embed(&[dup, same, diff]).unwrap();
        assert_eq!(vecs.len(), 3);
        for v in &vecs {
            assert_eq!(v.len(), EMBED_DIM);
            let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
            assert!((norm - 1.0).abs() < 1e-4, "vectors are L2-normalised");
        }
        // Identical inputs → cosine ~1; unrelated code → clearly lower.
        let self_cos = cosine(&vecs[0], &vecs[1]);
        let cross_cos = cosine(&vecs[0], &vecs[2]);
        assert!(
            self_cos > 0.999,
            "identical code near-identical: {self_cos}"
        );
        assert!(
            cross_cos < self_cos - 0.05,
            "unrelated code separates: self={self_cos} cross={cross_cos}"
        );
    }

    #[test]
    fn embeds_a_long_function_without_crashing() {
        // Regression: jina-code is an encoder; llama.cpp asserts
        // `n_ubatch >= n_tokens`, so a function longer than the default 512-token
        // ubatch used to crash. Build a body well over that.
        let Some(emb) = local_embedder() else {
            eprintln!("skipping: no local model (set {MODEL_ENV})");
            return;
        };
        let mut body = String::from("def huge():\n");
        for i in 0..1200 {
            body.push_str(&format!(
                "    value_{i} = compute_something({i}) + offset\n"
            ));
        }
        let v = emb.embed_one(&body).unwrap();
        assert_eq!(v.len(), EMBED_DIM);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4);
    }
}
