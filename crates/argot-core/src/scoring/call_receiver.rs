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

/// Callees of every call-expression whose start line falls inside the
/// 1-indexed inclusive `[start_line, end_line]` region of `source`.
///
/// Era-14 phase D: when a bare hunk's parse has root-level errors, callee
/// extraction falls back to the hunk's region within its host file's AST —
/// the host parses cleanly where the fragment did not.
pub fn callees_in_source_region(
    source: &str,
    language: Language,
    start_line: usize,
    end_line: usize,
) -> Vec<String> {
    let tree = match parse(source, language) {
        Some(t) => t,
        None => return Vec::new(),
    };
    let bytes = source.as_bytes();
    let is_call = match language {
        Language::Python => py_call_types as fn(&str) -> bool,
        Language::Typescript => ts_call_types as fn(&str) -> bool,
    };
    let extractor = match language {
        Language::Python => extract_python_callee as fn(Node, &[u8]) -> Option<String>,
        Language::Typescript => extract_typescript_callee as fn(Node, &[u8]) -> Option<String>,
    };
    let mut out = Vec::new();
    walk_preorder(tree.root_node(), |node| {
        if is_call(node.kind()) {
            let line = node.start_position().row + 1;
            if line >= start_line && line <= end_line {
                if let Some(c) = extractor(node, bytes) {
                    out.push(c);
                }
            }
        }
    });
    out
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

/// Which rule a distinct hunk callee triggered in
/// [`CallReceiverScorer::weighted_contribution_for_file`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ContributionBranch {
    /// Globally unattested, attested root → `alpha + root_bonus`.
    UnattestedKnownRoot,
    /// Globally unattested, unknown root → `alpha`.
    Unattested,
    /// Globally attested but absent from the file's cluster → `cluster_bonus`.
    ClusterAbsent,
    /// Attested in ≤ rare-threshold cluster files → `cluster_bonus`.
    ClusterRare,
}

/// One distinct callee's contribution decision (scout/evidence surface).
#[derive(Debug, Clone)]
pub struct ContributionEvent {
    pub callee: String,
    pub branch: ContributionBranch,
}

/// Era-14 rarity weighting for the cluster branches (`ClusterAbsent`,
/// `ClusterRare`): scales `cluster_bonus` by how globally common the callee is
/// in the corpus, so locally-rare-but-globally-rare callees (locale
/// identifiers, Zipf-tail helpers) stop firing full-magnitude bonuses while
/// globally-common callees absent from a cluster keep them. All weights are
/// derived from corpus document frequencies — no domain knowledge.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RarityWeighting {
    /// Era-13.5 behaviour: full `cluster_bonus` regardless of global rarity.
    Off,
    /// `weight = df / N` — proportional to document frequency.
    LinearDf,
    /// `weight = 1 if df ≥ min_df else 0` — hard gate on document frequency.
    GatedDf { min_df: usize },
    /// `weight = ln(1 + df) / ln(1 + N)` — logarithmic soft gate.
    LogDf,
}

impl RarityWeighting {
    /// Weight in [0, 1] for a callee seen in `df` of `n_files` corpus files.
    pub fn weight(&self, df: usize, n_files: usize) -> f64 {
        match self {
            RarityWeighting::Off => 1.0,
            RarityWeighting::LinearDf => {
                if n_files == 0 {
                    1.0
                } else {
                    df as f64 / n_files as f64
                }
            }
            RarityWeighting::GatedDf { min_df } => {
                if df >= *min_df {
                    1.0
                } else {
                    0.0
                }
            }
            RarityWeighting::LogDf => {
                if n_files == 0 {
                    1.0
                } else {
                    (1.0 + df as f64).ln() / (1.0 + n_files as f64).ln()
                }
            }
        }
    }
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
    /// Corpus-global document frequency: number of corpus files whose callee
    /// bag contains each callee (era-14 rarity weighting substrate).
    callee_file_counts: HashMap<String, usize>,
    /// Number of (non-data-dominant) corpus files behind `callee_file_counts`.
    n_corpus_files: usize,
    /// Rarity weighting applied to the cluster branches (era 14 phase A).
    rarity_weighting: RarityWeighting,
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

