//! The `SemanticIndex` — a per-repo, per-language table of every function's
//! embedding plus its provenance (`symbol`, `path`, `line`). Built once at
//! fit-time (embed every corpus function) and queried at check-time (embed each
//! diff-defined function, ask for nearest neighbours / margin / area vote).
//!
//! All three semantic features read this one index:
//! - **F1 reinvention**: nearest cross-file neighbour + margin.
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

use std::collections::BTreeMap;

use anyhow::{bail, Context, Result};
use base64::Engine as _;
use half::f16;
use serde::{Deserialize, Serialize};

use super::embedder::{Embedder, EMBED_DIM};
use crate::scoring::adapters::LanguageAdapter;

/// Artifact format version (bump on any breaking on-disk change).
const ARTIFACT_VERSION: u32 = 1;

/// Functions shorter than this (in lines) are skipped when indexing: one- and
/// two-line bodies are boilerplate (getters, trivial wrappers) that only add
/// near-duplicate noise and never make a meaningful reinvention target.
const MIN_BODY_LINES: usize = 3;

/// One indexed function: its embedding plus where it lives, and the set of
/// functions it calls (its callees) — the structural confirmation that turns a
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
    pub text: String,
    /// Sorted, deduped callee names within this function.
    pub callees: Vec<String>,
}

/// A scored neighbour returned by [`SemanticIndex::nearest`].
#[derive(Debug, Clone, Copy)]
pub struct Neighbor {
    pub entry_index: usize,
    pub cosine: f32,
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
        if funcs.is_empty() {
            return Ok(Self {
                dim: EMBED_DIM,
                entries: Vec::new(),
            });
        }
        let texts: Vec<&str> = funcs.iter().map(|f| f.text.as_str()).collect();
        let vecs = embedder.embed(&texts).context("embed corpus functions")?;
        let entries = funcs
            .iter()
            .zip(vecs)
            .map(|(f, vec)| IndexEntry {
                symbol: f.symbol.clone(),
                path: f.path.clone(),
                line: f.line,
                vec,
                callees: f.callees.clone(),
            })
            .collect();
        Ok(Self {
            dim: EMBED_DIM,
            entries,
        })
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

    fn to_json(&self, margin_bar: f32, area_norms: BTreeMap<String, f32>) -> LanguageIndexJson {
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
            margin_bar,
            area_norms,
            callees: self.entries.iter().map(|e| e.callees.clone()).collect(),
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
            });
        }
        Ok(Self {
            dim: j.dim,
            entries,
        })
    }
}

/// Dot product (== cosine for L2-normalised inputs).
fn dot(a: &[f32], b: &[f32]) -> f32 {
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
        let callees = callee_set(&text, adapter.language());
        out.push(FunctionRef {
            symbol: body.symbol,
            path: rel_path.to_string(),
            line: body.start_line,
            end_line: e,
            text,
            callees,
        });
    }
    out
}

/// The sorted, deduped callee names within a function's source — the structural
/// fingerprint the reinvention scorer confirms against. Reuses the base
/// call-receiver extraction, so "what counts as a call" matches the rest of argot.
fn callee_set(source: &str, language: crate::scoring::adapters::Language) -> Vec<String> {
    // BTreeSet dedups and sorts → a deterministic callee fingerprint.
    crate::scoring::call_receiver::extract_callees(source, language)
        .into_iter()
        .flatten()
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect()
}

/// Percentile of the corpus's own cross-file self-margins used as the F1 firing
/// bar: a diff function fires only if its margin exceeds this — i.e. it stands
/// out as a duplicate more strongly than ~all of the repo's own functions do.
/// Tuned to 0.97 against rich + scrapy. This is the *margin* path (a distinct
/// standout dup); the callee-confirm path in `redundant.rs` does the heavy
/// lifting now (combined recall ~60–70% at ~1% over-fire — see
/// `.scratch/semantic-layer/P5-tuning.md`). Kept as an OR path for callee-less
/// standouts.
const MARGIN_BAR_PERCENTILE: f64 = 0.97;
/// Cap on functions sampled for the margin distribution (bounds fit-time cost;
/// the high tail is stable well below this).
const MARGIN_SAMPLE_CAP: usize = 800;
/// Floor on the bar so a tiny or uniform corpus can't calibrate to ~0 (which
/// would then fire on everything).
const MARGIN_BAR_FLOOR: f32 = 0.05;

