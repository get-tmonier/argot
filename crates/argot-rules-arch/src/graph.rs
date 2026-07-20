//! Architecture-graph foreignness — the relationship analog of the foreign-
//! vocabulary gate. Feature-gated (`--features arch`), pure-Rust (no new deps).
//! When compiled in it emits the `layering` rule (default severity `error` —
//! it gates like any other rule; configurable via `[rules]`); its findings
//! stay out of the base benchmark metric, and without the feature the base
//! guardrail is byte-for-byte unchanged.
//!
//! # What it is
//!
//! The base gate catches a foreign **dependency** (an external import 0-usage in
//! the repo). This catches a foreign **relationship**: an *internal* module-
//! dependency edge the repo's own topology never has — a layer it never crosses,
//! or a dependency **direction** it never uses (a `models/` file importing
//! `views/`). Two properties make it gate where the node-kind *shape* gate could
//! not (see `docs/research/evidence/architecture-graph-foreignness.md`):
//!
//! - **Discrete + high-information** — an edge either reverses an established
//!   direction / leaves a sink layer, or it does not; the same property that
//!   makes the import gate ~98%. (Node-kind n-grams are continuous → 8–13%.)
//! - **Invisible to the base gate** — the imported module is the repo's *own*
//!   code, so vocabulary detection sees nothing. Non-overlapping signal.
//!
//! # The fire rule (the clean, low-FP tell)
//!
//! A hunk introduces edge `(a → b)` (a file in layer `a` importing layer `b`).
//! `FIRE` iff `(a → b)` is 0-usage in the repo AND either:
//! - **reversal:** `(b → a)` *is* attested (the repo layers b-on-a; this reverses
//!   it — a classic layering violation), or
//! - **(near-)sink-out:** `a` is a repo net-importee (imported at least as much as
//!   it imports out — `utils`/`models`/`core`) now importing outward.
//!
//! Firing on *any* novel edge over-fires (organic growth adds edges constantly,
//! up to ~36%); the reversal/near-sink discrimination is what keeps it low. On
//! the real bench (8 corpora, 2690 real commits, temporal holdout, voice files via
//! the mute system) this scores **~85% catch (coverage) at ≤2.7% over-fire per
//! corpus** — a gatable signal, the categorical opposite of the node-kind shape
//! gate. Domain-blind: "layer" = the path component under a package root, never a
//! hardcoded layer name; path exclusions come from the mute system, never here.
//!
//! # Language support
//!
//! Resolves internal imports for **Python, Go, TypeScript/JavaScript, Rust, Java,
//! PHP, C#, Ruby, C, and C++**. Each language plugs in via [`Language`]-dispatch
//! in [`RepoLayering::file_edges`] (per-language `*_targets`) plus a `detect_context`
//! arm (roots / internal names / module prefix). Two layer families:
//!
//! - **Path-anchored** (Python/Go/TS/JS/Rust/Ruby/C/C++): the layer is the first
//!   path component under a source root, resolved straight off the file path.
//! - **Namespace-anchored** (Java/PHP/C#): the layer is the first namespace
//!   component after the base package/root namespace. Because the package tree
//!   mirrors the directory tree (Maven layout, PSR-4, C# project namespaces), the
//!   source roots are *learned* at fit so `layer_of(path)` yields the same layer —
//!   the resolver only supplies the import's target layer from its namespace.
//!
//! A language with no resolver returns no edges — a graceful no-op (no findings).

use std::collections::{HashMap, HashSet};

use argot_lang::adapters::Language;
use argot_lang::ts_parse;

/// A directed layer→layer dependency edge (`from_layer`, `to_layer`).
pub type Edge = (String, String);

/// The persisted layering graph, written at fit and read at check — the same
/// fit-time/check-time decoupling as the semantic index and the import snapshot.
pub const LAYERING_FILE: &str = "layering.json";

/// Serializable form of [`RepoLayering`] (JSON can't key a map on a tuple, so
/// edges are a flat list). Carries the fit `repo_sha` for provenance.
#[derive(serde::Serialize, serde::Deserialize)]
struct LayeringArtifact {
    repo_sha: String,
    /// Language whose resolver built this graph (drives check-time dispatch).
    #[serde(default)]
    language: String,
    /// Top-level internal package/module names (for internal-import detection).
    internal: Vec<String>,
    /// Dirs a file's layer is computed relative to (package/source roots).
    roots: Vec<String>,
    /// Module/namespace prefix for languages that anchor internal imports on one
    /// (Go module path, PHP/C# root namespace); `None` for path-relative langs.
    #[serde(default)]
    module_path: Option<String>,
    edges: Vec<(String, String, u32)>,
    sinks: Vec<String>,
    /// Fit-time file→layer overrides for languages where the source layer is
    /// derived from the file's namespace, not its directory (C#). Empty otherwise.
    #[serde(default)]
    file_layer: Vec<(String, String)>,
}

fn lang_tag(l: Language) -> &'static str {
    match l {
        Language::Python => "python",
        Language::Typescript => "typescript",
        Language::Javascript => "javascript",
        Language::Go => "go",
        Language::Rust => "rust",
        Language::C => "c",
        Language::Java => "java",
        Language::CSharp => "csharp",
        Language::Php => "php",
        Language::Cpp => "cpp",
        Language::Ruby => "ruby",
    }
}

pub fn tag_lang(s: &str) -> Language {
    match s {
        "typescript" => Language::Typescript,
        "javascript" => Language::Javascript,
        "go" => Language::Go,
        "rust" => Language::Rust,
        "c" => Language::C,
        "java" => Language::Java,
        "csharp" => Language::CSharp,
        "php" => Language::Php,
        "cpp" => Language::Cpp,
        "ruby" => Language::Ruby,
        _ => Language::Python,
    }
}

/// A layer counts as a (near-)sink when its outgoing import mass is at most this
/// fraction of its total mass — the net-importee boundary. Tuned on the corpora:
/// 0.5 lifts coverage catch 77% → 85% while real over-fire stays flat (≤2.7%; the
/// repo's own commits don't create these edges regardless of the threshold).
/// Strict sinks (out-mass 0) are the special case.
const NEAR_SINK_RATIO: f64 = 0.5;

/// Why a novel edge is a violation — the discrete, low-FP tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    /// The reverse edge is attested — this reverses an established direction.
    Reversal,
    /// The target transitively depends on the source (`b →* a` in the learned
    /// graph), so a novel `a → b` closes a cycle against the established flow —
    /// a back-edge in the repo's near-DAG layering. Generalizes [`Reversal`]
    /// (the direct `b → a` case) to any path.
    TransitiveReversal,
    /// The source layer is a repo (near-)sink (a net-importee: imported at least
    /// as much as it imports out) now importing outward.
    SinkOut,
}

/// The repo's module-dependency topology, fitted from its files at a pinned SHA.
/// Language-agnostic: the per-language resolver supplies the context (roots,
/// internal names, module prefix) and the edge extraction; the graph + fire rule
/// are shared.
#[derive(Debug, Clone)]
pub struct RepoLayering {
    /// The language whose resolver built this graph.
    language: Language,
    /// Top-level internal package/module names (for internal-import detection):
    /// Python package basenames, Go module-path last component, Java base-package
    /// head, TS/JS: unused (relative imports).
    internal: HashSet<String>,
    /// Dirs a file's layer is computed relative to (package/source roots).
    roots: Vec<String>,
    /// Module/namespace prefix (Go module path, PHP/C# root namespace); `None`
    /// for path-relative languages (TS/JS, C/C++).
    module_path: Option<String>,
    /// Weighted directed cross-layer edges.
    edges: HashMap<Edge, u32>,
    /// Layers that are (near-)sinks: net-importees now importing outward.
    sinks: HashSet<String>,
    /// File→layer overrides for languages whose source layer comes from the
    /// file's namespace rather than its directory (C#, where `.NET` projects use
    /// flat dotted directories). Consulted first by [`layer_of`]. Empty for
    /// path-anchored languages.
    file_layer: HashMap<String, String>,
    /// Transitive reachability over the edge graph: `reach[b]` = every layer `b`
    /// can reach by following edges. Used to detect back-edges (`a → b` where
    /// `b →* a`). Derived from `edges`; recomputed at fit and after deserialize.
    reach: HashMap<String, HashSet<String>>,
}

impl Default for RepoLayering {
    fn default() -> Self {
        RepoLayering {
            language: Language::Python,
            internal: HashSet::new(),
            roots: Vec::new(),
            module_path: None,
            edges: HashMap::new(),
            sinks: HashSet::new(),
            file_layer: HashMap::new(),
            reach: HashMap::new(),
        }
    }
}

impl RepoLayering {
    /// Fit the layering graph from the repo's `language` files (`rel_path`,
    /// `source`). `rel_path` is repo-root-relative with `/` separators. The
    /// per-language resolver derives the context (roots / internal names / module
    /// prefix); the graph + fire rule are language-agnostic. No path exclusion
    /// here — the caller passes the repo's voice files (in production,
    /// `train::collect_source_files`, honoring the mute system).
    pub fn fit<'a, I>(files: I, language: Language) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a str)>,
    {
        let files: Vec<(&str, &str)> = files.into_iter().collect();
        let paths: Vec<&str> = files.iter().map(|(p, _)| *p).collect();
        let (roots, internal, module_path) = detect_context(language, &paths, &files);
        let mut me = RepoLayering {
            language,
            internal,
            roots,
            module_path,
            edges: HashMap::new(),
            sinks: HashSet::new(),
            file_layer: HashMap::new(),
            reach: HashMap::new(),
        };
        // Languages whose source layer is namespace-derived (not directory-derived)
        // need a fit-time file→layer map so `layer_of` is consistent with the edge
        // source layers. Populate it BEFORE building edges so `file_edges` (which
        // reads `layer_of`) sees the override.
        if me.namespace_self_layer() {
            for (path, source) in &files {
                if let Some(l) = me.derive_self_layer(source) {
                    me.file_layer.insert((*path).to_string(), l);
                }
            }
        }
        for (path, source) in &files {
            for e in me.file_edges(path, source) {
                *me.edges.entry(e).or_insert(0) += 1;
            }
        }
        me.recompute_sinks();
        me.recompute_reach();
        me
    }

