//! Language adapters — port of `engine/argot/scoring/adapters`.
//!
//! An adapter wraps a language's tree-sitter parser plus the structural
//! filters (data-dominant, auto-generated) behind a uniform surface used by
//! the scorers and the sampler.

pub mod c;
pub mod cpp;
pub mod csharp;
pub mod go;
pub mod java;
pub mod javascript;
pub mod php;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod typescript;

use std::collections::HashSet;
use std::path::Path;

/// How many head lines of a file are scanned for generated-file markers. Shared
/// by every adapter's `is_auto_generated` (a marker below this is treated as an
/// ordinary comment, not a machine-authorship banner).
pub(crate) const HEADER_LINE_LIMIT: usize = 20;

/// Scoring-side language tag. JavaScript is a first-class language with its own
/// adapter and model, so scoring distinguishes Python, TypeScript, JavaScript,
/// Go, Rust, C, Java, C#, PHP, C++, and Ruby.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Language {
    Python,
    Typescript,
    Javascript,
    Go,
    Rust,
    C,
    Java,
    CSharp,
    Php,
    Cpp,
    Ruby,
}

/// A callable definition (function or method) with its 1-indexed inclusive line
/// range — the unit the semantic index embeds. Distinct from
/// `callable_definitions` (which yields only *names*, for callee attestation):
/// the semantic layer needs the function's *body* to embed. Always compiled
/// (this crate has no cargo features); argot-core's base build simply never
/// calls `callable_bodies` unless built with `--features semantic`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallableBody {
    pub symbol: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// Uniform language-adapter surface (port of the Python `LanguageAdapter`
/// Protocol). Implemented by `PythonAdapter` and `TypeScriptAdapter`; the
/// scorers dispatch through `&dyn LanguageAdapter`.
pub trait LanguageAdapter {
    /// Function/method definitions with their line ranges — the embeddable units
    /// for the semantic index. Default empty: a language with no implementation
    /// simply produces no semantic index (no reinvention/placement findings),
    /// which is a graceful no-op. Implemented for Python and TypeScript in v1.
    fn callable_bodies(&self, _source: &str) -> Vec<CallableBody> {
        Vec::new()
    }
    fn language(&self) -> Language;
    fn extract_imports(&self, source: &str) -> HashSet<String>;
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)>;
    fn resolve_repo_modules(&self, repo_root: &Path) -> RepoModules;
    /// True if the file is overwhelmingly static data literals — `threshold` is
    /// the fraction of non-blank lines that must be data (from
    /// `[detect].data-threshold`).
    fn is_data_dominant(&self, source: &str, threshold: f64) -> bool;
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
    /// Names bound by *non-relative* imports, each paired with the top-level
    /// module it came from: `import numpy as np` → `(np, numpy)`,
    /// `from colorama import Fore, Style` → `(Fore, colorama), (Style, colorama)`.
    /// Lets the call-receiver gate tell "this hunk uses a symbol from a foreign
    /// dependency" (colorama's `Fore`, numpy's `default_rng`) apart from "this
    /// hunk is the repo's own code in a file that merely happens to import one".
    /// Default empty: a language without this simply gets no file-level foreign
    /// association, falling back to hunk-local reach.
    fn import_bindings(&self, _source: &str) -> Vec<(String, String)> {
        Vec::new()
    }
    /// Every other locally bound value name (destructured names, plain
    /// consts/vars, parameters). Attests bare calls only — calling a value
    /// you just bound is neighbourhood behaviour, not foreign voice.
    fn value_bindings(&self, source: &str) -> HashSet<String>;
    /// True if the file's head comments flag it as generated — `markers` is the
    /// configured comment-marker set (from `[detect].generated-markers`).
    /// Language-structural detectors (Go/Rust/C#) ignore `markers`.
    fn is_auto_generated(&self, source: &str, markers: &[String]) -> bool;
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)>;
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize>;
    fn identifier_noise(&self) -> &HashSet<String>;
    /// The language's line-comment token — drives inline suppression-comment
    /// parsing (`# argot: …` vs `// argot: …`).
    fn line_comment_prefix(&self) -> &'static str;
}