/// Calibrate the per-repo F1 margin bar from the index's own self-similarity:
/// for a deterministic sample of functions, compute each one's cross-file margin
/// (`cos₁ − cos₂` to its nearest other-file neighbours), then take a high
/// percentile. Empty / single-file indices calibrate to the floor. This is the
/// same "measure the repo against itself" discipline the base scorers use.
pub fn calibrate_margin_bar(index: &SemanticIndex) -> f32 {
    if index.len() < 2 {
        return MARGIN_BAR_FLOOR;
    }
    let step = (index.len() / MARGIN_SAMPLE_CAP).max(1);
    let mut margins: Vec<f32> = Vec::new();
    for (i, e) in index.entries.iter().enumerate() {
        if i % step != 0 {
            continue;
        }
        let neigh = index.nearest(&e.vec, 2, |o| o.path != e.path);
        if let (Some(a), Some(b)) = (neigh.first(), neigh.get(1)) {
            margins.push(a.cosine - b.cosine);
        }
    }
    if margins.is_empty() {
        return MARGIN_BAR_FLOOR;
    }
    margins.sort_by(|a, b| a.total_cmp(b));
    let rank = ((margins.len() as f64 - 1.0) * MARGIN_BAR_PERCENTILE).round() as usize;
    margins[rank.min(margins.len() - 1)].max(MARGIN_BAR_FLOOR)
}

/// Cross-file neighbours polled when calibrating area locality (matches the
/// check-time `K_NEIGHBORS`).
const AREA_K_NEIGHBORS: usize = 10;

/// Calibrate the per-area "belongs" fraction: for every corpus function, the
/// share of its nearest cross-file neighbours that share its area, averaged per
/// area. Areas whose functions naturally scatter (a catch-all package) get a low
/// norm — so placement fires there only on extreme disagreement — while focused
/// packages get a high norm. Uses the same depth-N area as the check-time scorer.
pub fn calibrate_area_norms(index: &SemanticIndex, area_depth: usize) -> BTreeMap<String, f32> {
    let mut sums: BTreeMap<String, (f32, usize)> = BTreeMap::new();
    for e in &index.entries {
        let area = super::placement::area_of(&e.path, area_depth);
        // Exclude only this exact entry (keep same-file siblings — see the
        // check-time scorer for why placement includes them).
        let neigh = index.nearest(&e.vec, AREA_K_NEIGHBORS, |o| {
            !(o.path == e.path && o.line == e.line)
        });
        if neigh.len() < 2 {
            continue;
        }
        let in_area = neigh
            .iter()
            .filter(|n| {
                super::placement::area_of(&index.entry(n.entry_index).path, area_depth) == area
            })
            .count() as f32
            / neigh.len() as f32;
        let slot = sums.entry(area).or_insert((0.0, 0));
        slot.0 += in_area;
        slot.1 += 1;
    }
    sums.into_iter()
        .map(|(area, (sum, n))| (area, sum / n as f32))
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
    /// F1 per-repo calibrated margin bar (default 0.0 for indices written before
    /// this field, which then fall back to the check-time absolute floor only).
    #[serde(default)]
    margin_bar: f32,
    /// F2 per-area "belongs" fraction: the typical share of a function's nearest
    /// neighbours that share its area. Empty for indices written before this
    /// field (placement then falls back to the check-time default norm).
    #[serde(default)]
    area_norms: BTreeMap<String, f32>,
    /// Per-function callee sets (aligned with `symbols`/`paths`) — the structural
    /// fingerprint the F1 scorer confirms against. Empty for indices written
    /// before this field (F1 then falls back to the margin path only).
    #[serde(default)]
    callees: Vec<Vec<String>>,
}

/// A language's loaded index plus its calibrated bars.
#[derive(Debug, Clone)]
pub struct LoadedIndex {
    pub index: SemanticIndex,
    pub margin_bar: f32,
    pub area_norms: std::collections::HashMap<String, f32>,
}

/// The whole-repo semantic artifact: one index per language, plus the fit's
/// `repo_sha` so a check can confirm the index matches the scorer-config it
/// scores with.
#[derive(Debug, Serialize, Deserialize)]
pub struct SemanticArtifact {
    pub version: u32,
    pub repo_sha: String,
    languages: BTreeMap<String, LanguageIndexJson>,
}

impl SemanticArtifact {
    pub fn new(repo_sha: String) -> Self {
        Self {
            version: ARTIFACT_VERSION,
            repo_sha,
            languages: BTreeMap::new(),
        }
    }