    fn recompute_sinks(&mut self) {
        // near-sink = a layer imported at least as much as it imports out
        // (out_mass <= NEAR_SINK_RATIO * total_mass). NEAR_SINK_RATIO = 0.5 (the
        // net-importee boundary) lifts coverage catch 82% -> 91% at no cost to
        // real over-fire (flat at ≤2.6% across ratios; see the evidence memo).
        // A strict sink (out_mass == 0) is the special case.
        let mut out_mass: HashMap<&str, u32> = HashMap::new();
        let mut in_mass: HashMap<&str, u32> = HashMap::new();
        for ((a, b), w) in &self.edges {
            *out_mass.entry(a).or_insert(0) += w;
            *in_mass.entry(b).or_insert(0) += w;
        }
        self.sinks = in_mass
            .iter()
            .filter(|(l, &im)| {
                let om = out_mass.get(**l).copied().unwrap_or(0);
                im > 0 && f64::from(om) <= NEAR_SINK_RATIO * f64::from(im + om)
            })
            .map(|(l, _)| l.to_string())
            .collect();
    }

    /// Compute transitive reachability from the edge set: `reach[n]` = every layer
    /// reachable from `n` by following directed edges (excluding `n` itself unless
    /// it sits on a cycle). A DFS per node; graphs are small (layers ≪ 1000).
    fn recompute_reach(&mut self) {
        let mut adj: HashMap<&str, Vec<&str>> = HashMap::new();
        for (a, b) in self.edges.keys() {
            adj.entry(a.as_str()).or_default().push(b.as_str());
        }
        let nodes: HashSet<&str> = adj.keys().copied().collect();
        let mut reach: HashMap<String, HashSet<String>> = HashMap::new();
        for &start in &nodes {
            let mut seen: HashSet<&str> = HashSet::new();
            let mut stack: Vec<&str> = adj.get(start).cloned().unwrap_or_default();
            while let Some(n) = stack.pop() {
                if seen.insert(n) {
                    if let Some(next) = adj.get(n) {
                        stack.extend(next.iter().copied());
                    }
                }
            }
            reach.insert(
                start.to_string(),
                seen.iter().map(|s| s.to_string()).collect(),
            );
        }
        self.reach = reach;
    }

    /// The layer of a file: the first path component under its enclosing root.
    /// An empty root (`""`) = repo root and matches any path (Go-style, layer =
    /// first path component). Language-agnostic.
    pub fn layer_of(&self, path: &str) -> Option<String> {
        if let Some(l) = self.file_layer.get(path) {
            return Some(l.clone());
        }
        match self.language {
            Language::Rust => rust_layer_under(&self.roots, path),
            _ => layer_under(&self.roots, path),
        }
    }

    /// Whether this language derives a file's source layer from its namespace/
    /// package declaration rather than its directory. C# .NET projects use flat
    /// dotted directories (`src/Company.Product.Module/`); Java's directory mirrors
    /// the package but the base-package split is ambiguous from the path alone — in
    /// both, deriving the layer from the declaration (with the same base-stripping
    /// as target resolution) keeps source and target vocabularies aligned.
    fn namespace_self_layer(&self) -> bool {
        matches!(self.language, Language::CSharp | Language::Java)
    }

    /// The source layer of a file from its own namespace/package, using the SAME
    /// base-stripping as target derivation (`layer_after_base`) so source and
    /// target vocabularies align. `None` if the file has no declaration under a
    /// known base. Only meaningful when [`namespace_self_layer`] is true.
    fn derive_self_layer(&self, source: &str) -> Option<String> {
        let ns = match self.language {
            Language::CSharp => cs_namespace(source)?,
            Language::Java => java_package(source)?,
            _ => return None,
        };
        let parts: Vec<&str> = ns.split('.').filter(|s| !s.is_empty()).collect();
        layer_after_base(&parts, &self.internal, '.')
    }

    /// The `roots`-relative path components of a file (for relative-import climbs).
    fn rel_parts(&self, path: &str) -> Vec<String> {
        let dir = parent_dir(path);
        let root = self
            .roots
            .iter()
            .filter(|r| r.is_empty() || dir == r.as_str() || dir.starts_with(&format!("{r}/")))
            .max_by_key(|r| r.len());
        let rel = match root {
            Some(r) if !r.is_empty() => path[r.len()..].trim_start_matches('/'),
            _ => path,
        };
        rel.split('/').map(|s| s.to_string()).collect()
    }

    /// Cross-layer edges a single file introduces, dispatched to the graph's
    /// per-language resolver. Each resolver yields the TARGET layers of the
    /// file's internal imports; the shared code forms `(src_layer → tgt)` edges.
    pub fn file_edges(&self, path: &str, source: &str) -> HashSet<Edge> {
        // Namespace-self-layer languages (C#) derive the source layer from the
        // file's own namespace so it aligns with the target vocabulary. Use the
        // fit-time map when the path is known; otherwise parse `source` directly
        // (a hunk in a file not present at fit — e.g. a newly added file).
        let src = if self.namespace_self_layer() {
            self.file_layer
                .get(path)
                .cloned()
                .or_else(|| self.derive_self_layer(source))
        } else {
            self.layer_of(path)
        };
        let Some(src) = src else {
            return HashSet::new();
        };
        let targets = match self.language {
            Language::Python => self.py_targets(path, source),
            Language::Go => self.go_targets(source),
            Language::Typescript | Language::Javascript => self.ts_targets(path, source),
            Language::Rust => self.rust_targets(source),
            Language::Java => self.java_targets(source),
            Language::Php => self.php_targets(source),
            Language::CSharp => self.cs_targets(source),
            Language::Ruby => self.ruby_targets(path, source),
            Language::C | Language::Cpp => self.c_targets(path, source),
        };
        targets
            .into_iter()
            .filter(|t| *t != src)
            .map(|t| (src.clone(), t))
            .collect()
    }

    /// Target layers of a Python file's internal imports.
    fn py_targets(&self, path: &str, source: &str) -> Vec<String> {
        let parts = self.rel_parts(path);
        let mut out = Vec::new();
        for (module, level) in py_imports(source) {
            let tgt = if level == 0 {
                let p: Vec<&str> = module.split('.').filter(|s| !s.is_empty()).collect();
                if p.is_empty() || !self.internal.contains(p[0]) {
                    continue; // external dep — the base gate's job
                }
                if p.len() > 1 {
                    p[1].to_string()
                } else {
                    "__root__".to_string()
                }
            } else {
                let mut base: Vec<&str> = parts[..parts.len().saturating_sub(1)]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let up = level - 1;
                if up <= base.len() {
                    base.truncate(base.len() - up);
                } else {
                    base.clear();
                }
                base.extend(module.split('.').filter(|s| !s.is_empty()));
                base.first()
                    .map(|f| f.to_string())
                    .unwrap_or_else(|| "__root__".to_string())
            };
            out.push(tgt);
        }
        out
    }

    /// Target layers of a Go file's internal imports (paths under the module).
    fn go_targets(&self, source: &str) -> Vec<String> {
        let Some(module) = &self.module_path else {
            return Vec::new();
        };
        let mut out = Vec::new();
        for spec in go_imports(source) {
            if let Some(rest) = spec.strip_prefix(module) {
                let rest = rest.trim_start_matches('/');
                let tgt = rest.split('/').next().unwrap_or("");
                out.push(if tgt.is_empty() {
                    "__root__".to_string()
                } else {
                    tgt.to_string()
                });
            }
        }
        out
    }

    /// Target layers of a TS/JS file's internal (relative) imports.
    fn ts_targets(&self, path: &str, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for spec in ts_imports(source) {
            if !spec.starts_with('.') {
                continue; // bare specifier = external / alias
            }
            // resolve the relative spec against the file's directory
            let resolved = normalize_join(parent_dir(path), &spec);
            if let Some(l) = self.layer_of(&resolved) {
                out.push(l);
            }
        }
        out
    }

    /// Target layers of a Rust file's internal imports: `use crate::<layer>::…`
    /// and `use <workspace_crate>::<layer>::…` (the crate is a known internal
    /// member). `super`/`self`/`std`/external crates are skipped.
    fn rust_targets(&self, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for spec in rust_use_paths(source) {
            let (head, rest) = match spec.split_once("::") {
                Some((h, r)) => (h.trim(), r.trim()),
                None => continue, // `use foo;` — no sub-path, no layer
            };
            // The head must anchor the import on the repo's own code.
            if head != "crate" && !self.internal.contains(head) {
                continue;
            }
            for layer in rust_path_layers(rest) {
                out.push(layer);
            }
        }
        out
    }

    /// Target layers of a Java file's internal imports: the package component
    /// after the base package (`import <base>.<layer>.…`).
    fn java_targets(&self, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for pkg in java_import_packages(source) {
            let parts: Vec<&str> = pkg.split('.').filter(|s| !s.is_empty()).collect();
            if let Some(layer) = layer_after_base(&parts, &self.internal, '.') {
                out.push(layer);
            }
        }
        out
    }

    /// Target layers of a PHP file's internal imports: `use <root>\<layer>\…`
    /// where `<root>` is a repo-observed root namespace (in `internal`).
    fn php_targets(&self, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for spec in php_use_names(source) {
            let parts: Vec<&str> = spec.split('\\').filter(|s| !s.is_empty()).collect();
            let Some(head) = parts.first() else { continue };
            if !self.internal.contains(*head) {
                continue; // foreign namespace — the base gate's job
            }
            out.push(after_prefix_layer(&parts[1..]));
        }
        out
    }

    /// Target layers of a C# file's internal imports: the namespace component
    /// after the base namespace (`using <base>.<layer>.…`).
    fn cs_targets(&self, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for spec in cs_using_names(source) {
            let parts: Vec<&str> = spec.split('.').filter(|s| !s.is_empty()).collect();
            if let Some(layer) = layer_after_base(&parts, &self.internal, '.') {
                out.push(layer);
            }
        }
        out
    }

