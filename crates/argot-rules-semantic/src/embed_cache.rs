//! The machine-wide embedding cache (`~/.cache/argot/embeddings/<model>/`) —
//! content-addressed function embeddings keyed by (embedding model, embed-text
//! hash), shared across repos, clones, and argot's own throwaway worktrees.
//!
//! Why it exists: the per-repo `.argot/semantic-index.json` already lets a
//! refit reuse unchanged functions' vectors, but that reuse only flows through
//! the one checkout that holds the artifact. A fresh clone — or the temp
//! worktree `argot audit` fits in — re-embeds every function from scratch
//! (~29 ms each; hundreds of seconds on a large corpus). Embeddings are pure
//! functions of (model bytes, embed text), so they are cacheable globally:
//! this cache turns any repeat encounter with the same function body on the
//! same machine into a lookup.
//!
//! Correctness: the cache directory is namespaced by the model's sha256, and
//! [`EmbeddingModel::embed`](super::embedding::EmbeddingModel::embed) canonicalises every
//! vector to f16 precision — exactly the precision the on-disk artifact and
//! this cache store — so a cache hit is *bit-identical* to a fresh embed and
//! can never change a finding.
//!
//! Storage: immutable fixed-record segment files (`seg-<ts>-<pid>-<n>.bin`,
//! written via temp + rename), each record `[8-byte key][768 × f16 LE]`.
//! Concurrent argot processes each write their own segment — no locking, no
//! torn writes; duplicate records across segments are harmless (same bytes).
//! A size cap evicts the oldest segments so the cache never grows unbounded.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use half::f16;

/// One record: the 8-byte content key + the f16 vector.
const KEY_BYTES: usize = 8;
/// On-disk record width, set from the model whose vectors the cache holds.
const EMBED_DIM: usize = 256;
const VEC_BYTES: usize = EMBED_DIM * 2;
const RECORD_BYTES: usize = KEY_BYTES + VEC_BYTES;

/// Cache size cap per model directory. Generous: the largest bench corpus
/// (rocksdb, ~29k functions) writes ~45 MB, so this holds ~10 such repos.
const CAP_BYTES: u64 = 512 * 1024 * 1024;

/// A loaded view of the cache for the pinned model: an in-memory map of every
/// record on disk, plus the directory to append new segments to.
pub struct EmbedCache {
    dir: PathBuf,
    map: HashMap<[u8; KEY_BYTES], Vec<f32>>,
}

impl EmbedCache {
    /// Open the cache for the pinned model under the standard cache root.
    /// `None` when no cache root can be resolved (no HOME) — callers just
    /// skip caching. An empty/absent directory is a valid (empty) cache.
    pub fn open_current() -> Option<Self> {
        let dir = argot_engine::cache::cache_dir()
            .ok()?
            .join("embeddings")
            .join(model_namespace()?);
        Some(Self::open_at(dir))
    }

    /// Open a cache rooted at an explicit directory (tests; the production
    /// path is [`Self::open_current`]).
    pub fn open_at(dir: PathBuf) -> Self {
        let mut map = HashMap::new();
        for path in segment_files(&dir) {
            let Ok(bytes) = std::fs::read(&path) else {
                continue;
            };
            // A trailing partial record (a crashed writer's tail) is ignored;
            // every whole record is valid by construction.
            for rec in bytes.chunks_exact(RECORD_BYTES) {
                let mut key = [0u8; KEY_BYTES];
                key.copy_from_slice(&rec[..KEY_BYTES]);
                let vec: Vec<f32> = rec[KEY_BYTES..]
                    .chunks_exact(2)
                    .map(|b| f16::from_le_bytes([b[0], b[1]]).to_f32())
                    .collect();
                map.insert(key, vec);
            }
        }
        Self { dir, map }
    }

    /// How many embeddings the cache holds.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    /// Look up a vector by its embed-text hash (the 16-hex-char key produced
    /// by [`super::index::embed_text_hash`]). Non-hex keys never match.
    pub fn get(&self, hash_hex: &str) -> Option<&Vec<f32>> {
        self.map.get(&decode_key(hash_hex)?)
    }

