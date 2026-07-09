//! Architecture-graph foreignness — the relationship analog of the foreign-
//! vocabulary gate. Feature-gated (`--features arch`), pure-Rust (no new deps),
//! advisory / measurement-only: **not** wired into the base gating path, so the
//! shipped guardrail is byte-for-byte unchanged with or without it.
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
//! v1 resolves internal imports for **Python** (the best-validated corpus set);
//! other languages return no edges — a graceful no-op (no findings), exactly like
//! the semantic layer shipped Python+TS first. Go / TypeScript / … plug in via
//! [`Language`]-dispatch in [`RepoLayering::file_edges`].

use std::collections::{HashMap, HashSet};

use crate::scoring::adapters::Language;
use crate::scoring::ts_parse;

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

fn tag_lang(s: &str) -> Language {
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
        };
        for (path, source) in &files {
            for e in me.file_edges(path, source) {
                *me.edges.entry(e).or_insert(0) += 1;
            }
        }
        me.recompute_sinks();
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

    /// The layer of a file: the first path component under its enclosing root.
    /// An empty root (`""`) = repo root and matches any path (Go-style, layer =
    /// first path component). Language-agnostic.
    pub fn layer_of(&self, path: &str) -> Option<String> {
        let dir = parent_dir(path);
        let root = self
            .roots
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
        let Some(src) = self.layer_of(path) else {
            return HashSet::new();
        };
        let targets = match self.language {
            Language::Python => self.py_targets(path, source),
            Language::Go => self.go_targets(source),
            Language::Typescript | Language::Javascript => self.ts_targets(path, source),
            // Other resolvers plug in here (Java/Rust/PHP/C#/Ruby/C/C++).
            _ => Vec::new(),
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

    /// Classify a *new* edge (one not already attested). Returns the violation
    /// tell, or `None` if the edge is a clean novel-forward edge (organic growth).
    pub fn classify(&self, edge: &Edge) -> Option<Violation> {
        if self.edges.contains_key(edge) {
            return None; // already attested — not novel
        }
        let (a, b) = edge;
        if self.edges.contains_key(&(b.clone(), a.clone())) {
            Some(Violation::Reversal)
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

    /// Serialize the fitted graph for persistence in `.argot/layering.json`.
    pub fn to_json(&self, repo_sha: &str) -> String {
        let art = LayeringArtifact {
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
        };
        serde_json::to_string(&art).unwrap_or_default()
    }

    /// Restore a fitted graph persisted by [`to_json`]. `None` on unreadable JSON.
    pub fn from_json(s: &str) -> Option<Self> {
        let art: LayeringArtifact = serde_json::from_str(s).ok()?;
        Some(RepoLayering {
            language: tag_lang(&art.language),
            internal: art.internal.into_iter().collect(),
            roots: art.roots,
            module_path: art.module_path,
            edges: art.edges.into_iter().map(|(a, b, w)| ((a, b), w)).collect(),
            sinks: art.sinks.into_iter().collect(),
        })
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

fn parent_dir(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[..i],
        None => "",
    }
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
        _ => (vec![String::new()], HashSet::new(), None),
    }
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
    fn unimplemented_language_is_graceful_noop() {
        // Rust has no resolver yet → an empty graph, no edges — a graceful no-op.
        let g = RepoLayering::fit(
            vec![("src/main.rs", "use crate::foo::bar;")],
            Language::Rust,
        );
        assert_eq!(g.edge_count(), 0);
        assert!(g.file_edges("src/lib.rs", "use crate::x::y;").is_empty());
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
