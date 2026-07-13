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

/// Sequences packed into one decode. At the ~150-token average function this
/// is what actually bounds a pack (64 × 150 ≈ 9.6k > the 8192-token budget),
/// so most packs fill the token budget; the cap only guards the many-tiny-
/// functions case from unbounded per-decode sequence bookkeeping.
const MAX_BATCH_SEQS: u32 = 64;

/// The pinned model — `jina-embeddings-v2-base-code`, Q4_K_M GGUF. These exact
/// bytes cleared parity 1.0 in the P0 spike; the sha256 is the ollama blob name.
pub const MODEL_NAME: &str = "jina-embeddings-v2-base-code";
pub const MODEL_FILENAME: &str = "jina-embeddings-v2-base-code-Q4_K_M.gguf";
pub const MODEL_SHA256: &str = "1cea691a59c9aeb48f5a95d631f51a8f67850eb6638398c88343de8a6815b496";

/// Pinned download URL: an argot-owned release asset mirroring the exact bytes
/// (chosen over the community `gandolfi/` HF repo so we own availability +
/// integrity). The release process must upload this file under this tag —
/// CI verifies the asset exists and matches [`MODEL_SHA256`].
const MODEL_URL: &str = "https://github.com/get-tmonier/argot/releases/download/semantic-model-v1/jina-embeddings-v2-base-code-Q4_K_M.gguf";

/// Env override for the model path — used by tests, offline installs, and CI to
/// point at a local GGUF instead of downloading. Highest-priority source.
pub const MODEL_ENV: &str = "ARGOT_SEMANTIC_MODEL";

/// Env kill-switch for all network access: when truthy (set, non-empty, not
/// `0`), argot never attempts a download — a missing model degrades with a
/// clear note instead of touching the network. Pattern: `HF_HUB_OFFLINE`.
pub const OFFLINE_ENV: &str = "ARGOT_OFFLINE";

