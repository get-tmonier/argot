//! Machine-readable renderers for `check` results (`--format json|sarif`).
//!
//! `check` flattens its internal hits into [`HitRecord`]s (plain data, no
//! scorer internals) and these functions turn them into complete stdout
//! documents. In a machine format the rendered document is the *only* thing
//! written to stdout — progress and warnings stay on stderr.
//!
//! Formats:
//! - `json` — argot's own stable schema (tool block, scan metadata, per-hit
//!   entries). Intended for scripting; field names are part of the contract.
//! - `sarif` — SARIF 2.1.0 for code-scanning integrations (GitHub
//!   `upload-sarif` etc.). Severity tiers map to SARIF levels:
//!   `unusual` → `note`, `suspicious` → `warning`, `foreign` → `error`.

use serde::Serialize;
use serde_json::{json, Value};

/// Output format for `check` (`--format`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum OutputFormat {
    /// The default terminal rendering (banner, grouped hits, hunk bodies).
    #[default]
    Human,
    /// argot's stable machine-readable JSON document.
    Json,
    /// SARIF 2.1.0 for code-scanning uploads.
    Sarif,
}

impl OutputFormat {
    /// Parse a CLI `--format` value. Returns `None` for unknown names.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "human" => Some(Self::Human),
            "json" => Some(Self::Json),
            "sarif" => Some(Self::Sarif),
            _ => None,
        }
    }

    /// True for the formats whose document owns stdout exclusively.
    pub fn is_machine(self) -> bool {
        !matches!(self, Self::Human)
    }
}

/// One above-threshold hit, flattened for serialization.
#[derive(Debug, Clone, Serialize)]
pub struct HitRecord {
    /// Repo-relative, `/`-separated file path.
    pub path: String,
    /// 1-based first line of the hunk.
    pub line_start: usize,
    /// 1-based last line of the hunk (>= `line_start`).
    pub line_end: usize,
    /// BPE-stage score for the hunk.
    pub score: f64,
    /// Calibrated threshold the severity tier is measured against.
    pub threshold: f64,
    /// Severity tier name (weakest to strongest: unusual, suspicious, foreign).
    pub severity: String,
    /// Scorer reason code (e.g. `bpe`, `import`, `call_receiver`).
    pub reason: String,
    /// User-facing translation of `reason` (e.g. "rare token sequence").
    pub reason_label: String,
    /// Where the hunk came from: `workdir`/`staged`/`untracked` or a short SHA.
    pub source: String,
    /// Content-based hit hash — paste into `argot mute <hash>` to suppress.
    pub hash: String,
    /// Rendered per-reason evidence lines (empty when the scorer had none).
    pub evidence: Vec<String>,
}

/// Run-level metadata shared by both machine formats.
pub struct ReportMeta {
    /// Tool version (the workspace-shared crate version).
    pub tool_version: String,
    /// Repository path the check ran against, as given on the CLI.
    pub repo: String,
    /// Human label of what was scanned (e.g. "workdir", "3 commit(s) (a..b)").
    pub scanned: String,
    /// Total hunks scored (including below-threshold ones).
    pub hunks_scanned: usize,
}

/// Map a severity tier to its SARIF result level.
///
/// Unknown tiers fall back to `warning` so a future tier never silently
/// disappears from code-scanning results.
fn sarif_level(severity: &str) -> &'static str {
    match severity {
        "unusual" => "note",
        "suspicious" => "warning",
        "foreign" => "error",
        _ => "warning",
    }
}

fn to_pretty(doc: &Value) -> String {
    let mut s = serde_json::to_string_pretty(doc).expect("serializing JSON value cannot fail");
    s.push('\n');
    s
}

/// Render the argot JSON document (`--format json`).
pub fn render_json(meta: &ReportMeta, hits: &[HitRecord]) -> String {
    let doc = json!({
        "tool": { "name": "argot", "version": meta.tool_version },
        "repo": meta.repo,
        "scanned": meta.scanned,
        "hunks_scanned": meta.hunks_scanned,
        "hits": hits,
    });
    to_pretty(&doc)
}

