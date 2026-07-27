//! Path-level suppressions: the built-in `argot:recommended` set plus the
//! `argot.toml` `[exclude].paths` patterns (gitignore-style).
//!
//! Lock-step principle: calibration sampling, the check-time scope filter, and
//! `argot inspect`'s corpus walk all consult one resolved [`PathSuppressions`]
//! so the three surfaces always agree on what is in scope.
//!
//! Both halves of `[exclude]` are gitignore-style pattern lists; they differ
//! only in how a match is treated (see [`PathScope`]):
//! - `[exclude].recommended` (default [`DEFAULT_RECOMMENDED_PATTERNS`]) →
//!   silently dropped, as if the file weren't there;
//! - `[exclude].paths` → still scored, but its hits are dropped from output and
//!   counted, so the exclusion stays auditable.
//!
//! Editing the `recommended` list is the fine-grained way to change the
//! built-ins (remove `test*/` to learn from tests, add a repo-wide directory);
//! an empty list turns the recommended set off entirely.

use crate::suppress::glob::{fnmatch, segments_match};
use std::path::Path;
use std::sync::OnceLock;

/// The built-in `argot:recommended` exclusions, as gitignore-style patterns.
/// `init` writes these into `[exclude].recommended` so they are visible and
/// editable; the code carries no other default. The set covers the directories
/// and files that are almost never a repo's authored voice.
pub const DEFAULT_RECOMMENDED_PATTERNS: &[&str] = &[
    // Directories (matched at any depth).
    "test*/", // test/, tests/, testdata/, testing/, … (any dir starting "test")
    "__tests__/",
    "doc/",
    "docs/",
    "example/",
    "examples/",
    "migration/",
    "migrations/",
    "benchmark/",
    "benchmarks/",
    "fixtures/",
    "scripts/",
    "build/",
    "dist/",
    "__pycache__/",
    ".git/",
    ".history/",
    ".tox/",
    ".eggs/",
    // Files.
    "test_*", // test_foo.py
    "conftest.py",
    "*.test.*",   // x.test.ts
    "*.spec.*",   // x.spec.js
    "*.config.*", // vite.config.ts
    ".*rc.*",     // .babelrc.js
];

/// The built-in `[exclude].check-only` set: paths that are checked like any
/// other, but never shape the voice. `init` writes these into the config so
/// they are visible and editable; the code carries no other default.
///
/// The default is the repo's tests, and it exists so the corpus walk carries no
/// hardcoded notion of what a test looks like. It deliberately does *not*
/// cover build or vendor trees — those are not authored code at all and are
/// pruned structurally (see `corpus::EXCLUDE_DIRS`), not by a scope decision.
///
/// By default these same paths are also in [`DEFAULT_RECOMMENDED_PATTERNS`], so
/// they are dropped from check entirely and this list only governs the corpus.
/// A repo that wants its tests guarded removes them from `recommended` and
/// leaves them here: they are then checked, argot learns their *dependency
/// vocabulary* (so a library only the tests use stops reading as foreign) but
/// never their *style*, and the voice reports only `foreign-import` on them.
pub const DEFAULT_CHECK_ONLY_PATTERNS: &[&str] = &[
    // Directories (matched at any depth).
    "test/",
    "tests/",
    "__tests__/",
    "benchmarks/",
    // Files.
    "test_*",   // test_foo.py
    "*.test.*", // x.test.ts
    "*.spec.*", // x.spec.js
];

/// The built-in recommended set as owned strings — the resolved default when
/// `[exclude].recommended` is absent.
pub fn default_recommended_patterns() -> Vec<String> {
    DEFAULT_RECOMMENDED_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// The built-in check-only set as owned strings — the resolved default when
/// `[exclude].check-only` is absent.
pub fn default_check_only_patterns() -> Vec<String> {
    DEFAULT_CHECK_ONLY_PATTERNS
        .iter()
        .map(|s| s.to_string())
        .collect()
}

/// The default recommended patterns, parsed once.
fn default_recommended_ignore() -> &'static [IgnorePattern] {
    static PARSED: OnceLock<Vec<IgnorePattern>> = OnceLock::new();
    PARSED.get_or_init(|| {
        DEFAULT_RECOMMENDED_PATTERNS
            .iter()
            .filter_map(|l| IgnorePattern::parse(l))
            .collect()
    })
}