        let mut callee_file_counts: HashMap<String, usize> = HashMap::new();
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
            let bag: HashSet<String> = callees.into_iter().collect();
            for callee in &bag {
                *callee_file_counts.entry(callee.clone()).or_insert(0) += 1;
            }
            if n_clusters > 1 {
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

        let n_corpus_files = files_list.len();
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
            callee_file_counts,
            n_corpus_files,
            rarity_weighting: RarityWeighting::Off,
            rare_branch_fire_count: 0,
            rare_branch_hunks_fired: 0,
            hunks_scored: 0,
        })
    }

    /// Set the rarity weighting for the cluster branches (era 14 phase A).
    pub fn with_rarity_weighting(mut self, weighting: RarityWeighting) -> Self {
        self.rarity_weighting = weighting;
        self
    }

    /// Corpus files that contain `callee` (document frequency; 0 if unseen).
    pub fn callee_file_count(&self, callee: &str) -> usize {
        self.callee_file_counts.get(callee).copied().unwrap_or(0)
    }

    /// Non-data-dominant corpus file count behind the document frequencies.
    pub fn n_corpus_files(&self) -> usize {
        self.n_corpus_files
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

    /// No cluster logic (`weighted_contribution`). `host_context` is the
    /// era-14 phase D parse-error fallback — see
    /// [`Self::contribution_events_for_file`].
    pub fn weighted_contribution(
        &self,
        hunk: &str,
        alpha: f64,
        root_bonus: f64,
        cap: f64,
        host_context: Option<(&str, usize, usize)>,
    ) -> f64 {
        let callees: Vec<String> = if has_root_error(hunk, self.language) {
            match host_context {
                Some((host_source, start_line, end_line)) => {
                    callees_in_source_region(host_source, self.language, start_line, end_line)
                }
                None => return 0.0,
            }
        } else {
            non_none_callees(hunk, self.language)
        };
        let mut weights = 0.0;
        let mut seen: HashSet<String> = HashSet::new();
        for c in callees {
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

    /// Per-callee contribution decisions for a hunk against its file's
    /// cluster — the single source of truth behind
    /// [`Self::weighted_contribution_for_file`], also consumed directly by
    /// research scouts and evidence tooling. Does not touch the fire counters.
    ///
    /// `host_context` is `(host_source, hunk_start_line, hunk_end_line)` with
    /// 1-indexed inclusive bounds. It is consulted ONLY when the bare hunk's
    /// parse has root-level errors (era-14 phase D): callees are then read
    /// from the hunk's region within the host AST. Hunks that parse cleanly
    /// never touch it, so the fallback is purely additive on the
    /// parse-error path (G4.d invariant).
    pub fn contribution_events_for_file(
        &self,
        hunk: &str,
        file_path: Option<&Path>,
        file_source: Option<&str>,
        host_context: Option<(&str, usize, usize)>,
    ) -> Vec<ContributionEvent> {
        let callees: Vec<String> = if has_root_error(hunk, self.language) {
            match host_context {
                Some((host_source, start_line, end_line)) => {
                    callees_in_source_region(host_source, self.language, start_line, end_line)
                }
                None => return Vec::new(),
            }
        } else {
            non_none_callees(hunk, self.language)
        };
        if callees.is_empty() {
            return Vec::new();
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

        let mut events = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();
        for c in callees {
            if seen.contains(&c) {
                continue;
            }
            seen.insert(c.clone());
            let branch = if !self.attested.contains(&c) {
                if self.attested_roots.contains(Self::root(&c)) {
                    Some(ContributionBranch::UnattestedKnownRoot)
                } else {
                    Some(ContributionBranch::Unattested)
                }
            } else if cluster_set.map(|s| !s.contains(&c)).unwrap_or(false) {
                Some(ContributionBranch::ClusterAbsent)
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
                Some(ContributionBranch::ClusterRare)
            } else {
                None
            };
            if let Some(branch) = branch {
                events.push(ContributionEvent { callee: c, branch });
            }
        }
        events
    }

    /// Cluster-conditional (`weighted_contribution_for_file`). `host_context`
    /// is the era-14 phase D parse-error fallback — see
    /// [`Self::contribution_events_for_file`].
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
        host_context: Option<(&str, usize, usize)>,
    ) -> f64 {
        self.hunks_scored += 1;
        let events = self.contribution_events_for_file(hunk, file_path, file_source, host_context);
        let mut weights = 0.0;
        let mut hunk_fired_rare = false;
        for ev in &events {
            // Rarity weighting scales only the cluster branches; the fire
            // counters keep counting branch *decisions* so the auto-detect
            // probe's fire-rate semantics are unchanged by the weighting.
            let rarity = self
                .rarity_weighting
                .weight(self.callee_file_count(&ev.callee), self.n_corpus_files);
            match ev.branch {
                ContributionBranch::UnattestedKnownRoot => weights += alpha + root_bonus,
                ContributionBranch::Unattested => weights += alpha,
                ContributionBranch::ClusterAbsent => weights += cluster_bonus * rarity,
                ContributionBranch::ClusterRare => {
                    self.rare_branch_fire_count += 1;
                    hunk_fired_rare = true;
                    weights += cluster_bonus * rarity;
                }
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
    /// (tests for the era-14 rarity weighting live at the bottom of this file)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scoring::adapters::python::PythonAdapter;

    #[test]
    fn rarity_weight_math() {
        assert_eq!(RarityWeighting::Off.weight(1, 100), 1.0);
        assert_eq!(RarityWeighting::LinearDf.weight(50, 100), 0.5);
        assert_eq!(RarityWeighting::LinearDf.weight(0, 0), 1.0);
        assert_eq!(RarityWeighting::GatedDf { min_df: 3 }.weight(2, 100), 0.0);
        assert_eq!(RarityWeighting::GatedDf { min_df: 3 }.weight(3, 100), 1.0);
        let w = RarityWeighting::LogDf.weight(100, 100);
        assert!(w > 0.99 && w <= 1.0, "log weight near 1 for df=N, got {w}");
        let w1 = RarityWeighting::LogDf.weight(1, 100);
        assert!(w1 < 0.2, "log weight small for df=1, got {w1}");
    }

    fn toy_scorer(weighting: RarityWeighting) -> CallReceiverScorer {
        let adapter = PythonAdapter::new();
        // Two stylistically distinct groups so k-means separates them: the
        // "alpha" files call foo/bar everywhere, the "beta" files call
        // baz/qux. `shared()` appears in exactly one file (globally rare).
        let mut files: Vec<(PathBuf, String)> = Vec::new();
        for i in 0..6 {
            files.push((
                PathBuf::from(format!("a{i}.py")),
                "def f():\n    foo()\n    bar()\n".to_string(),
            ));
        }
        for i in 0..6 {
            files.push((
                PathBuf::from(format!("b{i}.py")),
                "def g():\n    baz()\n    qux()\n".to_string(),
            ));
        }
        files.push((
            PathBuf::from("rare.py"),
            "def h():\n    rare_helper()\n".to_string(),
        ));
        CallReceiverScorer::new(&files, Language::Python, 2.0, 5, &adapter, 4, 0, 0, 0)
            .unwrap()
            .with_rarity_weighting(weighting)
    }

    #[test]
    fn df_counts_are_per_file_presence() {
        let cr = toy_scorer(RarityWeighting::Off);
        assert_eq!(cr.n_corpus_files(), 13);
        assert_eq!(cr.callee_file_count("foo"), 6);
        assert_eq!(cr.callee_file_count("rare_helper"), 1);
        assert_eq!(cr.callee_file_count("never_seen"), 0);
    }

    #[test]
    fn gated_df_at_one_matches_off_behaviour() {
        // Every globally-attested callee has df >= 1, so GatedDf{min_df: 1}
        // must reproduce era-13.5 contributions exactly on any hunk.
        let mut off = toy_scorer(RarityWeighting::Off);
        let mut gated = toy_scorer(RarityWeighting::GatedDf { min_df: 1 });
        for hunk in [
            "rare_helper()\nfoo()\n",
            "baz()\n",
            "unknown_callee()\n",
            "foo()\nbar()\nbaz()\nqux()\n",
        ] {
            let a = off.weighted_contribution_for_file(
                hunk,
                Some(Path::new("a0.py")),
                2.0,
                2.0,
                5.0,
                5.0,
                None,
                None,
            );
            let b = gated.weighted_contribution_for_file(
                hunk,
                Some(Path::new("a0.py")),
                2.0,
                2.0,
                5.0,
                5.0,
                None,
                None,
            );
            assert_eq!(a, b, "hunk {hunk:?}");
        }
    }

    #[test]
    fn linear_df_shrinks_rare_callee_cluster_contribution() {
        // `rare_helper` is globally attested (df=1) — in a file from the
        // foo/bar cluster it takes a cluster branch. Under LinearDf its
        // bonus is scaled by 1/13; alpha branches are untouched.
        let mut off = toy_scorer(RarityWeighting::Off);
        let mut lin = toy_scorer(RarityWeighting::LinearDf);
        let hunk = "rare_helper()\n";
        let a = off.weighted_contribution_for_file(
            hunk,
            Some(Path::new("a0.py")),
            0.0,
            0.0,
            5.0,
            10.0,
            None,
            None,
        );
        let b = lin.weighted_contribution_for_file(
            hunk,
            Some(Path::new("a0.py")),
            0.0,
            0.0,
            5.0,
            10.0,
            None,
            None,
        );
        if a > 0.0 {
            // Cluster branch fired: weighting must shrink it by df/N.
            let expected = a * (1.0 / 13.0);
            assert!(
                (b - expected).abs() < 1e-9,
                "expected {expected}, got {b} (off={a})"
            );
        } else {
            // Cluster assignment put rare.py with a0.py; nothing fired for
            // either mode — invariant still holds.
            assert_eq!(b, 0.0);
        }
    }

    #[test]
    fn parse_error_hunk_falls_back_to_host_region_callees() {
        // A bare `elif` fragment has root-level parse errors in Python.
        let hunk = "elif validate_payload(data):\n    send_alert(data)";
        assert!(has_root_error(hunk, Language::Python));
        let host = "def handler(data):\n    if data is None:\n        return\n    elif validate_payload(data):\n        send_alert(data)\n";
        let callees = callees_in_source_region(host, Language::Python, 4, 5);
        assert_eq!(callees, vec!["validate_payload", "send_alert"]);

        let cr = toy_scorer(RarityWeighting::Off);
        // Without host context: parse error blocks everything (era-13.5).
        assert!(cr
            .contribution_events_for_file(hunk, Some(Path::new("a0.py")), None, None)
            .is_empty());
        // With host context: both (unattested) callees produce events.
        let events = cr.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            Some((host, 4, 5)),
        );
        assert_eq!(events.len(), 2);
        assert!(events
            .iter()
            .all(|e| matches!(e.branch, ContributionBranch::Unattested)));
    }

    #[test]
    fn host_context_is_ignored_when_bare_hunk_parses() {
        // G4.d invariant: a cleanly-parsing hunk scores identically with and
        // without host context — the fallback is purely additive on the
        // parse-error path.
        let cr = toy_scorer(RarityWeighting::Off);
        let hunk = "foo()\nbar()\n";
        // Deliberately contradictory host region (would yield zero callees).
        let host = "x = 1\ny = 2\n";
        let without = cr.contribution_events_for_file(hunk, Some(Path::new("a0.py")), None, None);
        let with_host = cr.contribution_events_for_file(
            hunk,
            Some(Path::new("a0.py")),
            None,
            Some((host, 1, 2)),
        );
        assert_eq!(without.len(), with_host.len());
        for (a, b) in without.iter().zip(with_host.iter()) {
            assert_eq!(a.callee, b.callee);
            assert_eq!(a.branch, b.branch);
        }
    }
}
