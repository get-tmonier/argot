//! Language adapters — port of `engine/argot/scoring/adapters`.
//!
//! An adapter wraps a language's tree-sitter parser plus the structural
//! filters (data-dominant, auto-generated) behind a uniform surface used by
//! the scorers and the sampler.

pub mod java;
pub mod python;
pub mod typescript;

use std::collections::HashSet;
use std::path::Path;

/// Scoring-side language tag. JavaScript routes to the TypeScript adapter, so
/// scoring distinguishes Python, TypeScript, and Java.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    Typescript,
    Java,
}

/// Uniform language-adapter surface (port of the Python `LanguageAdapter`
/// Protocol). Implemented by `PythonAdapter` and `TypeScriptAdapter`; the
/// scorers dispatch through `&dyn LanguageAdapter`.
pub trait LanguageAdapter {
    fn language(&self) -> Language;
    fn extract_imports(&self, source: &str) -> HashSet<String>;
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)>;
    fn resolve_repo_modules(&self, repo_root: &Path) -> RepoModules;
    fn is_data_dominant(&self, source: &str) -> bool;
    /// 1-indexed line numbers covered by static data-literal spans — the
    /// row-granular view behind `is_data_dominant`, used to skip data-row
    /// hunks while still judging code rows in the same file.
    fn data_literal_lines(&self, source: &str) -> HashSet<usize>;
    /// Names the source binds to callable definitions (functions, classes,
    /// function-valued variables). Local-binding attestation: code calling
    /// what it defines is not foreign voice.
    fn callable_definitions(&self, source: &str) -> HashSet<String>;
    /// Names bound by imports from repo-internal (relative) specifiers.
    fn internal_import_bindings(&self, source: &str) -> HashSet<String>;
    /// Every other locally bound value name (destructured names, plain
    /// consts/vars, parameters). Attests bare calls only — calling a value
    /// you just bound is neighbourhood behaviour, not foreign voice.
    fn value_bindings(&self, source: &str) -> HashSet<String>;
    fn is_auto_generated(&self, source: &str) -> bool;
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)>;
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize>;
    fn identifier_noise(&self) -> &HashSet<String>;
    /// The language's line-comment token — drives inline suppression-comment
    /// parsing (`# argot: …` vs `// argot: …`).
    fn line_comment_prefix(&self) -> &'static str;
}

impl LanguageAdapter for python::PythonAdapter {
    fn language(&self) -> Language {
        Language::Python
    }
    fn extract_imports(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::extract_imports(self, source)
    }
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        python::PythonAdapter::extract_imports_with_spans(self, source)
    }
    fn resolve_repo_modules(&self, _repo_root: &Path) -> RepoModules {
        // Python internal modules are discovered via extract_imports at fit
        // time; there are no exact/prefix rules.
        RepoModules::default()
    }
    fn is_data_dominant(&self, source: &str) -> bool {
        python::PythonAdapter::is_data_dominant(self, source)
    }
    fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        python::PythonAdapter::data_literal_lines(self, source)
    }
    fn callable_definitions(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::callable_definitions(self, source)
    }
    fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::internal_import_bindings(self, source)
    }
    fn value_bindings(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::value_bindings(self, source)
    }
    fn is_auto_generated(&self, source: &str) -> bool {
        python::PythonAdapter::is_auto_generated(self, source)
    }
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        python::PythonAdapter::enumerate_sampleable_ranges(self, source)
    }
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        python::PythonAdapter::prose_line_ranges(self, source)
    }
    fn identifier_noise(&self) -> &HashSet<String> {
        python::PythonAdapter::identifier_noise(self)
    }
    fn line_comment_prefix(&self) -> &'static str {
        "#"
    }
}

/// The set of module specifiers a repo owns, split into exact matches and
/// prefix matches. Mirrors Python's `RepoModules` dataclass.
///
/// For Python this is always empty — internal modules are discovered via
/// `extract_imports` at fit time, and there are no prefix rules — but the
/// type exists so the import-graph scorer's `is_foreign` logic stays
/// language-agnostic.
#[derive(Debug, Clone, Default)]
pub struct RepoModules {
    pub exact: HashSet<String>,
    pub prefixes: HashSet<String>,
}