    /// Target layers of a Ruby file's internal requires: `require_relative "…"`
    /// resolved against the file dir, and `require "<layer>/…"` whose head is a
    /// repo layer. Ruby leans on autoloading, so graphs are expectedly sparse.
    fn ruby_targets(&self, path: &str, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (method, spec) in ruby_requires(source) {
            if method == "require_relative" {
                let resolved = normalize_join(parent_dir(path), &spec);
                if let Some(l) = self.layer_of(&resolved) {
                    out.push(l);
                }
            } else {
                // require/load "x/y" — resolves via the load path (usually a root);
                // its head is an internal layer only if the repo has that layer.
                let head = spec.split('/').next().unwrap_or("");
                if self.internal.contains(head) {
                    out.push(head.to_string());
                }
            }
        }
        out
    }

    /// Target layers of a C/C++ file's internal `#include "…"` (quoted). System
    /// (`<…>`) includes are external and skipped. A dotted-relative include is
    /// resolved against the file dir; a project-relative one contributes its first
    /// path component as the target layer.
    fn c_targets(&self, path: &str, source: &str) -> Vec<String> {
        let mut out = Vec::new();
        for (is_system, header) in c_includes(source, self.language) {
            if is_system {
                continue;
            }
            if header.starts_with('.') {
                let resolved = normalize_join(parent_dir(path), &header);
                if let Some(l) = self.layer_of(&resolved) {
                    out.push(l);
                }
            } else if let Some((first, _)) = header.split_once('/') {
                if !first.is_empty() {
                    out.push(first.to_string());
                }
            }
            // else: a bare `"foo.h"` is a same-directory include — no cross-layer edge.
        }
        out
    }

    /// Classify a *new* edge (one not already attested). Returns the violation
    /// tell, or `None` if the edge is a clean novel-forward edge (organic growth).
    pub fn classify(&self, edge: &Edge) -> Option<Violation> {
        if self.edges.contains_key(edge) {
            return None; // already attested — not novel
        }
        let (a, b) = edge;
        if self.edges.contains_key(&(b.clone(), a.clone())) {
            Some(Violation::Reversal)
        } else if self.reach.get(b).is_some_and(|r| r.contains(a)) {
            // b transitively depends on a already, so a → b closes a cycle against
            // the established near-DAG flow — a back-edge even without a direct b→a.
            Some(Violation::TransitiveReversal)
        } else if self.sinks.contains(a) {
            Some(Violation::SinkOut)
        } else {
            None
        }
    }

    /// The clean-tell fire decision for a hunk: does it introduce a
    /// reversal/sink-out edge? Non-gating — the caller reports it.
    pub fn fires(&self, path: &str, hunk: &str) -> Option<Violation> {
        self.file_edges(path, hunk)
            .iter()
            .filter_map(|e| self.classify(e))
            .next()
    }

    /// Synthesize an import line that, added to `host_path`, creates the
    /// cross-layer edge `host_layer → target_layer` — VERIFIED through the real
    /// resolver (`file_edges`). Returns `None` if no synthesis resolves. Prefix-
    /// import languages (Python/PHP/C#/Java/Go/Rust) need only the target layer;
    /// relative-import languages (TS/JS/Ruby) need a concrete `target_file` in the
    /// target layer to build a relative specifier from. Lets fixtures be authored
    /// valid-by-construction, closing the text-grep gap. The imported symbol is a
    /// placeholder — the resolver keys on the module path, not the symbol.
    pub fn example_import(
        &self,
        host_path: &str,
        target_layer: &str,
        target_file: Option<&str>,
    ) -> Option<String> {
        let src = self.layer_of(host_path)?;
        if src == *target_layer {
            return None;
        }
        let mut candidates: Vec<String> = Vec::new();
        match self.language {
            Language::Python => {
                for pkg in &self.internal {
                    candidates.push(format!("from {pkg}.{target_layer} import ArgotFixture\n"));
                }
            }
            Language::Php => {
                for root in &self.internal {
                    candidates.push(format!(
                        "<?php\nuse {root}\\{target_layer}\\ArgotFixture;\n"
                    ));
                }
            }
            Language::CSharp => {
                for base in &self.internal {
                    candidates.push(format!("using {base}.{target_layer};\n"));
                }
            }
            Language::Java => {
                for base in &self.internal {
                    candidates.push(format!("import {base}.{target_layer}.ArgotFixture;\n"));
                }
            }
            Language::Go => {
                if let Some(m) = &self.module_path {
                    candidates.push(format!("import \"{m}/{target_layer}\"\n"));
                }
            }
            Language::Rust => {
                candidates.push(format!("use crate::{target_layer}::ArgotFixture;\n"));
                for c in &self.internal {
                    candidates.push(format!("use {c}::{target_layer}::ArgotFixture;\n"));
                }
            }
            // Relative-import languages: build a relative specifier from the host
            // file's directory to a concrete target file in the target layer.
            Language::Typescript | Language::Javascript => {
                if let Some(tf) = target_file {
                    let spec = relative_spec(parent_dir(host_path), tf, true);
                    candidates.push(format!("import {{ ArgotFixture }} from '{spec}';\n"));
                }
            }
            Language::Ruby => {
                if let Some(tf) = target_file {
                    let spec = relative_spec(parent_dir(host_path), tf, true);
                    candidates.push(format!("require_relative \"{spec}\"\n"));
                }
            }
            Language::C | Language::Cpp => {
                // project-relative include: first path component is the layer.
                candidates.push(format!("#include \"{target_layer}/argot_fixture.h\"\n"));
                if let Some(tf) = target_file {
                    let spec = relative_spec(parent_dir(host_path), tf, false);
                    candidates.push(format!("#include \"{spec}\"\n"));
                }
            }
        }
        let want = (src, target_layer.to_string());
        candidates
            .into_iter()
            .find(|line| self.file_edges(host_path, line).contains(&want))
    }

    pub fn edge_count(&self) -> usize {
        self.edges.len()
    }

    /// Every layer that appears in the graph.
    pub fn layers(&self) -> HashSet<String> {
        let mut s = HashSet::new();
        for (a, b) in self.edges.keys() {
            s.insert(a.clone());
            s.insert(b.clone());
        }
        s
    }

    /// The tag of the language whose resolver built this graph — the routing
    /// key `check` matches a changed file against (`ext_to_lang`).
    pub fn language_tag(&self) -> &'static str {
        lang_tag(self.language)
    }

    fn to_artifact(&self, repo_sha: &str) -> LayeringArtifact {
        LayeringArtifact {
            repo_sha: repo_sha.to_string(),
            language: lang_tag(self.language).to_string(),
            internal: self.internal.iter().cloned().collect(),
            roots: self.roots.clone(),
            module_path: self.module_path.clone(),
            edges: self
                .edges
                .iter()
                .map(|((a, b), w)| (a.clone(), b.clone(), *w))
                .collect(),
            sinks: self.sinks.iter().cloned().collect(),
            file_layer: self
                .file_layer
                .iter()
                .map(|(p, l)| (p.clone(), l.clone()))
                .collect(),
        }
    }

    fn from_artifact(art: LayeringArtifact) -> Self {
        let mut me = RepoLayering {
            language: tag_lang(&art.language),
            internal: art.internal.into_iter().collect(),
            roots: art.roots,
            module_path: art.module_path,
            edges: art.edges.into_iter().map(|(a, b, w)| ((a, b), w)).collect(),
            sinks: art.sinks.into_iter().collect(),
            file_layer: art.file_layer.into_iter().collect(),
            reach: HashMap::new(),
        };
        me.recompute_reach();
        me
    }

    /// Serialize a single fitted graph (used in tests / single-language paths).
    pub fn to_json(&self, repo_sha: &str) -> String {
        serde_json::to_string(&self.to_artifact(repo_sha)).unwrap_or_default()
    }

    /// Restore a single graph persisted by [`to_json`]. `None` on unreadable JSON.
    pub fn from_json(s: &str) -> Option<Self> {
        let art: LayeringArtifact = serde_json::from_str(s).ok()?;
        Some(Self::from_artifact(art))
    }

    /// Serialize a per-language set of graphs for `.argot/layering.json` — one
    /// artifact per language whose resolver produced a non-empty graph, so
    /// `check` can route each changed file to its language's graph.
    pub fn set_to_json(graphs: &[RepoLayering], repo_sha: &str) -> String {
        let arts: Vec<LayeringArtifact> = graphs.iter().map(|g| g.to_artifact(repo_sha)).collect();
        serde_json::to_string(&arts).unwrap_or_default()
    }

    /// Restore the per-language set persisted by [`set_to_json`]. Empty on
    /// unreadable JSON. Falls back to a legacy single-graph artifact so a repo
    /// fitted before the multi-language format degrades to its one graph rather
    /// than erroring every check until it is re-fitted.
    pub fn set_from_json(s: &str) -> Vec<RepoLayering> {
        if let Ok(arts) = serde_json::from_str::<Vec<LayeringArtifact>>(s) {
            return arts.into_iter().map(Self::from_artifact).collect();
        }
        serde_json::from_str::<LayeringArtifact>(s)
            .map(|art| vec![Self::from_artifact(art)])
            .unwrap_or_default()
    }

    /// Import mass into `layer` (sum of incoming edge weights) — a layer's
    /// popularity, i.e. how likely an LLM is to reach for it.
    pub fn in_mass(&self, layer: &str) -> u32 {
        self.edges
            .iter()
            .filter(|((_, b), _)| b == layer)
            .map(|(_, w)| *w)
            .sum()
    }

    pub fn contains_edge(&self, edge: &Edge) -> bool {
        self.edges.contains_key(edge)
    }

    pub fn is_sink(&self, layer: &str) -> bool {
        self.sinks.contains(layer)
    }
}

// --- path helpers (repo-root-relative, `/`-separated) ---

