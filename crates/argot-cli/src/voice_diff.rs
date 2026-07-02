//! `argot voice-diff <target>` — a single PR-level out-of-voice number plus a
//! ranked hot-spots list. Pure aggregation over the per-hunk scores `check`
//! already produces (consumed via its stable `--format json` contract), so it
//! adds no new modeling.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

use serde::Serialize;
use serde_json::Value;

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;

/// One scored, above-threshold hunk reduced to what the aggregator needs.
pub struct HitScore {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub score: f64,
    pub severity: String,
}

/// Additive-smoothing denominator: pulls tiny diffs toward 0% so a lone
/// anomalous hunk in a 5-line PR doesn't read 100% out of voice. Large diffs are
/// barely affected (10/105 ≈ 10%).
const SMOOTHING: f64 = 5.0;

/// Default hot-spots shown.
pub const DEFAULT_TOP: usize = 10;

#[derive(Serialize, Clone)]
pub struct HotSpot {
    pub file: String,
    pub line_start: usize,
    pub line_end: usize,
    pub score: f64,
    pub severity: String,
}

#[derive(Serialize)]
pub struct VoiceDiffSummary {
    /// Smoothed proportion of scored hunks above threshold, as a percent.
    pub out_of_voice_pct: f64,
    pub hunks_total: usize,
    pub hunks_flagged: usize,
    /// Strongest tier that fired (`foreign`/`suspicious`/`unusual`/`none`).
    pub max_severity: String,
    /// Highest-scoring hunks first.
    pub hot_spots: Vec<HotSpot>,
}

/// Aggregate scored hits into the PR-level summary. Pure: the CLI just feeds it
/// what `check` produced.
pub fn summarize(hits: &[HitScore], hunks_total: usize, top_n: usize) -> VoiceDiffSummary {
    let flagged = hits.len();
    let denom = hunks_total as f64 + SMOOTHING;
    let out_of_voice_pct = if denom > 0.0 {
        (flagged as f64 / denom) * 100.0
    } else {
        0.0
    };
    let max_severity = ["foreign", "suspicious", "unusual"]
        .into_iter()
        .find(|t| hits.iter().any(|h| h.severity == *t))
        .unwrap_or("none")
        .to_string();
    let mut sorted: Vec<&HitScore> = hits.iter().collect();
    sorted.sort_by(|a, b| {
        b.score
            .partial_cmp(&a.score)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.file.cmp(&b.file))
            .then_with(|| a.line_start.cmp(&b.line_start))
    });
    let hot_spots = sorted
        .into_iter()
        .take(top_n)
        .map(|h| HotSpot {
            file: h.file.clone(),
            line_start: h.line_start,
            line_end: h.line_end,
            score: h.score,
            severity: h.severity.clone(),
        })
        .collect();
    VoiceDiffSummary {
        out_of_voice_pct,
        hunks_total,
        hunks_flagged: flagged,
        max_severity,
        hot_spots,
    }
}