/// Env override for the download URL (corporate mirrors / artifactory). The
/// downloaded bytes must still match [`MODEL_SHA256`] — the mirror changes
/// *where* the model comes from, never *what* it is.
pub const MODEL_URL_ENV: &str = "ARGOT_MODEL_URL";

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
    ///
    /// Vectors are canonicalised to f16 precision before returning — exactly
    /// the precision the on-disk index artifact and the machine-wide embed
    /// cache store — so a vector is bit-identical whether it was computed this
    /// run, reloaded from the artifact, or served from the cache.
    ///
    /// Throughput: texts are packed several sequences per decode (jina-code is
    /// an encoder; llama.cpp mean-pools each sequence independently under its
    /// per-sequence attention mask), which amortises the per-decode kernel
    /// launch overhead that dominates when functions are short. Measured ~4×
    /// on Metal vs one-decode-per-text at ~150-token average functions.
    pub fn embed(&self, texts: &[&str]) -> Result<Vec<Vec<f32>>> {
        let backend = backend()?;
        // jina-code is an *encoder*: llama.cpp processes a whole batch in a
        // single ubatch and asserts `n_ubatch >= n_tokens`, so n_batch/n_ubatch
        // must cover everything packed into one decode (the context length).
        // The default 512 crashes on any function longer than that.
        let ctx_params = LlamaContextParams::default()
            .with_n_ctx(std::num::NonZeroU32::new(N_CTX))
            .with_n_batch(N_CTX)
            .with_n_ubatch(N_CTX)
            .with_n_seq_max(MAX_BATCH_SEQS)
            .with_embeddings(true)
            .with_pooling_type(LlamaPoolingType::Mean);
        let mut ctx = self
            .model
            .new_context(backend, ctx_params)
            .context("create embedding context")?;

        let mut out = Vec::with_capacity(texts.len());
        // A text tokenized for the current pack but not fitting it — carried
        // into the next pack so it's never tokenized twice.
        let mut pending: Option<Vec<llama_cpp_2::token::LlamaToken>> = None;
        let mut idx = 0;
        while idx < texts.len() {
            // Greedily pack sequences until the token budget or the sequence
            // cap is reached. Each sequence keeps its own id: no padding, so
            // the mean pool is over exactly that sequence's real tokens.
            let mut seqs: Vec<Vec<llama_cpp_2::token::LlamaToken>> = Vec::new();
            let mut total = 0usize;
            while idx < texts.len() && seqs.len() < MAX_BATCH_SEQS as usize {
                let tokens = match pending.take() {
                    Some(t) => t,
                    None => {
                        let mut t = self
                            .model
                            .str_to_token(texts[idx], AddBos::Always)
                            .context("tokenize")?;
                        // Defensive: never exceed the context window (a
                        // pathologically long function is truncated rather
                        // than crashing the encoder assert).
                        t.truncate(N_CTX as usize);
                        t
                    }
                };
                if !seqs.is_empty() && total + tokens.len() > N_CTX as usize {
                    pending = Some(tokens);
                    break;
                }
                total += tokens.len();
                seqs.push(tokens);
                idx += 1;
            }
            let mut batch = LlamaBatch::new(total.max(1), seqs.len() as i32);
            for (si, tokens) in seqs.iter().enumerate() {
                batch.add_sequence(tokens, si as i32, false)?;
            }
            ctx.clear_kv_cache();
            ctx.decode(&mut batch).context("decode")?;
            for si in 0..seqs.len() {
                let mut vec = ctx
                    .embeddings_seq_ith(si as i32)
                    .context("read pooled embedding")?
                    .to_vec();
                l2_normalize(&mut vec);
                canonicalize_f16(&mut vec);
                out.push(vec);
            }
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

/// Round every component to its nearest f16 in place — the canonical
/// precision of argot's embedding space. The index artifact and the embed
/// cache both store f16, so canonicalising at the source makes "freshly
/// computed", "reloaded", and "cache hit" the same bits: a cache or artifact
/// round-trip can never move a cosine.
fn canonicalize_f16(v: &mut [f32]) {
    for x in v.iter_mut() {
        *x = half::f16::from_f32(*x).to_f32();
    }
}

use crate::cache::cache_dir;

/// The release tag the pinned model ships under (`semantic-model-v1`) — the
/// update notice compares it against the published `version.json` to announce
/// a model change after `argot update`.
pub fn model_tag() -> &'static str {
    MODEL_URL
        .split('/')
        .rev()
        .nth(1)
        .expect("MODEL_URL has a tag segment")
}

/// The directory the fetched model lives in (`<cache>/models`). Public so the
/// CLI (`argot model status`) and docs name the same real path.
pub fn models_dir() -> Result<PathBuf> {
    Ok(cache_dir()?.join("models"))
}

/// Is `ARGOT_OFFLINE` truthy (set, non-empty, not `0`)?
fn offline() -> bool {
    std::env::var(OFFLINE_ENV)
        .map(|v| !v.is_empty() && v != "0")
        .unwrap_or(false)
}

/// The effective download URL (`ARGOT_MODEL_URL` mirror override, else the
/// pinned release asset).
fn model_url() -> String {
    std::env::var(MODEL_URL_ENV)
        .ok()
        .filter(|u| !u.is_empty())
        .unwrap_or_else(|| MODEL_URL.to_string())
}

/// The advice tail appended to every "model unavailable" note.
const RETRY_HINT: &str =
    "retry with `argot model fetch`, or set ARGOT_SEMANTIC_MODEL to a local GGUF";

/// Resolve the model path: env override → verified cache → download. Returns
/// `Ok(None)` when the model isn't present and can't be fetched — ALWAYS after
/// printing a one-line reason on stderr (degradation is loud, never a silent
/// zero: the user must know the redundant/misplaced rules did not run).
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
    let cached = models_dir()?.join(MODEL_FILENAME);
    if cached.exists() {
        if sha256_file(&cached)? == MODEL_SHA256 {
            return Ok(Some(cached));
        }
        // Corrupt/partial: remove and fall through to re-fetch.
        eprintln!("argot: cached model failed its sha256 check — re-fetching");
        let _ = std::fs::remove_file(&cached);
    }

    // 3. Fetch-on-first-use — unless the user said "never touch the network".
    if offline() {
        eprintln!(
            "argot: {OFFLINE_ENV} is set and no cached model — \
             redundant/misplaced checks skipped this run"
        );
        return Ok(None);
    }
    // A network failure is a graceful degrade (None), not an error — the base
    // guardrail still runs — but the reason is always printed.
    match download_model(&cached) {
        Ok(()) => Ok(Some(cached)),
        Err(e) => {
            eprintln!(
                "argot: semantic model download failed ({e:#}) — \
                 redundant/misplaced checks skipped this run; {RETRY_HINT}"
            );
            Ok(None)
        }
    }
}

