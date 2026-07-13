//! The `SemanticIndex` — a per-repo, per-language table of every function's
//! embedding plus its provenance (`symbol`, `path`, `line`). Built once at
//! fit-time (embed every corpus function) and queried at check-time (embed each
//! diff-defined function, ask for nearest neighbours / margin / area vote).
//!
//! All three semantic features read this one index:
//! - **F1 reinvention**: nearest cross-file neighbour + callee/subtoken confirm.
//! - **F2 placement**: the areas of the top-k cross-file neighbours.
//! - **F4 evidence**: the nearest existing functions, rendered on a finding.
//!
//! Storage: vectors are L2-normalised f32 in memory (so cosine is a dot product)
//! and packed as little-endian **f16** on disk — ~4 MB for 2.6k functions. The
//! index lives in its own `.argot/semantic-index.json` artifact so the base
//! `scorer-config.json` (and its model hash) stay byte-for-byte unchanged.
//!
//! Query is a brute-force scan: ~2.6k × 768 mults per query is sub-millisecond,
//! and diffs define only a handful of functions, so LSH/ANN would be premature.

use std::collections::{BTreeMap, HashMap};

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use half::f16;
use serde::{Deserialize, Serialize};

use super::embedder::{Embedder, EMBED_DIM};
use argot_lang::adapters::LanguageAdapter;

/// Artifact format version (bump on any breaking on-disk change).
/// v2: `area_norms` replaced by the self-calibrated `placement` block.
/// v3: the artifact records the embedding model's identity (name/sha256/dim)
///     and `validate_current` gates loading — an index built by a different
///     model or argot version is declared stale instead of silently producing
///     wrong cosines.
const ARTIFACT_VERSION: u32 = 3;

/// Functions shorter than this (in lines) are skipped when indexing: one- and
/// two-line bodies are boilerplate (getters, trivial wrappers) that only add
/// near-duplicate noise and never make a meaningful reinvention target.
const MIN_BODY_LINES: usize = 3;

/// One indexed function: its embedding plus where it lives, and two structural
/// fingerprints — its callees and its identifier subtokens — that turn a
/// near-embedding-match into a confident reinvention.
#[derive(Debug, Clone)]
pub struct IndexEntry {
    pub symbol: String,
    /// Repo-relative path, forward-slashed.
    pub path: String,
    /// 1-indexed definition line.
    pub line: usize,
    /// L2-normalised embedding (dot product == cosine).
    pub vec: Vec<f32>,
    /// Sorted, deduped callee names — used for callee-Jaccard confirmation.
    pub callees: Vec<String>,
    /// Sorted, deduped identifier subtokens — used for IDF-weighted subtoken
    /// Jaccard confirmation (the main reinvention-recall driver).
    pub subtokens: Vec<String>,
    /// Truncated sha256 of the embedded text — lets a refit reuse this entry's
    /// vector when the function body is unchanged (incremental refresh).
    /// Empty on artifacts written before the field existed (no reuse, full
    /// re-embed — correct, just slower once).
    pub text_hash: String,
}

/// A function to index or query: identity plus the source text to embed, and its
/// callee set (extracted once, at fit and at check, by the same path).
#[derive(Debug, Clone)]
pub struct FunctionRef {
    pub symbol: String,
    pub path: String,
    /// 1-indexed definition (start) line.
    pub line: usize,
    /// 1-indexed last line of the definition (for check-time finding spans).
    pub end_line: usize,
    /// The function's real source text — what the user wrote, shown verbatim in
    /// a finding's hunk body.
    pub text: String,
    /// The text actually embedded (and content-hashed for reuse): `text` with
    /// the function's own name replaced by a neutral placeholder. Kept distinct
    /// from `text` so the normalisation never leaks into what a finding displays.
    pub embed_text: String,
    /// Sorted, deduped callee names within this function.
    pub callees: Vec<String>,
    /// Sorted, deduped identifier subtokens within this function.
    pub subtokens: Vec<String>,
}

/// A scored neighbour returned by [`SemanticIndex::nearest`].
#[derive(Debug, Clone, Copy)]
pub struct Neighbor {
    pub entry_index: usize,
    pub cosine: f32,
}

