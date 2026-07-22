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
//! - `github` — GitHub Actions workflow commands (`::error file=…`), one line
//!   per hit → inline PR annotations with no extra action or upload step.
//! - `sarif` — SARIF 2.1.0 for code-scanning integrations (GitHub
//!   `upload-sarif` etc.). Confidence tiers map to SARIF levels
//!   (`unusual` → `note`, `suspicious` → `warning`, `foreign` → `error`),
//!   capped at `warning` for `warn`-severity rules.

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
    /// GitHub Actions workflow commands (inline PR annotations).
    Github,
}

impl OutputFormat {
    /// Parse a CLI `--format` value. Returns `None` for unknown names.
    pub fn parse(s: &str) -> Option<Self> {
        match s {
            "human" => Some(Self::Human),
            "json" => Some(Self::Json),
            "sarif" => Some(Self::Sarif),
            "github" => Some(Self::Github),
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
    /// Calibrated threshold the confidence tier is measured against.
    pub threshold: f64,
    /// Confidence tier (weakest to strongest: unusual, suspicious, foreign) —
    /// how strong the evidence is. Display-graded, does not gate.
    pub confidence: String,
    /// The rule's configured severity for this run (`error` or `warn`).
    pub severity: String,
    /// Stable rule name (e.g. `foreign-import`, `redundant`) — the registry
    /// key usable in `argot.toml [rules]`, `--rule`, and suppressions.
    pub rule: String,
    /// Human label for the rule (e.g. "rare token sequence").
    pub rule_label: String,
    /// Where the hunk came from: `workdir`/`staged`/`untracked` or a short SHA.
    pub source: String,
    /// Content-based hit hash — paste into `argot mute <hash>` to suppress.
    pub hash: String,
    /// Rendered per-reason evidence lines (empty when the scorer had none).
    pub evidence: Vec<String>,
    /// The named symbol a finding is about, when the rule knows one — today
    /// the affected test of the integrity rules (`test-deleted` & co.), so
    /// machine consumers can act on the name without parsing evidence text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    /// The module specifiers a `foreign-import` finding flagged (verbatim,
    /// untruncated), so machine consumers can classify them without parsing the
    /// rendered — and truncated — evidence text. Empty for every other rule.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub foreign_specifiers: Vec<String>,
    /// Cosine similarity to the nearest existing function for a semantic
    /// `redundant` finding — the structured form of the "similarity 0.xx"
    /// evidence tail. `None` for every other rule (and every base build).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub similarity: Option<f32>,
}

/// Per-file count of scored hunks (below-threshold ones included), for
/// consumers that need a denominator per file rather than the run total.
#[derive(Debug, Clone, Serialize)]
pub struct FileScan {
    /// Repo-relative, `/`-separated file path.
    pub path: String,
    /// Hunks scored in this file during the run.
    pub hunks: usize,
}

/// Result counts and exit status for a complete `check` run.
#[derive(Debug, Clone, Serialize)]
pub struct CheckResult {
    /// The process status selected from every unsuppressed configured finding.
    pub exit_code: i32,
    /// Findings eligible to affect status after configuration and suppression.
    pub unsuppressed_hits: usize,
    /// Findings emitted in the selected output format.
    pub visible_hits: usize,
    /// Findings hidden only by `--min-confidence`.
    pub hidden_hits: usize,
    /// Findings removed by an exclude, inline suppression, or mute.
    pub suppressed_hits: usize,
    /// Unsuppressed findings whose configured rule severity is `error`.
    pub error_hits: usize,
    /// Unsuppressed findings whose configured rule severity is `warn`.
    pub warn_hits: usize,
    /// Unsuppressed findings which make this run exit non-zero.
    pub gating_hits: usize,
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
    /// Per-file breakdown of `hunks_scanned` (JSON format only).
    pub files_scanned: Vec<FileScan>,
    /// Combined fingerprint of the fit-time model that scored the diff.
    pub model: String,
    /// The selected status and count contract for this run.
    pub result: CheckResult,
}

/// Map a hit to its SARIF result level: the confidence tier grades the level
/// (`note`/`warning`/`error`), and a `warn`-severity rule caps it at
/// `warning` — SARIF `error` is reserved for findings that fail the check.
///
/// Unknown tiers fall back to `warning` so a future tier never silently
/// disappears from code-scanning results.
fn sarif_level(confidence: &str, severity: &str) -> &'static str {
    let level = match confidence {
        "unusual" => "note",
        "suspicious" => "warning",
        "foreign" => "error",
        _ => "warning",
    };
    if severity != "error" && level == "error" {
        "warning"
    } else {
        level
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
        "schema_version": 1,
        "tool": { "name": "argot", "version": meta.tool_version },
        "model": meta.model,
        "repo": meta.repo,
        "scanned": meta.scanned,
        "hunks_scanned": meta.hunks_scanned,
        "files_scanned": meta.files_scanned,
        "result": meta.result,
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
    // Rules: distinct rule names, first-appearance order.
    let mut rule_ids: Vec<(&str, &str)> = Vec::new();
    for h in hits {
        if !rule_ids.iter().any(|(id, _)| *id == h.rule) {
            rule_ids.push((&h.rule, &h.rule_label));
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
                .position(|(id, _)| *id == h.rule)
                .expect("rule registered above");
            let mut text = format!(
                "{} — score {:.2} vs threshold {:.2} ({})",
                h.rule_label, h.score, h.threshold, h.confidence
            );
            for line in &h.evidence {
                text.push('\n');
                text.push_str(line);
            }
            json!({
                "ruleId": h.rule,
                "ruleIndex": rule_index,
                "level": sarif_level(&h.confidence, &h.severity),
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
                    "confidence": h.confidence,
                    "severity": h.severity,
                    "source": h.source,
                    "hash": h.hash,
                    "evidence": h.evidence,
                },
            })
        })
        .collect();
    let notifications = if meta.result.hidden_hits > 0 && meta.result.gating_hits > 0 {
        vec![json!({
            "level": "warning",
            "message": {
                "text": format!(
                    "{} finding(s) hidden by --min-confidence affect this run's status; lower --min-confidence to reveal them.",
                    meta.result.hidden_hits
                )
            }
        })]
    } else {
        Vec::new()
    };

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
            "invocations": [{ "toolExecutionNotifications": notifications }],
            "properties": {
                "model": meta.model,
                "repo": meta.repo,
                "scanned": meta.scanned,
                "hunksScanned": meta.hunks_scanned,
                "result": meta.result,
            },
        }],
    });
    to_pretty(&doc)
}

