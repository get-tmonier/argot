//! Result persistence + the summary table printed at the end of a run.

use crate::run::CorpusReport;
use anyhow::Result;
use std::path::Path;

pub fn write_reports(results_dir: &Path, reports: &[CorpusReport]) -> Result<()> {
    std::fs::create_dir_all(results_dir)?;
    for r in reports {
        let name = r.corpus.replace([' ', '(', ')'], "");
        let path = results_dir.join(format!("{name}.json"));
        std::fs::write(&path, serde_json::to_string_pretty(r)?)?;
    }
    std::fs::write(results_dir.join("report.md"), summary_markdown(reports))?;
    Ok(())
}

pub fn summary_markdown(reports: &[CorpusReport]) -> String {
    let mut out = String::new();
    out.push_str("# argot-bench report\n\n");
    out.push_str(
        "| Corpus | Recall | FP rate | AUC | Threshold | CV | rare | Uncaught |\n\
         |:---|---:|---:|---:|---:|---:|---:|:---|\n",
    );
    let mut total_caught = 0usize;
    let mut total_fixtures = 0usize;
    for r in reports {
        total_caught += r.n_flagged_fixtures;
        total_fixtures += r.n_fixtures;
        out.push_str(&format!(
            "| {} | {}/{} ({:.1}%) | {:.2}% ({}/{}) | {:.3} | {:.4} | {:.2}% | {} | {} |\n",
            r.corpus,
            r.n_flagged_fixtures,
            r.n_fixtures,
            if r.n_fixtures > 0 {
                100.0 * r.n_flagged_fixtures as f64 / r.n_fixtures as f64
            } else {
                0.0
            },
            100.0 * r.fp_rate,
            r.n_false_positives,
            r.n_eligible_controls,
            r.auc,
            r.threshold,
            100.0 * r.threshold_cv,
            r.resolved_rare_threshold,
            if r.uncaught.is_empty() {
                "—".to_string()
            } else {
                r.uncaught.join(", ")
            },
        ));
    }
    if total_fixtures > 0 {
        out.push_str(&format!(
            "\n**Total recall: {total_caught}/{total_fixtures} ({:.1}%)**\n",
            100.0 * total_caught as f64 / total_fixtures as f64
        ));
    }
    out
}