/// The canonical adapter factory: the boxed [`LanguageAdapter`] for a scoring
/// language name (`"python"`, `"typescript"`, …), or `None` when unsupported.
pub fn adapter_for(lang: &str) -> Option<Box<dyn LanguageAdapter>> {
    match lang {
        "python" => Some(Box::new(python::PythonAdapter::new())),
        "typescript" => Some(Box::new(typescript::TypeScriptAdapter::new())),
        "javascript" => Some(Box::new(javascript::JavaScriptAdapter::new())),
        "go" => Some(Box::new(go::GoAdapter::new())),
        "rust" => Some(Box::new(rust::RustAdapter::new())),
        "c" => Some(Box::new(c::CAdapter::new())),
        "java" => Some(Box::new(java::JavaAdapter::new())),
        "csharp" => Some(Box::new(csharp::CSharpAdapter::new())),
        "php" => Some(Box::new(php::PhpAdapter::new())),
        "cpp" => Some(Box::new(cpp::CppAdapter::new())),
        "ruby" => Some(Box::new(ruby::RubyAdapter::new())),
        _ => None,
    }
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
    fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        python::PythonAdapter::is_data_dominant(self, source, threshold)
    }
    fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        python::PythonAdapter::data_literal_lines(self, source)
    }
    fn callable_definitions(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::callable_definitions(self, source)
    }
    fn callable_bodies(&self, source: &str) -> Vec<CallableBody> {
        python::PythonAdapter::callable_bodies(self, source)
    }
    fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::internal_import_bindings(self, source)
    }
    fn import_bindings(&self, source: &str) -> Vec<(String, String)> {
        python::PythonAdapter::import_bindings(self, source)
    }
    fn value_bindings(&self, source: &str) -> HashSet<String> {
        python::PythonAdapter::value_bindings(self, source)
    }
    fn is_auto_generated(&self, source: &str, markers: &[String]) -> bool {
        python::PythonAdapter::is_auto_generated(self, source, markers)
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

impl LanguageAdapter for go::GoAdapter {
    fn language(&self) -> Language {
        Language::Go
    }
    fn extract_imports(&self, source: &str) -> HashSet<String> {
        go::GoAdapter::extract_imports(self, source)
    }
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        go::GoAdapter::extract_imports_with_spans(self, source)
    }
    fn resolve_repo_modules(&self, repo_root: &Path) -> RepoModules {
        go::GoAdapter::resolve_repo_modules(self, repo_root)
    }
    fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        go::GoAdapter::is_data_dominant(self, source, threshold)
    }
    fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        go::GoAdapter::data_literal_lines(self, source)
    }
    fn callable_definitions(&self, source: &str) -> HashSet<String> {
        go::GoAdapter::callable_definitions(self, source)
    }
    fn callable_bodies(&self, source: &str) -> Vec<CallableBody> {
        go::GoAdapter::callable_bodies(self, source)
    }
    fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        go::GoAdapter::internal_import_bindings(self, source)
    }
    fn value_bindings(&self, source: &str) -> HashSet<String> {
        go::GoAdapter::value_bindings(self, source)
    }
    fn is_auto_generated(&self, source: &str, markers: &[String]) -> bool {
        go::GoAdapter::is_auto_generated(self, source, markers)
    }
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        go::GoAdapter::enumerate_sampleable_ranges(self, source)
    }
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        go::GoAdapter::prose_line_ranges(self, source)
    }
    fn identifier_noise(&self) -> &HashSet<String> {
        go::GoAdapter::identifier_noise(self)
    }
    fn line_comment_prefix(&self) -> &'static str {
        "//"
    }
}

impl LanguageAdapter for rust::RustAdapter {
    fn language(&self) -> Language {
        Language::Rust
    }
    fn extract_imports(&self, source: &str) -> HashSet<String> {
        rust::RustAdapter::extract_imports(self, source)
    }
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        rust::RustAdapter::extract_imports_with_spans(self, source)
    }
    fn resolve_repo_modules(&self, _repo_root: &Path) -> RepoModules {
        // Rust internal modules are discovered via extract_imports at fit time
        // (and `crate::`/`self::`/`super::` paths are already routed to
        // internal_import_bindings); there are no exact/prefix rules read from
        // Cargo.toml today.
        RepoModules::default()
    }
    fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        rust::RustAdapter::is_data_dominant(self, source, threshold)
    }
    fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        rust::RustAdapter::data_literal_lines(self, source)
    }
    fn callable_definitions(&self, source: &str) -> HashSet<String> {
        rust::RustAdapter::callable_definitions(self, source)
    }
    fn callable_bodies(&self, source: &str) -> Vec<CallableBody> {
        rust::RustAdapter::callable_bodies(self, source)
    }
    fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        rust::RustAdapter::internal_import_bindings(self, source)
    }
    fn value_bindings(&self, source: &str) -> HashSet<String> {
        rust::RustAdapter::value_bindings(self, source)
    }
    fn is_auto_generated(&self, source: &str, markers: &[String]) -> bool {
        rust::RustAdapter::is_auto_generated(self, source, markers)
    }
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        rust::RustAdapter::enumerate_sampleable_ranges(self, source)
    }
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        rust::RustAdapter::prose_line_ranges(self, source)
    }
    fn identifier_noise(&self) -> &HashSet<String> {
        rust::RustAdapter::identifier_noise(self)
    }
    fn line_comment_prefix(&self) -> &'static str {
        "//"
    }
}

