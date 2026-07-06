//! Import-graph foreign-module scorer — port of
//! `engine/argot/scoring/scorers/import_graph.py`.
//!
//! Counts the top-level modules a hunk imports that were never seen in the
//! repo's own source. Score 0 = no foreign imports; score ≥ 1 =
//! paradigm-break signal.
//!
//! Notes:
//! - Top-level module only: `from sqlalchemy.orm import Session` → `sqlalchemy`.
//! - Stdlib gets no special treatment: an unimported `threading` is foreign.
//! - Relative imports are ignored (handled by the adapter's `extract_imports`).
//! - The Python module also carries dead `_imports_from_ast`/`_imports_from_regex`
//!   helpers; those are not ported — the scorer uses `adapter.extract_imports`.

use std::collections::HashSet;

use crate::scoring::adapters::python::PythonAdapter;

/// Score a hunk by how many modules it imports that are foreign to the repo.
#[derive(Debug, Clone, Default)]
pub struct ImportGraphScorer {
    repo_modules: HashSet<String>,
    repo_modules_prefixes: HashSet<String>,
}

impl ImportGraphScorer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Collect the union of top-level modules imported across the repo corpus.
    ///
    /// Python's `resolve_repo_modules` returns an empty `RepoModules` for
    /// Python, so `repo_modules_prefixes` stays empty here.
    pub fn fit(&mut self, repo_sources: &[String], adapter: &PythonAdapter) {
        let mut seen: HashSet<String> = HashSet::new();
        for source in repo_sources {
            seen.extend(adapter.extract_imports(source));
        }
        self.repo_modules = seen;
        self.repo_modules_prefixes = HashSet::new();
    }

    /// Restore a fit-time snapshot of the foreign-module surface.
    ///
    /// At check time we flag imports the user just added, so we cannot derive
    /// `repo_modules` from current file contents (a fresh `import mongoose`
    /// would land in the set immediately). The snapshot persisted in
    /// `scorer-config.json` decouples "known at fit time" from "on disk now".
    pub fn load_snapshot<M, P>(&mut self, modules: M, prefixes: P)
    where
        M: IntoIterator<Item = String>,
        P: IntoIterator<Item = String>,
    {
        self.repo_modules = modules.into_iter().collect();
        self.repo_modules_prefixes = prefixes.into_iter().collect();
    }

    /// True if `spec` is not a known internal module specifier.
    ///
    /// `__future__` is a Python compiler directive available in every file, not
    /// a third-party dependency — a repo that never wrote `from __future__
    /// import …` must not have it read as a foreign import when a later commit
    /// adds one. Treated as always-known.
    pub fn is_foreign(&self, spec: &str) -> bool {
        spec != "__future__"
            && !self.repo_modules.contains(spec)
            && !self
                .repo_modules_prefixes
                .iter()
                .any(|p| spec.starts_with(p))
    }

    /// Count of top-level modules in `hunk_source` not seen in the repo.
    pub fn score_hunk(&self, hunk_source: &str, adapter: &PythonAdapter) -> f64 {
        let hunk_modules = adapter.extract_imports(hunk_source);
        hunk_modules
            .iter()
            .filter(|spec| self.is_foreign(spec))
            .count() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn foreign_imports_are_counted() {
        let adapter = PythonAdapter::new();
        let mut scorer = ImportGraphScorer::new();
        scorer.fit(&["import os\nimport json\n".to_string()], &adapter);

        // os is known, requests is foreign.
        assert!(!scorer.is_foreign("os"));
        assert!(scorer.is_foreign("requests"));
        assert_eq!(scorer.score_hunk("import os\n", &adapter), 0.0);
        assert_eq!(scorer.score_hunk("import requests\n", &adapter), 1.0);
        assert_eq!(
            scorer.score_hunk("import requests\nimport flask\n", &adapter),
            2.0
        );
    }

    #[test]
    fn snapshot_and_prefixes() {
        let mut scorer = ImportGraphScorer::new();
        scorer.load_snapshot(["numpy".to_string()], ["myapp".to_string()]);
        assert!(!scorer.is_foreign("numpy"));
        assert!(!scorer.is_foreign("myapp.sub")); // prefix match
        assert!(scorer.is_foreign("pandas"));
    }

    #[test]
    fn future_is_never_foreign() {
        // `from __future__ import annotations` is a compiler directive, not a
        // dependency — a repo that never used it must not flag it as foreign.
        let scorer = ImportGraphScorer::new(); // empty repo_modules
        assert!(!scorer.is_foreign("__future__"));
        assert!(scorer.is_foreign("requests"));
    }
}
