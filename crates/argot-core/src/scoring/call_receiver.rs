//! Call-receiver scorer — port of
//! `engine/argot/scoring/scorers/call_receiver.py`.
//!
//! Tracks distinct call-expression callees across the repo corpus and
//! penalises unattested callees in a hunk (soft additive BPE penalty).
//! Cluster-conditional attestation groups files by callee-bag similarity
//! (MinHash + KMeans) and adds `cluster_bonus` for globally-attested callees
//! absent from the hunk-file's cluster.
//!
//! KMeans note: sklearn's exact seed-0 partition is not reproducible
//! cross-implementation (see docs/rust-port/PORTING-NOTES.md). We use a
//! deterministic hand-rolled k-means++; scoring is invariant to cluster-label
//! numbering, and cluster-affected scores are gated on the benchmark AUC
//! rather than byte-parity, per the recorded decision.

use crate::scoring::adapters::{Language, LanguageAdapter};
use crate::scoring::minhash_params_seed0::{MINHASH_A_SEED0, MINHASH_B_SEED0};
use crate::scoring::ts_parse::parse;
use md5::{Digest, Md5};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use tree_sitter::Node;

const MINHASH_PRIME: u64 = (1 << 31) - 1;
const MINHASH_N_PERMS: usize = 128;

fn node_text(node: Node, src: &[u8]) -> String {
    let r = node.byte_range();
    if r.is_empty() {
        String::new()
    } else {
        String::from_utf8_lossy(&src[r]).into_owned()
    }
}

fn py_call_types(kind: &str) -> bool {
    kind == "call"
}
fn ts_call_types(kind: &str) -> bool {
    kind == "call_expression" || kind == "new_expression"
}

fn extract_python_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let mut callee = call_node.child_by_field_name("function")?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "attribute" {
        let attr = callee.child_by_field_name("attribute")?;
        let obj = callee.child_by_field_name("object")?;
        parts.insert(0, node_text(attr, src));
        callee = obj;
    }
    if callee.kind() == "identifier" {
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else if py_call_types(callee.kind()) {
        parts.insert(0, "<call>".to_string());
        Some(parts.join("."))
    } else {
        None
    }
}

fn extract_typescript_callee(call_node: Node, src: &[u8]) -> Option<String> {
    let field = if call_node.kind() == "new_expression" {
        "constructor"
    } else {
        "function"
    };
    let mut callee = call_node.child_by_field_name(field)?;
    let mut parts: Vec<String> = Vec::new();
    while callee.kind() == "member_expression" {
        let prop = callee.child_by_field_name("property")?;
        let obj = callee.child_by_field_name("object")?;
        parts.insert(0, node_text(prop, src));
        callee = obj;
    }
    if callee.kind() == "identifier" || callee.kind() == "type_identifier" {
        parts.insert(0, node_text(callee, src));
        Some(parts.join("."))
    } else if ts_call_types(callee.kind()) {
        parts.insert(0, "<call>".to_string());
        Some(parts.join("."))
    } else {
        None
    }
}

fn walk_preorder(root: Node, mut visit: impl FnMut(Node)) {
    // Stack DFS pushing reversed children, matching Python `_walk_nodes`.
    let mut stack = vec![root];
    while let Some(node) = stack.pop() {
        visit(node);
        let n = node.child_count();
        for i in (0..n).rev() {
            if let Some(c) = node.child(i) {
                stack.push(c);
            }
        }
    }
}

/// Whether any direct child of the parse-tree root is an ERROR node — a
/// fragment we should not extract callees from. Parse failure → true.
pub fn has_root_error(source: &str, language: Language) -> bool {
    match parse(source, language) {
        None => true,
        Some(tree) => {
            let root = tree.root_node();
            let mut cursor = root.walk();
            let has_error = root.children(&mut cursor).any(|c| c.kind() == "ERROR");
            has_error
        }
    }
}

