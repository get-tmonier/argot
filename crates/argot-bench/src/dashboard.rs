//! Versioned, aggregated benchmark result for the public dashboard (#64).
//!
//! `report::write_reports` emits the rich per-corpus JSON; this is the compact,
//! schema-stable summary the dashboard time-series and the PR comment read.
//! Bump `SCHEMA_VERSION` on any breaking shape change so old history entries
//! stay parseable.
//!
//! v2 (issue #92): false positives come from the leak-free temporal-holdout
//! mode, split into existing-file vs new-file rates with bootstrap CIs. The
//! old per-corpus `fp_rate_pct` (train-on-test ancestor replay) is gone.

use serde::Serialize;
use std::collections::BTreeMap;

use crate::holdout::HoldoutReport;
use crate::production::ProductionReport;
use crate::run::CorpusReport;

/// Schema version of `dashboard.json` / history entries.
pub const SCHEMA_VERSION: u32 = 2;

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

/// One temporal-holdout FP split for the dashboard.
#[derive(Serialize, Clone, Default)]
pub struct DashboardFp {
    pub hits: usize,
    pub hunks: usize,
    pub rate_pct: f64,
    /// Commit-level bootstrap 95% CI, percent.
    pub ci95_pct: Option<(f64, f64)>,
}

#[derive(Serialize)]
pub struct DashboardCorpus {
    pub corpus: String,
    pub language: String,
    pub kind: &'static str,
    /// Recall over the curated spliced catalog (production path). `None`
    /// when the corpus has no catalog (holdout-only corpora). This is the
    /// aggregate over ALL fixture classes — including the secondary naming /
    /// semantic classes argot reports but does not gate on; read
    /// `gated_recall_pct` for the headline metric.
    pub caught: Option<usize>,
    pub fixtures: Option<usize>,
    pub recall_pct: Option<f64>,
    /// **The headline metric** — recall over the gated *novel-pattern* classes
    /// only (`foreign_import` / `foreign_api` / `foreign_concurrency`; RUBRIC).
    /// `None` when the corpus has no gated fixtures (legacy catalogs predating
    /// the RUBRIC, or FP-only corpora).
    pub gated_caught: Option<usize>,
    pub gated_fixtures: Option<usize>,
    pub gated_recall_pct: Option<f64>,
    /// Broad **novel-pattern (foreign) catch** — recall over every foreign
    /// class (RUBRIC gated + legacy foreign names; `production::is_novel_pattern`),
    /// the consistent per-corpus number that excludes the naming/semantic
    /// classes argot never targets. `None` for FP-only corpora (no catalog).
    /// This is what the per-corpus "catch" column should show; `recall_pct`
    /// (the raw aggregate over *all* classes) is retained only for history.
    pub foreign_caught: Option<usize>,
    pub foreign_fixtures: Option<usize>,
    pub foreign_recall_pct: Option<f64>,
    /// Temporal-holdout FP, split by whether the file existed at the fit
    /// point. `None` when holdout did not run (quick bench).
    pub fp_existing: Option<DashboardFp>,
    pub fp_new_file: Option<DashboardFp>,
    /// `fp_existing`/`fp_new_file` decomposed into **over-fire** (the true
    /// false-alarm rate — argot fired on the repo's own code) and
    /// **novel-pattern detection** (argot fired on a symbol 0-usage in the repo
    /// at fit — a new dependency or API in a real commit, i.e. argot working).
    /// Over-fire is the headline risk number; detection is reported, not gated.
    pub fp_existing_overfire: Option<DashboardFp>,
    pub fp_existing_detection: Option<DashboardFp>,
    pub fp_new_overfire: Option<DashboardFp>,
    pub fp_new_detection: Option<DashboardFp>,
    /// Fewer eligible holdout hunks than the sample bar — widen the window.
    pub under_sampled: bool,
    pub threshold: f64,
}