/// The default check-only patterns, parsed once.
fn default_check_only_ignore() -> &'static [IgnorePattern] {
    static PARSED: OnceLock<Vec<IgnorePattern>> = OnceLock::new();
    PARSED.get_or_init(|| {
        DEFAULT_CHECK_ONLY_PATTERNS
            .iter()
            .filter_map(|l| IgnorePattern::parse(l))
            .collect()
    })
}

/// True if `rel_path` (repo-relative, `/`-separated) matches the built-in
/// `argot:recommended` set — the *default* patterns only, ignoring any repo
/// `[exclude].recommended` override. The benchmark harness applies this
/// default-recommended scope to real-PR control hunks.
pub fn recommended_excluded(rel_path: &str) -> bool {
    let parts: Vec<&str> = rel_path.split('/').collect();
    default_recommended_ignore()
        .iter()
        .any(|p| p.matches(&parts))
}

/// Repo-relative `/`-separated form of `path` under `root`. `None` when the
/// path is outside `root` or resolves to nothing (both are "not in scope" for
/// every caller).
pub fn rel_string(path: &Path, root: &Path) -> Option<String> {
    let rel = path.strip_prefix(root).ok()?;
    let comps: Vec<String> = rel
        .components()
        .map(|c| c.as_os_str().to_string_lossy().into_owned())
        .collect();
    if comps.is_empty() {
        return None;
    }
    Some(comps.join("/"))
}

/// One parsed `[exclude].paths` pattern (gitignore-style subset: `*`/`?`/`[...]`
/// within a segment, `**` across segments, leading `/` anchors, trailing `/`
/// is directory-only, leading `!` re-includes).
#[derive(Debug, Clone)]
pub struct IgnorePattern {
    /// Original line, for `list-mutes` display.
    pub raw: String,
    negated: bool,
    dir_only: bool,
    anchored: bool,
    segments: Vec<String>,
}

impl IgnorePattern {
    fn parse(line: &str) -> Option<Self> {
        let raw = line.to_string();
        let mut pat = line;
        let negated = pat.starts_with('!');
        if negated {
            pat = &pat[1..];
        }
        let dir_only = pat.ends_with('/');
        if dir_only {
            pat = &pat[..pat.len() - 1];
        }
        // A leading slash anchors; any interior slash also anchors (gitignore).
        let anchored = pat.starts_with('/') || pat.contains('/');
        let pat = pat.strip_prefix('/').unwrap_or(pat);
        if pat.is_empty() {
            return None;
        }
        Some(IgnorePattern {
            raw,
            negated,
            dir_only,
            anchored,
            segments: pat.split('/').map(String::from).collect(),
        })
    }

    /// Does this pattern match the repo-relative file path?
    ///
    /// A pattern matching a *directory* matches every file under it. Directory
    /// -only patterns (`p/`) never match on the final component.
    fn matches(&self, parts: &[&str]) -> bool {
        if parts.is_empty() {
            return false;
        }
        if !self.anchored && self.segments.len() == 1 {
            // Bare name pattern: matches any component at any depth
            // (dir-only: any non-final component).
            let last = if self.dir_only {
                parts.len().saturating_sub(1)
            } else {
                parts.len()
            };
            return parts[..last]
                .iter()
                .any(|part| fnmatch(part, &self.segments[0]));
        }
        // Anchored: match the full path, or any directory prefix of it
        // (a matched directory ignores its contents).
        let max_prefix = if self.dir_only {
            parts.len() - 1
        } else {
            parts.len()
        };
        (1..=max_prefix).any(|k| segments_match(&self.segments, &parts[..k]))
    }

    /// [`Self::matches`], but a bare name pattern with no trailing `/` names a
    /// **file**, not any component at any depth.
    ///
    /// `[exclude].check-only` classifies each path as "shapes the voice" or
    /// "only ever judged by it", and there the two readings genuinely differ:
    /// `test_*` should catch `test_helpers.py` without swallowing rocksdb's
    /// `test_util/` — a directory of production support code the voice has
    /// always learned from. Directory patterns (`tests/`) and path-shaped ones
    /// (`**/__tests__/**`) are unaffected, so the usual way of naming a tree
    /// still reads the same.
    fn matches_file_scoped(&self, parts: &[&str]) -> bool {
        if !self.anchored && self.segments.len() == 1 && !self.dir_only {
            return parts
                .last()
                .is_some_and(|name| fnmatch(name, &self.segments[0]));
        }
        self.matches(parts)
    }
}