/// Explicit pre-download for `argot model fetch`: resolves like the automatic
/// path but treats every failure as a hard error (a user who asked for the
/// model wants the real cause, not a degrade). Returns the cached path.
pub fn fetch_model() -> Result<PathBuf> {
    if let Ok(p) = std::env::var(MODEL_ENV) {
        if !p.is_empty() {
            let path = PathBuf::from(p);
            if path.exists() {
                return Ok(path);
            }
            bail!("{MODEL_ENV} points at a missing file: {}", path.display());
        }
    }
    let cached = models_dir()?.join(MODEL_FILENAME);
    if cached.exists() && sha256_file(&cached)? == MODEL_SHA256 {
        return Ok(cached);
    }
    if offline() {
        bail!("{OFFLINE_ENV} is set — unset it to allow the download");
    }
    download_model(&cached)?;
    Ok(cached)
}

/// Where `argot model status` finds the model, if anywhere.
pub enum ModelStatus {
    /// `ARGOT_SEMANTIC_MODEL` points at this file (trusted as-is).
    EnvOverride(PathBuf),
    /// Verified in the cache.
    Cached { path: PathBuf, size_bytes: u64 },
    /// Not present — fetched on first use (or via `argot model fetch`).
    Absent,
}

/// Report the model's presence without ever touching the network.
pub fn model_status() -> Result<ModelStatus> {
    if let Ok(p) = std::env::var(MODEL_ENV) {
        if !p.is_empty() {
            return Ok(ModelStatus::EnvOverride(PathBuf::from(p)));
        }
    }
    let cached = models_dir()?.join(MODEL_FILENAME);
    if cached.exists() && sha256_file(&cached)? == MODEL_SHA256 {
        let size_bytes = std::fs::metadata(&cached).map(|m| m.len()).unwrap_or(0);
        return Ok(ModelStatus::Cached {
            path: cached,
            size_bytes,
        });
    }
    Ok(ModelStatus::Absent)
}

/// Delete everything under the model cache (`argot model clean`) — stale
/// models from older argot versions, orphaned partial downloads, and the
/// current model. Returns (files removed, bytes freed).
pub fn clean_models() -> Result<(usize, u64)> {
    let dir = models_dir()?;
    let mut files = 0usize;
    let mut bytes = 0u64;
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Ok((0, 0));
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_file() {
            bytes += entry.metadata().map(|m| m.len()).unwrap_or(0);
            std::fs::remove_file(&path).with_context(|| format!("remove {}", path.display()))?;
            files += 1;
        }
    }
    Ok((files, bytes))
}

/// Drop cache-dir siblings the current model made obsolete: any other `*.gguf`
/// (a previous model version) and any orphaned `*.partial*` temp file. Runs
/// after a successful install so the cache never accumulates dead ~100 MB
/// files across model upgrades.
fn gc_stale_cache_files(dir: &Path, keep: &Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path == keep || !path.is_file() {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.ends_with(".gguf") || name.contains(".partial") {
            let _ = std::fs::remove_file(&path);
        }
    }
}

/// Standard Cache Directory Tagging signature — tells backup tools the
/// directory is regenerable and safe to skip (<https://bford.info/cachedir/>).
fn write_cachedir_tag(cache_root: &Path) {
    let tag = cache_root.join("CACHEDIR.TAG");
    if tag.exists() {
        return;
    }
    let _ = std::fs::write(
        &tag,
        "Signature: 8a477f597d28d172789f06886806bc55\n\
         # This directory holds argot's regenerable cache (semantic model).\n\
         # Everything here is re-fetched on demand — safe to delete or skip in backups.\n",
    );
}