/// Escape a workflow-command message value (GitHub's own rules: `%`, CR, LF).
fn github_escape(s: &str) -> String {
    s.replace('%', "%25")
        .replace('\r', "%0D")
        .replace('\n', "%0A")
}

/// Escape a workflow-command *property* value (adds `:` and `,`).
fn github_escape_property(s: &str) -> String {
    github_escape(s).replace(':', "%3A").replace(',', "%2C")
}

/// Render GitHub Actions workflow commands (`--format github`): one
/// `::error`/`::warning` line per hit — the runner turns these into inline PR
/// annotations with no upload step. Severity maps directly: `error` rules
/// annotate as errors, `warn` rules as warnings.
pub fn render_github(hits: &[HitRecord], result: &CheckResult) -> String {
    let mut out = String::new();
    for h in hits {
        let level = if h.severity == "error" {
            "error"
        } else {
            "warning"
        };
        let mut message = format!(
            "{} — score {:.2} vs threshold {:.2} ({} confidence)",
            h.rule_label, h.score, h.threshold, h.confidence
        );
        for line in &h.evidence {
            message.push('\n');
            message.push_str(line);
        }
        message.push_str(&format!("\nmute with: argot mute {}", h.hash));
        out.push_str(&format!(
            "::{level} file={},line={},endLine={},title={}::{}\n",
            github_escape_property(&h.path),
            h.line_start,
            h.line_end,
            github_escape_property(&format!("argot: {}", h.rule)),
            github_escape(&message),
        ));
    }
    if result.hidden_hits > 0 && result.gating_hits > 0 {
        out.push_str(&format!(
            "::notice title=argot::{} finding(s) hidden by --min-confidence affect this run's status; lower --min-confidence to reveal them.\n",
            result.hidden_hits
        ));
    }
    out
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
            files_scanned: vec![FileScan {
                path: "src/app.py".to_string(),
                hunks: 7,
            }],
            model: "abc123def456".to_string(),
            result: CheckResult {
                exit_code: 1,
                unsuppressed_hits: 1,
                visible_hits: 1,
                hidden_hits: 0,
                suppressed_hits: 0,
                error_hits: 1,
                warn_hits: 0,
                gating_hits: 1,
            },
        }
    }

    fn hit(conf: &str, rule: &str) -> HitRecord {
        HitRecord {
            path: "src/app.py".to_string(),
            line_start: 10,
            line_end: 16,
            score: 8.25,
            threshold: 6.75,
            confidence: conf.to_string(),
            severity: "error".to_string(),
            rule: rule.to_string(),
            rule_label: match rule {
                "rare-tokens" => "rare token sequence",
                "foreign-import" => "foreign import",
                _ => rule,
            }
            .to_string(),
            source: "workdir".to_string(),
            hash: "a1b2c3d4e5f6".to_string(),
            evidence: vec!["↳ axios — 0 of 47 module specifiers in repo".to_string()],
            symbol: None,
            foreign_specifiers: if rule == "foreign-import" {
                vec!["axios".to_string()]
            } else {
                Vec::new()
            },
            similarity: None,
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
        let out = render_json(&meta(), &[hit("suspicious", "rare-tokens")]);
        let doc: Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(doc["tool"]["name"], "argot");
        assert_eq!(doc["tool"]["version"], "0.0.0-test");
        assert_eq!(doc["repo"], "/tmp/repo");
        assert_eq!(doc["scanned"], "workdir");
        assert_eq!(doc["hunks_scanned"], 7);
        assert_eq!(doc["files_scanned"][0]["path"], "src/app.py");
        assert_eq!(doc["files_scanned"][0]["hunks"], 7);
        let h = &doc["hits"][0];
        assert_eq!(h["path"], "src/app.py");
        assert_eq!(h["line_start"], 10);
        assert_eq!(h["line_end"], 16);
        assert_eq!(h["score"], 8.25);
        assert_eq!(h["threshold"], 6.75);
        assert_eq!(h["confidence"], "suspicious");
        assert_eq!(h["severity"], "error");
        assert_eq!(h["rule"], "rare-tokens");
        assert_eq!(h["rule_label"], "rare token sequence");
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
        let out = render_sarif(&meta(), &[hit("foreign", "foreign-import")]);
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
    fn sarif_maps_confidence_tiers_to_levels() {
        let hits = [
            hit("unusual", "rare-tokens"),
            hit("suspicious", "rare-tokens"),
            hit("foreign", "rare-tokens"),
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
    fn sarif_caps_warn_severity_rules_at_warning() {
        let mut h = hit("foreign", "redundant");
        h.severity = "warn".to_string();
        let out = render_sarif(&meta(), &[h]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(doc["runs"][0]["results"][0]["level"], "warning");
    }

    #[test]
    fn sarif_result_carries_rule_location_and_properties() {
        let out = render_sarif(&meta(), &[hit("foreign", "foreign-import")]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let run = &doc["runs"][0];
        let r = &run["results"][0];
        assert_eq!(r["ruleId"], "foreign-import");
        assert_eq!(r["ruleIndex"], 0);
        let rule = &run["tool"]["driver"]["rules"][0];
        assert_eq!(rule["id"], "foreign-import");
        assert_eq!(rule["shortDescription"]["text"], "foreign import");
        let loc = &r["locations"][0]["physicalLocation"];
        assert_eq!(loc["artifactLocation"]["uri"], "src/app.py");
        assert_eq!(loc["region"]["startLine"], 10);
        assert_eq!(loc["region"]["endLine"], 16);
        assert_eq!(r["properties"]["score"], 8.25);
        assert_eq!(r["properties"]["confidence"], "foreign");
        assert_eq!(r["properties"]["severity"], "error");
        assert_eq!(r["properties"]["hash"], "a1b2c3d4e5f6");
        assert!(r["message"]["text"]
            .as_str()
            .unwrap()
            .contains("foreign import — score 8.25 vs threshold 6.75 (foreign)"));
        assert!(r["message"]["text"].as_str().unwrap().contains("axios"));
    }

    #[test]
    fn sarif_deduplicates_rules_by_rule_name() {
        let hits = [
            hit("unusual", "rare-tokens"),
            hit("foreign", "rare-tokens"),
            hit("foreign", "foreign-import"),
        ];
        let out = render_sarif(&meta(), &hits);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let rules = doc["runs"][0]["tool"]["driver"]["rules"]
            .as_array()
            .unwrap();
        assert_eq!(rules.len(), 2);
        assert_eq!(rules[0]["id"], "rare-tokens");
        assert_eq!(rules[1]["id"], "foreign-import");
        let results = doc["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results[2]["ruleIndex"], 1);
    }

    #[test]
    fn github_format_emits_one_annotation_per_hit_with_severity_level() {
        let mut warn_hit = hit("unusual", "redundant");
        warn_hit.severity = "warn".to_string();
        let result = meta().result;
        let out = render_github(&[hit("foreign", "foreign-import"), warn_hit], &result);
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].starts_with(
            "::error file=src/app.py,line=10,endLine=16,title=argot%3A foreign-import::"
        ));
        assert!(lines[0].contains("foreign import — score 8.25"));
        assert!(lines[0].contains("%0Amute with: argot mute a1b2c3d4e5f6"));
        assert!(lines[1].starts_with("::warning "));
    }

    #[test]
    fn github_format_escapes_workflow_command_metacharacters() {
        let mut h = hit("foreign", "foreign-import");
        h.path = "src/a,b:c.py".to_string();
        h.evidence = vec!["50% of\nlines".to_string()];
        let result = meta().result;
        let out = render_github(&[h], &result);
        assert!(out.contains("file=src/a%2Cb%3Ac.py"));
        assert!(out.contains("50%25 of%0Alines"));
    }

    #[test]
    fn github_format_reports_status_affecting_hidden_findings() {
        let mut result = meta().result;
        result.visible_hits = 0;
        result.hidden_hits = 2;
        result.gating_hits = 1;
        let out = render_github(&[], &result);
        assert!(out.contains("::notice title=argot::2 finding(s) hidden by --min-confidence"));
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

    #[test]
    fn sarif_reports_status_affecting_hidden_findings() {
        let mut meta = meta();
        meta.result.visible_hits = 0;
        meta.result.hidden_hits = 1;
        meta.result.gating_hits = 1;
        let out = render_sarif(&meta, &[]);
        let doc: Value = serde_json::from_str(&out).unwrap();
        let notification = &doc["runs"][0]["invocations"][0]["toolExecutionNotifications"][0];
        assert_eq!(notification["level"], "warning");
        assert!(notification["message"]["text"]
            .as_str()
            .unwrap()
            .contains("hidden by --min-confidence"));
    }
}
