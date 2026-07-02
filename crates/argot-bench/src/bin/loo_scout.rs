//! Dirty research scout (#92): does leave-one-file-out (LOO) calibration
//! close the honest-FP gap?
//!
//! Hypothesis: the calibrated threshold is train-on-test — calibration hunks
//! come from corpus files the BPE token counts memorized, so their scores are
//! deflated and genuinely-unseen-but-idiomatic code lands above the max. The
//! BPE model is a unigram count table, so exact LOO is just "subtract the
//! held-out file's token counts" — no retraining.
//!
//! BPE-only approximation (the FP flood is 85%+ reason=bpe): for one corpus,
//! compute the standard vs LOO bpe-only thresholds over the same calibration
//! sample, replay the holdout commits, and report FP(existing/new) under
//! both. Break fixtures (when a catalog exists) give the recall cost.

use anyhow::{bail, Context, Result};
use argot_core::bpe::BpeTokenizer;
use argot_core::git_walk::walk_commits;
use argot_core::scoring::adapters::{
    c::CAdapter, cpp::CppAdapter, csharp::CSharpAdapter, go::GoAdapter, java::JavaAdapter,
    php::PhpAdapter, python::PythonAdapter, ruby::RubyAdapter, rust::RustAdapter,
    typescript::TypeScriptAdapter, LanguageAdapter,
};
use argot_core::scoring::bpe_scorer::BpeScorer;
use argot_core::scoring::calibration::{collect_candidates, sample_indices};
use argot_core::text::splitlines;
use std::collections::{HashMap, HashSet};
use std::ops::ControlFlow;
use std::path::{Path, PathBuf};
use std::process::Command;

fn adapter_for(lang: &str) -> Result<Box<dyn LanguageAdapter>> {
    Ok(match lang {
        "python" => Box::new(PythonAdapter::new()),
        "typescript" => Box::new(TypeScriptAdapter::new()),
        "go" => Box::new(GoAdapter::new()),
        "rust" => Box::new(RustAdapter::new()),
        "c" => Box::new(CAdapter::new()),
        "java" => Box::new(JavaAdapter::new()),
        "csharp" => Box::new(CSharpAdapter::new()),
        "php" => Box::new(PhpAdapter::new()),
        "cpp" => Box::new(CppAdapter::new()),
        "ruby" => Box::new(RubyAdapter::new()),
        other => bail!("unknown language {other}"),
    })
}

fn exts_for(lang: &str) -> &'static [&'static str] {
    match lang {
        "python" => &[".py"],
        "typescript" => &[".ts", ".tsx"],
        "go" => &[".go"],
        "rust" => &[".rs"],
        "c" => &[".c", ".h"],
        "java" => &[".java"],
        "csharp" => &[".cs"],
        "php" => &[".php"],
        "cpp" => &[".cpp", ".cc", ".hpp", ".cxx"],
        "ruby" => &[".rb"],
        _ => &[],
    }
}

fn git_out(repo: &Path, args: &[&str]) -> Result<String> {
    let out = Command::new("git").arg("-C").arg(repo).args(args).output()?;
    if !out.status.success() {
        bail!("git {args:?} failed: {}", String::from_utf8_lossy(&out.stderr));
    }
    Ok(String::from_utf8_lossy(&out.stdout).into_owned())
}

fn blank_prose(adapter: &dyn LanguageAdapter, hunk: &str) -> String {
    let ranges = adapter.prose_line_ranges(hunk);
    splitlines(hunk)
        .iter()
        .enumerate()
        .map(|(i, l)| if ranges.contains(&(i + 1)) { "" } else { *l })
        .collect::<Vec<_>>()
        .join("\n")
}

/// BPE score with per-file LOO subtraction: token surprise against
/// (repo_counts − held-out file counts).
struct LooBpe {
    tokenizer: BpeTokenizer,
    generic: HashMap<u32, u64>,
    total_generic: f64,
    repo: HashMap<u32, u64>,
    total_repo: u64,
    id_to_token: HashMap<u32, String>,
}

const EPSILON: f64 = 1e-7;

