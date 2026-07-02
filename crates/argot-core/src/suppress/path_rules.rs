//! Path-level suppressions: the built-in `argot:recommended` set plus
//! `.argotignore` (gitignore-style patterns at the repo root).
//!
//! Lock-step principle: calibration sampling, the check-time scope filter, and
//! `argot inspect`'s corpus walk all consult one resolved [`PathSuppressions`]
//! so the three surfaces always agree on what is in scope.
//!
//! Resolution:
//! - no `.argotignore` → the recommended set applies exactly as it always has
//!   (byte-identical behaviour; the parity suites depend on it);
//! - `.argotignore` present → its patterns ADD to the recommended set;
//! - a line consisting of `!argot:recommended` drops the built-ins for users
//!   who want full control.

use crate::suppress::glob::{fnmatch, segments_match};
use std::path::Path;

/// The magic `.argotignore` line that disables the built-in recommended set.
pub const RECOMMENDED_TOKEN: &str = "argot:recommended";

/// Built-in `argot:recommended` directory exclusions (formerly the hardcoded
/// calibration list — test/docs/examples/migrations/etc.).
pub const RECOMMENDED_EXCLUDE_DIRS: &[&str] = &[
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

/// The built-in `argot:recommended` path exclusion. `rel_path` is
/// repo-relative and `/`-separated. Exact port of the original
/// `is_excluded_path` semantics (directory names, `test*` prefixes,
/// `test_*`/`conftest.py` filenames, `.test.`/`.spec.`/`.config.` infixes,
/// dotfile `rc.` configs).
pub fn recommended_excluded(rel_path: &str) -> bool {
    let parts: Vec<&str> = rel_path.split('/').collect();
    if parts.len() >= 2 {
        for part in &parts[..parts.len() - 1] {
            if RECOMMENDED_EXCLUDE_DIRS.contains(part)
                || part.starts_with("test")
                || *part == "__tests__"
            {
                return true;
            }
        }
    }
    let name = *parts.last().unwrap_or(&rel_path);
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

/// One parsed `.argotignore` pattern (gitignore-style subset: `*`/`?`/`[...]`
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
}

/// How a path resolved against the suppression set.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PathScope {
    /// Not suppressed — in scope for calibration and check.
    InScope,
    /// Excluded by the built-in `argot:recommended` set (silent, as always).
    Recommended,
    /// Suppressed by a user `.argotignore` pattern.
    UserIgnored,
}

/// The resolved path-level suppression set: recommended built-ins plus
/// `.argotignore` patterns.
#[derive(Debug, Clone)]
pub struct PathSuppressions {
    recommended_active: bool,
    patterns: Vec<IgnorePattern>,
    /// True when an `.argotignore` file was found by [`PathSuppressions::load`].
    pub from_file: bool,
}

impl PathSuppressions {
    /// The built-in set only — the behaviour of a repo with no `.argotignore`.
    pub fn recommended() -> Self {
        PathSuppressions {
            recommended_active: true,
            patterns: Vec::new(),
            from_file: false,
        }
    }

    /// Load `<repo_root>/.argotignore` when present; otherwise the recommended
    /// set applies exactly as before.
    pub fn load(repo_root: &Path) -> Self {
        match std::fs::read_to_string(repo_root.join(".argotignore")) {
            Ok(content) => {
                let mut s = Self::parse(&content);
                s.from_file = true;
                s
            }
            Err(_) => Self::recommended(),
        }
    }

    /// Parse `.argotignore` content. Blank lines and `#` comments are skipped;
    /// `!argot:recommended` drops the built-in set.
    pub fn parse(content: &str) -> Self {
        let mut recommended_active = true;
        let mut patterns = Vec::new();
        for line in content.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                continue;
            }
            if line == format!("!{RECOMMENDED_TOKEN}") {
                recommended_active = false;
                continue;
            }
            if let Some(p) = IgnorePattern::parse(line) {
                patterns.push(p);
            }
        }
        PathSuppressions {
            recommended_active,
            patterns,
            from_file: false,
        }
    }

    /// Is the built-in recommended set active?
    pub fn recommended_active(&self) -> bool {
        self.recommended_active
    }

    /// The user-supplied `.argotignore` patterns (raw lines, for display).
    pub fn user_patterns(&self) -> Vec<&str> {
        self.patterns.iter().map(|p| p.raw.as_str()).collect()
    }

    /// Classify a repo-relative `/`-separated path. Recommended-set exclusions
    /// win (they were always silent scope, and stay so); user patterns apply
    /// gitignore semantics — the last matching pattern decides, `!` re-includes.
    pub fn classify(&self, rel_path: &str) -> PathScope {
        if self.recommended_active && recommended_excluded(rel_path) {
            return PathScope::Recommended;
        }
        let parts: Vec<&str> = rel_path.split('/').collect();
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

    #[test]
    fn no_argotignore_resolves_to_recommended_exactly() {
        let dir =
            std::env::temp_dir().join(format!("argot_pathrules_nofile_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let loaded = PathSuppressions::load(&dir);
        assert!(!loaded.from_file);
        for path in MATRIX {
            assert_eq!(
                loaded.is_suppressed(path),
                legacy_is_excluded(path),
                "divergence on {path}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn argotignore_patterns_add_to_recommended() {
        let s = PathSuppressions::parse("vendored/\n*.gen.py\n");
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
    fn bang_recommended_drops_builtins() {
        let s = PathSuppressions::parse("!argot:recommended\nvendored/\n");
        assert!(!s.recommended_active());
        assert!(!s.is_suppressed("tests/app.py"), "built-ins dropped");
        assert!(
            s.is_suppressed("vendored/lib.py"),
            "own patterns still apply"
        );
    }

    #[test]
    fn gitignore_style_matching() {
        let s = PathSuppressions::parse(
            "# comment\n\
             \n\
             /generated.py\n\
             legacy/\n\
             **/snapshots/**\n\
             src/*.tmp.ts\n\
             *.min.js\n\
             !keep.min.js\n",
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