/// Where a [`SemanticIndex::build_with_reuse`] got its vectors from, beyond
/// fresh embedding: this repo's prior fit artifact and the machine-wide
/// content-addressed embed cache.
#[derive(Debug, Default, Clone, Copy)]
pub struct ReuseStats {
    pub from_prior: usize,
    pub from_cache: usize,
}

impl ReuseStats {
    pub fn total(&self) -> usize {
        self.from_prior + self.from_cache
    }
}

/// A per-language embedding index.
#[derive(Debug, Clone)]
pub struct SemanticIndex {
    pub dim: usize,
    pub entries: Vec<IndexEntry>,
}

impl SemanticIndex {
    pub fn len(&self) -> usize {
        self.entries.len()
    }
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
    pub fn entry(&self, i: usize) -> &IndexEntry {
        &self.entries[i]
    }

    /// Build an index by embedding `funcs` in one batch (amortises the inference
    /// context). Order of `entries` follows `funcs`.
    pub fn build(embedder: &Embedder, funcs: &[FunctionRef]) -> Result<Self> {
        Ok(Self::build_with_reuse(embedder, funcs, None, None)?.0)
    }

    /// Build the index, reusing vectors for functions whose embedded text is
    /// unchanged (keyed by [`embed_text_hash`]) from two sources, in order:
    /// this repo's `prior` fit artifact (the incremental-refit path) and the
    /// machine-wide [`EmbedCache`] (the fresh-clone / audit-worktree path).
    /// Only the residual is embedded; every vector this build learned lands in
    /// the cache for the next encounter. All three sources hold the same
    /// f16-canonical bits, so where a vector came from can never change a
    /// finding. A `prior` from another model must never reach here — the
    /// caller gates on [`SemanticArtifact::validate_current`].
    pub fn build_with_reuse(
        embedder: &Embedder,
        funcs: &[FunctionRef],
        prior: Option<&SemanticIndex>,
        cache: Option<&super::embed_cache::EmbedCache>,
    ) -> Result<(Self, ReuseStats)> {
        if funcs.is_empty() {
            return Ok((
                Self {
                    dim: EMBED_DIM,
                    entries: Vec::new(),
                },
                ReuseStats::default(),
            ));
        }
        // hash → vector of the prior fit (skip pre-hash entries).
        let mut reusable: HashMap<&str, &Vec<f32>> = HashMap::new();
        if let Some(p) = prior {
            for e in &p.entries {
                if !e.text_hash.is_empty() && e.vec.len() == EMBED_DIM {
                    reusable.insert(e.text_hash.as_str(), &e.vec);
                }
            }
        }

        let hashes: Vec<String> = funcs
            .iter()
            .map(|f| embed_text_hash(&f.embed_text))
            .collect();
        let mut stats = ReuseStats::default();
        let mut resolved: Vec<Option<Vec<f32>>> = Vec::with_capacity(funcs.len());
        for hash in &hashes {
            if let Some(v) = reusable.get(hash.as_str()) {
                stats.from_prior += 1;
                resolved.push(Some((*v).clone()));
            } else if let Some(v) = cache.and_then(|c| c.get(hash)) {
                stats.from_cache += 1;
                resolved.push(Some(v.clone()));
            } else {
                resolved.push(None);
            }
        }
        let texts: Vec<&str> = resolved
            .iter()
            .zip(funcs)
            .filter(|(r, _)| r.is_none())
            .map(|(_, f)| f.embed_text.as_str())
            .collect();
        let mut fresh = embedder
            .embed(&texts)
            .context("embed corpus functions")?
            .into_iter();

        let mut entries = Vec::with_capacity(funcs.len());
        // Warm the machine-wide cache with everything it doesn't hold yet:
        // freshly embedded vectors AND prior-artifact reuses (so one fit of a
        // seeded checkout makes every future clone/worktree a cache hit).
        let mut persist: Vec<(String, Vec<f32>)> = Vec::new();
        for ((f, hash), reuse) in funcs.iter().zip(&hashes).zip(resolved) {
            let (vec, cache_has_it) = match reuse {
                Some(v) => {
                    // Reused but absent from the prior artifact ⇒ it came
                    // from the cache, so the cache already holds it.
                    let from_cache = !reusable.contains_key(hash.as_str());
                    (v, from_cache)
                }
                None => (
                    fresh.next().expect("one fresh vector per to-embed fn"),
                    false,
                ),
            };
            if cache.is_some() && !cache_has_it {
                persist.push((hash.clone(), vec.clone()));
            }
            entries.push(IndexEntry {
                symbol: f.symbol.clone(),
                path: f.path.clone(),
                line: f.line,
                vec,
                callees: f.callees.clone(),
                subtokens: f.subtokens.clone(),
                text_hash: hash.clone(),
            });
        }
        if let Some(c) = cache {
            c.persist(&persist);
        }
        Ok((
            Self {
                dim: EMBED_DIM,
                entries,
            },
            stats,
        ))
    }

