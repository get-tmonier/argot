//! Versioned, aggregated benchmark result for the public dashboard (#64).
//!
//! `report::write_reports` emits the rich per-corpus JSON; this is the compact,
//! schema-stable summary the dashboard time-series and the PR comment read.
//! Bump `SCHEMA_VERSION` on any breaking shape change so old history entries
//! stay parseable.

use serde::Serialize;

use crate::run::CorpusReport;

/// Schema version of `dashboard.json` / history entries.
pub const SCHEMA_VERSION: u32 = 1;

/// Corpora benchmarked as *applications* (vs libraries). Eval-only knowledge —
/// this is bench code, so naming corpora here is fine.
const APPLICATION_CORPORA: &[&str] = &["saleor", "wagtail", "excalidraw", "outline"];

fn corpus_kind(name: &str) -> &'static str {
    if APPLICATION_CORPORA.contains(&name) {
        "application"
    } else {
        "library"
    }
}

#[derive(Serialize)]
pub struct DashboardCorpus {
    pub corpus: String,
    pub language: String,
    pub kind: &'static str,
    pub caught: usize,
    pub fixtures: usize,
    pub recall_pct: f64,
    pub false_positives: usize,
    pub eligible_controls: usize,
    pub fp_rate_pct: f64,
    pub threshold: f64,
    pub threshold_cv: f64,
}

#[derive(Serialize)]
pub struct DashboardTotals {
    pub caught: usize,
    pub fixtures: usize,
    pub recall_pct: f64,
    /// Worst per-corpus false-positive rate (the headline risk number).
    pub worst_fp_rate_pct: f64,
}

/// The compact, versioned dashboard payload.
#[derive(Serialize)]
pub struct BenchDashboard {
    pub schema_version: u32,
    /// ISO-8601 UTC; supplied by the caller (the binary stamps it).
    pub generated_at: String,
    /// Commit the bench ran against (short sha), supplied by the caller.
    pub commit: String,
    pub corpora: Vec<DashboardCorpus>,
    pub totals: DashboardTotals,
}

impl BenchDashboard {
    /// Aggregate per-corpus reports into the dashboard summary.
    pub fn from_reports(reports: &[CorpusReport], commit: String, generated_at: String) -> Self {
        let corpora: Vec<DashboardCorpus> = reports
            .iter()
            .map(|r| DashboardCorpus {
                corpus: r.corpus.clone(),
                language: r.language.clone(),
                kind: corpus_kind(&r.corpus),
                caught: r.n_flagged_fixtures,
                fixtures: r.n_fixtures,
                recall_pct: pct(r.n_flagged_fixtures, r.n_fixtures),
                false_positives: r.n_false_positives,
                eligible_controls: r.n_eligible_controls,
                fp_rate_pct: r.fp_rate * 100.0,
                threshold: r.threshold,
                threshold_cv: r.threshold_cv,
            })
            .collect();
        let caught: usize = corpora.iter().map(|c| c.caught).sum();
        let fixtures: usize = corpora.iter().map(|c| c.fixtures).sum();
        let worst_fp_rate_pct = corpora
            .iter()
            .map(|c| c.fp_rate_pct)
            .fold(0.0_f64, f64::max);
        BenchDashboard {
            schema_version: SCHEMA_VERSION,
            generated_at,
            commit,
            totals: DashboardTotals {
                caught,
                fixtures,
                recall_pct: pct(caught, fixtures),
                worst_fp_rate_pct,
            },
            corpora,
        }
    }

    /// A GitHub-flavored Markdown table for the PR comment.
    pub fn markdown_table(&self) -> String {
        use std::fmt::Write as _;
        let mut out = String::new();
        let _ = writeln!(
            out,
            "**argot benchmark** — {} caught / {} fixtures (**{:.1}%** recall), worst FP {:.2}% · `{}`\n",
            self.totals.caught, self.totals.fixtures, self.totals.recall_pct,
            self.totals.worst_fp_rate_pct, self.commit
        );
        let _ = writeln!(out, "| Corpus | Type | Recall | FP rate |");
        let _ = writeln!(out, "|---|---|---:|---:|");
        for c in &self.corpora {
            let _ = writeln!(
                out,
                "| {} ({}) | {} | {}/{} ({:.1}%) | {:.2}% |",
                c.corpus, c.language, c.kind, c.caught, c.fixtures, c.recall_pct, c.fp_rate_pct
            );
        }
        out
    }
}

fn pct(num: usize, den: usize) -> f64 {
    if den == 0 {
        0.0
    } else {
        100.0 * num as f64 / den as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::run::CorpusReport;
    use std::collections::BTreeMap;

    fn report(corpus: &str, caught: usize, fixtures: usize, fp_rate: f64) -> CorpusReport {
        CorpusReport {
            corpus: corpus.to_string(),
            language: "python".to_string(),
            n_fixtures: fixtures,
            n_flagged_fixtures: caught,
            uncaught: vec![],
            n_controls: 1000,
            n_eligible_controls: 900,
            n_false_positives: (fp_rate * 900.0) as usize,
            fp_rate,
            auc: 0.99,
            threshold: 4.2,
            seed_thresholds: vec![],
            threshold_cv: 0.05,
            resolved_rare_threshold: 2,
            recall_by_category: BTreeMap::new(),
            stage_attribution: BTreeMap::new(),
            fixture_results: vec![],
            control_results: vec![],
        }
    }

    #[test]
    fn aggregates_totals_and_worst_fp() {
        let reports = vec![
            report("fastapi", 32, 32, 0.0),
            report("excalidraw", 9, 14, 0.0317),
        ];
        let d = BenchDashboard::from_reports(&reports, "abc123".into(), "t".into());
        assert_eq!(d.schema_version, SCHEMA_VERSION);
        assert_eq!(d.totals.caught, 41);
        assert_eq!(d.totals.fixtures, 46);
        // worst FP is excalidraw's 3.17%.
        assert!((d.totals.worst_fp_rate_pct - 3.17).abs() < 0.01);
        // excalidraw is categorized as an application.
        let exc = d.corpora.iter().find(|c| c.corpus == "excalidraw").unwrap();
        assert_eq!(exc.kind, "application");
        assert!(d.markdown_table().contains("| Corpus |"));
    }
}