#[derive(Serialize)]
pub struct DashboardTotals {
    pub caught: usize,
    pub fixtures: usize,
    pub recall_pct: f64,
    /// **The headline metric** — gated novel-pattern recall across all corpora
    /// (RUBRIC-tagged corpora only; the cleanest, most conservative number).
    pub gated_caught: usize,
    pub gated_fixtures: usize,
    pub gated_recall_pct: f64,
    /// Broad novel-pattern (foreign) recall across every catalogued corpus —
    /// RUBRIC + legacy foreign classes. Wider coverage than `gated_*`, still
    /// excludes the naming/semantic classes argot never targets.
    pub foreign_caught: usize,
    pub foreign_fixtures: usize,
    pub foreign_recall_pct: f64,
    /// Worst per-corpus temporal-holdout FP rates (total: over-fire + detection).
    pub worst_fp_existing_pct: f64,
    pub worst_fp_new_file_pct: f64,
    /// Worst per-corpus **over-fire** rate — the true false-alarm headline
    /// (argot firing on the repo's own code; excludes correct novel-pattern
    /// detections on real commits). This is the number to keep near zero.
    pub worst_fp_existing_overfire_pct: f64,
    pub worst_fp_new_overfire_pct: f64,
}

/// The compact, versioned dashboard payload.
#[derive(Serialize)]
pub struct BenchDashboard {
    pub schema_version: u32,
    /// ISO-8601 UTC; supplied by the caller (the binary stamps it).
    pub generated_at: String,
    /// Commit the bench ran against (short sha), supplied by the caller.
    pub commit: String,
    /// How the numbers were measured — spelled out so a reader of the raw
    /// JSON cannot mistake these for the retired train-on-test rates.
    pub methodology: &'static str,
    pub corpora: Vec<DashboardCorpus>,
    pub totals: DashboardTotals,
}

const METHODOLOGY: &str = "recall: curated spliced break fixtures planted on disk and judged by \
     the production fit+check pipeline; FP: temporal holdout — fit at an old SHA, replay only \
     commits strictly after it, split by file-existed-at-fit, commit-level bootstrap 95% CIs. \
     Each FP rate is further split into over-fire (argot fired on the repo's own code — the true \
     false-alarm rate) and novel-pattern detection (argot fired on a symbol/module 0-usage in the \
     repo at fit — a new dependency or API in a real commit, i.e. argot working). Reason import / \
     call_receiver = detection (fires only on a 0-usage symbol by construction); bpe / convention \
     = over-fire (distributional surprise on the repo's own tokens; conservatively counted as \
     false alarm).";

/// Gated novel-pattern recall for a production report: `(caught, total)` over
/// the `foreign_import`/`foreign_api`/`foreign_concurrency` fixtures. `None`
/// when the corpus has no gated fixtures (legacy catalogs, FP-only corpora).
fn gated_recall(r: &ProductionReport) -> Option<(usize, usize)> {
    let gated: Vec<_> = r
        .fixture_results
        .iter()
        .filter(|f| crate::production::tier_of(&f.category) == "gated")
        .collect();
    if gated.is_empty() {
        return None;
    }
    let caught = gated.iter().filter(|f| f.flagged).count();
    Some((caught, gated.len()))
}

/// Broad novel-pattern (foreign) recall: `(caught, total)` over every fixture
/// whose class names a foreign symbol/dep — RUBRIC gated classes plus the
/// legacy catalogs' foreign names (see `production::is_novel_pattern`). This is
/// the consistent per-corpus foreign-catch number; unlike `recall_pct` it does
/// not average in the naming/semantic classes argot never targets. `None` when
/// the corpus has no foreign fixtures (FP-only corpora).
fn foreign_recall(r: &ProductionReport) -> Option<(usize, usize)> {
    let foreign: Vec<_> = r
        .fixture_results
        .iter()
        .filter(|f| crate::production::is_novel_pattern(&f.category))
        .collect();
    if foreign.is_empty() {
        return None;
    }
    let caught = foreign.iter().filter(|f| f.flagged).count();
    Some((caught, foreign.len()))
}

fn fp(split: &crate::holdout::SplitFp) -> DashboardFp {
    DashboardFp {
        hits: split.hits,
        hunks: split.hunks,
        rate_pct: split.rate * 100.0,
        ci95_pct: split.ci95.map(|(lo, hi)| (lo * 100.0, hi * 100.0)),
    }
}

