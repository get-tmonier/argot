//! The rule manifest — `.argot/rules/<name>/rule.toml`.
//!
//! The manifest is the declarative frame of a scripted rule: identity,
//! default severity, language scope, and which host-API generation the
//! script targets. The detection logic lives next to it in a Rhai script
//! (`check.rhai` by default). One rule per directory; the directory name and
//! the manifest `name` must agree, so a rule is addressable by its path.

use argot_engine::rules::{CustomRule, Severity};
use serde::Deserialize;
use std::path::{Path, PathBuf};

/// The newest host-API generation this build understands. A manifest asking
/// for a newer one is skipped with a clear message — an old binary must never
/// half-run a rule written against calls it doesn't have.
pub const HOST_API_VERSION: u32 = 1;

/// The manifest schema generation (the TOML shape itself).
pub const MANIFEST_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawManifest {
    rule: RawRule,
    #[serde(default)]
    engine: RawEngine,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawRule {
    schema: u32,
    name: String,
    /// Short human label shown next to a finding. Defaults to the name.
    label: Option<String>,
    #[serde(default)]
    description: String,
    /// Default severity (`error` / `warn` / `off`), overridable via
    /// `[rules]` / `--rule` like any built-in. Defaults to `warn`: a rule
    /// just dropped into a repo should report before it gates.
    severity: Option<String>,
    /// Scoring language names this rule runs on (empty = every supported
    /// language).
    #[serde(default)]
    languages: Vec<String>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawEngine {
    /// Host-API generation the script targets (default 1).
    api: Option<u32>,
    /// Script file, relative to the rule directory (default `check.rhai`).
    script: Option<String>,
}

/// One loaded, validated scripted rule (manifest + script source).
#[derive(Debug, Clone)]
pub struct ScriptRule {
    pub name: String,
    pub label: String,
    pub description: String,
    pub default_severity: Severity,
    /// Scoring language names this rule runs on (empty = all).
    pub languages: Vec<String>,
    /// The Rhai source, read at discovery.
    pub script: String,
    /// The script path, for diagnostics.
    pub script_path: PathBuf,
}

impl ScriptRule {
    /// The registry entry for this rule (group `custom`, reason
    /// `custom:<name>` — derived by the registry).
    pub fn custom_rule(&self) -> CustomRule {
        CustomRule {
            name: self.name.clone(),
            label: self.label.clone(),
            description: self.description.clone(),
            default_severity: self.default_severity,
        }
    }

    /// Does this rule run on `language`? (Empty scope = every language.)
    pub fn covers_language(&self, language: &str) -> bool {
        self.languages.is_empty() || self.languages.iter().any(|l| l == language)
    }
}

/// Parse and validate one rule directory. `Err` is a human-readable reason
/// the rule was skipped — discovery degrades per rule, never for the run.
pub fn load_rule_dir(dir: &Path) -> Result<ScriptRule, String> {
    let dir_name = dir
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_default();
    let manifest_path = dir.join("rule.toml");
    let raw_toml = std::fs::read_to_string(&manifest_path)
        .map_err(|e| format!("{dir_name}: reading rule.toml failed: {e}"))?;
    let raw: RawManifest =
        toml::from_str(&raw_toml).map_err(|e| format!("{dir_name}: rule.toml: {e}"))?;

    if raw.rule.schema != MANIFEST_SCHEMA_VERSION {
        return Err(format!(
            "{dir_name}: rule.toml schema {} — this argot understands schema {}",
            raw.rule.schema, MANIFEST_SCHEMA_VERSION
        ));
    }
    let api = raw.engine.api.unwrap_or(1);
    if api > HOST_API_VERSION {
        return Err(format!(
            "{dir_name}: targets host API {api} — this argot provides API {HOST_API_VERSION}; \
             update argot to run this rule"
        ));
    }
    if raw.rule.name != dir_name {
        return Err(format!(
            "{dir_name}: manifest name '{}' does not match the rule directory name",
            raw.rule.name
        ));
    }
    let default_severity = match &raw.rule.severity {
        None => Severity::Warn,
        Some(s) => Severity::parse(s).ok_or_else(|| {
            format!("{dir_name}: invalid severity '{s}' — expected error, warn, or off")
        })?,
    };
    let script_rel = raw.engine.script.as_deref().unwrap_or("check.rhai");
    let script_path = dir.join(script_rel);
    let script = std::fs::read_to_string(&script_path)
        .map_err(|e| format!("{dir_name}: reading {script_rel} failed: {e}"))?;

    Ok(ScriptRule {
        label: raw.rule.label.unwrap_or_else(|| raw.rule.name.clone()),
        name: raw.rule.name,
        description: raw.rule.description,
        default_severity,
        languages: raw.rule.languages,
        script,
        script_path,
    })
}

#[cfg(test)]
mod tests;
