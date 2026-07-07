//! `benchmarks/targets.yaml` — the pinned corpus definitions.

use anyhow::{Context, Result};
use serde::Deserialize;
use std::path::Path;

/// One pinned snapshot: `pr: 0` is the primary HEAD SHA (controls + fixture
/// injection host); the rest are pre-merge PR snapshots.
#[derive(Debug, Clone, Deserialize)]
pub struct PrPin {
    pub pr: u64,
    pub sha: String,
}

/// A pinned corpus. `language` is one of the ten scoring languages, or
/// `multi` for polyglot monorepos.
#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub language: String,
    #[serde(default)]
    pub prs: Vec<PrPin>,
    /// Corpus-specific holdout window (first-parent commits behind the pin
    /// for the fit point). Overrides the CLI default — docs-heavy or
    /// slow-moving repos need wider windows to reach the sample bar.
    #[serde(default)]
    pub holdout_window: Option<usize>,
}

const TARGET_LANGUAGES: &[&str] = &[
    "python",
    "typescript",
    "javascript",
    "go",
    "rust",
    "c",
    "java",
    "csharp",
    "php",
    "cpp",
    "ruby",
    "multi",
];

#[derive(Debug, Deserialize)]
struct TargetsFile {
    targets: Vec<Target>,
}

pub fn load_targets(path: &Path) -> Result<Vec<Target>> {
    let raw = std::fs::read_to_string(path)
        .with_context(|| format!("cannot read targets file {}", path.display()))?;
    let parsed: TargetsFile = serde_yaml::from_str(&raw)
        .with_context(|| format!("invalid YAML in {}", path.display()))?;
    for t in &parsed.targets {
        if !TARGET_LANGUAGES.contains(&t.language.as_str()) {
            anyhow::bail!("target {}: unsupported language {:?}", t.name, t.language);
        }
        if t.prs.is_empty() {
            anyhow::bail!("target {}: no pinned PRs", t.name);
        }
    }
    Ok(parsed.targets)
}
