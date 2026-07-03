//! Per-corpus bench flow — port of the retired `argot_bench.run`.
//!
//! For each corpus: check out the primary pinned SHA, extract (or reuse) its
//! diff-hunk dataset, build one calibrated scorer per language, score the
//! catalog break fixtures (host-injection path) and every real-PR control
//! hunk, then repeat control scoring for the secondary pinned PR SHAs.

use crate::catalog::{load_catalog, read_hunk, Fixture};
use crate::metrics;
use crate::scorer::{build_scorer, parse_language, BenchKnobs, BenchScorer};
use crate::targets::Target;
use anyhow::{bail, Context, Result};
use argot_core::scoring::adapters::Language;
use argot_core::scoring::calibration::is_excluded_path;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;

const EXTRACT_LIMIT: usize = 10_000;
const QUICK_CONTROL_CAP: usize = 50;

#[derive(Debug, Clone, Serialize)]
pub struct FixtureResult {
    pub id: String,
    pub category: String,
    pub difficulty: Option<String>,
    pub import_score: f64,
    pub bpe_score: f64,
    pub flagged: bool,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ControlResult {
    pub file_path: String,
    pub hunk_start_line: usize,
    pub hunk_end_line: usize,
    pub bpe_score: f64,
    pub flagged: bool,
    pub reason: String,
}

#[derive(Debug, Serialize)]
pub struct CorpusReport {
    pub corpus: String,
    pub language: String,
    pub n_fixtures: usize,
    pub n_flagged_fixtures: usize,
    pub uncaught: Vec<String>,
    pub n_controls: usize,
    pub n_eligible_controls: usize,
    pub n_false_positives: usize,
    pub fp_rate: f64,
    pub auc: f64,
    pub threshold: f64,
    pub seed_thresholds: Vec<f64>,
    pub threshold_cv: f64,
    pub resolved_rare_threshold: usize,
    pub recall_by_category: BTreeMap<String, (usize, usize)>,
    pub stage_attribution: BTreeMap<String, usize>,
    pub fixture_results: Vec<FixtureResult>,
    /// Per-control scores; populated only with `--keep-controls` (large).
    pub control_results: Vec<ControlResult>,
}

pub struct RunOptions {
    pub data_dir: PathBuf,
    pub catalogs_dir: PathBuf,
    pub knobs: BenchKnobs,
    pub quick: bool,
    pub sample_controls: Option<usize>,
    /// Keep per-control scores in the report JSON (large; off by default).
    pub keep_control_results: bool,
}

fn git(repo_dir: &Path, args: &[&str]) -> Result<()> {
    let st = Command::new("git")
        .arg("-C")
        .arg(repo_dir)
        .args(args)
        .status()
        .with_context(|| format!("running git {args:?}"))?;
    if !st.success() {
        bail!("git {args:?} failed in {}", repo_dir.display());
    }
    Ok(())
}

/// Clone (or reuse) the corpus repo under `data_dir/<corpus>/.repo`.
pub fn ensure_clone(data_dir: &Path, corpus: &str, url: &str) -> Result<PathBuf> {
    let repo_dir = data_dir.join(corpus).join(".repo");
    if !repo_dir.join(".git").exists() {
        std::fs::create_dir_all(repo_dir.parent().unwrap())?;
        let st = Command::new("git")
            .args(["clone", "--quiet", url])
            .arg(&repo_dir)
            .status()?;
        if !st.success() {
            bail!("git clone {url} failed");
        }
    }
    Ok(repo_dir)
}

/// Install the per-corpus `.argotignore` a real user of this repo would write,
/// so the bench fits and checks each corpus the way it would actually be used
/// (e.g. redis' vendored `deps/` tree is out of the voice model). The config
/// lives at `catalogs_dir/<corpus>/argotignore`; when absent, any stale
/// `.argotignore` in the clone is removed so the corpus runs against the
/// built-in defaults. Corpus-specific knowledge stays here in the bench, never
/// in the production core.
pub fn sync_corpus_argotignore(catalogs_dir: &Path, corpus: &str, repo_dir: &Path) -> Result<()> {
    let src = catalogs_dir.join(corpus).join("argotignore");
    let dst = repo_dir.join(".argotignore");
    if src.exists() {
        std::fs::copy(&src, &dst).with_context(|| format!("installing {corpus} .argotignore"))?;
    } else if dst.exists() {
        std::fs::remove_file(&dst)?;
    }
    Ok(())
}

/// Detached-checkout `sha`, fetching once if the object is missing locally.
pub fn ensure_sha_checked_out(repo_dir: &Path, sha: &str) -> Result<()> {
    if git(repo_dir, &["checkout", "--quiet", "--detach", sha]).is_ok() {
        return Ok(());
    }
    git(repo_dir, &["fetch", "--quiet"])?;
    git(repo_dir, &["checkout", "--quiet", "--detach", sha])
}

/// Run extract on the checkout (or reuse the cached dataset).
pub fn ensure_extracted(repo_dir: &Path, out_path: &Path) -> Result<PathBuf> {
    if out_path.exists() && out_path.metadata()?.len() > 0 {
        return Ok(out_path.to_path_buf());
    }
    std::fs::create_dir_all(out_path.parent().unwrap())?;
    let tmp = out_path.with_extension("jsonl.tmp");
    let mut f = std::io::BufWriter::new(std::fs::File::create(&tmp)?);
    argot_core::extract::write_dataset(
        repo_dir.to_str().context("non-utf8 repo path")?,
        None,
        Some(EXTRACT_LIMIT),
        &mut f,
    )?;
    drop(f);
    std::fs::rename(&tmp, out_path)?;
    Ok(out_path.to_path_buf())
}

/// True for the `// Break: ...` / `# Break: ...` fixture-design meta-comments
/// that must never reach the scorer.
fn is_break_meta_line(line: &str) -> bool {
    let s = line.trim_start();
    let rest = if let Some(r) = s.strip_prefix("//") {
        r
    } else if let Some(r) = s.strip_prefix('#') {
        r
    } else {
        return false;
    };
    let rest = rest.trim_start();
    match rest.strip_prefix("Break") {
        Some(r) => r.trim_start().starts_with(':'),
        None => false,
    }
}

/// Strip break-meta lines from catalog content; returns the cleaned content
/// and the fixture's hunk bounds shifted to the cleaned line numbering.
fn strip_break_meta(content: &str, chs: usize, che: usize) -> (String, usize, usize) {
    let lines: Vec<&str> = content.lines().collect();
    let mut old_to_new: Vec<usize> = Vec::with_capacity(lines.len()); // 0 = dropped
    let mut kept: Vec<&str> = Vec::with_capacity(lines.len());
    let mut new_idx = 0usize;
    for ln in &lines {
        if is_break_meta_line(ln) {
            old_to_new.push(0);
        } else {
            new_idx += 1;
            old_to_new.push(new_idx);
            kept.push(ln);
        }
    }
    if chs < 1 || che > old_to_new.len() {
        return (content.to_string(), chs, che);
    }
    let new_chs = (chs..=che).map(|k| old_to_new[k - 1]).find(|&v| v != 0);
    let new_che = (chs..=che)
        .rev()
        .map(|k| old_to_new[k - 1])
        .find(|&v| v != 0);
    match (new_chs, new_che) {
        (Some(s), Some(e)) if s <= e => {
            let mut cleaned = kept.join("\n");
            if content.ends_with('\n') {
                cleaned.push('\n');
            }
            (cleaned, s, e)
        }
        _ => (content.to_string(), chs, che),
    }
}

/// The exact scorer inputs for one catalog fixture. When host-injection
/// metadata is present, the hunk is scored with `file_path = <host path>`
/// (correct cluster routing) and `file_source = None` (skip file-level
/// typicality + prose blanking on a synthesized file); otherwise the legacy
/// catalog-path behaviour applies. Shared by the bench and research scouts so
/// both score fixtures identically.
pub struct FixtureScoringInput {
    pub hunk: String,
    pub file_source: Option<String>,
    pub hunk_start_line: Option<usize>,
    pub hunk_end_line: Option<usize>,
    pub file_path: PathBuf,
    /// Era-14 phase D: synthesized hunk-in-host content + the hunk's
    /// 1-indexed inclusive bounds within it. Only consulted when the bare
    /// hunk's parse has root errors; never passed as `file_source`.
    pub host_context: Option<(String, usize, usize)>,
}

pub fn fixture_scoring_input(
    catalog_dir: &Path,
    fx: &Fixture,
    repo_dir: &Path,
) -> Result<FixtureScoringInput> {
    let (src, mut hunk) = read_hunk(catalog_dir, fx)?;
    let mut file_path = repo_dir.join(&fx.file);
    let mut scored_src: Option<String> = Some(src);
    let mut scored_hs = Some(fx.hunk_start_line);
    let mut scored_he = Some(fx.hunk_end_line);

    let mut host_context: Option<(String, usize, usize)> = None;
    if let (Some(host_file), Some(inject_at)) = (&fx.host_file, fx.host_inject_at_line) {
        let host_path = repo_dir.join(host_file);
        if let Ok(host_content) = std::fs::read_to_string(&host_path) {
            if let Ok(catalog_content) = std::fs::read_to_string(catalog_dir.join(&fx.file)) {
                let (cleaned, clean_hs, clean_he) =
                    strip_break_meta(&catalog_content, fx.hunk_start_line, fx.hunk_end_line);
                let cleaned_lines: Vec<&str> = cleaned.lines().collect();
                if clean_hs >= 1 && clean_hs <= clean_he && clean_he <= cleaned_lines.len() {
                    hunk = cleaned_lines[clean_hs - 1..clean_he].join("\n");
                }
                // Phase D: synthesize the hunk-in-host content the catalog
                // metadata describes; the parse-error callee fallback reads
                // the hunk's region from this spliced AST.
                let host_lines: Vec<&str> = host_content.lines().collect();
                let inject_idx = inject_at.saturating_sub(1).min(host_lines.len());
                let mut spliced: Vec<&str> =
                    Vec::with_capacity(host_lines.len() + cleaned_lines.len());
                spliced.extend_from_slice(&host_lines[..inject_idx]);
                spliced.extend_from_slice(&cleaned_lines);
                spliced.extend_from_slice(&host_lines[inject_idx..]);
                host_context = Some((
                    spliced.join("\n"),
                    inject_idx + clean_hs,
                    inject_idx + clean_he,
                ));
                file_path = host_path;
                scored_src = None;
                scored_hs = None;
                scored_he = None;
            }
        }
    }
    Ok(FixtureScoringInput {
        hunk,
        file_source: scored_src,
        hunk_start_line: scored_hs,
        hunk_end_line: scored_he,
        file_path,
        host_context,
    })
}

fn score_fixtures(
    bench: &mut BenchScorer,
    catalog_dir: &Path,
    fixtures: &[&Fixture],
    repo_dir: &Path,
) -> Result<Vec<FixtureResult>> {
    let mut out = Vec::new();
    for fx in fixtures {
        let input = fixture_scoring_input(catalog_dir, fx, repo_dir)?;
        let r = bench.scorer.score_hunk_with_host_context(
            &input.hunk,
            input.file_source.as_deref(),
            input.hunk_start_line,
            input.hunk_end_line,
            Some(&input.file_path),
            input
                .host_context
                .as_ref()
                .map(|(src, hs, he)| (src.as_str(), *hs, *he)),
        );
        out.push(FixtureResult {
            id: fx.id.clone(),
            category: fx.category.clone(),
            difficulty: fx.difficulty.clone(),
            import_score: r.stages.import_score,
            bpe_score: r.stages.bpe_score,
            flagged: r.flagged,
            reason: r.reason.as_str().to_string(),
        });
    }
    Ok(out)
}

/// Lightweight projection of one dataset record.
#[derive(serde::Deserialize)]
struct HunkRec {
    file_path: String,
    hunk_start_line: usize,
    hunk_end_line: usize,
    language: String,
}

/// Stream `dataset.jsonl`, keeping only the fields control scoring needs.
fn read_control_hunks(dataset_path: &Path) -> Result<Vec<HunkRec>> {
    use std::io::BufRead;
    let f = std::fs::File::open(dataset_path)
        .with_context(|| format!("cannot open {}", dataset_path.display()))?;
    let reader = std::io::BufReader::new(f);
    let mut out = Vec::new();
    for line in reader.lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        if let Ok(rec) = serde_json::from_str::<HunkRec>(&line) {
            out.push(rec);
        }
    }
    Ok(out)
}