/// Compute the summary for a ref/range by running `check` in JSON mode and
/// aggregating its hits. `None` when the model can't be loaded (check errored).
pub fn summary_for_ref(repo: &Path, reference: &str, top_n: usize) -> Option<VoiceDiffSummary> {
    let outcome = run_check(CheckArgs {
        repo_path: repo.to_string_lossy().into_owned(),
        reference: reference.to_string(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: repo.join(".argot"),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_severity: "unusual".to_string(),
        use_color: false,
        format: OutputFormat::Json,
        today: crate::today_utc(),
    });
    if outcome.exit_code >= 2 {
        return None;
    }
    let doc: Value = serde_json::from_str(&outcome.stdout).ok()?;
    let hunks_total = doc
        .get("hunks_scanned")
        .and_then(Value::as_u64)
        .unwrap_or(0) as usize;
    let hits: Vec<HitScore> = doc
        .get("hits")
        .and_then(Value::as_array)
        .map(|arr| {
            arr.iter()
                .map(|h| HitScore {
                    file: h
                        .get("path")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                    line_start: h.get("line_start").and_then(Value::as_u64).unwrap_or(0) as usize,
                    line_end: h.get("line_end").and_then(Value::as_u64).unwrap_or(0) as usize,
                    score: h.get("score").and_then(Value::as_f64).unwrap_or(0.0),
                    severity: h
                        .get("severity")
                        .and_then(Value::as_str)
                        .unwrap_or("")
                        .to_string(),
                })
                .collect()
        })
        .unwrap_or_default();
    Some(summarize(&hits, hunks_total, top_n))
}

/// One-line headline for the human render + `argot review` header.
pub fn one_liner(s: &VoiceDiffSummary) -> String {
    format!(
        "voice-diff · {:.0}% out of voice ({}/{} hunks · max {})",
        s.out_of_voice_pct, s.hunks_flagged, s.hunks_total, s.max_severity
    )
}

pub fn run_voice_diff(target: &str, repo: PathBuf, format: &str, top_n: usize) -> ExitCode {
    let Some(summary) = summary_for_ref(&repo, target, top_n) else {
        eprintln!("error: could not score '{target}' — run `argot fit` first?");
        return ExitCode::from(2);
    };
    if format == "json" {
        println!(
            "{}",
            serde_json::to_string_pretty(&summary).unwrap_or_default()
        );
        return ExitCode::SUCCESS;
    }
    println!("{}", one_liner(&summary));
    if summary.hot_spots.is_empty() {
        println!("  no hot spots — this diff sounds like the repo.");
    } else {
        println!("  hot spots:");
        for h in &summary.hot_spots {
            let loc = if h.line_start == h.line_end {
                format!("L{}", h.line_start)
            } else {
                format!("L{}-L{}", h.line_start, h.line_end)
            };
            println!(
                "    {:>6.2}  {:<10}  {}:{}",
                h.score, h.severity, h.file, loc
            );
        }
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(file: &str, line: usize, score: f64, sev: &str) -> HitScore {
        HitScore {
            file: file.to_string(),
            line_start: line,
            line_end: line,
            score,
            severity: sev.to_string(),
        }
    }

    #[test]
    fn empty_diff_is_zero_percent_and_no_hot_spots() {
        let s = summarize(&[], 12, 10);
        assert_eq!(s.out_of_voice_pct, 0.0);
        assert_eq!(s.max_severity, "none");
        assert!(s.hot_spots.is_empty());
    }

    #[test]
    fn smoothing_keeps_a_lone_hunk_in_a_tiny_diff_off_one_hundred_percent() {
        // 1 flagged of 1 total would be 100% unsmoothed; smoothing pulls it down.
        let s = summarize(&[hit("a.py", 1, 9.0, "foreign")], 1, 10);
        assert!(s.out_of_voice_pct < 50.0, "got {}", s.out_of_voice_pct);
        assert_eq!(s.hunks_flagged, 1);
    }

    #[test]
    fn large_diffs_are_barely_smoothed() {
        // 10 of 100 → ~9.5%, close to the naive 10%.
        let hits: Vec<HitScore> = (0..10).map(|i| hit("a.py", i, 6.0, "unusual")).collect();
        let s = summarize(&hits, 100, 10);
        assert!(
            (s.out_of_voice_pct - 9.5).abs() < 0.6,
            "got {}",
            s.out_of_voice_pct
        );
    }

    #[test]
    fn max_severity_is_the_strongest_tier_present() {
        let hits = vec![
            hit("a.py", 1, 5.0, "unusual"),
            hit("b.py", 2, 6.0, "foreign"),
            hit("c.py", 3, 5.5, "suspicious"),
        ];
        assert_eq!(summarize(&hits, 20, 10).max_severity, "foreign");
    }

    #[test]
    fn hot_spots_are_ranked_by_score_and_capped() {
        let hits = vec![
            hit("a.py", 1, 5.0, "unusual"),
            hit("b.py", 2, 9.0, "foreign"),
            hit("c.py", 3, 7.0, "suspicious"),
        ];
        let s = summarize(&hits, 20, 2);
        assert_eq!(s.hot_spots.len(), 2);
        assert_eq!(s.hot_spots[0].file, "b.py"); // highest score first
        assert_eq!(s.hot_spots[1].file, "c.py");
    }
}