/// The layer of `path` under `roots`: the first path component below the longest
/// matching root (`""` = repo root, matches anything). A file sitting directly in
/// a root (no sub-component) is `__root__`. The path-anchored layer primitive.
fn layer_under(roots: &[String], path: &str) -> Option<String> {
    let dir = parent_dir(path);
    let root = roots
        .iter()
        .filter(|r| r.is_empty() || dir == r.as_str() || dir.starts_with(&format!("{r}/")))
        .max_by_key(|r| r.len())?;
    let rel = if root.is_empty() {
        path
    } else {
        path[root.len()..].trim_start_matches('/')
    };
    let parts: Vec<&str> = rel.split('/').collect();
    Some(if parts.len() > 1 {
        parts[0].to_string()
    } else {
        "__root__".to_string()
    })
}

/// Rust layer of a file under its crate `src` root. A directory module
/// (`src/foo/bar.rs`) is layer `foo`, exactly like [`layer_under`]. But a *file*
/// module directly under `src` (`src/color.rs`) is layer `color` — its module
/// name — not `__root__`, so it aligns with the `use crate::color::…` target
/// vocabulary. `lib.rs`/`main.rs`/`mod.rs` are the crate root (`__root__`).
fn rust_layer_under(roots: &[String], path: &str) -> Option<String> {
    let dir = parent_dir(path);
    let root = roots
        .iter()
        .filter(|r| r.is_empty() || dir == r.as_str() || dir.starts_with(&format!("{r}/")))
        .max_by_key(|r| r.len())?;
    let rel = if root.is_empty() {
        path
    } else {
        path[root.len()..].trim_start_matches('/')
    };
    let parts: Vec<&str> = rel.split('/').collect();
    if parts.len() > 1 {
        return Some(parts[0].to_string());
    }
    let stem = parts[0].strip_suffix(".rs").unwrap_or(parts[0]);
    Some(if matches!(stem, "lib" | "main" | "mod") {
        "__root__".to_string()
    } else {
        stem.to_string()
    })
}

fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
}

/// A `./`-relative specifier from `from_dir` to `to_file`. TS/JS/Ruby imports
/// drop the extension (`strip_ext`); C/C++ `#include` keeps it. Inverse of
/// [`normalize_join`] on the path components.
fn relative_spec(from_dir: &str, to_file: &str, strip_ext: bool) -> String {
    let to_noext = if strip_ext {
        match to_file.rfind('.') {
            Some(i) if !to_file[i..].contains('/') => &to_file[..i],
            _ => to_file,
        }
    } else {
        to_file
    };
    let from: Vec<&str> = from_dir.split('/').filter(|s| !s.is_empty()).collect();
    let to: Vec<&str> = to_noext.split('/').filter(|s| !s.is_empty()).collect();
    let common = from.iter().zip(&to).take_while(|(a, b)| a == b).count();
    let ups = from.len() - common;
    let mut spec = if ups == 0 {
        "./".to_string()
    } else {
        "../".repeat(ups)
    };
    spec.push_str(&to[common..].join("/"));
    spec
}

/// Resolve `spec` (a `.`/`..` relative path) against directory `dir`, normalizing.
fn normalize_join(dir: &str, spec: &str) -> String {
    let mut stack: Vec<&str> = if dir.is_empty() {
        Vec::new()
    } else {
        dir.split('/').collect()
    };
    for part in spec.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                stack.pop();
            }
            p => stack.push(p),
        }
    }
    stack.join("/")
}

/// Per-language fit context: `(roots, internal-names, module-prefix)`. `roots` are
/// the dirs a file's layer is computed relative to (`""` = repo root); `internal`
/// are the top-level names that mark an import as repo-internal; `module_path` is
/// the Go-module / root-namespace prefix. Extend the match to add a language.
fn detect_context(
    language: Language,
    paths: &[&str],
    files: &[(&str, &str)],
) -> (Vec<String>, HashSet<String>, Option<String>) {
    match language {
        Language::Python => {
            let init_dirs: HashSet<String> = paths
                .iter()
                .filter(|p| p.ends_with("__init__.py"))
                .map(|p| parent_dir(p).to_string())
                .collect();
            let mut roots: Vec<String> = init_dirs
                .iter()
                .filter(|d| !init_dirs.contains(parent_dir(d)))
                .cloned()
                .collect();
            roots.sort_by_key(|d| std::cmp::Reverse(d.len()));
            let internal: HashSet<String> = roots.iter().map(|d| basename(d).to_string()).collect();
            (roots, internal, None)
        }
        Language::Go => {
            let module = files
                .iter()
                .find(|(p, _)| p.ends_with("go.mod"))
                .and_then(|(_, s)| go_module(s));
            (vec![String::new()], HashSet::new(), module)
        }
        Language::Typescript | Language::Javascript => {
            (ts_source_roots(paths), HashSet::new(), None)
        }
        Language::Rust => detect_rust_context(paths),
        Language::Java => detect_java_context(paths),
        Language::Php => detect_php_context(files),
        Language::CSharp => detect_csharp_context(files),
        Language::Ruby => detect_ruby_context(paths),
        Language::C | Language::Cpp => (c_source_roots(paths), HashSet::new(), None),
    }
}

/// Rust: roots are the crate source dirs (a `src` component); the layer is the
/// first path component under `src`. `internal` collects workspace-member crate
/// names (the dir owning each `src`, normalized `-`→`_`) so `use <crate>::<layer>`
/// resolves like `use crate::<layer>`.
fn detect_rust_context(paths: &[&str]) -> (Vec<String>, HashSet<String>, Option<String>) {
    let mut roots: HashSet<String> = HashSet::new();
    let mut internal: HashSet<String> = HashSet::new();
    for p in paths {
        let comps: Vec<&str> = p.split('/').collect();
        if let Some(i) = comps.iter().position(|c| *c == "src") {
            roots.insert(comps[..=i].join("/"));
            if i >= 1 {
                internal.insert(comps[i - 1].replace('-', "_"));
            }
        }
    }
    (sorted_roots(roots), internal, None)
}

/// Java: roots are `<src-root>/<base-package-path>` so `layer_of` yields the
/// package component after the base package. Test roots are skipped. The base
/// package (the longest common package-dir prefix across main sources) is stored
/// dotted in `module_path` for import matching.
fn detect_java_context(paths: &[&str]) -> (Vec<String>, HashSet<String>, Option<String>) {
    // Group each file's package-dir vector BY its src-root. A monorepo has several
    // src trees (e.g. guava's `android/guava/src` + `guava/src` + module variants);
    // a GLOBAL common-prefix collapses to empty as soon as two trees diverge. So
    // compute the base package PER src-root, and keep every base namespace in
    // `internal` (dotted) so an import matching ANY base resolves.
    let mut by_root: HashMap<String, Vec<Vec<String>>> = HashMap::new();
    for p in paths {
        let comps: Vec<&str> = p.split('/').collect();
        let (root, is_test) = java_src_root(&comps);
        if is_test {
            continue;
        }
        let root_len = if root.is_empty() {
            0
        } else {
            root.split('/').count()
        };
        if comps.len() <= root_len + 1 {
            continue; // file sits directly at the root — no package
        }
        let pkg: Vec<String> = comps[root_len..comps.len() - 1]
            .iter()
            .map(|s| s.to_string())
            .collect();
        by_root.entry(root).or_default().push(pkg);
    }
    let mut roots: HashSet<String> = HashSet::new();
    let mut internal: HashSet<String> = HashSet::new();
    for (root, pkgs) in &by_root {
        // A single LCP collapses when a root ships sibling top-level namespaces
        // (guava: `com.google.common.*` + `com.google.thirdparty.*` → base
        // `com.google`, bucketing everything into layer `common`). `detect_bases`
        // descends single-child and *dominant*-child chains, splitting minority
        // siblings into their own bases, so the layer is the package after the
        // *right* base (`common.collect` → `collect`, `thirdparty.publicsuffix`
        // → `publicsuffix`).
        for base in detect_bases(pkgs) {
            if base.is_empty() {
                continue;
            }
            internal.insert(base.join("."));
            roots.insert(join_root(root, &base.join("/")));
        }
    }
    (sorted_roots(roots), internal, None)
}

/// Detect the base package prefix(es) of a set of package-component vectors — the
/// prefix(es) below which the next component is a *layer*. Descends single-child
/// chains (`com`→`google`) and, at a fan-out, descends the **dominant** child
/// (≥70% of files) while splitting each **minority** sibling into its own base, so
/// a repo with one big package root plus a small sibling (guava's `common` +
/// `thirdparty`) yields both bases instead of collapsing to their short common
/// prefix. A *balanced* fan-out (no dominant child, e.g. junit's
/// `jupiter`/`platform`/`vintage`) stops — that prefix is the base, its children
/// are the layers.
fn detect_bases(pkgs: &[Vec<String>]) -> Vec<Vec<String>> {
    fn recurse(pkgs: Vec<Vec<String>>, prefix: Vec<String>, out: &mut Vec<Vec<String>>) {
        let depth = prefix.len();
        let mut children: HashMap<String, Vec<Vec<String>>> = HashMap::new();
        for p in pkgs.iter() {
            if p.len() > depth {
                children
                    .entry(p[depth].clone())
                    .or_default()
                    .push(p.clone());
            }
        }
        if children.is_empty() {
            if !prefix.is_empty() {
                out.push(prefix);
            }
            return;
        }
        if children.len() == 1 {
            let (c, sub) = children.into_iter().next().unwrap();
            let mut np = prefix;
            np.push(c);
            recurse(sub, np, out);
            return;
        }
        let total = pkgs.len();
        let dominant = children
            .iter()
            .find(|(_, v)| v.len() * 100 >= total * 70)
            .map(|(k, _)| k.clone());
        match dominant {
            Some(dom) => {
                for (c, sub) in children {
                    let mut np = prefix.clone();
                    np.push(c.clone());
                    if c == dom {
                        recurse(sub, np, out); // descend the dominant root for deeper branches
                    } else {
                        out.push(np); // minority sibling is its own base; its children are layers
                    }
                }
            }
            None => out.push(prefix), // balanced fan-out: prefix is the base, children are layers
        }
    }
    let mut out = Vec::new();
    recurse(pkgs.to_vec(), Vec::new(), &mut out);
    out
}

