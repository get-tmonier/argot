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
//! - **sink-out:** `a` is a repo **sink** (a leaf that is imported but never
//!   imports cross-layer — `utils`/`models`/`constants`) now importing outward.
//!
//! Firing on *any* novel edge over-fires (organic growth adds edges constantly,
//! up to ~36%); the reversal/sink discrimination is what keeps it ≤2% on real
//! temporal holdout. Domain-blind: "layer" = the path component under a package
//! root, never a hardcoded layer name.
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

/// Why a novel edge is a violation — the discrete, low-FP tells.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Violation {
    /// The reverse edge is attested — this reverses an established direction.
    Reversal,
    /// The source layer is a repo sink (imported-but-never-imports-out).
    SinkOut,
}

/// The repo's module-dependency topology, fitted from its files at a pinned SHA.
#[derive(Debug, Clone, Default)]
pub struct RepoLayering {
    /// Basenames of top-level package-root dirs (for internal-import detection).
    py_packages: HashSet<String>,
    /// Absolute-ish package-root dir paths (for `layer_of` resolution).
    py_roots: Vec<String>,
    /// Weighted directed cross-layer edges.
    edges: HashMap<Edge, u32>,
    /// Layers that are pure sinks: cross-layer in-degree > 0, out-degree == 0.
    sinks: HashSet<String>,
}

impl RepoLayering {
    /// Fit the layering graph from the repo's files (`rel_path`, `source`, lang).
    /// `rel_path` is repo-root-relative with `/` separators.
    pub fn fit<'a, I>(files: I) -> Self
    where
        I: IntoIterator<Item = (&'a str, &'a str, Language)>,
    {
        let files: Vec<(&str, &str, Language)> = files.into_iter().collect();
        // Python package roots: a dir with an __init__.py whose parent has none.
        let init_dirs: HashSet<String> = files
            .iter()
            .filter(|(p, _, _)| p.ends_with("__init__.py"))
            .map(|(p, _, _)| parent_dir(p).to_string())
            .collect();
        let mut py_roots: Vec<String> = Vec::new();
        for d in &init_dirs {
            if !init_dirs.contains(parent_dir(d)) && !is_noise_path(&format!("{d}/")) {
                py_roots.push(d.clone());
            }
        }
        py_roots.sort_by_key(|d| std::cmp::Reverse(d.len())); // longest match first
        let py_packages: HashSet<String> =
            py_roots.iter().map(|d| basename(d).to_string()).collect();

        let mut me = RepoLayering {
            py_packages,
            py_roots,
            edges: HashMap::new(),
            sinks: HashSet::new(),
        };
        for (path, source, lang) in &files {
            if is_noise_path(path) {
                continue;
            }
            for e in me.file_edges(path, source, *lang) {
                *me.edges.entry(e).or_insert(0) += 1;
            }
        }
        me.recompute_sinks();
        me
    }

    fn recompute_sinks(&mut self) {
        let mut out_deg: HashMap<&str, u32> = HashMap::new();
        let mut in_deg: HashMap<&str, u32> = HashMap::new();
        for (a, b) in self.edges.keys() {
            *out_deg.entry(a).or_insert(0) += 1;
            *in_deg.entry(b).or_insert(0) += 1;
        }
        self.sinks = in_deg
            .keys()
            .filter(|l| out_deg.get(**l).copied().unwrap_or(0) == 0)
            .map(|l| l.to_string())
            .collect();
    }

    /// The layer of a file: the path component under its enclosing package root.
    fn py_layer_of(&self, path: &str) -> Option<String> {
        // enclosing root = longest py_root that is an ancestor dir of `path`
        let dir = parent_dir(path);
        let root = self
            .py_roots
            .iter()
            .find(|r| dir == r.as_str() || dir.starts_with(&format!("{r}/")))?;
        let rel = &path[root.len()..].trim_start_matches('/');
        let parts: Vec<&str> = rel.split('/').collect();
        Some(if parts.len() > 1 {
            parts[0].to_string()
        } else {
            "__root__".to_string()
        })
    }

    /// Cross-layer edges a single file introduces (language-dispatched).
    pub fn file_edges(&self, path: &str, source: &str, lang: Language) -> HashSet<Edge> {
        match lang {
            Language::Python => self.py_file_edges(path, source),
            _ => HashSet::new(), // graceful no-op until a resolver is added
        }
    }