impl BenchDashboard {
    /// Combine production-path recall with temporal-holdout FP. Either side
    /// may be missing for a corpus (no catalog / quick mode).
    pub fn from_honest(
        production: &[ProductionReport],
        holdout: &[HoldoutReport],
        languages: &BTreeMap<String, String>,
        commit: String,
        generated_at: String,
    ) -> Self {
        let mut names: Vec<String> = production.iter().map(|r| r.corpus.clone()).collect();
        for h in holdout {
            if !names.contains(&h.corpus) {
                names.push(h.corpus.clone());
            }
        }
        let corpora: Vec<DashboardCorpus> = names
            .iter()
            .map(|name| {
                let prod = production.iter().find(|r| &r.corpus == name);
                let hold = holdout.iter().find(|r| &r.corpus == name);
                let threshold = prod
                    .map(|r| r.thresholds.values().cloned().fold(0.0_f64, f64::max))
                    .or_else(|| {
                        hold.map(|h| h.thresholds.values().cloned().fold(0.0_f64, f64::max))
                    })
                    .unwrap_or(0.0);
                let gated = prod.and_then(gated_recall);
                let foreign = prod.and_then(foreign_recall);
                DashboardCorpus {
                    corpus: name.clone(),
                    language: languages.get(name).cloned().unwrap_or_default(),
                    kind: corpus_kind(name),
                    caught: prod.map(|r| r.n_caught),
                    fixtures: prod.map(|r| r.n_fixtures),
                    recall_pct: prod.map(|r| pct(r.n_caught, r.n_fixtures)),
                    gated_caught: gated.map(|(c, _)| c),
                    gated_fixtures: gated.map(|(_, t)| t),
                    gated_recall_pct: gated.map(|(c, t)| pct(c, t)),
                    foreign_caught: foreign.map(|(c, _)| c),
                    foreign_fixtures: foreign.map(|(_, t)| t),
                    foreign_recall_pct: foreign.map(|(c, t)| pct(c, t)),
                    fp_existing: hold.map(|h| fp(&h.existing)),
                    fp_new_file: hold.map(|h| fp(&h.new_file)),
                    fp_existing_overfire: hold.map(|h| fp(&h.existing_overfire)),
                    fp_existing_detection: hold.map(|h| fp(&h.existing_detection)),
                    fp_new_overfire: hold.map(|h| fp(&h.new_overfire)),
                    fp_new_detection: hold.map(|h| fp(&h.new_detection)),
                    under_sampled: hold.map(|h| h.under_sampled).unwrap_or(false),
                    threshold,
                }
            })
            .collect();
        Self::finalize(corpora, commit, generated_at)
    }

    /// Catalog-mode aggregation (historical continuity; recall only).
    pub fn from_reports(reports: &[CorpusReport], commit: String, generated_at: String) -> Self {
        let corpora: Vec<DashboardCorpus> = reports
            .iter()
            .map(|r| DashboardCorpus {
                corpus: r.corpus.clone(),
                language: r.language.clone(),
                kind: corpus_kind(&r.corpus),
                caught: Some(r.n_flagged_fixtures),
                fixtures: Some(r.n_fixtures),
                recall_pct: Some(pct(r.n_flagged_fixtures, r.n_fixtures)),
                gated_caught: None,
                gated_fixtures: None,
                gated_recall_pct: None,
                foreign_caught: None,
                foreign_fixtures: None,
                foreign_recall_pct: None,
                fp_existing: None,
                fp_new_file: None,
                fp_existing_overfire: None,
                fp_existing_detection: None,
                fp_new_overfire: None,
                fp_new_detection: None,
                under_sampled: false,
                threshold: r.threshold,
            })
            .collect();
        Self::finalize(corpora, commit, generated_at)
    }

