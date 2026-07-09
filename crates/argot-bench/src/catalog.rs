//! Catalog manifests — the hand-crafted break-fixture ground truth under
//! `benchmarks/catalogs/<corpus>/manifest.yaml`.

use anyhow::{bail, Context, Result};
use serde::Deserialize;
use std::path::Path;

/// The five canonical RUBRIC scoring classes. A fixture's `class()` must be one
/// of these; the descriptive `category` (jquery, xhr_network, …) is free-text for
/// reporting only. Gating is decided purely from the canonical class, so no code
/// ever needs to know a corpus's ad-hoc category vocabulary.
pub const CANONICAL_CLASSES: [&str; 5] = [
    "foreign_import",
    "foreign_api",
    "foreign_concurrency",
    "naming_shape_break",
    "semantic_convention",
];

#[derive(Debug, Clone, Deserialize)]
pub struct Fixture {
    pub id: String,
    /// Path relative to the catalog dir (e.g. `breaks/break_http_sink_1.ts`).
    pub file: String,
    /// Free-text descriptive break type (jquery, xhr_network, …) — for reporting.
    pub category: String,
    /// Canonical RUBRIC scoring class. Optional: when the `category` is itself a
    /// canonical class the author may omit it (`class()` falls back to `category`);
    /// a descriptive category REQUIRES an explicit `class:` mapping it to one of
    /// `CANONICAL_CLASSES` (enforced at load). This is where the gated-vs-secondary
    /// decision lives — authored per fixture, never inferred from the name.
    #[serde(default)]
    pub class: Option<String>,
    pub hunk_start_line: usize,
    pub hunk_end_line: usize,
    #[serde(default)]
    #[allow(dead_code)] // schema completeness; surfaced in manifests, not reports
    pub rationale: String,
    #[serde(default)]
    pub difficulty: Option<String>,
    /// Required for `language: multi` catalogs; absent for single-language.
    #[serde(default)]
    pub language: Option<String>,
    /// Host-injection metadata: path relative to the corpus repo root, plus
    /// the 1-indexed line before which the catalog content is spliced.
    /// Both present or both absent.
    #[serde(default)]
    pub host_file: Option<String>,
    #[serde(default)]
    pub host_inject_at_line: Option<usize>,
}

impl Fixture {
    /// Canonical scoring class: the explicit `class:` field, else the `category`
    /// (which must then already be canonical). Enforced canonical at load.
    pub fn class(&self) -> &str {
        self.class.as_deref().unwrap_or(&self.category)
    }
}

#[derive(Debug, Deserialize)]
pub struct Catalog {
    pub corpus: String,
    pub language: String,
    #[allow(dead_code)] // schema completeness; recall is grouped from fixtures directly
    pub categories: Vec<String>,
    #[serde(default)]
    pub fixtures: Vec<Fixture>,
}

pub fn load_catalog(dir: &Path) -> Result<Catalog> {
    let manifest = dir.join("manifest.yaml");
    let raw = std::fs::read_to_string(&manifest)
        .with_context(|| format!("cannot read {}", manifest.display()))?;
    let catalog: Catalog = serde_yaml::from_str(&raw)
        .with_context(|| format!("invalid YAML in {}", manifest.display()))?;
    for fx in &catalog.fixtures {
        if fx.host_file.is_some() != fx.host_inject_at_line.is_some() {
            bail!(
                "fixture {}: host_file and host_inject_at_line must be specified together",
                fx.id
            );
        }
        if let Some(line) = fx.host_inject_at_line {
            if line < 1 {
                bail!("fixture {}: host_inject_at_line must be >= 1", fx.id);
            }
        }
        if let Some(lang) = &fx.language {
            if !matches!(lang.as_str(), "python" | "typescript") {
                bail!("fixture {}: invalid language {:?}", fx.id, lang);
            }
        }
        // The scoring class must be canonical — a descriptive category (jquery,
        // xhr_network, …) requires an explicit `class:` mapping it to one of the
        // five RUBRIC classes. Hard-error rather than silently mis-gate.
        if !CANONICAL_CLASSES.contains(&fx.class()) {
            bail!(
                "fixture {}: scoring class {:?} is not canonical — add a `class:` field \
                 mapping category {:?} to one of {:?}",
                fx.id,
                fx.class(),
                fx.category,
                CANONICAL_CLASSES
            );
        }
    }
    if catalog.language == "multi" {
        let missing: Vec<&str> = catalog
            .fixtures
            .iter()
            .filter(|fx| fx.language.is_none())
            .map(|fx| fx.id.as_str())
            .collect();
        if !missing.is_empty() {
            bail!(
                "multi catalog {}: fixtures missing per-fixture language: {missing:?}",
                catalog.corpus
            );
        }
    }
    Ok(catalog)
}

/// `(full_file_source, hunk_content)` for a fixture, from the raw catalog file.
pub fn read_hunk(catalog_dir: &Path, fx: &Fixture) -> Result<(String, String)> {
    let full = std::fs::read_to_string(catalog_dir.join(&fx.file))
        .with_context(|| format!("cannot read fixture file {}", fx.file))?;
    let lines: Vec<&str> = full.lines().collect();
    let start = fx.hunk_start_line.saturating_sub(1).min(lines.len());
    let end = fx.hunk_end_line.min(lines.len());
    let hunk = lines[start..end].join("\n");
    Ok((full, hunk))
}