/// Score real-PR control hunks against the current checkout. Excluded paths
/// short-circuit with `reason=excluded_path` (they are outside calibration
/// scope, so flagging them would not be a true FP).
fn score_real_hunks(
    bench: &mut BenchScorer,
    hunks: &[&HunkRec],
    repo_dir: &Path,
) -> Vec<ControlResult> {
    let mut out = Vec::new();
    for h in hunks {
        let file_abs = repo_dir.join(&h.file_path);
        if is_excluded_path(&file_abs, repo_dir) {
            out.push(ControlResult {
                file_path: h.file_path.clone(),
                hunk_start_line: h.hunk_start_line,
                hunk_end_line: h.hunk_end_line,
                bpe_score: 0.0,
                flagged: false,
                reason: "excluded_path".to_string(),
            });
            continue;
        }
        let Ok(file_source) = std::fs::read_to_string(&file_abs) else {
            continue;
        };
        let lines: Vec<&str> = file_source.lines().collect();
        // Python-style clamping slice: lines[hs:he].
        let s = h.hunk_start_line.min(lines.len());
        let e = h.hunk_end_line.min(lines.len());
        let hunk = lines[s..e.max(s)].join("\n");
        let r = bench.scorer.score_hunk(
            &hunk,
            Some(&file_source),
            Some(h.hunk_start_line + 1),
            Some(h.hunk_end_line),
            Some(&file_abs),
        );
        out.push(ControlResult {
            file_path: h.file_path.clone(),
            hunk_start_line: h.hunk_start_line,
            hunk_end_line: h.hunk_end_line,
            bpe_score: r.stages.bpe_score,
            flagged: r.flagged,
            reason: r.reason.as_str().to_string(),
        });
    }
    out
}