    /// Production-path recall only (quick bench; no holdout FP).
    pub fn from_production(
        reports: &[ProductionReport],
        commit: String,
        generated_at: String,
    ) -> Self {
        let corpora: Vec<DashboardCorpus> = reports
            .iter()
            .map(|r| {
                let language = if r.thresholds.is_empty() {
                    "mixed".to_string()
                } else {
                    r.thresholds.keys().cloned().collect::<Vec<_>>().join("+")
                };
                let gated = gated_recall(r);
                let foreign = foreign_recall(r);
                DashboardCorpus {
                    corpus: r.corpus.clone(),
                    language,
                    kind: corpus_kind(&r.corpus),
                    caught: Some(r.n_caught),
                    fixtures: Some(r.n_fixtures),
                    recall_pct: Some(pct(r.n_caught, r.n_fixtures)),
                    gated_caught: gated.map(|(c, _)| c),
                    gated_fixtures: gated.map(|(_, t)| t),
                    gated_recall_pct: gated.map(|(c, t)| pct(c, t)),
                    foreign_caught: foreign.map(|(c, _)| c),
                    foreign_fixtures: foreign.map(|(_, t)| t),
                    foreign_recall_pct: foreign.map(|(c, t)| pct(c, t)),
                    fp_existing: None,
                    fp_new_file: None,
                    fp_existing_overfire: None,
                    fp_existing_detection: None,
                    fp_new_overfire: None,
                    fp_new_detection: None,
                    under_sampled: false,
                    threshold: r.thresholds.values().cloned().fold(0.0_f64, f64::max),
                }
            })
            .collect();
        Self::finalize(corpora, commit, generated_at)
    }