    fn py_file_edges(&self, path: &str, source: &str) -> HashSet<Edge> {
        let mut out = HashSet::new();
        let Some(src_layer) = self.py_layer_of(path) else {
            return out;
        };
        let dir = parent_dir(path);
        let root = self
            .py_roots
            .iter()
            .find(|r| dir == r.as_str() || dir.starts_with(&format!("{r}/")));
        let pkg_rel_parts: Vec<String> = match root {
            Some(r) => path[r.len()..]
                .trim_start_matches('/')
                .split('/')
                .map(|s| s.to_string())
                .collect(),
            None => return out,
        };
        for (module, level) in py_imports(source) {
            let tgt = if level == 0 {
                let p: Vec<&str> = module.split('.').filter(|s| !s.is_empty()).collect();
                if p.is_empty() || !self.py_packages.contains(p[0]) {
                    continue; // external dep — the base gate's job
                }
                if p.len() > 1 {
                    p[1].to_string()
                } else {
                    "__root__".to_string()
                }
            } else {
                // relative: climb (level-1) from the file's package dir
                let mut base: Vec<&str> = pkg_rel_parts[..pkg_rel_parts.len().saturating_sub(1)]
                    .iter()
                    .map(|s| s.as_str())
                    .collect();
                let up = level - 1;
                if up <= base.len() {
                    base.truncate(base.len() - up);
                } else {
                    base.clear();
                }
                let tail: Vec<&str> = module.split('.').filter(|s| !s.is_empty()).collect();
                base.extend(tail);
                match base.first() {
                    Some(f) => f.to_string(),
                    None => "__root__".to_string(),
                }
            };
            if tgt != src_layer {
                out.insert((src_layer.clone(), tgt));
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
    pub fn fires(&self, path: &str, hunk: &str, lang: Language) -> Option<Violation> {
        self.file_edges(path, hunk, lang)
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

fn basename(path: &str) -> &str {
    match path.rfind('/') {
        Some(i) => &path[i + 1..],
        None => path,
    }
}

fn is_noise_path(path: &str) -> bool {
    const SKIP: &[&str] = &[
        "/test",
        "/tests/",
        "test_",
        "_test.",
        "/migrations/",
        "/vendor/",
        "/third_party/",
        "/examples/",
        "/example/",
        "/docs/",
        "/node_modules/",
    ];
    SKIP.iter().any(|s| path.contains(s))
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
    fn fixture() -> Vec<(&'static str, &'static str, Language)> {
        vec![
            ("app/__init__.py", "", Language::Python),
            ("app/models/__init__.py", "", Language::Python),
            ("app/models/user.py", "class User: pass\n", Language::Python),
            ("app/views/__init__.py", "", Language::Python),
            (
                "app/views/page.py",
                "from app.models import user\n",
                Language::Python,
            ),
        ]
    }

    #[test]
    fn fit_builds_directional_graph() {
        let g = RepoLayering::fit(fixture());
        // views -> models attested; models is a sink (imported, never imports out).
        assert!(g.contains_edge(&("views".into(), "models".into())));
        assert!(g.is_sink("models"));
        assert!(!g.is_sink("views"));
    }

    #[test]
    fn reversal_fires_sink_out_fires_forward_does_not() {
        let g = RepoLayering::fit(fixture());
        // REVERSAL: a model importing a view (reverses views->models).
        assert_eq!(
            g.fires(
                "app/models/user.py",
                "from app.views import page\n",
                Language::Python
            ),
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
            g.fires(
                "app/views/page.py",
                "from app.models import user\n",
                Language::Python
            ),
            None
        );
    }

    #[test]
    fn external_imports_are_ignored() {
        let g = RepoLayering::fit(fixture());
        // numpy is not a repo package — no internal edge, nothing fires.
        assert_eq!(
            g.fires(
                "app/models/user.py",
                "import numpy as np\n",
                Language::Python
            ),
            None
        );
    }

    #[test]
    fn relative_imports_resolve() {
        let g = RepoLayering::fit(fixture());
        // `from ..views import page` inside models/ -> edge models->views (reversal).
        assert_eq!(
            g.fires(
                "app/models/user.py",
                "from ..views import page\n",
                Language::Python
            ),
            Some(Violation::Reversal)
        );
    }

    #[test]
    fn non_python_is_graceful_noop() {
        let g = RepoLayering::fit(fixture());
        assert!(g
            .file_edges("x.rs", "use crate::foo::bar;", Language::Rust)
            .is_empty());
    }
}
