//! Accepted-change replay for the proposed advisory combined brief.
//!
//! The runner deliberately consumes [`argot_core::check::run_check`] instead
//! of rebuilding detectors in the benchmark. That public facade closes over
//! `argot-core`'s release composition root, so the feature set selected for a
//! bench build is the feature set whose findings are measured.

use anyhow::{bail, Context, Result};
#[cfg(feature = "release-composition")]
use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
#[cfg(feature = "release-composition")]
use argot_core::output::OutputFormat;
use serde::{Deserialize, Serialize};
#[cfg(feature = "release-composition")]
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
#[cfg(feature = "release-composition")]
use std::time::Instant;

pub const SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Deserialize)]
pub struct ReplayManifest {
    pub schema: u32,
    pub population_id: String,
    pub seed: u64,
    pub changes: Vec<AcceptedChange>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AcceptedChange {
    pub repo: String,
    pub repo_path: PathBuf,
    pub repo_revision: String,
    pub base: String,
    pub head: String,
    pub accepted_unit: String,
    pub stratum: String,
    #[serde(default = "selected")]
    pub selected: bool,
}

fn selected() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Sampling {
    pub population_id: String,
    pub stratum: String,
    pub seed: u64,
    pub selected: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Environment {
    pub argot_version: String,
    pub features: Vec<String>,
    pub config_fingerprint: String,
    pub model_fingerprint: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct TimingMs {
    pub fit: u64,
    pub scan: u64,
    pub render: u64,
    pub total: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Counts {
    pub findings: usize,
    pub displayed_hits: usize,
    pub briefs: usize,
    pub diagnostics: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct FindingRecord {
    pub hash: String,
    pub rule: String,
    pub severity: String,
    pub path: String,
    pub line: usize,
    pub suppressed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Adjudication {
    pub hash: String,
    pub rater_a: String,
    pub rater_b: String,
    #[serde(rename = "final")]
    pub final_label: String,
    pub note: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RawRecord {
    pub schema: u32,
    pub repo: String,
    pub repo_revision: String,
    pub base: String,
    pub head: String,
    pub accepted_unit: String,
    pub sampling: Sampling,
    pub environment: Environment,
    pub timing_ms: TimingMs,
    pub counts: Counts,
    pub findings: Vec<FindingRecord>,
    pub adjudication: Vec<Adjudication>,
    pub status: String,
    pub raw_output_path: String,
}

/// Read protocol records without normalising them: pinned evidence must remain
/// inspectable exactly as captured.
pub fn load_records(path: &Path) -> Result<Vec<RawRecord>> {
    let input =
        std::fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    input
        .lines()
        .filter(|line| !line.trim().is_empty())
        .enumerate()
        .map(|(line, raw)| {
            let record: RawRecord = serde_json::from_str(raw).with_context(|| {
                format!(
                    "{}:{} is not a raw-record JSON object",
                    path.display(),
                    line + 1
                )
            })?;
            validate_record(&record)?;
            Ok(record)
        })
        .collect()
}

pub fn validate_record(record: &RawRecord) -> Result<()> {
    if record.schema != SCHEMA_VERSION {
        bail!("unsupported raw-record schema {}", record.schema);
    }
    if !matches!(record.accepted_unit.as_str(), "merge" | "commit") {
        bail!("{}: accepted_unit must be merge or commit", record.repo);
    }
    if !matches!(
        record.status.as_str(),
        "ok" | "setup_diagnostic" | "execution_error"
    ) {
        bail!("{}: unknown status {}", record.repo, record.status);
    }
    if record.counts.findings != record.findings.len() {
        bail!(
            "{}: findings count does not match finding records",
            record.repo
        );
    }
    let displayed = record
        .findings
        .iter()
        .filter(|finding| !finding.suppressed)
        .count();
    if record.counts.displayed_hits != displayed {
        bail!(
            "{}: displayed_hits must equal unsuppressed finding records",
            record.repo
        );
    }
    let expected_briefs = usize::from(record.status == "ok" && displayed > 0);
    if record.counts.briefs != expected_briefs {
        bail!(
            "{}: policy permits one advisory brief only for an ok displayed state",
            record.repo
        );
    }
    if record.adjudication.iter().any(|a| {
        !matches!(
            a.final_label.as_str(),
            "actionable" | "not-actionable" | "uncertain"
        )
    }) {
        bail!(
            "{}: adjudication labels must be actionable, not-actionable, or uncertain",
            record.repo
        );
    }
    Ok(())
}

/// Replays a manifest through the distributed check facade. The caller owns
/// corpus acquisition; this function never substitutes a repository or SHA.
pub fn replay_manifest(manifest: &ReplayManifest, output_dir: &Path) -> Result<Vec<RawRecord>> {
    #[cfg(not(feature = "release-composition"))]
    {
        let _ = (manifest, output_dir);
        bail!(
            "accepted-change replay requires --features release-composition so it cannot measure a partial build"
        );
    }
    #[cfg(feature = "release-composition")]
    {
        if manifest.schema != SCHEMA_VERSION {
            bail!("unsupported replay-manifest schema {}", manifest.schema);
        }
        std::fs::create_dir_all(output_dir)?;
        let mut records = Vec::new();
        for change in manifest.changes.iter().filter(|change| change.selected) {
            records.push(replay_change(
                change,
                &manifest.population_id,
                manifest.seed,
                output_dir,
            )?);
        }
        Ok(records)
    }
}

#[cfg(feature = "release-composition")]
fn replay_change(
    change: &AcceptedChange,
    population_id: &str,
    seed: u64,
    output_dir: &Path,
) -> Result<RawRecord> {
    if !change.repo_path.join(".git").exists() {
        bail!(
            "{}: pinned repository is unavailable at {}",
            change.repo,
            change.repo_path.display()
        );
    }
    if !matches!(change.accepted_unit.as_str(), "merge" | "commit") {
        bail!("{}: accepted_unit must be merge or commit", change.repo);
    }
    let fit_started = Instant::now();
    checkout(&change.repo_path, &change.base)?;
    let argot_dir = change.repo_path.join(".argot");
    if argot_dir.exists() {
        std::fs::remove_dir_all(&argot_dir)
            .with_context(|| format!("clearing {}", argot_dir.display()))?;
    }
    std::fs::create_dir_all(&argot_dir)?;
    argot_core::train::run_train(
        &change.repo_path,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )?;
    argot_core::scoring::calibration::run_calibrate(
        &change.repo_path,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &argot_core::scoring::calibration::CalibrateOptions {
            repo_sha: change.base.clone(),
            timestamp_utc: "1970-01-01T00:00:00+00:00".to_owned(),
            ..Default::default()
        },
    )?;
    let fit = elapsed_ms(fit_started);
    let scan_started = Instant::now();
    let outcome = run_check(CheckArgs {
        repo_path: change.repo_path.to_string_lossy().into_owned(),
        reference: format!("{}..{}", change.base, change.head),
        staged: false,
        unstaged: false,
        commit: None,
        only: Vec::new(),
        exclude: Vec::new(),
        threshold: None,
        argot_dir: argot_dir.clone(),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_confidence: "unusual".to_owned(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: OutputFormat::Json,
        today: "2026-01-01".to_owned(),
    });
    let scan = elapsed_ms(scan_started);
    let raw_name = format!(
        "{}-{}.json",
        safe_name(&change.repo),
        &change.head[..12.min(change.head.len())]
    );
    let raw_path = output_dir.join(raw_name);
    std::fs::write(&raw_path, &outcome.stdout)?;
    let doc: serde_json::Value = serde_json::from_str(&outcome.stdout)
        .with_context(|| format!("{}: check did not emit JSON", change.repo))?;
    let findings = doc["hits"]
        .as_array()
        .context("check JSON misses hits")?
        .iter()
        .map(|hit| FindingRecord {
            hash: hit["hash"].as_str().unwrap_or_default().to_owned(),
            rule: hit["rule"].as_str().unwrap_or_default().to_owned(),
            severity: hit["severity"].as_str().unwrap_or_default().to_owned(),
            path: hit["path"].as_str().unwrap_or_default().to_owned(),
            line: hit["line_start"].as_u64().unwrap_or(0) as usize,
            suppressed: false,
        })
        .collect::<Vec<_>>();
    let render_started = Instant::now();
    let model = doc["model"].as_str().unwrap_or_default().to_owned();
    let record = RawRecord {
        schema: SCHEMA_VERSION,
        repo: change.repo.clone(),
        repo_revision: change.repo_revision.clone(),
        base: change.base.clone(),
        head: change.head.clone(),
        accepted_unit: change.accepted_unit.clone(),
        sampling: Sampling {
            population_id: population_id.to_owned(),
            stratum: change.stratum.clone(),
            seed,
            selected: true,
        },
        environment: Environment {
            argot_version: env!("CARGO_PKG_VERSION").to_owned(),
            features: release_features(),
            config_fingerprint: fingerprint_file(&change.repo_path.join("argot.toml"))?,
            model_fingerprint: model,
        },
        timing_ms: TimingMs {
            fit,
            scan,
            render: elapsed_ms(render_started),
            total: fit + scan,
        },
        counts: Counts {
            findings: findings.len(),
            displayed_hits: findings.len(),
            briefs: usize::from(!findings.is_empty()),
            diagnostics: 0,
        },
        findings,
        adjudication: Vec::new(),
        status: if outcome.exit_code == 2 {
            "execution_error".to_owned()
        } else {
            "ok".to_owned()
        },
        raw_output_path: raw_path.to_string_lossy().into_owned(),
    };
    validate_record(&record)?;
    Ok(record)
}

#[cfg(feature = "release-composition")]
fn checkout(repo: &Path, sha: &str) -> Result<()> {
    let status = std::process::Command::new("git")
        .args(["-C"])
        .arg(repo)
        .args(["checkout", "--quiet", "--detach", sha])
        .status()?;
    if !status.success() {
        bail!("{}: cannot check out pinned SHA {sha}", repo.display());
    }
    Ok(())
}

#[cfg(feature = "release-composition")]
fn elapsed_ms(start: Instant) -> u64 {
    start.elapsed().as_millis() as u64
}
#[cfg(feature = "release-composition")]
fn safe_name(value: &str) -> String {
    value.replace(['/', '\\'], "_")
}
#[cfg(feature = "release-composition")]
fn fingerprint_file(path: &Path) -> Result<String> {
    let bytes = if path.exists() {
        std::fs::read(path)?
    } else {
        Vec::new()
    };
    Ok(Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}
#[cfg(feature = "release-composition")]
fn release_features() -> Vec<String> {
    ["voice", "semantic", "arch", "integrity", "script"]
        .into_iter()
        .map(str::to_owned)
        .collect()
}

#[derive(Debug, Serialize, PartialEq)]
pub struct Aggregate {
    pub schema: u32,
    pub accepted_changes: usize,
    pub findings: usize,
    pub displayed_hits: usize,
    pub briefs: usize,
    pub diagnostics: usize,
    pub execution_errors: usize,
    pub findings_per_accepted_change: f64,
    pub displayed_hits_per_accepted_change: f64,
    pub briefs_per_accepted_change: f64,
    pub adjudication: BTreeMap<String, usize>,
    pub per_rule: BTreeMap<String, RuleAggregate>,
    pub timing_ms: TimingPercentiles,
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct RuleAggregate {
    pub findings: usize,
    pub displayed_hits: usize,
    /// Briefs that disappear if this rule's displayed hits are removed.
    pub marginal_union_briefs: usize,
}

#[derive(Debug, Default, Serialize, PartialEq, Eq)]
pub struct TimingPercentiles {
    pub clean_p95: u64,
    pub noisy_p95: u64,
}

pub fn aggregate(records: &[RawRecord]) -> Result<Aggregate> {
    for record in records {
        validate_record(record)?;
    }
    let mut out = Aggregate {
        schema: SCHEMA_VERSION,
        accepted_changes: records.len(),
        findings: 0,
        displayed_hits: 0,
        briefs: 0,
        diagnostics: 0,
        execution_errors: 0,
        findings_per_accepted_change: 0.0,
        displayed_hits_per_accepted_change: 0.0,
        briefs_per_accepted_change: 0.0,
        adjudication: BTreeMap::new(),
        per_rule: BTreeMap::new(),
        timing_ms: TimingPercentiles::default(),
    };
    let mut clean = Vec::new();
    let mut noisy = Vec::new();
    for record in records {
        out.findings += record.counts.findings;
        out.displayed_hits += record.counts.displayed_hits;
        out.briefs += record.counts.briefs;
        out.diagnostics += record.counts.diagnostics;
        out.execution_errors += usize::from(record.status == "execution_error");
        if record.counts.displayed_hits == 0 {
            clean.push(record.timing_ms.total);
        } else {
            noisy.push(record.timing_ms.total);
        }
        let displayed_rules: BTreeSet<&str> = record
            .findings
            .iter()
            .filter(|finding| !finding.suppressed)
            .map(|finding| finding.rule.as_str())
            .collect();
        for finding in &record.findings {
            let rule = out.per_rule.entry(finding.rule.clone()).or_default();
            rule.findings += 1;
            if !finding.suppressed {
                rule.displayed_hits += 1;
            }
        }
        if displayed_rules.len() == 1 && record.counts.briefs == 1 {
            out.per_rule
                .entry((*displayed_rules.iter().next().expect("one rule")).to_owned())
                .or_default()
                .marginal_union_briefs += 1;
        }
        for label in record
            .adjudication
            .iter()
            .map(|entry| entry.final_label.clone())
        {
            *out.adjudication.entry(label).or_default() += 1;
        }
    }
    out.timing_ms = TimingPercentiles {
        clean_p95: percentile(&mut clean, 95),
        noisy_p95: percentile(&mut noisy, 95),
    };
    if out.accepted_changes > 0 {
        let denominator = out.accepted_changes as f64;
        out.findings_per_accepted_change = out.findings as f64 / denominator;
        out.displayed_hits_per_accepted_change = out.displayed_hits as f64 / denominator;
        out.briefs_per_accepted_change = out.briefs as f64 / denominator;
    }
    Ok(out)
}

fn percentile(values: &mut [u64], pct: usize) -> u64 {
    if values.is_empty() {
        return 0;
    }
    values.sort_unstable();
    values[((values.len() * pct).div_ceil(100)).saturating_sub(1)]
}

#[cfg(test)]
mod tests {
    use super::*;
    fn record(findings: Vec<FindingRecord>, total: u64) -> RawRecord {
        let displayed = findings
            .iter()
            .filter(|finding| !finding.suppressed)
            .count();
        RawRecord {
            schema: 1,
            repo: "org/repo".to_owned(),
            repo_revision: "r".to_owned(),
            base: "b".to_owned(),
            head: "h".to_owned(),
            accepted_unit: "commit".to_owned(),
            sampling: Sampling {
                population_id: "dry-run".to_owned(),
                stratum: "rust/small".to_owned(),
                seed: 7,
                selected: true,
            },
            environment: Environment {
                argot_version: "x".to_owned(),
                features: vec!["voice".to_owned()],
                config_fingerprint: "c".to_owned(),
                model_fingerprint: "m".to_owned(),
            },
            timing_ms: TimingMs {
                total,
                ..Default::default()
            },
            counts: Counts {
                findings: findings.len(),
                displayed_hits: displayed,
                briefs: usize::from(displayed > 0),
                diagnostics: 0,
            },
            findings,
            adjudication: Vec::new(),
            status: "ok".to_owned(),
            raw_output_path: "raw.json".to_owned(),
        }
    }
    fn finding(rule: &str, suppressed: bool) -> FindingRecord {
        FindingRecord {
            hash: format!("{rule}-{suppressed}"),
            rule: rule.to_owned(),
            severity: "error".to_owned(),
            path: "src/a.rs".to_owned(),
            line: 1,
            suppressed,
        }
    }
    #[test]
    fn protocol_dry_run_preserves_clean_many_and_suppressed_denominators() {
        let records = vec![
            record(vec![], 10),
            record(
                vec![
                    finding("foreign-import", false),
                    finding("test-weakened", false),
                ],
                20,
            ),
            record(vec![finding("foreign-import", true)], 30),
        ];
        let totals = aggregate(&records).unwrap();
        assert_eq!(
            (totals.findings, totals.displayed_hits, totals.briefs),
            (3, 2, 1)
        );
        assert_eq!(
            (totals.timing_ms.clean_p95, totals.timing_ms.noisy_p95),
            (30, 20)
        );
    }
    #[test]
    fn per_rule_reports_marginal_union_contribution() {
        let records = vec![
            record(vec![finding("foreign-import", false)], 1),
            record(
                vec![finding("foreign-import", false), finding("layering", false)],
                1,
            ),
        ];
        let totals = aggregate(&records).unwrap();
        assert_eq!(totals.per_rule["foreign-import"].marginal_union_briefs, 1);
        assert_eq!(totals.per_rule["layering"].marginal_union_briefs, 0);
    }
    #[test]
    fn rejects_a_brief_for_only_suppressed_findings() {
        let mut invalid = record(vec![finding("foreign-import", true)], 1);
        invalid.counts.briefs = 1;
        assert!(validate_record(&invalid).is_err());
    }

    #[test]
    fn pinned_protocol_dry_run_recomputes_the_declared_denominators() {
        let records: Vec<RawRecord> =
            include_str!("../../../benchmarks/accept-brief/dry-run-records.jsonl")
                .lines()
                .map(serde_json::from_str)
                .collect::<serde_json::Result<_>>()
                .unwrap();
        let totals = aggregate(&records).unwrap();
        assert_eq!(totals.accepted_changes, 3);
        assert_eq!(
            (totals.findings, totals.displayed_hits, totals.briefs),
            (3, 2, 1)
        );
        assert_eq!(totals.adjudication["uncertain"], 1);
        assert_eq!(totals.adjudication["not-actionable"], 1);
    }

    #[cfg(feature = "release-composition")]
    #[test]
    fn release_composition_build_names_every_distributed_detector_slice() {
        assert_eq!(
            release_features(),
            vec!["voice", "semantic", "arch", "integrity", "script"]
        );
    }
}