/// Deterministic control subsample via the numpy-exact sampler.
fn sample_refs<'a>(hunks: &[&'a HunkRec], n: usize, seed: u64) -> Vec<&'a HunkRec> {
    if hunks.len() <= n {
        return hunks.to_vec();
    }
    argot_core::scoring::calibration::sample_indices(hunks.len(), n, seed)
        .into_iter()
        .map(|i| hunks[i])
        .collect()
}

fn quick_fixture_subset(fixtures: &[Fixture], multi: bool) -> Vec<&Fixture> {
    let mut seen: std::collections::HashSet<(String, String)> = std::collections::HashSet::new();
    fixtures
        .iter()
        .filter(|fx| {
            let lang_key = if multi {
                fx.language.clone().unwrap_or_default()
            } else {
                String::new()
            };
            seen.insert((lang_key, fx.category.clone()))
        })
        .collect()
}

fn record_matches_language(rec: &HunkRec, lang: Language) -> bool {
    match lang {
        Language::Python => rec.language == "python",
        // JS records score against the TypeScript scorer, matching extract's
        // language routing.
        Language::Typescript => rec.language == "typescript" || rec.language == "javascript",
        Language::Go => rec.language == "go",
        Language::Rust => rec.language == "rust",
        Language::C => rec.language == "c",
        Language::Java => rec.language == "java",
        Language::CSharp => rec.language == "csharp",
        Language::Php => rec.language == "php",
        Language::Cpp => rec.language == "cpp",
        Language::Ruby => rec.language == "ruby",
    }
}