/// PHP (PSR-4): the root namespace maps to a source dir, so the second namespace
/// component (the layer) equals the first path component under that dir. Learn
/// each file's source root from its `namespace` vs. its path; `internal` collects
/// the observed root namespaces for import matching.
fn detect_php_context(files: &[(&str, &str)]) -> (Vec<String>, HashSet<String>, Option<String>) {
    let mut roots: HashSet<String> = HashSet::new();
    let mut internal: HashSet<String> = HashSet::new();
    for (p, src) in files {
        let Some(ns) = php_namespace(src) else {
            continue;
        };
        let parts: Vec<&str> = ns.split('\\').filter(|s| !s.is_empty()).collect();
        let Some(head) = parts.first() else { continue };
        internal.insert((*head).to_string());
        roots.insert(namespace_source_root(p, parts.len()));
    }
    (sorted_roots(roots), internal, None)
}

/// C#: the base namespace (longest common prefix of file namespaces) maps to a
/// project source dir, so the component after it (the layer) equals the first path
/// component under that dir. Learn each root from namespace vs. path; the base
/// namespace is stored dotted in `module_path` for import matching.
fn detect_csharp_context(files: &[(&str, &str)]) -> (Vec<String>, HashSet<String>, Option<String>) {
    // Group by the top namespace component (the project root); a multi-project repo
    // (powershell/jellyfin) has several, and a GLOBAL common-prefix collapses to
    // empty. Compute the base namespace PER group, keep every base in `internal`.
    let mut by_head: HashMap<String, Vec<(&str, Vec<String>)>> = HashMap::new();
    for (p, src) in files {
        if let Some(ns) = cs_namespace(src) {
            let parts: Vec<String> = ns
                .split('.')
                .filter(|s| !s.is_empty())
                .map(String::from)
                .collect();
            if let Some(head) = parts.first().cloned() {
                by_head.entry(head).or_default().push((*p, parts));
            }
        }
    }
    let mut roots: HashSet<String> = HashSet::new();
    let mut internal: HashSet<String> = HashSet::new();
    for group in by_head.values() {
        let base = longest_common_prefix(group.iter().map(|(_, p)| p.as_slice()));
        if base.is_empty() {
            continue;
        }
        internal.insert(base.join("."));
        for (p, parts) in group {
            let trailing = parts.len().saturating_sub(base.len()) + 1;
            roots.insert(namespace_source_root(p, trailing));
        }
    }
    (sorted_roots(roots), internal, None)
}

/// Ruby: roots are the repo root plus any top-level `lib`/`app`; the layer is the
/// first component under a root. `internal` collects those layer names so a
/// load-path `require "<layer>/…"` resolves to a repo layer.
fn detect_ruby_context(paths: &[&str]) -> (Vec<String>, HashSet<String>, Option<String>) {
    let mut roots: HashSet<String> = HashSet::from([String::new()]);
    for p in paths {
        let top = p.split('/').next().unwrap_or("");
        if matches!(top, "lib" | "app") && p.contains('/') {
            roots.insert(top.to_string());
        }
    }
    let roots = sorted_roots(roots);
    let mut internal: HashSet<String> = HashSet::new();
    for p in paths {
        if let Some(layer) = layer_under(&roots, p) {
            if layer != "__root__" {
                internal.insert(layer);
            }
        }
    }
    (roots, internal, None)
}

/// The source root of a namespace-anchored file: drop `trailing` path components
/// (the sub-namespace dirs beyond the root/base + the filename) so the remaining
/// prefix is the dir the root/base namespace maps to. Too few components ⇒ repo
/// root. Works whether the base maps to one dir (`src`) or many (`Acme/App`).
fn namespace_source_root(path: &str, trailing: usize) -> String {
    let comps: Vec<&str> = path.split('/').collect();
    if comps.len() > trailing {
        comps[..comps.len() - trailing].join("/")
    } else {
        String::new()
    }
}

/// Join a source root with a `/`-separated sub-path, handling empty operands.
fn join_root(root: &str, sub: &str) -> String {
    match (root.is_empty(), sub.is_empty()) {
        (true, _) => sub.to_string(),
        (false, true) => root.to_string(),
        (false, false) => format!("{root}/{sub}"),
    }
}

/// Sort roots longest-first (so `layer_under` prefers the most specific) into a Vec.
fn sorted_roots(roots: HashSet<String>) -> Vec<String> {
    let mut v: Vec<String> = roots.into_iter().collect();
    v.sort_by_key(|d| std::cmp::Reverse(d.len()));
    v
}

/// The layer name for the path components below a namespace prefix: the first, or
/// `__root__` when the file sits directly in the prefix.
fn after_prefix_layer(rest: &[&str]) -> String {
    rest.first()
        .map(|s| (*s).to_string())
        .unwrap_or_else(|| "__root__".to_string())
}

/// Given a segmented FQN (`parts`) and a set of base namespaces (each `sep`-joined),
/// find the LONGEST base that is a strict prefix of `parts` and return the component
/// after it (the layer). `None` if no base matches or it consumes the whole path.
/// Handles multi-module repos where each src-root/project has its own base namespace.
fn layer_after_base(parts: &[&str], bases: &HashSet<String>, sep: char) -> Option<String> {
    let mut best: Option<usize> = None;
    for base in bases {
        let bparts: Vec<&str> = base.split(sep).filter(|s| !s.is_empty()).collect();
        if bparts.len() < parts.len()
            && parts[..bparts.len()] == bparts[..]
            && best.is_none_or(|b| bparts.len() > b)
        {
            best = Some(bparts.len());
        }
    }
    best.map(|n| after_prefix_layer(&parts[n..]))
}

/// Longest common prefix of a set of component slices.
fn longest_common_prefix<'a, I>(mut iter: I) -> Vec<String>
where
    I: Iterator<Item = &'a [String]>,
{
    let Some(first) = iter.next() else {
        return Vec::new();
    };
    let mut prefix: Vec<String> = first.to_vec();
    for pkg in iter {
        let n = prefix.iter().zip(pkg).take_while(|(a, b)| a == b).count();
        prefix.truncate(n);
        if prefix.is_empty() {
            break;
        }
    }
    prefix
}

/// The Java source root of a path and whether it is a *test* root. Recognises
/// `src/main/java`, `src/test/java` (test), and a bare `src`; else the repo root.
fn java_src_root(comps: &[&str]) -> (String, bool) {
    for i in 0..comps.len() {
        if comps[i] != "src" {
            continue;
        }
        if i + 2 < comps.len() && comps[i + 1] == "main" && comps[i + 2] == "java" {
            return (comps[..=i + 2].join("/"), false);
        }
        if i + 2 < comps.len() && comps[i + 1] == "test" && comps[i + 2] == "java" {
            return (comps[..=i + 2].join("/"), true);
        }
        return (comps[..=i].join("/"), false);
    }
    (String::new(), false)
}

/// Distinct C/C++ source roots: the deepest `src`/`include`/`lib` component of
/// each path, else the repo root. The layer is the first dir below the root.
fn c_source_roots(paths: &[&str]) -> Vec<String> {
    let mut roots: HashSet<String> = HashSet::new();
    for p in paths {
        let comps: Vec<&str> = p.split('/').collect();
        if let Some(i) = comps
            .iter()
            .rposition(|c| matches!(*c, "src" | "include" | "lib"))
        {
            roots.insert(comps[..=i].join("/"));
        } else {
            roots.insert(String::new());
        }
    }
    sorted_roots(roots)
}

/// Distinct TS/JS source roots (a file's layer is the first dir under one): the
/// deepest `src` component of each path, else `packages/<x>`, else the repo root.
fn ts_source_roots(paths: &[&str]) -> Vec<String> {
    let mut roots: HashSet<String> = HashSet::new();
    for p in paths {
        let comps: Vec<&str> = p.split('/').collect();
        if let Some(i) = comps.iter().rposition(|c| *c == "src") {
            roots.insert(comps[..=i].join("/"));
        } else if comps.first() == Some(&"packages") && comps.len() > 2 {
            roots.insert(comps[..2].join("/"));
        } else {
            roots.insert(String::new());
        }
    }
    let mut v: Vec<String> = roots.into_iter().collect();
    v.sort_by_key(|d| std::cmp::Reverse(d.len()));
    v
}

/// The `module X` line of a `go.mod`.
fn go_module(gomod: &str) -> Option<String> {
    gomod
        .lines()
        .find_map(|l| l.strip_prefix("module ").map(|m| m.trim().to_string()))
}

/// Import path strings in a Go source file (via tree-sitter `import_spec`).
fn go_imports(source: &str) -> Vec<String> {
    let Some(tree) = ts_parse::parse(source, Language::Go) else {
        return Vec::new();
    };
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        if node.kind() == "import_spec" || node.kind() == "import_declaration" {
            let mut c = node.walk();
            collect_go_strings(node, bytes, &mut out, &mut c);
        }
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            stack.push(ch);
        }
    }
    out
}

fn collect_go_strings(
    node: tree_sitter::Node,
    bytes: &[u8],
    out: &mut Vec<String>,
    _c: &mut tree_sitter::TreeCursor,
) {
    let mut cur = node.walk();
    for ch in node.named_children(&mut cur) {
        if ch.kind() == "interpreted_string_literal" || ch.kind() == "raw_string_literal" {
            let t = node_text(ch, bytes);
            out.push(t.trim_matches(|c| c == '"' || c == '`').to_string());
        } else {
            collect_go_strings(ch, bytes, out, _c);
        }
    }
}

/// Import specifier strings in a TS/JS file (`import … from "x"`, `require("x")`,
/// `export … from "x"`) via tree-sitter string literals under import/export/call.
fn ts_imports(source: &str) -> Vec<String> {
    let lang = Language::Typescript;
    let Some(tree) = ts_parse::parse(source, lang) else {
        return Vec::new();
    };
    let bytes = source.as_bytes();
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        let k = node.kind();
        if k == "import_statement" || k == "export_statement" {
            if let Some(src) = node.child_by_field_name("source") {
                out.push(strip_quotes(&node_text(src, bytes)));
            }
        } else if k == "call_expression" {
            // require("x") / import("x")
            let fname = node
                .child_by_field_name("function")
                .map(|f| node_text(f, bytes));
            if matches!(fname.as_deref(), Some("require") | Some("import")) {
                if let Some(args) = node.child_by_field_name("arguments") {
                    let mut c = args.walk();
                    for a in args.named_children(&mut c) {
                        if a.kind() == "string" {
                            out.push(strip_quotes(&node_text(a, bytes)));
                        }
                    }
                }
            }
        }
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            stack.push(ch);
        }
    }
    out
}