    /// The top-`k` entries admitted by `include`, by descending cosine. `include`
    /// lets callers exclude the query's own file (reinvention/placement) or self.
    pub fn nearest(
        &self,
        query: &[f32],
        k: usize,
        include: impl Fn(&IndexEntry) -> bool,
    ) -> Vec<Neighbor> {
        let mut scored: Vec<Neighbor> = self
            .entries
            .iter()
            .enumerate()
            .filter(|(_, e)| include(e))
            .map(|(entry_index, e)| Neighbor {
                entry_index,
                cosine: dot(query, &e.vec),
            })
            .collect();
        // Descending cosine; total_cmp is NaN-safe and deterministic.
        scored.sort_by(|a, b| b.cosine.total_cmp(&a.cosine));
        scored.truncate(k);
        scored
    }

    // --- serialization (compact f16, its own artifact) ---

    fn to_json(
        &self,
        placement: super::placement::PlacementConfig,
        reinvention: super::redundant::ReinventionConfig,
    ) -> LanguageIndexJson {
        let mut bytes = Vec::with_capacity(self.entries.len() * self.dim * 2);
        for e in &self.entries {
            for &x in &e.vec {
                bytes.extend_from_slice(&f16::from_f32(x).to_le_bytes());
            }
        }
        LanguageIndexJson {
            dim: self.dim,
            count: self.entries.len(),
            symbols: self.entries.iter().map(|e| e.symbol.clone()).collect(),
            paths: self.entries.iter().map(|e| e.path.clone()).collect(),
            lines: self.entries.iter().map(|e| e.line).collect(),
            vectors_b64: base64::engine::general_purpose::STANDARD.encode(&bytes),
            placement,
            reinvention,
            callees: self.entries.iter().map(|e| e.callees.clone()).collect(),
            subtokens: self.entries.iter().map(|e| e.subtokens.clone()).collect(),
            text_hashes: self.entries.iter().map(|e| e.text_hash.clone()).collect(),
        }
    }

    fn from_json(j: &LanguageIndexJson) -> Result<Self> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(&j.vectors_b64)
            .context("decode index vectors")?;
        let expect = j.count * j.dim * 2;
        if bytes.len() != expect {
            bail!(
                "index vector blob is {} bytes, expected {} ({}×{}×2)",
                bytes.len(),
                expect,
                j.count,
                j.dim
            );
        }
        if j.symbols.len() != j.count || j.paths.len() != j.count || j.lines.len() != j.count {
            bail!("index metadata arrays disagree with count {}", j.count);
        }
        let mut entries = Vec::with_capacity(j.count);
        for i in 0..j.count {
            let mut vec = Vec::with_capacity(j.dim);
            for d in 0..j.dim {
                let off = (i * j.dim + d) * 2;
                let bits = u16::from_le_bytes([bytes[off], bytes[off + 1]]);
                vec.push(f16::from_bits(bits).to_f32());
            }
            entries.push(IndexEntry {
                symbol: j.symbols[i].clone(),
                path: j.paths[i].clone(),
                line: j.lines[i],
                vec,
                callees: j.callees.get(i).cloned().unwrap_or_default(),
                subtokens: j.subtokens.get(i).cloned().unwrap_or_default(),
                text_hash: j.text_hashes.get(i).cloned().unwrap_or_default(),
            });
        }
        Ok(Self {
            dim: j.dim,
            entries,
        })
    }
}

