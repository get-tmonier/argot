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

/// A pinned corpus. `language` is `python`, `typescript`, or `multi`.
#[derive(Debug, Clone, Deserialize)]
pub struct Target {
    pub name: String,
    pub url: String,
    pub language: String,
    #[serde(default)]
    pub prs: Vec<PrPin>,
}

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
        if !matches!(t.language.as_str(), "python" | "typescript" | "multi") {
            anyhow::bail!("target {}: unsupported language {:?}", t.name, t.language);
        }
        if t.prs.is_empty() {
            anyhow::bail!("target {}: no pinned PRs", t.name);
        }
    }
    Ok(parsed.targets)
}
