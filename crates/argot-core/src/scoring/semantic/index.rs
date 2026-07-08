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
    /// Sorted, deduped identifier subtokens within this function.
    pub subtokens: Vec<String>,
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
                subtokens: f.subtokens.clone(),
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

    fn to_json(&self, area_norms: BTreeMap<String, f32>) -> LanguageIndexJson {
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
            area_norms,
            callees: self.entries.iter().map(|e| e.callees.clone()).collect(),
            subtokens: self.entries.iter().map(|e| e.subtokens.clone()).collect(),
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
            text: embed_text,
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
fn callee_set(source: &str, language: crate::scoring::adapters::Language) -> Vec<String> {
    use crate::scoring::adapters::Language;
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
    crate::scoring::call_receiver::extract_callees(source, language)
        .into_iter()
        .flatten()
        .collect::<std::collections::BTreeSet<String>>()
        .into_iter()
        .collect()
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
    /// F2 per-area "belongs" fraction: the typical share of a function's nearest
    /// neighbours that share its area. Empty for indices written before this
    /// field (placement then falls back to the check-time default norm).
    #[serde(default)]
    area_norms: BTreeMap<String, f32>,
    /// Per-function callee sets (aligned with `symbols`/`paths`) — one of the two
    /// structural fingerprints the F1 scorer confirms against.
    #[serde(default)]
    callees: Vec<Vec<String>>,
    /// Per-function identifier subtokens (aligned with `symbols`/`paths`) — the
    /// IDF-weighted fingerprint that drives most of F1's reinvention recall.
    #[serde(default)]
    subtokens: Vec<Vec<String>>,
}

/// A language's loaded index plus its calibrated area norms.
#[derive(Debug, Clone)]
pub struct LoadedIndex {
    pub index: SemanticIndex,
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

    /// Add a language's index and its calibrated area norms (skipped upstream if
    /// the index is empty).
    pub fn insert(
        &mut self,
        language: &str,
        index: &SemanticIndex,
        area_norms: BTreeMap<String, f32>,
    ) {
        self.languages
            .insert(language.to_string(), index.to_json(area_norms));
    }

    pub fn is_empty(&self) -> bool {
        self.languages.is_empty()
    }

    /// Deserialize one language's index and bars, if present.
    pub fn load(&self, language: &str) -> Result<Option<LoadedIndex>> {
        match self.languages.get(language) {
            Some(j) => Ok(Some(LoadedIndex {
                index: SemanticIndex::from_json(j)?,
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
            subtokens: Vec::new(),
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
        let mut idx = tiny_index();
        idx.entries[0].callees = vec!["lower".into(), "strip".into()];
        idx.entries[0].subtokens = vec!["normalize".into(), "slug".into()];
        let mut art = SemanticArtifact::new("deadbeef".into());
        art.insert(
            "python",
            &idx,
            BTreeMap::from([("src".to_string(), 0.5f32)]),
        );
        let json = art.to_json_string().unwrap();
        let back = SemanticArtifact::from_json_str(&json).unwrap();
        assert_eq!(back.repo_sha, "deadbeef");
        let loaded = back.load("python").unwrap().unwrap();
        assert!((loaded.area_norms["src"] - 0.5).abs() < 1e-6);
        let idx2 = loaded.index;
        assert_eq!(idx2.len(), idx.len());
        // Callee + subtoken fingerprints survive the round-trip.
        assert_eq!(idx2.entries[0].callees, vec!["lower", "strip"]);
        assert_eq!(idx2.entries[0].subtokens, vec!["normalize", "slug"]);
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
        // Subtokens extracted from the body identifiers (≥3 chars).
        assert!(
            big.subtokens.contains(&"total".to_string()),
            "{:?}",
            big.subtokens
        );
    }

    #[test]
    fn callee_set_parses_bare_php_method_body() {
        use crate::scoring::adapters::Language;
        // A PHP method sliced out of its class has no `<?php` tag; without the
        // re-added tag tree-sitter reads it as inert HTML and finds no calls.
        let body = "public static function slug($t) {\n    $t = preg_replace('/x/', '', $t);\n    return str_replace('a', 'b', $t);\n}";
        let callees = callee_set(body, Language::Php);
        // Without the re-added `<?php` tag this is empty (tree-sitter reads it as
        // HTML). Plain function calls are captured (scoped `static::` calls are not,
        // which is fine — the plain callees carry the fingerprint).
        assert!(
            callees.iter().any(|c| c == "preg_replace"),
            "bare PHP method yields plain callees: {callees:?}"
        );
        assert!(callees.iter().any(|c| c == "str_replace"), "{callees:?}");
    }

    #[test]
    fn normalize_own_name_replaces_whole_identifier_only() {
        // Own name + recursive self-call both normalised; substring matches left alone.
        let got = normalize_own_name("def slugify(s): return slugify(s) + slugifyish", "slugify");
        assert_eq!(got, "def f(s): return f(s) + slugifyish");
        // A different function's body is untouched.
        assert_eq!(normalize_own_name("return address(x)", "add"), "return address(x)");
        // Two functions differing only in name normalise to the same embed text.
        let a = normalize_own_name("func DisplayURL(u string) string { return u }", "DisplayURL");
        let b = normalize_own_name("func ShrinkURL(u string) string { return u }", "ShrinkURL");
        assert_eq!(a, b);
    }

    #[test]
    fn subtoken_set_splits_camel_snake_and_acronyms() {
        let set = subtoken_set("def parseHTTPResponse(url): return read_json_body(url)");
        // camelCase + acronym: parseHTTPResponse → parse, http, response
        for w in ["parse", "http", "response", "url", "read", "json", "body"] {
            assert!(set.contains(&w.to_string()), "missing {w} in {set:?}");
        }
        // sorted + deduped, all lowercase, all ≥3 chars.
        assert!(
            set.windows(2).all(|w| w[0] < w[1]),
            "sorted+deduped: {set:?}"
        );
        assert!(set
            .iter()
            .all(|s| s.len() >= 3 && s == &s.to_ascii_lowercase()));
    }
}