    /// Add a language's index and its calibrated bars (skipped upstream if the
    /// index is empty).
    pub fn insert(
        &mut self,
        language: &str,
        index: &SemanticIndex,
        margin_bar: f32,
        area_norms: BTreeMap<String, f32>,
    ) {
        self.languages
            .insert(language.to_string(), index.to_json(margin_bar, area_norms));
    }

    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }

    /// Deserialize one language's index and bars, if present.
    pub fn load(&self, language: &str) -> Result<Option<LoadedIndex>> {
        match self.languages.get(language) {
            Some(j) => Ok(Some(LoadedIndex {
                index: SemanticIndex::from_json(j)?,
                margin_bar: j.margin_bar,
                area_norms: j.area_norms.clone().into_iter().collect(),
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
mod tests {
    use super::*;
    use crate::scoring::adapters::python::PythonAdapter;

    fn entry(symbol: &str, path: &str, line: usize, vec: Vec<f32>) -> IndexEntry {
        IndexEntry {
            symbol: symbol.into(),
            path: path.into(),
            line,
            vec,
            callees: Vec::new(),
        }
    }

    fn unit(v: Vec<f32>) -> Vec<f32> {
        let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        v.into_iter().map(|x| x / n).collect()
    }

    fn tiny_index() -> SemanticIndex {
        SemanticIndex {
            dim: 3,
            entries: vec![
                entry("a", "src/a.py", 1, unit(vec![1.0, 0.0, 0.0])),
                entry("b", "src/b.py", 1, unit(vec![0.9, 0.1, 0.0])),
                entry("c", "src/c.py", 1, unit(vec![0.0, 1.0, 0.0])),
            ],
        }
    }

    #[test]
    fn nearest_ranks_by_cosine_and_respects_filter() {
        let idx = tiny_index();
        let q = unit(vec![1.0, 0.05, 0.0]);
        // All entries: a and b are closest, c far.
        let all = idx.nearest(&q, 3, |_| true);
        assert_eq!(all.len(), 3);
        assert_eq!(idx.entry(all[0].entry_index).symbol, "a");
        assert_eq!(idx.entry(all[1].entry_index).symbol, "b");
        // Exclude a's file → b wins.
        let cross = idx.nearest(&q, 3, |e| e.path != "src/a.py");
        assert_eq!(idx.entry(cross[0].entry_index).symbol, "b");
        // Margin = cos1 - cos2 is positive and small for near-duplicates a,b.
        let m = all[0].cosine - all[1].cosine;
        assert!(m > 0.0 && m < 0.2, "near-dup margin small: {m}");
    }

    #[test]
    fn artifact_roundtrip_preserves_index_within_f16_tolerance() {
        let idx = tiny_index();
        let mut art = SemanticArtifact::new("deadbeef".into());
        art.insert(
            "python",
            &idx,
            0.42,
            BTreeMap::from([("src".to_string(), 0.5f32)]),
        );
        let json = art.to_json_string().unwrap();
        let back = SemanticArtifact::from_json_str(&json).unwrap();
        assert_eq!(back.repo_sha, "deadbeef");
        let loaded = back.load("python").unwrap().unwrap();
        assert!((loaded.margin_bar - 0.42).abs() < 1e-6);
        assert!((loaded.area_norms["src"] - 0.5).abs() < 1e-6);
        let idx2 = loaded.index;
        assert_eq!(idx2.len(), idx.len());
        for (a, b) in idx.entries.iter().zip(&idx2.entries) {
            assert_eq!(a.symbol, b.symbol);
            assert_eq!(a.path, b.path);
            assert_eq!(a.line, b.line);
            // f16 round-trip: cosine of original vs restored ~1.
            let c = dot(&a.vec, &b.vec);
            assert!(c > 0.999, "f16 storage preserves direction: {c}");
        }
        assert!(back.load("typescript").unwrap().is_none());
    }

    #[test]
    fn functions_in_file_extracts_and_filters_trivial() {
        let src = "\
def big(a, b):
    total = a + b
    return total

def tiny():
    return 1

class C:
    def method(self, x):
        y = x * 2
        return y
";
        let adapter = PythonAdapter::new();
        let funcs = functions_in_file(&adapter, "src/m.py", src);
        let names: Vec<&str> = funcs.iter().map(|f| f.symbol.as_str()).collect();
        // `big` (3 lines) and `method` (3 lines) kept; `tiny` (2 lines) dropped.
        assert!(names.contains(&"big"), "got {names:?}");
        assert!(names.contains(&"method"), "methods indexed: {names:?}");
        assert!(!names.contains(&"tiny"), "trivial body dropped: {names:?}");
        // Path + line provenance recorded.
        let big = funcs.iter().find(|f| f.symbol == "big").unwrap();
        assert_eq!(big.path, "src/m.py");
        assert_eq!(big.line, 1);
        assert!(big.text.contains("return total"));
    }
}