    /// Persist `entries` (hash, canonical vector) that the cache doesn't hold
    /// yet, as one new immutable segment. Best-effort: an unwritable cache
    /// directory silently skips (caching is an optimisation, never
    /// load-bearing). Evicts oldest segments first when over the size cap.
    pub fn persist(&self, entries: &[(String, Vec<f32>)]) {
        let mut payload = Vec::new();
        for (hash, vec) in entries {
            let Some(key) = decode_key(hash) else {
                continue;
            };
            if self.map.contains_key(&key) || vec.len() != EMBED_DIM {
                continue;
            }
            payload.extend_from_slice(&key);
            for &x in vec {
                payload.extend_from_slice(&f16::from_f32(x).to_le_bytes());
            }
        }
        if payload.is_empty() {
            return;
        }
        if std::fs::create_dir_all(&self.dir).is_err() {
            return;
        }
        gc_to_cap(&self.dir, CAP_BYTES.saturating_sub(payload.len() as u64));
        // Timestamped name keeps eviction order == write order; pid + a
        // random suffix keeps concurrent writers apart.
        let ts = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        let name = format!("seg-{ts:012}-{}.bin", std::process::id());
        let tmp = self.dir.join(format!("{name}.tmp"));
        let dst = self.dir.join(name);
        if std::fs::write(&tmp, &payload).is_ok() {
            let _ = std::fs::rename(&tmp, &dst);
        }
    }
}

/// Embed `texts` through the cache: serve every text whose vector is already
/// cached, embed only the misses, and persist those for the next encounter.
/// Order-preserving; with `cache: None` it is exactly `embedder.embed`.
/// Because [`EmbeddingModel::embed`](super::embedding::EmbeddingModel::embed) canonicalises
/// to the cache's f16 precision, a hit is bit-identical to a fresh embed.
pub fn embed_with_cache(
    embedder: &dyn super::embedding::EmbeddingModel,
    texts: &[&str],
    cache: Option<&EmbedCache>,
) -> anyhow::Result<Vec<Vec<f32>>> {
    let Some(cache) = cache else {
        return embedder.embed(texts);
    };
    let hashes: Vec<String> = texts
        .iter()
        .map(|t| super::index::embed_text_hash(t))
        .collect();
    let misses: Vec<usize> = (0..texts.len())
        .filter(|&i| cache.get(&hashes[i]).is_none())
        .collect();
    let miss_texts: Vec<&str> = misses.iter().map(|&i| texts[i]).collect();
    let mut fresh = embedder.embed(&miss_texts)?.into_iter();

    let mut out = Vec::with_capacity(texts.len());
    let mut persist: Vec<(String, Vec<f32>)> = Vec::with_capacity(misses.len());
    for hash in &hashes {
        match cache.get(hash) {
            Some(v) => out.push(v.clone()),
            None => {
                let v = fresh.next().expect("one fresh vector per miss");
                persist.push((hash.clone(), v.clone()));
                out.push(v);
            }
        }
    }
    cache.persist(&persist);
    Ok(out)
}

/// Decode a 16-hex-char hash into the 8-byte record key.
fn decode_key(hash_hex: &str) -> Option<[u8; KEY_BYTES]> {
    if hash_hex.len() != KEY_BYTES * 2 {
        return None;
    }
    let mut key = [0u8; KEY_BYTES];
    for (i, chunk) in hash_hex.as_bytes().chunks_exact(2).enumerate() {
        let hi = (chunk[0] as char).to_digit(16)?;
        let lo = (chunk[1] as char).to_digit(16)?;
        key[i] = ((hi << 4) | lo) as u8;
    }
    Some(key)
}

/// The cache's segment files, sorted by name (== write order: names begin
/// with a zero-padded timestamp).
fn segment_files(dir: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    let mut files: Vec<PathBuf> = entries
        .flatten()
        .map(|e| e.path())
        .filter(|p| {
            p.is_file()
                && p.file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("seg-") && n.ends_with(".bin"))
        })
        .collect();
    files.sort();
    files
}

/// Delete oldest segments until the directory fits in `cap` bytes.
fn gc_to_cap(dir: &Path, cap: u64) {
    let files = segment_files(dir);
    let mut sizes: Vec<(PathBuf, u64)> = files
        .into_iter()
        .map(|p| {
            let len = std::fs::metadata(&p).map(|m| m.len()).unwrap_or(0);
            (p, len)
        })
        .collect();
    let mut total: u64 = sizes.iter().map(|(_, l)| l).sum();
    sizes.reverse(); // pop() yields oldest first
    while total > cap {
        let Some((oldest, len)) = sizes.pop() else {
            break;
        };
        if std::fs::remove_file(&oldest).is_ok() {
            total = total.saturating_sub(len);
        }
    }
}

#[cfg(test)]
mod tests;

/// Cache namespace: the fingerprint of the model whose vectors it holds, so a
/// model change starts a fresh namespace instead of mixing two spaces.
fn model_namespace() -> Option<String> {
    use super::embedding::EmbeddingModel;
    let m = super::static_embedder::StaticEmbedder::embedded().ok()?;
    Some(m.fingerprint()[..16].to_string())
}