/// Truncated sha256 of an embed input — the reuse key for incremental refits.
/// 16 hex chars ≈ 64 bits: collisions across a repo's few thousand functions
/// are negligible, and a collision only reuses a *valid* vector for the wrong
/// (near-identical-population) text, never corrupts the index shape.
pub fn embed_text_hash(text: &str) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    let digest = hasher.finalize();
    digest.iter().take(8).map(|b| format!("{b:02x}")).collect()
}

/// Dot product (== cosine for L2-normalised inputs). A length mismatch means
/// the index was built by a different-dimensional model — `validate_current`
/// prevents that upstream; this guard makes the failure impossible to miss
/// (a mismatched entry can never be "nearest") instead of a silently
/// truncated `zip`.
fn dot(a: &[f32], b: &[f32]) -> f32 {
    debug_assert_eq!(a.len(), b.len(), "embedding dimension mismatch");
    if a.len() != b.len() {
        return f32::NEG_INFINITY;
    }
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Extract the embeddable functions of one file: run the adapter's
/// `callable_bodies`, slice each function's source, drop trivially short bodies.
/// The **same** extraction runs at fit (per corpus file) and check (per diff
/// file), so an indexed function and its check-time re-derivation are identical.
pub fn functions_in_file(
    adapter: &dyn LanguageAdapter,
    rel_path: &str,
    source: &str,
) -> Vec<FunctionRef> {
    let lines: Vec<&str> = source.split('\n').collect();
    let mut out = Vec::new();
    for body in adapter.callable_bodies(source) {
        if body.end_line.saturating_sub(body.start_line) + 1 < MIN_BODY_LINES {
            continue;
        }
        let s = body.start_line.saturating_sub(1);
        let e = body.end_line.min(lines.len());
        if s >= e {
            continue;
        }
        let text = lines[s..e].join("\n");
        // Callees and subtokens are read from the *original* text (they already
        // discount a function's own name via IDF). The embedding, however, uses a
        // copy with the function's own name replaced by a neutral placeholder: a
        // reinvention keeps the body and *renames* the function, and some models
        // (jina-code Q4 on Go especially) let that one identifier dominate a short
        // function's embedding — a name-only rename dropped a Go pair from ~1.0 to
        // ~0.6 while the same edit left Python at ~0.94. Normalising the own-name
        // makes a renamed reimplementation embed next to its original regardless of
        // the model's name sensitivity, and cannot pull unrelated bodies together
        // (their bodies still differ).
        let callees = callee_set(&text, adapter.language());
        let subtokens = subtoken_set(&text);
        let embed_text = normalize_own_name(&text, &body.symbol);
        out.push(FunctionRef {
            symbol: body.symbol,
            path: rel_path.to_string(),
            line: body.start_line,
            end_line: e,
            text,
            embed_text,
            callees,
            subtokens,
        });
    }
    out
}

/// The neutral token a function's own name is rewritten to before embedding.
/// Constant across all functions, so it carries no discriminating signal — it
/// only removes the renamed identifier that would otherwise dominate a short
/// function's embedding.
const OWN_NAME_PLACEHOLDER: &str = "f";

/// Replace whole-identifier occurrences of `symbol` in `text` with
/// [`OWN_NAME_PLACEHOLDER`]. Only exact identifier tokens match (`add` inside
/// `address` is left alone), and recursive self-calls are normalised the same
/// way so an original and its renamed reinvention embed identically at the name.
fn normalize_own_name(text: &str, symbol: &str) -> String {
    if symbol.is_empty() {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut ident = String::new();
    for ch in text.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else {
            out.push_str(if ident == symbol {
                OWN_NAME_PLACEHOLDER
            } else {
                &ident
            });
            ident.clear();
            out.push(ch);
        }
    }
    out.push_str(if ident == symbol {
        OWN_NAME_PLACEHOLDER
    } else {
        &ident
    });
    out
}

/// Language-agnostic identifier subtokens of a function's source: every
/// identifier, split on underscores/digits and camelCase into lowercased pieces
/// of ≥3 chars, deduped and sorted. The reinvention scorer weights these by
/// corpus rarity (IDF), so ubiquitous pieces (`self`, `get`) carry ~0 weight and
/// no per-language stop-list is needed. Runs at fit (per corpus fn) and check
/// (per diff fn) by the same path, so an indexed function and its check-time
/// re-derivation are identical.
pub(super) fn subtoken_set(source: &str) -> Vec<String> {
    let mut set = std::collections::BTreeSet::new();
    let mut ident = String::new();
    for ch in source.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' {
            ident.push(ch);
        } else if !ident.is_empty() {
            split_identifier(&ident, &mut set);
            ident.clear();
        }
    }
    if !ident.is_empty() {
        split_identifier(&ident, &mut set);
    }
    set.into_iter().collect()
}