impl LanguageAdapter for cpp::CppAdapter {
    fn language(&self) -> Language {
        Language::Cpp
    }
    fn extract_imports(&self, source: &str) -> HashSet<String> {
        cpp::CppAdapter::extract_imports(self, source)
    }
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        cpp::CppAdapter::extract_imports_with_spans(self, source)
    }
    fn resolve_repo_modules(&self, _repo_root: &Path) -> RepoModules {
        // C++ has no package-manifest analog; repo-internal headers are the
        // quoted `#include "..."` targets, surfaced via internal_import_bindings.
        RepoModules::default()
    }
    fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        cpp::CppAdapter::is_data_dominant(self, source, threshold)
    }
    fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        cpp::CppAdapter::data_literal_lines(self, source)
    }
    fn callable_definitions(&self, source: &str) -> HashSet<String> {
        cpp::CppAdapter::callable_definitions(self, source)
    }
    fn callable_bodies(&self, source: &str) -> Vec<CallableBody> {
        cpp::CppAdapter::callable_bodies(self, source)
    }
    fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        cpp::CppAdapter::internal_import_bindings(self, source)
    }
    fn value_bindings(&self, source: &str) -> HashSet<String> {
        cpp::CppAdapter::value_bindings(self, source)
    }
    fn is_auto_generated(&self, source: &str, markers: &[String]) -> bool {
        cpp::CppAdapter::is_auto_generated(self, source, markers)
    }
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        cpp::CppAdapter::enumerate_sampleable_ranges(self, source)
    }
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        cpp::CppAdapter::prose_line_ranges(self, source)
    }
    fn identifier_noise(&self) -> &HashSet<String> {
        cpp::CppAdapter::identifier_noise(self)
    }
    fn line_comment_prefix(&self) -> &'static str {
        "//"
    }
}

impl LanguageAdapter for ruby::RubyAdapter {
    fn language(&self) -> Language {
        Language::Ruby
    }
    fn extract_imports(&self, source: &str) -> HashSet<String> {
        ruby::RubyAdapter::extract_imports(self, source)
    }
    fn extract_imports_with_spans(&self, source: &str) -> Vec<(String, usize, usize, usize)> {
        ruby::RubyAdapter::extract_imports_with_spans(self, source)
    }
    fn resolve_repo_modules(&self, _repo_root: &Path) -> RepoModules {
        // Like Python: Ruby internal modules are discovered via extract_imports
        // at fit time; there are no exact/prefix rules from a manifest.
        RepoModules::default()
    }
    fn is_data_dominant(&self, source: &str, threshold: f64) -> bool {
        ruby::RubyAdapter::is_data_dominant(self, source, threshold)
    }
    fn data_literal_lines(&self, source: &str) -> HashSet<usize> {
        ruby::RubyAdapter::data_literal_lines(self, source)
    }
    fn callable_definitions(&self, source: &str) -> HashSet<String> {
        ruby::RubyAdapter::callable_definitions(self, source)
    }
    fn callable_bodies(&self, source: &str) -> Vec<CallableBody> {
        ruby::RubyAdapter::callable_bodies(self, source)
    }
    fn internal_import_bindings(&self, source: &str) -> HashSet<String> {
        ruby::RubyAdapter::internal_import_bindings(self, source)
    }
    fn value_bindings(&self, source: &str) -> HashSet<String> {
        ruby::RubyAdapter::value_bindings(self, source)
    }
    fn is_auto_generated(&self, source: &str, markers: &[String]) -> bool {
        ruby::RubyAdapter::is_auto_generated(self, source, markers)
    }
    fn enumerate_sampleable_ranges(&self, source: &str) -> Vec<(usize, usize)> {
        ruby::RubyAdapter::enumerate_sampleable_ranges(self, source)
    }
    fn prose_line_ranges(&self, source: &str) -> HashSet<usize> {
        ruby::RubyAdapter::prose_line_ranges(self, source)
    }
    fn identifier_noise(&self) -> &HashSet<String> {
        ruby::RubyAdapter::identifier_noise(self)
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

/// Recursively list files under `root` whose basename ends with one of
/// `suffixes`, skipping VCS/vendored/build trees. Backs the adapters'
/// repo-ownership resolution (`resolve_repo_modules`). The scan is broader
/// than the training corpus on purpose: a namespace declared anywhere in the
/// repo (tests included) is repo-owned, and importing repo-owned code is
/// never foreign voice — the honest-FP flood on fine-grained-import
/// languages was exactly never-before-imported internal symbols (issue #92).
pub(crate) fn walk_files_with_suffix(root: &Path, suffixes: &[&str]) -> Vec<std::path::PathBuf> {
    const SKIP_DIRS: &[&str] = &[
        ".git",
        "node_modules",
        "vendor",
        "third_party",
        "third-party",
        "target",
        "build",
        "dist",
    ];
    fn rec(dir: &Path, suffixes: &[&str], out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for e in entries.flatten() {
            let Ok(t) = e.file_type() else { continue };
            let name = e.file_name();
            let name = name.to_string_lossy();
            let path = e.path();
            if t.is_dir() {
                if SKIP_DIRS.contains(&name.as_ref()) {
                    continue;
                }
                rec(&path, suffixes, out);
            } else if t.is_file() && suffixes.iter().any(|s| name.ends_with(s)) {
                out.push(path);
            }
        }
    }
    let mut out = Vec::new();
    rec(root, suffixes, &mut out);
    out.sort();
    out
}