/// Run one corpus end-to-end; returns one report per language present.
pub fn run_corpus(target: &Target, opts: &RunOptions) -> Result<Vec<CorpusReport>> {
    let catalog_dir = opts.catalogs_dir.join(&target.name);
    let catalog = load_catalog(&catalog_dir)?;
    let repo_dir = ensure_clone(&opts.data_dir, &target.name, &target.url)?;
    let primary = &target.prs[0];

    let mut knobs = opts.knobs.clone();
    if opts.quick {
        knobs.n_cal = knobs.n_cal.min(20);
    }

    let languages: Vec<Language> = if catalog.language == "multi" {
        let mut langs = Vec::new();
        for cand in ["python", "typescript"] {
            if catalog
                .fixtures
                .iter()
                .any(|f| f.language.as_deref() == Some(cand))
            {
                langs.push(parse_language(cand)?);
            }
        }
        langs
    } else {
        vec![parse_language(&catalog.language)?]
    };

    ensure_sha_checked_out(&repo_dir, &primary.sha)?;
    let dataset = ensure_extracted(
        &repo_dir,
        &opts
            .data_dir
            .join(&target.name)
            .join(&primary.sha)
            .join("dataset.jsonl"),
    )?;

    let all_fixtures: Vec<&Fixture> = if opts.quick {
        quick_fixture_subset(&catalog.fixtures, catalog.language == "multi")
    } else {
        catalog.fixtures.iter().collect()
    };

    let mut reports = Vec::new();
    for &language in &languages {
        eprintln!(
            "[{}] building {:?} scorer (primary {})",
            target.name,
            language,
            &primary.sha[..8.min(primary.sha.len())]
        );
        let mut bench = build_scorer(&repo_dir, language, Some(&dataset), &knobs)?;

        // Fixtures for this language (all of them for single-language catalogs).
        let lang_name = match language {
            Language::Python => "python",
            Language::Typescript => "typescript",
            Language::Go => "go",
            Language::Rust => "rust",
            Language::C => "c",
            Language::Java => "java",
            Language::CSharp => "csharp",
            Language::Php => "php",
            Language::Cpp => "cpp",
            Language::Ruby => "ruby",
        };
        let fixtures: Vec<&Fixture> = if catalog.language == "multi" {
            all_fixtures
                .iter()
                .copied()
                .filter(|fx| fx.language.as_deref() == Some(lang_name))
                .collect()
        } else {
            all_fixtures.clone()
        };
        let fixture_results = score_fixtures(&mut bench, &catalog_dir, &fixtures, &repo_dir)?;

        // Controls: primary PR. Always language-filtered — production check
        // routes files to their language's scorer, so a Python-corpus scorer
        // never sees TS hunks (a polyglot app's dataset contains both).
        let records = read_control_hunks(&dataset)?;
        let mut refs: Vec<&HunkRec> = records
            .iter()
            .filter(|r| record_matches_language(r, language))
            .collect();
        if opts.quick {
            refs.truncate(QUICK_CONTROL_CAP);
        }
        if let Some(n) = opts.sample_controls {
            refs = sample_refs(&refs, n, knobs.seed);
        }
        let mut control_results = score_real_hunks(&mut bench, &refs, &repo_dir);

        // Controls: secondary PR snapshots (skipped in quick mode).
        if !opts.quick {
            for pin in &target.prs[1..] {
                ensure_sha_checked_out(&repo_dir, &pin.sha)?;
                let ds = ensure_extracted(
                    &repo_dir,
                    &opts
                        .data_dir
                        .join(&target.name)
                        .join(&pin.sha)
                        .join("dataset.jsonl"),
                )?;
                eprintln!(
                    "[{}] {:?} controls @ PR #{} ({})",
                    target.name,
                    language,
                    pin.pr,
                    &pin.sha[..8.min(pin.sha.len())]
                );
                let mut bench2 = build_scorer(&repo_dir, language, Some(&ds), &knobs)?;
                let records2 = read_control_hunks(&ds)?;
                let mut refs2: Vec<&HunkRec> = records2
                    .iter()
                    .filter(|r| record_matches_language(r, language))
                    .collect();
                if let Some(n) = opts.sample_controls {
                    refs2 = sample_refs(&refs2, n, knobs.seed);
                }
                control_results.extend(score_real_hunks(&mut bench2, &refs2, &repo_dir));
            }
            // Leave the repo back on the primary SHA for reproducibility.
            ensure_sha_checked_out(&repo_dir, &primary.sha)?;
        }

        if let Some(counts) = bench.scorer.primitive_fire_counts() {
            for (name, n) in counts {
                eprintln!("[{}] primitive fire count: {name}={n}", target.name);
            }
        }
        let corpus_name = if catalog.language == "multi" {
            format!("{} ({lang_name})", target.name)
        } else {
            target.name.clone()
        };
        reports.push(make_report(
            corpus_name,
            lang_name.to_string(),
            &bench,
            fixture_results,
            control_results,
            opts.keep_control_results,
        ));
    }
    Ok(reports)
}