fn strip_quotes(s: &str) -> String {
    s.trim_matches(|c| c == '"' || c == '\'' || c == '`')
        .to_string()
}

fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

/// Extract `(dotted_module, relative_level)` for each Python import.
/// `level` = number of leading dots (0 = absolute). Domain-blind: text only.
fn py_imports(source: &str) -> Vec<(String, usize)> {
    let Some(tree) = ts_parse::parse(source, Language::Python) else {
        return Vec::new();
    };
    let mut out = Vec::new();
    let mut stack = vec![tree.root_node()];
    let bytes = source.as_bytes();
    while let Some(node) = stack.pop() {
        match node.kind() {
            "import_statement" => {
                // import a.b.c  /  import a.b as x
                let mut c = node.walk();
                for ch in node.named_children(&mut c) {
                    if ch.kind() == "dotted_name" {
                        out.push((node_text(ch, bytes), 0));
                    } else if ch.kind() == "aliased_import" {
                        if let Some(dn) = ch.child_by_field_name("name") {
                            if dn.kind() == "dotted_name" {
                                out.push((node_text(dn, bytes), 0));
                            }
                        }
                    }
                }
            }
            "import_from_statement" => {
                // from a.b import x  /  from ..a import x  /  from . import x
                let mut level = 0usize;
                let mut module = String::new();
                let mut c = node.walk();
                for ch in node.named_children(&mut c) {
                    match ch.kind() {
                        "relative_import" => {
                            let t = node_text(ch, bytes);
                            level = t.chars().take_while(|&c| c == '.').count();
                            let tail = t.trim_start_matches('.');
                            if !tail.is_empty() {
                                module = tail.to_string();
                            }
                        }
                        // the module_name field (absolute import target)
                        "dotted_name" if node.child_by_field_name("module_name") == Some(ch) => {
                            module = node_text(ch, bytes);
                        }
                        _ => {}
                    }
                }
                if level == 0 && module.is_empty() {
                    continue;
                }
                out.push((module, level));
            }
            _ => {}
        }
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            stack.push(ch);
        }
    }
    out
}

fn node_text(node: tree_sitter::Node, bytes: &[u8]) -> String {
    String::from_utf8_lossy(&bytes[node.start_byte()..node.end_byte()]).into_owned()
}

/// Depth-first named-node walk of a parse over `source` for `language`, invoking
/// `visit` on each node. Shared spine for the per-language import extractors.
fn walk_named(source: &str, language: Language, mut visit: impl FnMut(tree_sitter::Node, &[u8])) {
    let Some(tree) = ts_parse::parse(source, language) else {
        return;
    };
    let bytes = source.as_bytes();
    let mut stack = vec![tree.root_node()];
    while let Some(node) = stack.pop() {
        visit(node, bytes);
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            stack.push(ch);
        }
    }
}

/// The `use`-path text of each `use_declaration` (the `argument` field, e.g.
/// `crate::foo::bar` or `crate::{a, b}`), visibility and `;` excluded.
fn rust_use_paths(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk_named(source, Language::Rust, |node, bytes| {
        if node.kind() == "use_declaration" {
            if let Some(arg) = node.child_by_field_name("argument") {
                out.push(node_text(arg, bytes));
            }
        }
    });
    out
}

/// The layer name(s) a `use`-path tail (everything after `crate::`/`<crate>::`)
/// contributes: `foo::bar` → `foo`; a brace group `{foo, bar::baz}` → `foo`,
/// `bar`. `self`/`*` and empty segments are ignored.
fn rust_path_layers(rest: &str) -> Vec<String> {
    let rest = rest.trim();
    if let Some(inner) = rest.strip_prefix('{').and_then(|r| r.strip_suffix('}')) {
        // A group directly under the crate root: each element heads its own layer.
        return split_top_level_commas(inner)
            .into_iter()
            .filter_map(|e| rust_first_segment(e.trim()))
            .collect();
    }
    rust_first_segment(rest).into_iter().collect()
}

/// The leading path segment of a `use` element (up to the first `::` or `{`),
/// filtering non-layer heads (`self`, `*`, empty).
fn rust_first_segment(s: &str) -> Option<String> {
    let head = s
        .split("::")
        .next()
        .unwrap_or(s)
        .split('{')
        .next()
        .unwrap_or(s)
        .trim();
    if head.is_empty() || head == "self" || head == "*" {
        None
    } else {
        Some(head.to_string())
    }
}

/// Split on commas that are not nested inside `{ … }` (Rust `use` groups).
fn split_top_level_commas(s: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let mut depth = 0i32;
    let mut start = 0usize;
    for (i, ch) in s.char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(&s[start..i]);
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(&s[start..]);
    out
}

/// The package (dotted) of each Java `import_declaration`: the `scoped_identifier`
/// FQN, dropping the trailing type name for a plain import, kept whole for a
/// wildcard (`import a.b.*` → `a.b`).
fn java_import_packages(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk_named(source, Language::Java, |node, bytes| {
        if node.kind() != "import_declaration" {
            return;
        }
        let mut fqn: Option<String> = None;
        let mut wildcard = false;
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            match ch.kind() {
                "scoped_identifier" | "identifier" => fqn = Some(node_text(ch, bytes)),
                "asterisk" => wildcard = true,
                _ => {}
            }
        }
        if let Some(fqn) = fqn {
            if wildcard {
                out.push(fqn);
            } else if let Some(i) = fqn.rfind('.') {
                out.push(fqn[..i].to_string());
            }
        }
    });
    out
}

/// The `package a.b.c;` declaration of a Java file (dotted).
fn java_package(source: &str) -> Option<String> {
    let mut found = None;
    walk_named(source, Language::Java, |node, bytes| {
        if found.is_none() && node.kind() == "package_declaration" {
            let mut c = node.walk();
            for ch in node.named_children(&mut c) {
                if matches!(ch.kind(), "scoped_identifier" | "identifier") {
                    found = Some(node_text(ch, bytes));
                    break;
                }
            }
        }
    });
    found
}

/// The `namespace X\Y\…;` of a PHP file (the `name` of its `namespace_definition`).
fn php_namespace(source: &str) -> Option<String> {
    let mut found = None;
    walk_named(source, Language::Php, |node, bytes| {
        if found.is_none() && node.kind() == "namespace_definition" {
            if let Some(name) = node.child_by_field_name("name") {
                found = Some(node_text(name, bytes));
            }
        }
    });
    found
}

/// The imported namespace text of each PHP `use` clause (`Root\Layer\Class`).
fn php_use_names(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk_named(source, Language::Php, |node, bytes| {
        if node.kind() != "namespace_use_clause" {
            return;
        }
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            if matches!(ch.kind(), "qualified_name" | "name") {
                out.push(node_text(ch, bytes));
                break;
            }
        }
    });
    out
}

/// The declared namespace of a C# file (block or file-scoped form).
fn cs_namespace(source: &str) -> Option<String> {
    let mut found = None;
    walk_named(source, Language::CSharp, |node, bytes| {
        if found.is_none()
            && matches!(
                node.kind(),
                "namespace_declaration" | "file_scoped_namespace_declaration"
            )
        {
            if let Some(name) = node.child_by_field_name("name") {
                found = Some(node_text(name, bytes));
            }
        }
    });
    found
}

/// The imported namespace of each C# `using_directive` (the `qualified_name` /
/// `identifier` that is not the alias LHS carried in the `name` field).
fn cs_using_names(source: &str) -> Vec<String> {
    let mut out = Vec::new();
    walk_named(source, Language::CSharp, |node, bytes| {
        if node.kind() != "using_directive" {
            return;
        }
        let alias = node.child_by_field_name("name");
        let mut c = node.walk();
        for ch in node.named_children(&mut c) {
            if Some(ch) == alias {
                continue; // alias LHS, not the imported namespace
            }
            if matches!(ch.kind(), "qualified_name" | "identifier") {
                out.push(node_text(ch, bytes));
                break;
            }
        }
    });
    out
}

/// `(method, spec)` for each top-level `require` / `require_relative` / `load`
/// call in a Ruby file (receiver-less `call` with a string argument).
fn ruby_requires(source: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    walk_named(source, Language::Ruby, |node, bytes| {
        if node.kind() != "call" || node.child_by_field_name("receiver").is_some() {
            return;
        }
        let Some(method) = node.child_by_field_name("method") else {
            return;
        };
        let m = node_text(method, bytes);
        if !matches!(m.as_str(), "require" | "require_relative" | "load") {
            return;
        }
        let Some(args) = node.child_by_field_name("arguments") else {
            return;
        };
        let mut ac = args.walk();
        for a in args.named_children(&mut ac) {
            if a.kind() != "string" {
                continue;
            }
            let mut sc = a.walk();
            for s in a.named_children(&mut sc) {
                if s.kind() == "string_content" {
                    out.push((m.clone(), node_text(s, bytes)));
                }
            }
        }
    });
    out
}