/// Return dotted-callee signatures for every call-expression in `source`
/// (`None` entries preserved for auditing). `[]` on parse error / empty.
pub fn extract_callees(source: &str, language: Language) -> Vec<Option<String>> {
    if source.trim().is_empty() {
        return Vec::new();
    }
    let tree = match parse(source, language) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let bytes = source.as_bytes();
    let mut out: Vec<Option<String>> = Vec::new();
    let is_call = match language {
        Language::Python => py_call_types as fn(&str) -> bool,
        Language::Typescript => ts_call_types as fn(&str) -> bool,
    };
    let extractor = match language {
        Language::Python => extract_python_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Typescript => extract_typescript_callee as fn(Node, &[u8]) -> Option<String>,
    };
    walk_preorder(tree.root_node(), |node| {
        if is_call(node.kind()) {
            out.push(extractor(node, bytes));
        }
    });
    out
}

fn non_none_callees(source: &str, language: Language) -> Vec<String> {
    extract_callees(source, language)
        .into_iter()
        .flatten()
        .collect()
}

/// 128-element MinHash signature over a callee bag using the seed-0 universal
/// hash family. Empty bag → all zeros.
pub fn minhash_signature(bag: &HashSet<String>) -> Vec<i64> {
    if bag.is_empty() {
        return vec![0; MINHASH_N_PERMS];
    }
    let mut sig = vec![MINHASH_PRIME; MINHASH_N_PERMS];
    for callee in bag {
        let mut hasher = Md5::new();
        hasher.update(callee.as_bytes());
        let digest = hasher.finalize();
        let mut first8 = [0u8; 8];
        first8.copy_from_slice(&digest[..8]);
        let h = u64::from_le_bytes(first8) % MINHASH_PRIME;
        for i in 0..MINHASH_N_PERMS {
            let v = (MINHASH_A_SEED0[i]
                .wrapping_mul(h)
                .wrapping_add(MINHASH_B_SEED0[i]))
                % MINHASH_PRIME;
            if v < sig[i] {
                sig[i] = v;
            }
        }
    }
    sig.into_iter().map(|v| v as i64).collect()
}

// ---------------------------------------------------------------------------
// Deterministic hand-rolled k-means++ (AUC-fallback for sklearn KMeans).
// ---------------------------------------------------------------------------

struct SplitMix64 {
    state: u64,
}
impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }
    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.state;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    fn next_f64(&mut self) -> f64 {
        // 53-bit mantissa uniform in [0,1).
        (self.next_u64() >> 11) as f64 / (1u64 << 53) as f64
    }
}

fn dist_sq(a: &[f64], b: &[f64]) -> f64 {
    a.iter().zip(b).map(|(x, y)| (x - y) * (x - y)).sum()
}

fn kmeans_plus_plus(data: &[Vec<f64>], k: usize, rng: &mut SplitMix64) -> Vec<Vec<f64>> {
    let n = data.len();
    let mut centers: Vec<Vec<f64>> = Vec::with_capacity(k);
    let first = (rng.next_f64() * n as f64) as usize % n;
    centers.push(data[first].clone());
    let mut closest: Vec<f64> = data.iter().map(|p| dist_sq(p, &centers[0])).collect();
    while centers.len() < k {
        let total: f64 = closest.iter().sum();
        if total <= 0.0 {
            // All remaining points coincide with a center; pad with an
            // arbitrary point deterministically.
            centers.push(data[centers.len() % n].clone());
            continue;
        }
        let target = rng.next_f64() * total;
        let mut acc = 0.0;
        let mut chosen = n - 1;
        for (i, d) in closest.iter().enumerate() {
            acc += d;
            if acc >= target {
                chosen = i;
                break;
            }
        }
        centers.push(data[chosen].clone());
        let c = centers.last().unwrap();
        for (i, p) in data.iter().enumerate() {
            let d = dist_sq(p, c);
            if d < closest[i] {
                closest[i] = d;
            }
        }
    }
    centers
}