fn make_report(
    corpus: String,
    language: String,
    bench: &BenchScorer,
    fixture_results: Vec<FixtureResult>,
    control_results: Vec<ControlResult>,
    keep_controls: bool,
) -> CorpusReport {
    let eligible: Vec<&ControlResult> = control_results
        .iter()
        .filter(|c| !metrics::is_excluded_reason(&c.reason))
        .collect();
    let n_fp = eligible.iter().filter(|c| c.flagged).count();
    let fp_rate = if eligible.is_empty() {
        0.0
    } else {
        n_fp as f64 / eligible.len() as f64
    };
    let break_scores: Vec<f64> = fixture_results.iter().map(|f| f.bpe_score).collect();
    let ctrl_scores: Vec<f64> = eligible.iter().map(|c| c.bpe_score).collect();
    let auc = metrics::auc(&break_scores, &ctrl_scores);

    let mut recall_by_category: BTreeMap<String, (usize, usize)> = BTreeMap::new();
    for f in &fixture_results {
        let e = recall_by_category
            .entry(f.category.clone())
            .or_insert((0, 0));
        e.1 += 1;
        if f.flagged {
            e.0 += 1;
        }
    }
    let mut stage_attribution: BTreeMap<String, usize> = BTreeMap::new();
    for f in &fixture_results {
        *stage_attribution.entry(f.reason.clone()).or_insert(0) += 1;
    }
    let n_controls = control_results.len();
    let n_eligible_controls = eligible.len();
    drop(eligible);

    CorpusReport {
        corpus,
        language,
        n_fixtures: fixture_results.len(),
        n_flagged_fixtures: fixture_results.iter().filter(|f| f.flagged).count(),
        uncaught: fixture_results
            .iter()
            .filter(|f| !f.flagged)
            .map(|f| f.id.clone())
            .collect(),
        n_controls,
        n_eligible_controls,
        n_false_positives: n_fp,
        fp_rate,
        auc,
        threshold: bench.threshold,
        seed_thresholds: bench.seed_thresholds.clone(),
        threshold_cv: metrics::threshold_cv(&bench.seed_thresholds),
        resolved_rare_threshold: bench.resolved_rare_threshold,
        recall_by_category,
        stage_attribution,
        fixture_results,
        control_results: if keep_controls {
            control_results
        } else {
            Vec::new()
        },
    }
}