/// Split one identifier on underscores and digit runs, then camelCase each part.
fn split_identifier(ident: &str, set: &mut std::collections::BTreeSet<String>) {
    for part in ident.split(|c: char| c == '_' || c.is_ascii_digit()) {
        if !part.is_empty() {
            split_camel(part, set);
        }
    }
}

/// Split an alphabetic run on camelCase / acronym boundaries, emitting lowercased
/// pieces of ≥3 chars: `HTTPServer` → `http`,`server`; `parseURL` → `parse`,`url`;
/// `getName` → `get`,`name`.
fn split_camel(word: &str, set: &mut std::collections::BTreeSet<String>) {
    let chars: Vec<char> = word.chars().collect();
    let n = chars.len();
    let mut start = 0;
    for i in 1..n {
        let prev = chars[i - 1];
        let cur = chars[i];
        // Boundary at lower→Upper, or at the last Upper of an acronym run that is
        // followed by a lowercase (…P|Server in HTTPServer).
        let boundary = (prev.is_ascii_lowercase() && cur.is_ascii_uppercase())
            || (prev.is_ascii_uppercase()
                && cur.is_ascii_uppercase()
                && i + 1 < n
                && chars[i + 1].is_ascii_lowercase());
        if boundary {
            emit_subtoken(&chars[start..i], set);
            start = i;
        }
    }
    emit_subtoken(&chars[start..n], set);
}

fn emit_subtoken(word: &[char], set: &mut std::collections::BTreeSet<String>) {
    if word.len() >= 3 {
        set.insert(word.iter().collect::<String>().to_ascii_lowercase());
    }
}

/// The sorted, deduped callee names within a function's source — the structural
/// fingerprint the reinvention scorer confirms against. Reuses the base
/// call-receiver extraction, so "what counts as a call" matches the rest of argot.
fn callee_set(source: &str, language: argot_lang::adapters::Language) -> Vec<String> {
    use argot_lang::adapters::Language;
    // PHP quirk: a method/function body sliced out of its file has no leading
    // `<?php` tag, and tree-sitter-php then reads the whole thing as inert HTML
    // `text` — so `extract_callees` finds *zero* calls and the callee-confirmation
    // path dies for every PHP function (the base scorer never hits this because it
    // parses whole files). Re-add the tag so the body parses as PHP. We only take
    // callee *names*, so the one-line shift is irrelevant; other languages parse a
    // bare function body fine and are left untouched.
    let php_wrapped;
    let source = if language == Language::Php && !source.trim_start().starts_with("<?php") {
        php_wrapped = format!("<?php\n{source}");
        php_wrapped.as_str()
    } else {
        source
    };
    // BTreeSet dedups and sorts → a deterministic callee fingerprint.
    argot_lang::callees::extract_callees(source, language)
        .into_iter()
        .flatten()
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect()
}

// --- the on-disk artifact (`.argot/semantic-index.json`) ---

#[derive(Debug, Serialize, Deserialize)]
struct LanguageIndexJson {
    dim: usize,
    count: usize,
    symbols: Vec<String>,
    paths: Vec<String>,
    lines: Vec<usize>,
    vectors_b64: String,
    /// F2 self-calibrated placement configuration (adaptive areas, entangled
    /// merges, vote parameters, or disabled). Default (disabled) for indices
    /// written before this field — placement then abstains.
    #[serde(default)]
    placement: super::placement::PlacementConfig,
    /// F1 self-calibrated reinvention configuration (conservative mode for
    /// repos practicing systematic parallel implementation). Default = the
    /// standard rule.
    #[serde(default)]
    reinvention: super::redundant::ReinventionConfig,
    /// Per-function callee sets (aligned with `symbols`/`paths`) — one of the two
    /// structural fingerprints the F1 scorer confirms against.
    #[serde(default)]
    callees: Vec<Vec<String>>,
    /// Per-function identifier subtokens (aligned with `symbols`/`paths`) — the
    /// IDF-weighted fingerprint that drives most of F1's reinvention recall.
    #[serde(default)]
    subtokens: Vec<Vec<String>>,
    /// Per-function embed-text hashes (aligned) — the incremental-refit reuse
    /// key. Default (empty) for indices written before the field: no reuse.
    #[serde(default)]
    text_hashes: Vec<String>,
}