/// How a path resolved against the suppression set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathScope {
    /// Not suppressed — in scope for calibration and check.
    InScope,
    /// Excluded by the built-in `argot:recommended` set (silent, as always).
    Recommended,
    /// Suppressed by a user `[exclude].paths` pattern.
    UserIgnored,
}

/// The resolved path-level suppression set: the `argot.toml` `[exclude]`
/// `recommended` patterns (dropped silently), the `paths` patterns (scored but
/// reported), and the `check-only` patterns (scored, but never learned from —
/// orthogonal to the other two, so it is a predicate rather than a
/// [`PathScope`] variant).
#[derive(Debug, Clone)]
pub struct PathSuppressions {
    recommended: Vec<IgnorePattern>,
    patterns: Vec<IgnorePattern>,
    check_only: Vec<IgnorePattern>,
    /// True when an `argot.toml` backed these values (for display surfaces).
    pub from_file: bool,
}

fn parse_patterns(lines: &[String]) -> Vec<IgnorePattern> {
    lines
        .iter()
        .filter_map(|line| IgnorePattern::parse(line.trim()))
        .collect()
}

impl PathSuppressions {
    /// The built-in recommended and check-only sets only — the behaviour of a
    /// repo with no `argot.toml`.
    pub fn recommended() -> Self {
        PathSuppressions {
            recommended: default_recommended_ignore().to_vec(),
            patterns: Vec::new(),
            check_only: default_check_only_ignore().to_vec(),
            from_file: false,
        }
    }

    /// Build from resolved `argot.toml` `[exclude]` values: the `recommended`
    /// patterns (dropped silently), the `paths` patterns (scored, reported) and
    /// the `check-only` patterns (scored, never learned from). `from_file`
    /// records whether a config file backed these values.
    pub fn from_parts(
        recommended: &[String],
        patterns: &[String],
        check_only: &[String],
        from_file: bool,
    ) -> Self {
        PathSuppressions {
            recommended: parse_patterns(recommended),
            patterns: parse_patterns(patterns),
            check_only: parse_patterns(check_only),
            from_file,
        }
    }

    /// Is the built-in recommended set active (non-empty)?
    pub fn recommended_active(&self) -> bool {
        !self.recommended.is_empty()
    }

    /// The `[exclude].recommended` patterns (raw lines, for display).
    pub fn recommended_patterns(&self) -> Vec<&str> {
        self.recommended.iter().map(|p| p.raw.as_str()).collect()
    }

    /// The `[exclude].paths` patterns (raw lines, for display).
    pub fn user_patterns(&self) -> Vec<&str> {
        self.patterns.iter().map(|p| p.raw.as_str()).collect()
    }

    /// Classify a repo-relative `/`-separated path. A `recommended` match wins
    /// (silent drop); otherwise `paths` apply gitignore semantics — the last
    /// matching pattern decides, `!` re-includes.
    pub fn classify(&self, rel_path: &str) -> PathScope {
        let parts: Vec<&str> = rel_path.split('/').collect();
        if self.recommended.iter().any(|p| p.matches(&parts)) {
            return PathScope::Recommended;
        }
        let mut ignored = false;
        for p in &self.patterns {
            if p.matches(&parts) {
                ignored = !p.negated;
            }
        }
        if ignored {
            PathScope::UserIgnored
        } else {
            PathScope::InScope
        }
    }

    /// True when a user `[exclude].paths` pattern mutes this path, regardless of
    /// whether the recommended set also covers it (gitignore last-match-wins
    /// over the user patterns only). Corpus collection uses this: a user who
    /// mutes a directory the recommended set happens to cover must still see
    /// it pruned from the voice-model corpus.
    pub fn matches_user_pattern(&self, rel_path: &str) -> bool {
        let parts: Vec<&str> = rel_path.split('/').collect();
        let mut ignored = false;
        for p in &self.patterns {
            if p.matches(&parts) {
                ignored = !p.negated;
            }
        }
        ignored
    }

    /// True when the path is checked but must never shape the voice
    /// (`[exclude].check-only`). Orthogonal to [`Self::classify`]: a path can
    /// be both in scope and check-only — that is the whole point — and a path
    /// the recommended set already drops is simply never scored, so this
    /// predicate is moot for it.
    pub fn is_check_only(&self, rel_path: &str) -> bool {
        let parts: Vec<&str> = rel_path.split('/').collect();
        self.check_only
            .iter()
            .any(|p| p.matches_file_scoped(&parts))
    }