fn lloyd(data: &[Vec<f64>], mut centers: Vec<Vec<f64>>, max_iter: usize) -> (Vec<usize>, f64) {
    let k = centers.len();
    let dim = data[0].len();
    let mut labels = vec![0usize; data.len()];
    for _ in 0..max_iter {
        let mut changed = false;
        for (i, p) in data.iter().enumerate() {
            let mut best = 0;
            let mut best_d = f64::INFINITY;
            for (c, center) in centers.iter().enumerate() {
                let d = dist_sq(p, center);
                if d < best_d {
                    best_d = d;
                    best = c;
                }
            }
            if labels[i] != best {
                labels[i] = best;
                changed = true;
            }
        }
        // Recompute centers.
        let mut sums = vec![vec![0.0f64; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, p) in data.iter().enumerate() {
            let c = labels[i];
            counts[c] += 1;
            for d in 0..dim {
                sums[c][d] += p[d];
            }
        }
        for c in 0..k {
            if counts[c] > 0 {
                for d in 0..dim {
                    centers[c][d] = sums[c][d] / counts[c] as f64;
                }
            }
        }
        if !changed {
            break;
        }
    }
    let inertia: f64 = data
        .iter()
        .enumerate()
        .map(|(i, p)| dist_sq(p, &centers[labels[i]]))
        .sum();
    (labels, inertia)
}

fn kmeans(data: &[Vec<f64>], k: usize, seed: u64, n_init: usize) -> Vec<usize> {
    let mut best_labels: Option<Vec<usize>> = None;
    let mut best_inertia = f64::INFINITY;
    for init in 0..n_init {
        let mut rng = SplitMix64::new(seed.wrapping_add(init as u64).wrapping_add(1));
        let centers = kmeans_plus_plus(data, k, &mut rng);
        let (labels, inertia) = lloyd(data, centers, 300);
        if inertia < best_inertia {
            best_inertia = inertia;
            best_labels = Some(labels);
        }
    }
    best_labels.unwrap_or_else(|| vec![0; data.len()])
}

/// Cluster files by normalized MinHash signatures. Returns
/// (file→cluster, cluster→size). Deterministic.
fn cluster_by_signatures(
    file_sigs: &[(PathBuf, Vec<i64>)],
    n_clusters: usize,
    seed: u64,
) -> (HashMap<PathBuf, usize>, HashMap<usize, usize>) {
    let n = file_sigs.len();
    let effective_k = n_clusters.min(n);
    let data: Vec<Vec<f64>> = file_sigs
        .iter()
        .map(|(_, s)| s.iter().map(|&v| v as f64 / MINHASH_PRIME as f64).collect())
        .collect();
    let labels: Vec<usize> = if effective_k <= 1 {
        vec![0; n]
    } else {
        kmeans(&data, effective_k, seed, 10)
    };
    let mut file_to_cluster = HashMap::new();
    for (i, (p, _)) in file_sigs.iter().enumerate() {
        file_to_cluster.insert(p.clone(), labels[i]);
    }
    let mut cluster_sizes = HashMap::new();
    for cid in 0..effective_k {
        cluster_sizes.insert(cid, labels.iter().filter(|&&l| l == cid).count());
    }
    (file_to_cluster, cluster_sizes)
}

/// Call-receiver scorer.
pub struct CallReceiverScorer {
    language: Language,
    pub alpha: f64,
    pub cap: usize,
    cluster_rare_threshold: usize,
    cluster_size_min: usize,
    attested: HashSet<String>,
    attested_roots: HashSet<String>,
    pub n_skipped_data_dominant: usize,
    file_to_cluster: HashMap<PathBuf, usize>,
    cluster_attested: HashMap<usize, HashSet<String>>,
    cluster_callee_counts: HashMap<usize, HashMap<String, usize>>,
    cluster_sizes: HashMap<usize, usize>,
    pub rare_branch_fire_count: usize,
    pub rare_branch_hunks_fired: usize,
    pub hunks_scored: usize,
}

impl CallReceiverScorer {
    /// Fit over `repo_files` (path + already-read source). `adapter` is used
    /// only for `is_data_dominant`.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        repo_files: &[(PathBuf, String)],
        language: Language,
        alpha: f64,
        cap: usize,
        adapter: &dyn LanguageAdapter,
        n_clusters: usize,
        cluster_seed: u64,
        cluster_rare_threshold: usize,
        cluster_size_min: usize,
    ) -> Result<Self, &'static str> {
        let mut attested: HashSet<String> = HashSet::new();
        let mut skipped = 0usize;
        let mut files_list: Vec<PathBuf> = Vec::new();
        let mut file_bags: Vec<(PathBuf, HashSet<String>)> = Vec::new();
        let mut file_sigs: Vec<(PathBuf, Vec<i64>)> = Vec::new();

        for (path, src) in repo_files {
            if adapter.is_data_dominant(src) {
                skipped += 1;
                continue;
            }
            let callees = non_none_callees(src, language);
            for c in &callees {
                attested.insert(c.clone());
            }
            files_list.push(path.clone());
            if n_clusters > 1 {
                let bag: HashSet<String> = callees.into_iter().collect();
                let sig = minhash_signature(&bag);
                file_bags.push((path.clone(), bag));
                file_sigs.push((path.clone(), sig));
            }
        }

        if files_list.is_empty() {
            return Err("repo_corpus_files must be non-empty");
        }

        let attested_roots: HashSet<String> = attested
            .iter()
            .map(|c| c.split_once('.').map(|(h, _)| h).unwrap_or(c).to_string())
            .collect();

        let mut file_to_cluster = HashMap::new();
        let mut cluster_attested: HashMap<usize, HashSet<String>> = HashMap::new();
        let mut cluster_callee_counts: HashMap<usize, HashMap<String, usize>> = HashMap::new();
        let mut cluster_sizes = HashMap::new();

        if n_clusters > 1 && !file_sigs.is_empty() {
            let (ftc, sizes) = cluster_by_signatures(&file_sigs, n_clusters, cluster_seed);
            file_to_cluster = ftc;
            cluster_sizes = sizes;
            let effective_k = cluster_sizes.len();
            for cid in 0..effective_k {
                cluster_callee_counts.insert(cid, HashMap::new());
            }
            // Per-file presence counts.
            for (path, bag) in &file_bags {
                if let Some(&cid) = file_to_cluster.get(path) {
                    let counts = cluster_callee_counts.get_mut(&cid).unwrap();
                    for callee in bag {
                        *counts.entry(callee.clone()).or_insert(0) += 1;
                    }
                }
            }
            for (cid, counts) in &cluster_callee_counts {
                cluster_attested.insert(*cid, counts.keys().cloned().collect());
            }
        }

        Ok(Self {
            language,
            alpha,
            cap,
            cluster_rare_threshold,
            cluster_size_min,
            attested,
            attested_roots,
            n_skipped_data_dominant: skipped,
            file_to_cluster,
            cluster_attested,
            cluster_callee_counts,
            cluster_sizes,
            rare_branch_fire_count: 0,
            rare_branch_hunks_fired: 0,
            hunks_scored: 0,
        })
    }

    fn root(callee: &str) -> &str {
        callee.split_once('.').map(|(h, _)| h).unwrap_or(callee)
    }

    fn distinct_unattested_impl(&self, hunk: &str) -> Vec<String> {
        if has_root_error(hunk, self.language) {
            return Vec::new();
        }
        let mut seen: HashSet<String> = HashSet::new();
        let mut out = Vec::new();
        for c in extract_callees(hunk, self.language).into_iter().flatten() {
            if !self.attested.contains(&c) && !seen.contains(&c) {
                seen.insert(c.clone());
                out.push(c);
            }
        }
        out
    }

    pub fn distinct_unattested(&self, hunk: &str) -> Vec<String> {
        self.distinct_unattested_impl(hunk)
    }

    pub fn count_unattested(&self, hunk: &str) -> usize {
        self.distinct_unattested_impl(hunk).len()
    }

    /// Per-cluster callee counts (for the evidence corpus builder).
    pub fn cluster_callee_counts_for_evidence(&self) -> &HashMap<usize, HashMap<String, usize>> {
        &self.cluster_callee_counts
    }

    /// No cluster logic (`weighted_contribution`).
    pub fn weighted_contribution(&self, hunk: &str, alpha: f64, root_bonus: f64, cap: f64) -> f64 {
        if has_root_error(hunk, self.language) {
            return 0.0;
        }
        let mut weights = 0.0;
        let mut seen: HashSet<String> = HashSet::new();
        for c in extract_callees(hunk, self.language).into_iter().flatten() {
            if seen.contains(&c) {
                continue;
            }
            seen.insert(c.clone());
            if self.attested.contains(&c) {
                continue;
            }
            if self.attested_roots.contains(Self::root(&c)) {
                weights += alpha + root_bonus;
            } else {
                weights += alpha;
            }
        }
        weights.min(cap)
    }

    /// Cluster-conditional (`weighted_contribution_for_file`).
    #[allow(clippy::too_many_arguments)]
    pub fn weighted_contribution_for_file(
        &mut self,
        hunk: &str,
        file_path: Option<&Path>,
        alpha: f64,
        root_bonus: f64,
        cluster_bonus: f64,
        cap: f64,
        file_source: Option<&str>,
    ) -> f64 {
        self.hunks_scored += 1;
        if has_root_error(hunk, self.language) {
            return 0.0;
        }
        let mut cluster_id: Option<usize> =
            file_path.and_then(|p| self.file_to_cluster.get(p).copied());
        if cluster_id.is_none() {
            if let Some(src) = file_source {
                if !self.cluster_attested.is_empty() {
                    cluster_id = self.nearest_cluster_for_source(src).map(|(c, _)| c);
                }
            }
        }
        let cluster_set = cluster_id.and_then(|c| self.cluster_attested.get(&c));
        let cluster_counts = cluster_id.and_then(|c| self.cluster_callee_counts.get(&c));

        let mut weights = 0.0;
        let mut seen: HashSet<String> = HashSet::new();
        let mut hunk_fired_rare = false;
        for c in extract_callees(hunk, self.language).into_iter().flatten() {
            if seen.contains(&c) {
                continue;
            }
            seen.insert(c.clone());
            if !self.attested.contains(&c) {
                if self.attested_roots.contains(Self::root(&c)) {
                    weights += alpha + root_bonus;
                } else {
                    weights += alpha;
                }
            } else if cluster_set.map(|s| !s.contains(&c)).unwrap_or(false) {
                weights += cluster_bonus;
            } else if self.cluster_rare_threshold > 0
                && cluster_id.is_some()
                && cluster_counts.is_some()
                && cluster_counts.unwrap().get(&c).copied().unwrap_or(0)
                    <= self.cluster_rare_threshold
                && self
                    .cluster_sizes
                    .get(&cluster_id.unwrap())
                    .copied()
                    .unwrap_or(0)
                    >= self.cluster_size_min
            {
                self.rare_branch_fire_count += 1;
                hunk_fired_rare = true;
                weights += cluster_bonus;
            }
        }
        if hunk_fired_rare {
            self.rare_branch_hunks_fired += 1;
        }
        weights.min(cap)
    }

    pub fn cluster_id_for_hunk_file(
        &self,
        file_path: Option<&Path>,
        file_source: Option<&str>,
    ) -> Option<usize> {
        if let Some(p) = file_path {
            if let Some(&cid) = self.file_to_cluster.get(p) {
                return Some(cid);
            }
        }
        if let Some(src) = file_source {
            if !self.cluster_attested.is_empty() {
                return self.nearest_cluster_for_source(src).map(|(c, _)| c);
            }
        }
        None
    }

    /// Jaccard-nearest cluster for an arbitrary file source. Ties → smallest
    /// cluster id. None if no clusters or empty callee bag.
    pub fn nearest_cluster_for_source(&self, file_source: &str) -> Option<(usize, f64)> {
        if self.cluster_attested.is_empty() {
            return None;
        }
        let bag: HashSet<String> = non_none_callees(file_source, self.language)
            .into_iter()
            .collect();
        if bag.is_empty() {
            return None;
        }
        let mut best_cid: Option<usize> = None;
        let mut best_jaccard = -1.0f64;
        let mut cids: Vec<usize> = self.cluster_attested.keys().copied().collect();
        cids.sort_unstable();
        for cid in cids {
            let attested = &self.cluster_attested[&cid];
            let inter = bag.iter().filter(|c| attested.contains(*c)).count();
            let union = bag.len() + attested.len() - inter;
            let jaccard = if union == 0 {
                0.0
            } else {
                inter as f64 / union as f64
            };
            if jaccard > best_jaccard {
                best_jaccard = jaccard;
                best_cid = Some(cid);
            }
        }
        best_cid.map(|c| (c, best_jaccard))
    }
}