    /// Shared totals assembly.
    fn finalize(corpora: Vec<DashboardCorpus>, commit: String, generated_at: String) -> Self {
        let caught: usize = corpora.iter().filter_map(|c| c.caught).sum();
        let fixtures: usize = corpora.iter().filter_map(|c| c.fixtures).sum();
        let gated_caught: usize = corpora.iter().filter_map(|c| c.gated_caught).sum();
        let gated_fixtures: usize = corpora.iter().filter_map(|c| c.gated_fixtures).sum();
        let foreign_caught: usize = corpora.iter().filter_map(|c| c.foreign_caught).sum();
        let foreign_fixtures: usize = corpora.iter().filter_map(|c| c.foreign_fixtures).sum();
        // Under-sampled corpora are excluded from the headline worst-rates:
        // a 1-hunk denominator reads as 0% or 100% and misleads either way.
        // Their per-corpus rows stay visible with the flag.
        let worst = |f: fn(&DashboardCorpus) -> Option<&DashboardFp>| {
            corpora
                .iter()
                .filter(|c| !c.under_sampled)
                .filter_map(f)
                .map(|s| s.rate_pct)
                .fold(0.0_f64, f64::max)
        };
        BenchDashboard {
            schema_version: SCHEMA_VERSION,
            generated_at,
            commit,
            methodology: METHODOLOGY,
            totals: DashboardTotals {
                caught,
                fixtures,
                recall_pct: pct(caught, fixtures),
                gated_caught,
                gated_fixtures,
                gated_recall_pct: pct(gated_caught, gated_fixtures),
                foreign_caught,
                foreign_fixtures,
                foreign_recall_pct: pct(foreign_caught, foreign_fixtures),
                worst_fp_existing_pct: worst(|c| c.fp_existing.as_ref()),
                worst_fp_new_file_pct: worst(|c| c.fp_new_file.as_ref()),
                worst_fp_existing_overfire_pct: worst(|c| c.fp_existing_overfire.as_ref()),
                worst_fp_new_overfire_pct: worst(|c| c.fp_new_overfire.as_ref()),
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
            "**argot benchmark** — {} caught / {} fixtures (**{:.1}%** recall), worst holdout FP existing {:.2}% / new-file {:.2}% · `{}`\n",
            self.totals.caught,
            self.totals.fixtures,
            self.totals.recall_pct,
            self.totals.worst_fp_existing_pct,
            self.totals.worst_fp_new_file_pct,
            self.commit
        );
        let _ = writeln!(
            out,
            "| Corpus | Type | Recall | FP existing | FP new-file |"
        );
        let _ = writeln!(out, "|---|---|---:|---:|---:|");
        let cell = |s: &Option<DashboardFp>, flagged: bool| match s {
            Some(f) => format!(
                "{:.2}% ({}/{}){}",
                f.rate_pct,
                f.hits,
                f.hunks,
                if flagged { " ⚠️" } else { "" }
            ),
            None => "—".to_string(),
        };
        for c in &self.corpora {
            let recall = match (c.caught, c.fixtures) {
                (Some(k), Some(n)) => format!("{}/{} ({:.1}%)", k, n, pct(k, n)),
                _ => "—".to_string(),
            };
            let _ = writeln!(
                out,
                "| {} ({}) | {} | {} | {} | {} |",
                c.corpus,
                c.language,
                c.kind,
                recall,
                cell(&c.fp_existing, c.under_sampled),
                cell(&c.fp_new_file, c.under_sampled),
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
    use crate::holdout::SplitFp;
    use crate::run::CorpusReport;
    use std::collections::BTreeMap;

    fn report(corpus: &str, caught: usize, fixtures: usize) -> CorpusReport {
        CorpusReport {
            corpus: corpus.to_string(),
            language: "python".to_string(),
            n_fixtures: fixtures,
            n_flagged_fixtures: caught,
            uncaught: vec![],
            n_controls: 1000,
            n_eligible_controls: 900,
            n_false_positives: 0,
            fp_rate: 0.0,
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
    fn aggregates_totals_from_catalog_reports() {
        let reports = vec![report("fastapi", 32, 32), report("excalidraw", 9, 14)];
        let d = BenchDashboard::from_reports(&reports, "abc123".into(), "t".into());
        assert_eq!(d.schema_version, SCHEMA_VERSION);
        assert_eq!(d.totals.caught, 41);
        assert_eq!(d.totals.fixtures, 46);
        let exc = d.corpora.iter().find(|c| c.corpus == "excalidraw").unwrap();
        assert_eq!(exc.kind, "application");
        assert!(d.markdown_table().contains("| Corpus |"));
    }

    #[test]
    fn honest_dashboard_merges_recall_and_holdout_fp() {
        let prod = vec![crate::production::ProductionReport {
            corpus: "rich".to_string(),
            n_fixtures: 16,
            n_caught: 11,
            uncaught: vec![],
            thresholds: BTreeMap::from([("python".to_string(), 7.8)]),
            resolved_rare: BTreeMap::new(),
            fixture_results: vec![],
        }];
        let hold = vec![HoldoutReport {
            corpus: "rich".to_string(),
            fit_sha: "a".into(),
            head_sha: "b".into(),
            window: 120,
            n_commits: 309,
            thresholds: BTreeMap::from([("python".to_string(), 7.8)]),
            existing: SplitFp {
                hunks: 391,
                hits: 11,
                rate: 11.0 / 391.0,
                ci95: Some((0.013, 0.047)),
            },
            new_file: SplitFp {
                hunks: 289,
                hits: 9,
                rate: 9.0 / 289.0,
                ci95: None,
            },
            overall: SplitFp::default(),
            existing_overfire: SplitFp {
                hunks: 391,
                hits: 2,
                rate: 2.0 / 391.0,
                ci95: None,
            },
            existing_detection: SplitFp {
                hunks: 391,
                hits: 9,
                rate: 9.0 / 391.0,
                ci95: None,
            },
            new_overfire: SplitFp::default(),
            new_detection: SplitFp::default(),
            languages: BTreeMap::new(),
            min_hunks: 300,
            under_sampled: false,
            reason_counts_existing: BTreeMap::new(),
            reason_counts_new: BTreeMap::new(),
            commits: vec![],
            hits: vec![],
        }];
        let langs = BTreeMap::from([("rich".to_string(), "python".to_string())]);
        let d = BenchDashboard::from_honest(&prod, &hold, &langs, "c".into(), "t".into());
        let rich = &d.corpora[0];
        assert_eq!(rich.caught, Some(11));
        let fpx = rich.fp_existing.as_ref().unwrap();
        assert_eq!(fpx.hits, 11);
        assert!((fpx.rate_pct - 2.813).abs() < 0.01);
        assert_eq!(fpx.ci95_pct, Some((1.3, 4.7)));
        assert!((d.totals.worst_fp_existing_pct - 2.813).abs() < 0.01);
        assert!(d.markdown_table().contains("FP existing"));
    }
}
