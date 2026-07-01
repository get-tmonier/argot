//! Era-14 Phase A scout (dirty; delete after evidence is recorded).
//!
//! Dumps per-callee contribution events with corpus-global document
//! frequencies, on (a) ~1000 real-PR diff hunks per corpus (the calibration /
//! FP side) and (b) every catalog fixture (the recall side). The df
//! distributions decide the rarity-weighting formula for Phase A.
//!
//! Usage: scout_idf <corpus>[,<corpus>...] [--out DIR]

use anyhow::{Context, Result};
use argot_bench::catalog::load_catalog;
use argot_bench::run::{ensure_clone, ensure_extracted, ensure_sha_checked_out, fixture_scoring_input};
use argot_bench::scorer::{adapter_for, load_diff_hunks_for_probe, parse_language, source_files};
use argot_bench::targets::load_targets;
use argot_core::scoring::call_receiver::{CallReceiverScorer, ContributionBranch};
use argot_core::scoring::typicality::TypicalityModel;
use argot_core::text::read_text_lossy;
use serde::Serialize;
use std::path::{Path, PathBuf};

#[derive(Serialize)]
struct Event {
    callee: String,
    branch: &'static str,
    df: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    fixture: Option<String>,
}

#[derive(Serialize)]
struct ScoutOut {
    corpus: String,
    language: String,
    n_corpus_files: usize,
    n_cal_hunks_scored: usize,
    cal_events: Vec<Event>,
    fixture_events: Vec<Event>,
}

fn branch_name(b: ContributionBranch) -> &'static str {
    match b {
        ContributionBranch::UnattestedKnownRoot => "unattested_known_root",
        ContributionBranch::Unattested => "unattested",
        ContributionBranch::ClusterAbsent => "cluster_absent",
        ContributionBranch::ClusterRare => "cluster_rare",
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let corpora: Vec<String> = args
        .first()
        .context("usage: scout_idf <corpus>[,..] [--out DIR]")?
        .split(',')
        .map(str::to_string)
        .collect();
    let out_dir = args
        .iter()
        .position(|a| a == "--out")
        .and_then(|i| args.get(i + 1))
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(".scratch/era-14/scout-a"));
    std::fs::create_dir_all(&out_dir)?;

    let targets = load_targets(Path::new("benchmarks/targets.yaml"))?;
    for name in &corpora {
        let target = targets
            .iter()
            .find(|t| &t.name == name)
            .with_context(|| format!("unknown corpus {name}"))?;
        let catalog_dir = PathBuf::from("benchmarks/catalogs").join(name);
        let catalog = load_catalog(&catalog_dir)?;
        let data_dir = PathBuf::from("benchmarks/data");
        let repo = ensure_clone(&data_dir, name, &target.url)?;
        let primary = &target.prs[0];
        ensure_sha_checked_out(&repo, &primary.sha)?;
        let dataset = ensure_extracted(
            &repo,
            &data_dir.join(name).join(&primary.sha).join("dataset.jsonl"),
        )?;

        let languages: Vec<&str> = if catalog.language == "multi" {
            vec!["python", "typescript"]
        } else {
            vec![catalog.language.as_str()]
        };
        for lang_name in languages {
            let language = parse_language(lang_name)?;
            let adapter = adapter_for(language);
            let files = source_files(&repo, language);
            let repo_files: Vec<(PathBuf, String)> = files
                .iter()
                .filter_map(|p| read_text_lossy(p).ok().map(|s| (p.clone(), s)))
                .collect();
            // rare=2, clusters=8: era-13.5 rule enabled so ClusterRare events appear.
            let cr = CallReceiverScorer::new(
                &repo_files,
                language,
                2.0,
                5,
                adapter.as_ref(),
                8,
                0,
                2,
                0,
            )
            .map_err(anyhow::Error::msg)?;
            let typicality = TypicalityModel::new(language);

            let probe = load_diff_hunks_for_probe(&dataset, &repo, 1000, 0);
            let mut cal_events = Vec::new();
            let mut scored = 0usize;
            for (hunk, path, source) in &probe {
                if typicality.is_atypical(hunk).0 {
                    continue;
                }
                scored += 1;
                for ev in cr.contribution_events_for_file(hunk, Some(path), Some(source)) {
                    cal_events.push(Event {
                        df: cr.callee_file_count(&ev.callee),
                        branch: branch_name(ev.branch),
                        callee: ev.callee,
                        fixture: None,
                    });
                }
            }

            let mut fixture_events = Vec::new();
            for fx in &catalog.fixtures {
                if catalog.language == "multi" && fx.language.as_deref() != Some(lang_name) {
                    continue;
                }
                let input = fixture_scoring_input(&catalog_dir, fx, &repo)?;
                for ev in cr.contribution_events_for_file(
                    &input.hunk,
                    Some(&input.file_path),
                    input.file_source.as_deref(),
                ) {
                    fixture_events.push(Event {
                        df: cr.callee_file_count(&ev.callee),
                        branch: branch_name(ev.branch),
                        callee: ev.callee,
                        fixture: Some(fx.id.clone()),
                    });
                }
            }

            let out = ScoutOut {
                corpus: name.clone(),
                language: lang_name.to_string(),
                n_corpus_files: cr.n_corpus_files(),
                n_cal_hunks_scored: scored,
                cal_events,
                fixture_events,
            };
            let path = out_dir.join(format!("{name}-{lang_name}.json"));
            std::fs::write(&path, serde_json::to_string_pretty(&out)?)?;
            eprintln!("scout → {}", path.display());
        }
    }
    Ok(())
}