/// HTTP agent for the model download: bounded connect/read timeouts (a stalled
/// proxy must degrade, never hang the fit) and standard proxy-env support
/// (`HTTPS_PROXY` / `HTTP_PROXY` / `ALL_PROXY`).
fn download_agent() -> ureq::Agent {
    let mut builder = ureq::AgentBuilder::new()
        .timeout_connect(std::time::Duration::from_secs(10))
        .timeout_read(std::time::Duration::from_secs(60));
    let proxy_env = [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
    ]
    .iter()
    .find_map(|k| std::env::var(k).ok().filter(|v| !v.is_empty()));
    if let Some(p) = proxy_env {
        if let Ok(proxy) = ureq::Proxy::new(p) {
            builder = builder.proxy(proxy);
        }
    }
    builder.build()
}

/// Download the pinned GGUF to `dest`: announce what/why/where up front,
/// stream to a PID-unique temp file (concurrent runs can't corrupt each
/// other), show progress on a tty, verify the sha256, then atomically rename
/// into place. One retry on a transient network failure.
fn download_model(dest: &Path) -> Result<()> {
    if let Some(parent) = dest.parent() {
        std::fs::create_dir_all(parent).context("create model cache dir")?;
        gc_stale_cache_files(parent, dest); // clear orphaned partials up front
        if let Some(cache_root) = parent.parent() {
            write_cachedir_tag(cache_root);
        }
    }
    let tmp = dest.with_extension(format!("gguf.partial.{}", std::process::id()));

    eprintln!(
        "argot: downloading {MODEL_NAME} (~100 MB, one-time) to {} — powers the redundant/misplaced rules",
        dest.parent().map(|p| p.display().to_string()).unwrap_or_default()
    );
    let mut last_err: Option<anyhow::Error> = None;
    for attempt in 0..2 {
        if attempt > 0 {
            eprintln!("argot: retrying download…");
        }
        match try_download(&tmp, dest) {
            Ok(()) => {
                if let Some(parent) = dest.parent() {
                    gc_stale_cache_files(parent, dest); // drop obsolete older models
                }
                eprintln!("argot: semantic model ready ({})", dest.display());
                return Ok(());
            }
            Err(e) => last_err = Some(e),
        }
    }
    let _ = std::fs::remove_file(&tmp);
    Err(last_err.expect("two attempts, at least one error"))
}

/// One download attempt: stream → verify → rename.
fn try_download(tmp: &Path, dest: &Path) -> Result<()> {
    let resp = download_agent()
        .get(&model_url())
        .call()
        .context("fetch model")?;
    let total: Option<u64> = resp.header("Content-Length").and_then(|v| v.parse().ok());
    let mut reader = resp.into_reader();
    {
        let mut file = std::fs::File::create(tmp).context("create temp model file")?;
        stream_with_progress(&mut reader, &mut file, total).context("stream model to disk")?;
    }

    let got = sha256_file(tmp)?;
    if got != MODEL_SHA256 {
        let _ = std::fs::remove_file(tmp);
        bail!("downloaded model sha256 mismatch: got {got}, expected {MODEL_SHA256}");
    }
    std::fs::rename(tmp, dest).context("install model into cache")?;
    Ok(())
}