/// Render a SARIF 2.1.0 document (`--format sarif`).
///
/// One rule per distinct scorer reason code (in first-appearance order); each
/// result carries the physical location, the mapped level, and the raw
/// score/threshold/severity/evidence in `properties`.
pub fn render_sarif(meta: &ReportMeta, hits: &[HitRecord]) -> String {
    // Rules: distinct reason codes, first-appearance order.
    let mut rule_ids: Vec<(&str, &str)> = Vec::new();
    for h in hits {
        if !rule_ids.iter().any(|(id, _)| *id == h.reason) {
            rule_ids.push((&h.reason, &h.reason_label));
        }
    }
    let rules: Vec<Value> = rule_ids
        .iter()
        .map(|(id, label)| {
            json!({
                "id": id,
                "shortDescription": { "text": label },
                "fullDescription": {
                    "text": format!(
                        "argot flagged this hunk as out of the repo's voice: {label}."
                    )
                },
                "helpUri": "https://github.com/get-tmonier/argot",
            })
        })
        .collect();

    let results: Vec<Value> = hits
        .iter()
        .map(|h| {
            let rule_index = rule_ids
                .iter()
                .position(|(id, _)| *id == h.reason)
                .expect("rule registered above");
            let mut text = format!(
                "{} — score {:.2} vs threshold {:.2} ({})",
                h.reason_label, h.score, h.threshold, h.severity
            );
            for line in &h.evidence {
                text.push('\n');
                text.push_str(line);
            }
            json!({
                "ruleId": h.reason,
                "ruleIndex": rule_index,
                "level": sarif_level(&h.severity),
                "message": { "text": text },
                "locations": [{
                    "physicalLocation": {
                        "artifactLocation": { "uri": h.path },
                        "region": { "startLine": h.line_start, "endLine": h.line_end },
                    }
                }],
                "properties": {
                    "score": h.score,
                    "threshold": h.threshold,
                    "severity": h.severity,
                    "source": h.source,
                    "hash": h.hash,
                    "evidence": h.evidence,
                },
            })
        })
        .collect();

    let doc = json!({
        "$schema": "https://raw.githubusercontent.com/oasis-tcs/sarif-spec/master/Schemata/sarif-schema-2.1.0.json",
        "version": "2.1.0",
        "runs": [{
            "tool": {
                "driver": {
                    "name": "argot",
                    "version": meta.tool_version,
                    "informationUri": "https://github.com/get-tmonier/argot",
                    "rules": rules,
                }
            },
            "results": results,
            "properties": {
                "repo": meta.repo,
                "scanned": meta.scanned,
                "hunksScanned": meta.hunks_scanned,
            },
        }],
    });
    to_pretty(&doc)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn meta() -> ReportMeta {
        ReportMeta {
            tool_version: "0.0.0-test".to_string(),
            repo: "/tmp/repo".to_string(),
            scanned: "workdir".to_string(),
            hunks_scanned: 7,
        }
    }

    fn hit(severity: &str, reason: &str) -> HitRecord {
        HitRecord {
            path: "src/app.py".to_string(),
            line_start: 10,
            line_end: 16,
            score: 8.25,
            threshold: 6.75,
            severity: severity.to_string(),
            reason: reason.to_string(),
            reason_label: match reason {
                "bpe" => "rare token sequence",
                "import" => "foreign import",
                _ => reason,
            }
            .to_string(),
            source: "workdir".to_string(),
            hash: "a1b2c3d4e5f6".to_string(),
            evidence: vec!["↳ axios — 0 of 47 module specifiers in repo".to_string()],
        }
    }

    #[test]
    fn output_format_parse_roundtrip() {
        assert_eq!(OutputFormat::parse("human"), Some(OutputFormat::Human));
        assert_eq!(OutputFormat::parse("json"), Some(OutputFormat::Json));
        assert_eq!(OutputFormat::parse("sarif"), Some(OutputFormat::Sarif));
        assert_eq!(OutputFormat::parse("yaml"), None);
        assert!(!OutputFormat::Human.is_machine());
        assert!(OutputFormat::Json.is_machine());
        assert!(OutputFormat::Sarif.is_machine());
    }

    #[test]
    fn json_document_carries_tool_meta_and_hit_fields() {
        let out = render_json(&meta(), &[hit("suspicious", "bpe")]);
        let doc: Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["tool"]["name"], "argot");
        assert_eq!(doc["tool"]["version"], "0.0.0-test");
        assert_eq!(doc["repo"], "/tmp/repo");
        assert_eq!(doc["scanned"], "workdir");
        assert_eq!(doc["hunks_scanned"], 7);
        let h = &doc["hits"][0];
        assert_eq!(h["path"], "src/app.py");
        assert_eq!(h["line_start"], 10);
        assert_eq!(h["line_end"], 16);
        assert_eq!(h["score"], 8.25);
        assert_eq!(h["threshold"], 6.75);
        assert_eq!(h["severity"], "suspicious");
        assert_eq!(h["reason"], "bpe");
        assert_eq!(h["reason_label"], "rare token sequence");
        assert_eq!(h["source"], "workdir");
        assert_eq!(h["hash"], "a1b2c3d4e5f6");
        assert_eq!(
            h["evidence"][0],
            "↳ axios — 0 of 47 module specifiers in repo"
        );
    }

    #[test]
    fn json_document_with_no_hits_is_valid_and_empty() {
        let out = render_json(&meta(), &[]);
        let doc: Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["hits"].as_array().unwrap().len(), 0);
    }

    #[test]
    fn sarif_has_required_top_level_fields() {
        let out = render_sarif(&meta(), &[hit("foreign", "import")]);
        let doc: Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["version"], "2.1.0");
        assert!(doc["$schema"]
            .as_str()
            .unwrap()
            .contains("sarif-schema-2.1.0"));
        let driver = &doc["runs"][0]["tool"]["driver"];
        assert_eq!(driver["name"], "argot");
        assert_eq!(driver["version"], "0.0.0-test");
        assert!(driver["rules"].is_array());
        assert!(doc["runs"][0]["results"].is_array());
    }

    #[test]
    fn sarif_maps_severity_tiers_to_levels() {
        let hits = [
            hit("unusual", "bpe"),
            hit("suspicious", "bpe"),
            hit("foreign", "bpe"),
        ];
        let out = render_sarif(&meta(), &hits);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let results = doc["runs"][0]["results"].as_array().unwrap();
        let levels: Vec<&str> = results
            .iter()
            .map(|r| r["level"].as_str().unwrap())
            .collect();
        assert_eq!(levels, ["note", "warning", "error"]);
    }

    #[test]
    fn sarif_result_carries_rule_location_and_properties() {
        let out = render_sarif(&meta(), &[hit("foreign", "import")]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let run = &doc["runs"][0];
        let r = &run["results"][0];
        assert_eq!(r["ruleId"], "import");
        assert_eq!(r["ruleIndex"], 0);
        let rule = &run["tool"]["driver"]["rules"][0];
        assert_eq!(rule["id"], "import");
        assert_eq!(rule["shortDescription"]["text"], "foreign import");
        let loc = &r["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "src/app.py");
        assert_eq!(loc["region"]["startLine"], 10);
        assert_eq!(loc["region"]["endLine"], 16);
        assert_eq!(r["properties"]["score"], 8.25);
        assert_eq!(r["properties"]["severity"], "foreign");
        assert_eq!(r["properties"]["hash"], "a1b2c3d4e5f6");
        assert!(r["message"]["text"]
            .as_str()
            .unwrap()
            .contains("foreign import — score 8.25 vs threshold 6.75 (foreign)"));
        assert!(r["message"]["text"].as_str().unwrap().contains("axios"));
    }

    #[test]
    fn sarif_deduplicates_rules_by_reason_code() {
        let hits = [
            hit("unusual", "bpe"),
            hit("foreign", "bpe"),
            hit("foreign", "import"),
        ];
        let out = render_sarif(&meta(), &hits);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let rules = doc["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0]["id"], "bpe");
        assert_eq!(rules[1]["id"], "import");
        let results = doc["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results[2]["ruleIndex"], 1);
    }

    #[test]
    fn sarif_with_no_hits_yields_empty_results_and_rules() {
        let out = render_sarif(&meta(), &[]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(doc["runs"][0]["results"].as_array().unwrap().len(), 0);
        assert_eq!(
            doc["runs"][0]["tool"]["driver"]["rules"]
                .as_array()
                .unwrap()
                .len(),
            0
        );
    }
}