    /// [`Self::is_check_only`] for an absolute path under `root`. Paths outside
    /// `root` never shape the voice, so they read as check-only.
    pub fn is_check_only_abs(&self, path: &Path, root: &Path) -> bool {
        match rel_string(path, root) {
            Some(rel) => self.is_check_only(&rel),
            None => true,
        }
    }

    /// The `[exclude].check-only` patterns (raw lines, for display and for the
    /// fitted model, which records them so check applies the fit's scope).
    pub fn check_only_patterns(&self) -> Vec<&str> {
        self.check_only.iter().map(|p| p.raw.as_str()).collect()
    }

    /// True when the path is suppressed by any surface.
    pub fn is_suppressed(&self, rel_path: &str) -> bool {
        self.classify(rel_path) != PathScope::InScope
    }

    /// [`Self::is_suppressed`] for an absolute path under `root`. Paths outside
    /// `root` are out of scope (suppressed), matching the historical
    /// `is_excluded_path(path, source_dir)` behaviour.
    pub fn is_suppressed_abs(&self, path: &Path, root: &Path) -> bool {
        match rel_string(path, root) {
            Some(rel) => self.is_suppressed(&rel),
            None => true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    /// Verbatim copy of the pre-refactor `check.rs` implementation — the
    /// contract [`recommended_excluded`] must agree with on every path.
    fn legacy_is_excluded(file_path: &str) -> bool {
        const DIRS: &[&str] = &[
            "test",
            "tests",
            "doc",
            "docs",
            "examples",
            "example",
            "migrations",
            "migration",
            "benchmarks",
            "benchmark",
            "fixtures",
            "scripts",
            "build",
            "dist",
            "__pycache__",
            ".git",
            ".history",
            ".tox",
            ".eggs",
        ];
        let parts: Vec<&str> = file_path.split('/').collect();
        if parts.len() >= 2 {
            for part in &parts[..parts.len() - 1] {
                if DIRS.contains(part) || part.starts_with("test") || *part == "__tests__" {
                    return true;
                }
            }
        }
        let name = *parts.last().unwrap_or(&file_path);
        if name.starts_with("test_") || name == "conftest.py" {
            return true;
        }
        if name.contains(".test.") || name.contains(".spec.") {
            return true;
        }
        if name.contains(".config.") {
            return true;
        }
        name.starts_with('.') && name.get(1..).map(|r| r.contains("rc.")).unwrap_or(false)
    }

    const MATRIX: &[&str] = &[
        "src/app.py",
        "app.py",
        "src/main.ts",
        "tests/app.py",
        "a/tests/b.py",
        "testdata/x.py",
        "src/testing/y.py",
        "a/__tests__/x.ts",
        "src/test_x.py",
        "test_x.py",
        "conftest.py",
        "pkg/conftest.py",
        "x.test.ts",
        "src/x.spec.ts",
        "vite.config.ts",
        ".babelrc.js",
        ".rc",
        "docs/x.py",
        "doc/y.py",
        "examples/y.ts",
        "example/y.ts",
        "migrations/0001.py",
        "migration/0001.py",
        "benchmarks/b.py",
        "benchmark/b.py",
        "fixtures/f.py",
        "scripts/tool.py",
        "build/gen.py",
        "dist/out.js",
        "__pycache__/x.py",
        ".git/hooks/x.py",
        ".history/x.py",
        ".tox/x.py",
        ".eggs/x.py",
        "src/latest/x.py",
        "attests/x.py",
        "protest/x.py",
        "src/deep/nested/mod.py",
    ];

    #[test]
    fn recommended_matches_legacy_on_matrix() {
        for path in MATRIX {
            assert_eq!(
                recommended_excluded(path),
                legacy_is_excluded(path),
                "divergence on {path}"
            );
        }
    }

    fn from_lines(recommended: bool, lines: &[&str]) -> PathSuppressions {
        let rec = if recommended {
            default_recommended_patterns()
        } else {
            Vec::new()
        };
        let patterns: Vec<String> = lines.iter().map(|s| s.to_string()).collect();
        PathSuppressions::from_parts(&rec, &patterns, &default_check_only_patterns(), true)
    }

    #[test]
    fn recommended_list_is_editable_per_entry() {
        // Drop just `test*/` from the recommended set → tests/ is learned again,
        // but docs/ is still dropped. This is the win over the old on/off toggle.
        let rec: Vec<String> = default_recommended_patterns()
            .into_iter()
            .filter(|p| p != "test*/")
            .collect();
        let s = PathSuppressions::from_parts(&rec, &[], &default_check_only_patterns(), true);
        assert!(!s.is_suppressed("tests/app.py"), "tests/ now in scope");
        assert!(s.is_suppressed("docs/x.py"), "docs/ still dropped");
        // Adding a repo-wide dir to the recommended set drops it silently.
        let mut rec2 = default_recommended_patterns();
        rec2.push("vendor/".to_string());
        let s2 = PathSuppressions::from_parts(&rec2, &[], &default_check_only_patterns(), true);
        assert_eq!(s2.classify("vendor/lib.rs"), PathScope::Recommended);
    }

    #[test]
    fn no_config_resolves_to_recommended_exactly() {
        let loaded = PathSuppressions::recommended();
        assert!(!loaded.from_file);
        for path in MATRIX {
            assert_eq!(
                loaded.is_suppressed(path),
                legacy_is_excluded(path),
                "divergence on {path}"
            );
        }
    }

    #[test]
    fn exclude_paths_add_to_recommended() {
        let s = from_lines(true, &["vendored/", "*.gen.py"]);
        // Recommended set still applies…
        assert!(s.is_suppressed("tests/app.py"));
        // …and the new patterns add to it.
        assert!(s.is_suppressed("vendored/lib.py"));
        assert!(s.is_suppressed("src/models.gen.py"));
        assert!(!s.is_suppressed("src/app.py"));
        assert_eq!(s.classify("vendored/lib.py"), PathScope::UserIgnored);
        assert_eq!(s.classify("tests/app.py"), PathScope::Recommended);
    }

    #[test]
    fn nested_star_directory_pattern_suppresses_deep_paths() {
        // A monorepo muting per-tenant scratch: `tenants/*/one-offs/` must prune
        // the whole subtree, not just its immediate children — otherwise that
        // scratch code leaks into the corpus and the semantic index as a
        // reinvention target.
        let s = from_lines(true, &["tenants/*/one-offs/"]);
        assert!(
            s.is_suppressed("tenants/acme/one-offs/bench/foo.ts"),
            "deep path under a matched dir must be suppressed"
        );
        assert!(s.is_suppressed("tenants/acme/one-offs/foo.ts"));
        assert!(s.matches_user_pattern("tenants/acme/one-offs/bench/foo.ts"));
        assert!(
            !s.is_suppressed("tenants/acme/src/foo.ts"),
            "a sibling dir stays in scope"
        );
    }

    #[test]
    fn recommended_false_drops_builtins() {
        let s = from_lines(false, &["vendored/"]);
        assert!(!s.recommended_active());
        assert!(!s.is_suppressed("tests/app.py"), "built-ins dropped");
        assert!(
            s.is_suppressed("vendored/lib.py"),
            "own patterns still apply"
        );
    }

    #[test]
    fn gitignore_style_matching() {
        let s = from_lines(
            true,
            &[
                "/generated.py",
                "legacy/",
                "**/snapshots/**",
                "src/*.tmp.ts",
                "*.min.js",
                "!keep.min.js",
            ],
        );
        // Leading slash anchors to the root.
        assert!(s.is_suppressed("generated.py"));
        assert!(!s.is_suppressed("pkg/generated.py"));
        // Directory-only pattern: contents at any depth, not a same-named file.
        assert!(s.is_suppressed("legacy/mod.py"));
        assert!(s.is_suppressed("a/legacy/mod.py"));
        assert!(!s.is_suppressed("legacy"));
        // `**` spans directories.
        assert!(s.is_suppressed("a/b/snapshots/c/d.py"));
        // Anchored `*` stays within one segment.
        assert!(s.is_suppressed("src/x.tmp.ts"));
        assert!(!s.is_suppressed("src/deep/x.tmp.ts"));
        // Bare name patterns match at any depth; `!` re-includes.
        assert!(s.is_suppressed("dist2/app.min.js"));
        assert!(!s.is_suppressed("dist2/keep.min.js"));
    }

    #[test]
    fn is_suppressed_abs_handles_out_of_root_paths() {
        let s = PathSuppressions::recommended();
        let root = PathBuf::from("/repo");
        assert!(s.is_suppressed_abs(&PathBuf::from("/elsewhere/x.py"), &root));
        assert!(
            s.is_suppressed_abs(&PathBuf::from("/repo"), &root),
            "empty rel"
        );
        assert!(!s.is_suppressed_abs(&PathBuf::from("/repo/src/x.py"), &root));
        assert!(s.is_suppressed_abs(&PathBuf::from("/repo/tests/x.py"), &root));
    }
}