impl LooBpe {
    fn score(&self, hunk: &str, holdout: Option<&HashMap<u32, u64>>) -> f64 {
        let (sub, sub_total): (Option<&HashMap<u32, u64>>, u64) = match holdout {
            Some(h) => (Some(h), h.values().sum()),
            None => (None, 0),
        };
        let total_repo = (self.total_repo.saturating_sub(sub_total)).max(1) as f64;
        let ids = self.tokenizer.encode(hunk);
        let meaningful: Vec<u32> = ids
            .iter()
            .copied()
            .filter(|i| {
                self.id_to_token
                    .get(i)
                    .map(|s| argot_core::scoring::bpe_scorer::is_meaningful_token(s))
                    .unwrap_or(false)
            })
            .collect();
        let use_ids: &[u32] = if meaningful.is_empty() { &ids } else { &meaningful };
        if use_ids.is_empty() {
            return 0.0;
        }
        use_ids
            .iter()
            .map(|&i| {
                let g = *self.generic.get(&i).unwrap_or(&0) as f64;
                let mut r = *self.repo.get(&i).unwrap_or(&0);
                if let Some(s) = sub {
                    r = r.saturating_sub(*s.get(&i).unwrap_or(&0));
                }
                (g / self.total_generic + EPSILON).ln() - (r as f64 / total_repo + EPSILON).ln()
            })
            .fold(f64::NEG_INFINITY, f64::max)
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let n = v.len();
    if n == 0 {
        return 0.0;
    }
    if n % 2 == 1 {
        v[n / 2]
    } else {
        (v[n / 2 - 1] + v[n / 2]) / 2.0
    }
}

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    let mut repo = None;
    let mut fit_sha = None;
    let mut head_sha = None;
    let mut lang = None;
    let mut catalog: Option<PathBuf> = None;
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--repo" => repo = Some(PathBuf::from(&args[i + 1])),
            "--fit-sha" => fit_sha = Some(args[i + 1].clone()),
            "--head-sha" => head_sha = Some(args[i + 1].clone()),
            "--language" => lang = Some(args[i + 1].clone()),
            "--catalog" => catalog = Some(PathBuf::from(&args[i + 1])),
            other => bail!("unknown arg {other}"),
        }
        i += 2;
    }
    let repo = repo.context("--repo")?;
    let fit_sha = fit_sha.context("--fit-sha")?;
    let head_sha = head_sha.context("--head-sha")?;
    let lang = lang.context("--language")?;
    let adapter = adapter_for(&lang)?;
    let exts = exts_for(&lang);

    // --- Fit-tree corpus at the fit SHA.
    git_out(&repo, &["checkout", "--quiet", "--detach", &fit_sha])?;
    let candidates = collect_candidates(&repo, adapter.as_ref());
    eprintln!("candidates: {}", candidates.len());

    // Corpus sources exactly as train sees them.
    let sources: Vec<(PathBuf, String)> = argot_core::train::collect_source_files(&repo)
        .into_iter()
        .filter(|p| {
            let name = p.to_string_lossy().to_lowercase();
            exts.iter().any(|e| name.ends_with(e))
        })
        .filter_map(|p| {
            argot_core::text::read_text_lossy(&p)
                .ok()
                .map(|s| (p, s))
        })
        .collect();
    eprintln!("corpus files: {}", sources.len());

    let tokenizer = BpeTokenizer::load();
    let mut repo_counts: HashMap<u32, u64> = HashMap::new();
    let mut per_file: HashMap<PathBuf, HashMap<u32, u64>> = HashMap::new();
    for (p, s) in &sources {
        let mut f: HashMap<u32, u64> = HashMap::new();
        for id in tokenizer.encode(s) {
            *f.entry(id).or_insert(0) += 1;
            *repo_counts.entry(id).or_insert(0) += 1;
        }
        per_file.insert(p.clone(), f);
    }
    let total_repo: u64 = repo_counts.values().sum();

    // Generic baseline via a throwaway BpeScorer (parse the embedded JSON).
    let base_scorer = BpeScorer::new(
        BpeTokenizer::load(),
        argot_core::train::GENERIC_BASELINE_JSON,
        &[],
    )?;
    let _ = base_scorer; // parsed independently below
    #[derive(serde::Deserialize)]
    struct GB {
        token_counts: HashMap<String, u64>,
        total_tokens: u64,
    }
    let gb: GB = serde_json::from_slice(argot_core::train::GENERIC_BASELINE_JSON)?;
    let generic: HashMap<u32, u64> = gb
        .token_counts
        .iter()
        .map(|(k, v)| (k.parse::<u32>().unwrap(), *v))
        .collect();
    let loo = LooBpe {
        id_to_token: tokenizer.vocab().into_iter().map(|(k, v)| (v, k)).collect(),
        tokenizer,
        generic,
        total_generic: gb.total_tokens as f64,
        repo: repo_counts,
        total_repo,
    };

    // --- Thresholds over the same calibration samples, standard vs LOO.
    let n_cal = 100.min(candidates.len());
    let mut std_thresholds = Vec::new();
    let mut loo_thresholds = Vec::new();
    for seed in 0u64..7 {
        let idx = sample_indices(candidates.len(), n_cal, seed);
        let mut std_max = f64::NEG_INFINITY;
        let mut loo_max = f64::NEG_INFINITY;
        for &i in &idx {
            let c = &candidates[i];
            let blanked = blank_prose(adapter.as_ref(), &c.hunk);
            std_max = std_max.max(loo.score(&blanked, None));
            loo_max = loo_max.max(loo.score(&blanked, per_file.get(&c.file_path)));
        }
        std_thresholds.push(std_max);
        loo_thresholds.push(loo_max);
    }
    let t_std = median(std_thresholds.clone());
    let t_loo = median(loo_thresholds.clone());
    println!("bpe-only threshold standard: {t_std:.3}  (per-seed {std_thresholds:?})");
    println!("bpe-only threshold LOO:      {t_loo:.3}  (per-seed {loo_thresholds:?})");

    // --- Replay the holdout commits, bpe-only, both thresholds.
    let fit_files: HashSet<String> = git_out(&repo, &["ls-tree", "-r", "--name-only", &fit_sha])?
        .lines()
        .map(String::from)
        .collect();
    let replay: HashSet<String> = git_out(
        &repo,
        &["rev-list", "--no-merges", &format!("{fit_sha}..{head_sha}")],
    )?
    .lines()
    .map(String::from)
    .collect();
    git_out(&repo, &["checkout", "--quiet", "--detach", &head_sha])?;

    // (hunks, fp_std, fp_loo) per class
    let mut existing = (0usize, 0usize, 0usize);
    let mut newfile = (0usize, 0usize, 0usize);
    walk_commits(repo.to_str().unwrap(), &replay, |item| {
        let name = item.file_path.to_lowercase();
        if !exts.iter().any(|e| name.ends_with(e)) {
            return Ok(ControlFlow::Continue(()));
        }
        let src = String::from_utf8_lossy(&item.post_blob).into_owned();
        let lines = splitlines(&src);
        let is_new = !fit_files.contains(&item.file_path);
        for h in &item.hunks {
            let s = (h.new_start as usize).saturating_sub(1);
            let e = (s + h.new_lines as usize).min(lines.len());
            if s >= e {
                continue;
            }
            let hunk = lines[s..e].join("\n");
            let score = loo.score(&blank_prose(adapter.as_ref(), &hunk), None);
            let slot = if is_new { &mut newfile } else { &mut existing };
            slot.0 += 1;
            if score >= t_std {
                slot.1 += 1;
            }
            if score >= t_loo {
                slot.2 += 1;
            }
        }
        Ok(ControlFlow::Continue(()))
    })?;
    let pct = |k: usize, n: usize| 100.0 * k as f64 / n.max(1) as f64;
    println!(
        "FP existing: std {}/{} ({:.2}%) → LOO {}/{} ({:.2}%)",
        existing.1,
        existing.0,
        pct(existing.1, existing.0),
        existing.2,
        existing.0,
        pct(existing.2, existing.0)
    );
    println!(
        "FP new-file: std {}/{} ({:.2}%) → LOO {}/{} ({:.2}%)",
        newfile.1,
        newfile.0,
        pct(newfile.1, newfile.0),
        newfile.2,
        newfile.0,
        pct(newfile.2, newfile.0)
    );

    // --- Break fixtures (bpe-only recall proxy).
    if let Some(cat) = catalog {
        let catalog = argot_bench::catalog::load_catalog(&cat)?;
        let mut fired_std = 0usize;
        let mut fired_loo = 0usize;
        let mut n = 0usize;
        let mut scores: Vec<(String, f64)> = Vec::new();
        for fx in &catalog.fixtures {
            if let Some(l) = &fx.language {
                if l != &lang {
                    continue;
                }
            }
            let (_, hunk) = argot_bench::catalog::read_hunk(&cat, fx)?;
            let score = loo.score(&blank_prose(adapter.as_ref(), &hunk), None);
            n += 1;
            if score >= t_std {
                fired_std += 1;
            }
            if score >= t_loo {
                fired_loo += 1;
            }
            scores.push((fx.id.clone(), score));
        }
        println!("breaks (bpe-only): std {fired_std}/{n} → LOO {fired_loo}/{n}");
        scores.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap());
        for (id, s) in scores.iter().take(8) {
            println!("  lowest: {s:.2} {id}");
        }
    }
    Ok(())
}