/// `(is_system, header)` for each `#include` in a C/C++ file: `is_system` for
/// `<…>` (external, empty header), else the quoted path with quotes stripped.
fn c_includes(source: &str, language: Language) -> Vec<(bool, String)> {
    let mut out = Vec::new();
    walk_named(source, language, |node, bytes| {
        if node.kind() != "preproc_include" {
            return;
        }
        let Some(path) = node.child_by_field_name("path") else {
            return;
        };
        match path.kind() {
            "system_lib_string" => out.push((true, String::new())),
            "string_literal" => {
                let inner = node_text(path, bytes).trim_matches('"').to_string();
                out.push((false, inner));
            }
            _ => {}
        }
    });
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    // A tiny layered repo: views -> models (attested), models is imported-only.
    fn fixture() -> Vec<(&'static str, &'static str)> {
        vec![
            ("app/__init__.py", ""),
            ("app/models/__init__.py", ""),
            ("app/models/user.py", "class User: pass\n"),
            ("app/views/__init__.py", ""),
            ("app/views/page.py", "from app.models import user\n"),
        ]
    }

    fn fit_py() -> RepoLayering {
        RepoLayering::fit(fixture(), Language::Python)
    }

    #[test]
    fn fit_builds_directional_graph() {
        let g = fit_py();
        // views -> models attested; models is a sink (imported, never imports out).
        assert!(g.contains_edge(&("views".into(), "models".into())));
        assert!(g.is_sink("models"));
        assert!(!g.is_sink("views"));
    }

    #[test]
    fn reversal_fires_sink_out_fires_forward_does_not() {
        let g = fit_py();
        // REVERSAL: a model importing a view (reverses views->models).
        assert_eq!(
            g.fires("app/models/user.py", "from app.views import page\n"),
            Some(Violation::Reversal)
        );
        // SINK-OUT: the sink layer `models` importing some other layer.
        assert_eq!(
            g.classify(&("models".into(), "controllers".into())),
            Some(Violation::SinkOut)
        );
        // CLEAN novel-forward: views importing a brand-new non-sink target — no tell.
        assert_eq!(g.classify(&("views".into(), "controllers".into())), None);
        // Already-attested edge does not fire.
        assert_eq!(
            g.fires("app/views/page.py", "from app.models import user\n"),
            None
        );
    }

    #[test]
    fn external_imports_are_ignored() {
        let g = fit_py();
        // numpy is not a repo package — no internal edge, nothing fires.
        assert_eq!(g.fires("app/models/user.py", "import numpy as np\n"), None);
    }

    #[test]
    fn relative_imports_resolve() {
        let g = fit_py();
        // `from ..views import page` inside models/ -> edge models->views (reversal).
        assert_eq!(
            g.fires("app/models/user.py", "from ..views import page\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn external_only_imports_yield_empty_graph() {
        // A repo whose imports are all external (std / third-party crates) has no
        // internal edges — a graceful empty graph, no findings.
        let g = RepoLayering::fit(
            vec![("src/main.rs", "use std::fmt;\nuse serde::Serialize;\n")],
            Language::Rust,
        );
        assert_eq!(g.edge_count(), 0);
    }

    #[test]
    fn rust_resolver_builds_directional_graph() {
        // crate src; api -> core (via `use crate::core`), core is a sink.
        let files = vec![
            ("src/api/handler.rs", "use crate::core::db;\n"),
            ("src/core/db.rs", "pub fn query() {}\n"),
        ];
        let g = RepoLayering::fit(files, Language::Rust);
        assert!(g.contains_edge(&("api".into(), "core".into())));
        assert!(g.is_sink("core"));
        // a core file importing api reverses the attested direction.
        assert_eq!(
            g.fires("src/core/db.rs", "use crate::api::handler;\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn rust_file_modules_use_stem_layer_not_root() {
        // A *file* module directly under src (`src/color.rs`) is its own layer
        // `color` — matching the `use crate::color::…` target — not `__root__`.
        // Otherwise every top-level file collapses to one bucket and no edge forms.
        let files = vec![
            (
                "crates/printer/src/printer.rs",
                "use crate::color::Style;\n",
            ),
            ("crates/printer/src/color.rs", "pub struct Style;\n"),
            (
                "crates/printer/src/lib.rs",
                "pub mod color;\npub mod printer;\n",
            ),
        ];
        let g = RepoLayering::fit(files, Language::Rust);
        assert_eq!(
            g.layer_of("crates/printer/src/color.rs").as_deref(),
            Some("color")
        );
        assert_eq!(
            g.layer_of("crates/printer/src/lib.rs").as_deref(),
            Some("__root__")
        );
        assert!(g.contains_edge(&("printer".into(), "color".into())));
        assert!(g.is_sink("color"));
        assert_eq!(
            g.fires("crates/printer/src/color.rs", "use crate::printer::P;\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn rust_workspace_crate_import_is_internal() {
        // `use argot_core::…` where crate `argot-core` is a workspace member
        // (owns a `src/`) resolves like `crate::` — an internal cross-layer edge.
        let files = vec![
            ("crates/cli/src/app/run.rs", "use argot_core::scoring::x;\n"),
            ("crates/argot-core/src/scoring/x.rs", "pub fn x() {}\n"),
        ];
        let g = RepoLayering::fit(files, Language::Rust);
        assert!(g.contains_edge(&("app".into(), "scoring".into())));
    }

    #[test]
    fn java_resolver_builds_directional_graph() {
        // base package com.x; web -> core (attested), core is a sink.
        let files = vec![
            (
                "src/main/java/com/x/web/Ctrl.java",
                "package com.x.web;\nimport com.x.core.Service;\n",
            ),
            (
                "src/main/java/com/x/core/Service.java",
                "package com.x.core;\npublic class Service {}\n",
            ),
        ];
        let g = RepoLayering::fit(files, Language::Java);
        assert!(g.contains_edge(&("web".into(), "core".into())));
        assert!(g.is_sink("core"));
        assert_eq!(
            g.fires(
                "src/main/java/com/x/core/Service.java",
                "import com.x.web.Ctrl;\n"
            ),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn relative_spec_round_trips_through_normalize_join() {
        // sibling layer
        let s = relative_spec("src/components", "src/data/store.ts", true);
        assert_eq!(s, "../data/store");
        assert_eq!(normalize_join("src/components", &s), "src/data/store");
        // deeper host
        let s = relative_spec("src/a/b", "src/x/y.tsx", true);
        assert_eq!(normalize_join("src/a/b", &s), "src/x/y");
    }

    #[test]
    fn typescript_example_import_synthesizes_relative_edge() {
        let files = vec![
            ("src/ui/App.tsx", "import { store } from '../data/store';\n"),
            ("src/data/store.ts", "export const store = {};\n"),
        ];
        let g = RepoLayering::fit(files, Language::Typescript);
        // ui -> data attested; synthesize the reverse (data importing ui) → a real edge.
        let line = g
            .example_import("src/data/store.ts", "ui", Some("src/ui/App.tsx"))
            .expect("should synthesize a relative import to layer ui");
        assert!(g
            .file_edges("src/data/store.ts", &line)
            .contains(&("data".into(), "ui".into())));
    }

    #[test]
    fn detect_bases_splits_dominant_and_minority_roots() {
        // guava shape: com.google.common.* (dominant) + com.google.thirdparty.* (minority).
        let p = |s: &str| s.split('.').map(String::from).collect::<Vec<_>>();
        let mut pkgs = vec![p("com.google.thirdparty.publicsuffix")];
        for layer in ["collect", "base", "cache", "io", "math", "net", "hash"] {
            pkgs.push(p(&format!("com.google.common.{layer}")));
            pkgs.push(p(&format!("com.google.common.{layer}"))); // weight the dominant root
        }
        let mut bases: Vec<String> = detect_bases(&pkgs).iter().map(|b| b.join(".")).collect();
        bases.sort();
        assert_eq!(bases, vec!["com.google.common", "com.google.thirdparty"]);

        // junit shape: balanced fan-out under org.junit → single base, children are layers.
        let jpkgs = vec![
            p("org.junit.jupiter.api"),
            p("org.junit.platform.engine"),
            p("org.junit.vintage.runner"),
        ];
        assert_eq!(
            detect_bases(&jpkgs)
                .iter()
                .map(|b| b.join("."))
                .collect::<Vec<_>>(),
            vec!["org.junit".to_string()]
        );
    }

    #[test]
    fn java_multi_base_repo_does_not_collapse_layers() {
        // A repo shipping a dominant package root plus a minority sibling (guava's
        // common + thirdparty). Layers must be the package after the RIGHT base, not
        // the short common prefix — else everything buckets into one layer.
        let mut files = vec![
            (
                "guava/src/com/google/common/collect/Lists.java".to_string(),
                "package com.google.common.collect;\nimport com.google.common.base.Preconditions;\n"
                    .to_string(),
            ),
            (
                "guava/src/com/google/common/base/Preconditions.java".to_string(),
                "package com.google.common.base;\npublic class Preconditions {}\n".to_string(),
            ),
            (
                "guava/src/com/google/thirdparty/publicsuffix/TrieParser.java".to_string(),
                "package com.google.thirdparty.publicsuffix;\npublic class TrieParser {}\n"
                    .to_string(),
            ),
        ];
        // Weight `common` so it is the dominant root (≥70%), as in real guava.
        for layer in ["io", "math", "net", "hash", "cache", "escape"] {
            files.push((
                format!("guava/src/com/google/common/{layer}/X.java"),
                format!("package com.google.common.{layer};\npublic class X {{}}\n"),
            ));
        }
        let g = RepoLayering::fit(
            files.iter().map(|(p, s)| (p.as_str(), s.as_str())),
            Language::Java,
        );
        assert_eq!(
            g.layer_of("guava/src/com/google/common/collect/Lists.java")
                .as_deref(),
            Some("collect")
        );
        assert_eq!(
            g.layer_of("guava/src/com/google/thirdparty/publicsuffix/TrieParser.java")
                .as_deref(),
            Some("publicsuffix")
        );
        assert!(g.contains_edge(&("collect".into(), "base".into())));
    }

    #[test]
    fn php_resolver_builds_directional_graph() {
        // PSR-4 root namespace App; Http -> Models (attested), Models is a sink.
        let files = vec![
            (
                "src/Http/Kernel.php",
                "<?php\nnamespace App\\Http;\nuse App\\Models\\User;\n",
            ),
            (
                "src/Models/User.php",
                "<?php\nnamespace App\\Models;\nclass User {}\n",
            ),
        ];
        let g = RepoLayering::fit(files, Language::Php);
        assert!(g.contains_edge(&("Http".into(), "Models".into())));
        assert!(g.is_sink("Models"));
        // src layer comes from the path (the PSR-4 root is learned at fit), so the
        // hunk needs no `namespace` line to reverse the attested direction — only the
        // `<?php` tag the whole engine requires to parse PHP into non-inert nodes.
        assert_eq!(
            g.fires("src/Models/User.php", "<?php\nuse App\\Http\\Kernel;\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn csharp_resolver_builds_directional_graph() {
        // base namespace Acme.App; Web -> Core (attested), Core is a sink.
        let files = vec![
            (
                "src/Web/Controller.cs",
                "namespace Acme.App.Web;\nusing Acme.App.Core;\nclass Controller {}\n",
            ),
            (
                "src/Core/Service.cs",
                "namespace Acme.App.Core;\nclass Service {}\n",
            ),
        ];
        let g = RepoLayering::fit(files, Language::CSharp);
        assert!(g.contains_edge(&("Web".into(), "Core".into())));
        assert!(g.is_sink("Core"));
        assert_eq!(
            g.fires("src/Core/Service.cs", "using Acme.App.Web;\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn csharp_flat_dotted_dirs_use_namespace_layer_not_directory() {
        // The .NET convention (powershell/jellyfin): each project is ONE flat dir
        // whose name IS the dotted namespace — `src/Acme.App.Web/` not `src/Web/`.
        // The directory's first component (`src`) is a single bucket; the layer must
        // come from the namespace (`Web`/`Core`), or every file collapses to `src`
        // and no cross-layer edge forms.
        let files = vec![
            (
                "src/Acme.App.Web/Controller.cs",
                "namespace Acme.App.Web;\nusing Acme.App.Core;\nclass Controller {}\n",
            ),
            (
                "src/Acme.App.Core/Service.cs",
                "namespace Acme.App.Core;\nclass Service {}\n",
            ),
        ];
        let g = RepoLayering::fit(files, Language::CSharp);
        // Source layer is namespace-derived, so the edge is Web -> Core, not src -> *.
        assert!(g.contains_edge(&("Web".into(), "Core".into())));
        assert_eq!(
            g.layer_of("src/Acme.App.Web/Controller.cs").as_deref(),
            Some("Web")
        );
        assert!(g.is_sink("Core"));
        assert_eq!(
            g.fires("src/Acme.App.Core/Service.cs", "using Acme.App.Web;\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn transitive_reversal_fires_on_back_edge_without_direct_reverse() {
        // Chain a -> b -> c (a near-DAG layering). A novel c -> a has no *direct*
        // reverse edge (a -> c is not attested), but a reaches c transitively, so
        // c -> a closes a cycle — a back-edge the direct-reversal rule alone misses.
        let files = vec![
            ("pkg/__init__.py", ""),
            ("pkg/a/__init__.py", ""),
            ("pkg/a/x.py", "from pkg.b import y\n"),
            ("pkg/b/__init__.py", ""),
            ("pkg/b/y.py", "from pkg.c import z\n"),
            ("pkg/c/__init__.py", ""),
            ("pkg/c/z.py", "value = 1\n"),
        ];
        let g = RepoLayering::fit(files, Language::Python);
        assert!(g.contains_edge(&("a".into(), "b".into())));
        assert!(g.contains_edge(&("b".into(), "c".into())));
        assert_eq!(
            g.classify(&("c".into(), "a".into())),
            Some(Violation::TransitiveReversal)
        );
        // a -> c is forward (a is upstream of c) — legit organic growth, no fire.
        assert_eq!(g.classify(&("a".into(), "c".into())), None);
    }

    #[test]
    fn ruby_resolver_resolves_relative_requires() {
        // app/controllers -> app/models via `require_relative`; models is a sink.
        let files = vec![
            (
                "app/controllers/users.rb",
                "require_relative \"../models/user\"\n",
            ),
            ("app/models/user.rb", "class User\nend\n"),
        ];
        let g = RepoLayering::fit(files, Language::Ruby);
        assert!(g.contains_edge(&("controllers".into(), "models".into())));
        assert!(g.is_sink("models"));
        assert_eq!(
            g.fires(
                "app/models/user.rb",
                "require_relative \"../controllers/users\"\n"
            ),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn c_resolver_builds_directional_graph_from_includes() {
        // src/net -> src/core via a quoted include; core is a sink.
        let files = vec![
            (
                "src/net/conn.c",
                "#include \"core/alloc.h\"\n#include <stdio.h>\n",
            ),
            ("src/core/alloc.c", "int alloc(void) { return 0; }\n"),
            ("src/core/alloc.h", "int alloc(void);\n"),
        ];
        let g = RepoLayering::fit(files, Language::C);
        assert!(g.contains_edge(&("net".into(), "core".into())));
        assert!(g.is_sink("core"));
        // a core file including net reverses the attested direction.
        assert_eq!(
            g.fires("src/core/alloc.c", "#include \"net/conn.h\"\n"),
            Some(Violation::Reversal)
        );
        // a system include never fires.
        assert_eq!(g.fires("src/core/alloc.c", "#include <stdlib.h>\n"), None);
    }

    #[test]
    fn cpp_resolver_builds_directional_graph_from_includes() {
        // src/render -> src/scene via a quoted include; scene is a sink.
        let files = vec![
            (
                "src/render/pass.cpp",
                "#include \"scene/node.hpp\"\n#include <vector>\n",
            ),
            ("src/scene/node.cpp", "int node() { return 0; }\n"),
            ("src/scene/node.hpp", "int node();\n"),
        ];
        let g = RepoLayering::fit(files, Language::Cpp);
        assert!(g.contains_edge(&("render".into(), "scene".into())));
        assert!(g.is_sink("scene"));
        assert_eq!(
            g.fires("src/scene/node.cpp", "#include \"render/pass.hpp\"\n"),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn go_resolver_builds_directional_graph() {
        // module gohugoio/hugo; tpl imports common (attested); common is a sink.
        let files = vec![
            ("go.mod", "module github.com/x/hugo\n\ngo 1.21\n"),
            (
                "tpl/tpl.go",
                "package tpl\nimport \"github.com/x/hugo/common/loggers\"\n",
            ),
            ("common/loggers/log.go", "package loggers\n"),
        ];
        let g = RepoLayering::fit(files, Language::Go);
        assert!(g.contains_edge(&("tpl".into(), "common".into())));
        assert!(g.is_sink("common"));
        // a common file importing tpl reverses the attested direction.
        assert_eq!(
            g.fires(
                "common/loggers/log.go",
                "import \"github.com/x/hugo/tpl\"\n"
            ),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn ts_resolver_resolves_relative_imports() {
        // src/middleware imports src/helper (attested); helper is a sink.
        let files = vec![
            (
                "src/middleware/auth.ts",
                "import { cookie } from '../helper/cookie';\n",
            ),
            ("src/helper/cookie.ts", "export const cookie = 1;\n"),
        ];
        let g = RepoLayering::fit(files, Language::Typescript);
        assert!(g.contains_edge(&("middleware".into(), "helper".into())));
        // helper importing middleware reverses it.
        assert_eq!(
            g.fires(
                "src/helper/cookie.ts",
                "import { x } from '../middleware/auth';\n"
            ),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn per_language_set_json_roundtrips_and_routes_by_language() {
        let ts = RepoLayering::fit(
            vec![
                (
                    "src/middleware/auth.ts",
                    "import { cookie } from '../helper/cookie';\n",
                ),
                ("src/helper/cookie.ts", "export const cookie = 1;\n"),
            ],
            Language::Typescript,
        );
        let java = RepoLayering::fit(
            vec![
                (
                    "guava/src/com/google/common/collect/Lists.java",
                    "package com.google.common.collect;\nimport com.google.common.base.Preconditions;\n",
                ),
                (
                    "guava/src/com/google/common/base/Preconditions.java",
                    "package com.google.common.base;\n",
                ),
            ],
            Language::Java,
        );
        assert!(ts.edge_count() > 0 && java.edge_count() > 0);

        let json = RepoLayering::set_to_json(&[ts, java], "sha1");
        let set = RepoLayering::set_from_json(&json);
        assert_eq!(set.len(), 2);
        let by: std::collections::HashMap<&str, &RepoLayering> =
            set.iter().map(|g| (g.language_tag(), g)).collect();
        // Each language's graph routes and fires its own reversal.
        assert_eq!(
            by["typescript"].fires(
                "src/helper/cookie.ts",
                "import { x } from '../middleware/auth';\n"
            ),
            Some(Violation::Reversal)
        );

        // A legacy single-graph artifact still loads as a one-element set.
        let legacy = RepoLayering::set_from_json(&by["java"].to_json("sha2"));
        assert_eq!(legacy.len(), 1);
        assert_eq!(legacy[0].language_tag(), "java");
    }

    #[test]
    fn java_multi_module_shares_base_package() {
        // Two src trees (android/guava/src + guava/src) sharing base com.google.common
        // — the GLOBAL common prefix collapses, but per-src-root detection works.
        let files = vec![
            (
                "android/guava/src/com/google/common/collect/Lists.java",
                "package com.google.common.collect;\nimport com.google.common.base.Preconditions;\n",
            ),
            (
                "guava/src/com/google/common/base/Preconditions.java",
                "package com.google.common.base;\n",
            ),
            (
                "guava/src/com/google/common/collect/Sets.java",
                "package com.google.common.collect;\nimport com.google.common.base.Objects;\n",
            ),
        ];
        let g = RepoLayering::fit(files, Language::Java);
        assert!(
            g.contains_edge(&("collect".into(), "base".into())),
            "collect->base must be attested across both src trees"
        );
        // base importing collect reverses the attested direction.
        assert_eq!(
            g.fires(
                "guava/src/com/google/common/base/Preconditions.java",
                "import com.google.common.collect.Lists;\n"
            ),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn json_round_trip_preserves_edges_and_sinks() {
        let g = fit_py();
        let restored = RepoLayering::from_json(&g.to_json("deadbeef")).expect("valid json");
        assert!(restored.contains_edge(&("views".into(), "models".into())));
        assert!(restored.is_sink("models"));
        assert_eq!(
            restored.classify(&("models".into(), "views".into())),
            Some(Violation::Reversal)
        );
    }
}