/// A language's loaded index plus its self-calibrated placement config.
#[derive(Debug, Clone)]
pub struct LoadedIndex {
    pub index: SemanticIndex,
    pub placement: super::placement::PlacementConfig,
    pub reinvention: super::redundant::ReinventionConfig,
}

/// The embedding model an index was built with — persisted in the artifact so
/// a model upgrade (new argot release, new constants) invalidates old indices
/// loudly instead of comparing vectors from two different embedding spaces.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ModelIdentity {
    pub name: String,
    pub sha256: String,
    pub dim: usize,
}

impl ModelIdentity {
    /// The identity of the model this binary pins.
    pub fn current() -> Self {
        Self {
            name: super::embedder::MODEL_NAME.to_string(),
            sha256: super::embedder::MODEL_SHA256.to_string(),
            dim: EMBED_DIM,
        }
    }
}

/// The whole-repo semantic artifact: one index per language, plus the fit's
/// `repo_sha` so a check can confirm the index matches the scorer-config it
/// scores with, and the embedding model's identity so a model change is
/// detected instead of silently mis-scored.
#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticArtifact {
    pub version: u32,
    pub repo_sha: String,
    /// Absent only in pre-v3 artifacts — `validate_current` rejects those.
    #[serde(default)]
    pub model: Option<ModelIdentity>,
    languages: BTreeMap<String, LanguageIndexJson>,
}

impl SemanticArtifact {
    pub fn new(repo_sha: String) -> Self {
        Self {
            version: ARTIFACT_VERSION,
            repo_sha,
            model: Some(ModelIdentity::current()),
            languages: BTreeMap::new(),
        }
    }

    /// Is this on-disk artifact usable by *this* binary? `Err(reason)` when it
    /// was written by an older format or a different embedding model — the
    /// caller reports the reason and skips the semantic rules for the run
    /// (`argot fit` rebuilds the index with the current model).
    pub fn validate_current(&self) -> std::result::Result<(), String> {
        if self.version != ARTIFACT_VERSION {
            return Err(format!(
                "was written by another argot version (format v{}, this binary expects v{ARTIFACT_VERSION})",
                self.version
            ));
        }
        let current = ModelIdentity::current();
        match &self.model {
            None => return Err("predates model-identity tracking".to_string()),
            Some(m) if *m != current => {
                return Err(format!(
                    "was built with a different embedding model ({} dim {})",
                    m.name, m.dim
                ));
            }
            Some(_) => {}
        }
        for (lang, j) in &self.languages {
            if j.dim != EMBED_DIM {
                return Err(format!(
                    "{lang} index is {}-dimensional, this model embeds {EMBED_DIM}",
                    j.dim
                ));
            }
        }
        Ok(())
    }

    /// Add a language's index and its self-calibrated placement config
    /// (skipped upstream if the index is empty).
    pub fn insert(
        &mut self,
        language: &str,
        index: &SemanticIndex,
        placement: super::placement::PlacementConfig,
        reinvention: super::redundant::ReinventionConfig,
    ) {
        self.languages
            .insert(language.to_string(), index.to_json(placement, reinvention));
    }

    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }

    /// Deserialize one language's index and bars, if present.
    pub fn load(&self, language: &str) -> Result<Option<LoadedIndex>> {
        match self.languages.get(language) {
            Some(j) => Ok(Some(LoadedIndex {
                index: SemanticIndex::from_json(j)?,
                placement: j.placement.clone(),
                reinvention: j.reinvention.clone(),
            })),
            None => Ok(None),
        }
    }

    pub fn to_json_string(&self) -> Result<String> {
        serde_json::to_string(self).context("serialize semantic artifact")
    }

    pub fn from_json_str(s: &str) -> Result<Self> {
        serde_json::from_str(s).context("parse semantic artifact")
    }
}

#[cfg(test)]
mod tests;