/// `io::copy` with a live progress line on stderr when it's a tty (silent
/// otherwise — CI logs must not fill with carriage returns).
fn stream_with_progress(
    reader: &mut impl std::io::Read,
    file: &mut std::fs::File,
    total: Option<u64>,
) -> std::io::Result<()> {
    use std::io::{IsTerminal, Write};
    let show = std::io::stderr().is_terminal();
    let mut buf = vec![0u8; 1 << 20];
    let mut done: u64 = 0;
    let mut last_shown: u64 = 0;
    loop {
        let n = reader.read(&mut buf)?;
        if n == 0 {
            break;
        }
        file.write_all(&buf[..n])?;
        done += n as u64;
        // Repaint at most every 4 MB — enough to feel alive, cheap to render.
        if show && done - last_shown >= (4 << 20) {
            last_shown = done;
            let mb = done as f64 / (1024.0 * 1024.0);
            match total {
                Some(t) if t > 0 => {
                    let pct = (done as f64 / t as f64 * 100.0).min(100.0);
                    eprint!(
                        "\rargot: downloading… {mb:.0} / {:.0} MB ({pct:.0}%)",
                        t as f64 / (1024.0 * 1024.0)
                    );
                }
                _ => eprint!("\rargot: downloading… {mb:.0} MB"),
            }
            let _ = std::io::stderr().flush();
        }
    }
    if show && last_shown > 0 {
        eprintln!();
    }
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

    /// Load an embedder iff a model is already on disk (env override or a
    /// present cache file). Never calls `resolve_model_path` — a unit test must
    /// not hit the network, so on CI (no model, no download) it returns `None`
    /// and the model-dependent tests skip.
    fn local_embedder() -> Option<Embedder> {
        let path = std::env::var(MODEL_ENV)
            .ok()
            .map(PathBuf::from)
            .filter(|p| p.exists())
            .or_else(|| {
                let cached = cache_dir().ok()?.join("models").join(MODEL_FILENAME);
                cached.exists().then_some(cached)
            })?;
        Embedder::load(&path).ok()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("argot_embedder_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn gc_removes_stale_ggufs_and_partials_but_keeps_current() {
        let dir = scratch("gc");
        let keep = dir.join(MODEL_FILENAME);
        std::fs::write(&keep, b"current").unwrap();
        std::fs::write(dir.join("old-model-v0.gguf"), b"old").unwrap();
        std::fs::write(dir.join("x.gguf.partial.123"), b"orphan").unwrap();
        std::fs::write(dir.join("CACHEDIR.TAG"), b"tag").unwrap();
        gc_stale_cache_files(&dir, &keep);
        assert!(keep.exists(), "current model kept");
        assert!(!dir.join("old-model-v0.gguf").exists(), "old model gone");
        assert!(!dir.join("x.gguf.partial.123").exists(), "orphan gone");
        assert!(dir.join("CACHEDIR.TAG").exists(), "unrelated file kept");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn cachedir_tag_written_once_with_signature() {
        let dir = scratch("tag");
        write_cachedir_tag(&dir);
        let content = std::fs::read_to_string(dir.join("CACHEDIR.TAG")).unwrap();
        assert!(content.starts_with("Signature: 8a477f597d28d172789f06886806bc55"));
        // Idempotent: a hand-edited tag is left untouched.
        std::fs::write(dir.join("CACHEDIR.TAG"), "custom").unwrap();
        write_cachedir_tag(&dir);
        assert_eq!(
            std::fs::read_to_string(dir.join("CACHEDIR.TAG")).unwrap(),
            "custom"
        );
        let _ = std::fs::remove_dir_all(&dir);
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
    fn embed_output_is_f16_canonical_and_batching_matches_single() {
        let Some(emb) = local_embedder() else {
            eprintln!("skipping: no local model (set {MODEL_ENV})");
            return;
        };
        let texts = [
            "def one(a):\n    b = a + 1\n    return b\n",
            "def two(a):\n    b = a * 2\n    return b\n",
            "def three(a):\n    b = a - 3\n    return b\n",
        ];
        let batched = emb.embed(&texts).unwrap();
        for v in &batched {
            // Canonical: every component is exactly f16-representable, so the
            // artifact/cache round-trip is bit-identical.
            assert!(v.iter().all(|&x| x == half::f16::from_f32(x).to_f32()));
        }
        // Packing several sequences into one decode must not change what a
        // text embeds to (per-sequence attention masking): same direction as
        // embedding each text alone.
        for (i, text) in texts.iter().enumerate() {
            let solo = emb.embed_one(text).unwrap();
            let cos = cosine(&batched[i], &solo);
            assert!(cos > 0.999, "batch[{i}] vs solo cosine {cos}");
        }
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
