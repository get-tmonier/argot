//! Architecture-graph floor/gate validation (`--mode arch`, feature-gated).
//!
//! Drives argot-core's feature-gated `arch_graph` sense over real corpora with a
//! REAL temporal holdout — the productization of the cheap probe in
//! `benchmarks/arch_graph_temporal.py`, now through the real Rust module.
//! Self-contained and NON-GATING: reuses the clone/holdout helpers, never touches
//! the base metric or `dashboard.json`; base guardrail byte-for-byte unchanged.
//!
//! Per corpus:
//!   fit the layer-dependency graph at `HEAD~window`, then
//!   - **over-fire** = share of post-fit clean commits that introduce a
//!     reversal/sink-out edge vs the fit graph (attributed per commit: file edges
//!     at `sha` minus at `sha^`). The honest false-alarm rate.
//!   - **catch** = popularity-weighted coverage: of cross-layer edges the repo
//!     does NOT have (target imported by someone), the share the reversal/sink
//!     rule flags if an LLM created it — via the real `classify`.
//!
//! v1 resolves Python imports (the validated corpus set); non-Python corpora
//! produce an empty graph and are skipped — a graceful no-op.

use std::path::Path;

use anyhow::{Context, Result};
use argot_core::scoring::adapters::Language;
use argot_core::scoring::arch_graph::RepoLayering;
use serde::Serialize;

use crate::holdout::{ensure_full_history, fit_tree_files, plan_holdout};
use crate::production::git_stdout;
use crate::run::{ensure_clone, ensure_sha_checked_out};
use crate::targets::Target;

const DEFAULT_WINDOW: usize = 150;

fn ext_lang(path: &str) -> Option<Language> {
    match path.rsplit('.').next()? {
        "py" => Some(Language::Python),
        _ => None, // v1: Python resolver only
    }
}

fn is_src(path: &str) -> bool {
    ext_lang(path).is_some()
        && !path.contains("/test")
        && !path.contains("test_")
        && !path.contains("/migrations/")
}

fn show(repo: &Path, sha: &str, path: &str) -> String {
    git_stdout(repo, &["show", &format!("{sha}:{path}")]).unwrap_or_default()
}

#[derive(Serialize)]
struct ArchResult {
    corpus: String,
    language: String,
    layers: usize,
    edges: usize,
    commits: usize,
    fires: usize,
    over_fire: f64,
    catch: f64,
}

fn run_corpus(target: &Target, data_dir: &Path) -> Result<Option<ArchResult>> {
    let repo = ensure_clone(data_dir, &target.name, &target.url)?;
    let head = target.prs[0].sha.clone();
    ensure_full_history(&repo)?;
    ensure_sha_checked_out(&repo, &head)?;
    let window = target.holdout_window.unwrap_or(DEFAULT_WINDOW);
    let (fit_sha, replay) = plan_holdout(&repo, &head, window)?;

    // Fit the layering graph from the fit-SHA source tree.
    ensure_sha_checked_out(&repo, &fit_sha)?;
    let files = fit_tree_files(&repo, &fit_sha)?;
    let mut sources: Vec<(String, String)> = Vec::new(); // (path, content)
    for path in &files {
        if is_src(path) {
            if let Ok(src) = std::fs::read_to_string(repo.join(path)) {
                sources.push((path.clone(), src));
            }
        }
    }
    let graph = RepoLayering::fit(
        sources
            .iter()
            .map(|(p, s)| (p.as_str(), s.as_str(), Language::Python)),
    );
    if graph.edge_count() == 0 {
        eprintln!("[{}] arch: no internal edges — skipped", target.name);
        return Ok(None);
    }
    eprintln!(
        "[{}] arch: fit @ {} — {} layers, {} edges, replaying {} commits",
        target.name,
        &fit_sha[..8],
        graph.layers().len(),
        graph.edge_count(),
        replay.len()
    );

    // over-fire: per-commit added edges that are reversal/sink-out.
    let mut fires = 0usize;
    for sha in &replay {
        let changed = git_stdout(
            &repo,
            &["diff-tree", "--no-commit-id", "--name-only", "-r", sha],
        )
        .unwrap_or_default();
        let mut fired = false;
        for path in changed.lines() {
            if !is_src(path) {
                continue;
            }
            let cur = show(&repo, sha, path);
            if cur.is_empty() {
                continue;
            }
            let parent = show(&repo, &format!("{sha}~1"), path);
            let cur_e = graph.file_edges(path, &cur, Language::Python);
            let par_e = graph.file_edges(path, &parent, Language::Python);
            for e in cur_e.difference(&par_e) {
                if graph.classify(e).is_some() {
                    fired = true;
                }
            }
        }
        if fired {
            fires += 1;
        }
    }
    let commits = replay.len();
    let over_fire = if commits > 0 {
        100.0 * fires as f64 / commits as f64
    } else {
        0.0
    };

    // catch (popularity-weighted coverage over plausible missing edges).
    let layers: Vec<String> = graph.layers().into_iter().collect();
    let mut num = 0f64;
    let mut den = 0f64;
    for a in &layers {
        for b in &layers {
            if a == b {
                continue;
            }
            let mass = graph.in_mass(b);
            if mass == 0 || graph.contains_edge(&(a.clone(), b.clone())) {
                continue;
            }
            den += mass as f64;
            if graph.classify(&(a.clone(), b.clone())).is_some() {
                num += mass as f64;
            }
        }
    }
    let catch = if den > 0.0 { 100.0 * num / den } else { 0.0 };

    Ok(Some(ArchResult {
        corpus: target.name.clone(),
        language: target.language.clone(),
        layers: graph.layers().len(),
        edges: graph.edge_count(),
        commits,
        fires,
        over_fire,
        catch,
    }))
}

pub fn run_arch(targets: &[Target], data_dir: &Path, results_dir: &Path) -> Result<String> {
    let mut results: Vec<ArchResult> = Vec::new();
    for t in targets {
        match run_corpus(t, data_dir) {
            Ok(Some(r)) => results.push(r),
            Ok(None) => {}
            Err(e) => eprintln!("[{}] arch: skipped ({e:#})", t.name),
        }
    }
    let md = render(&results);
    std::fs::create_dir_all(results_dir).ok();
    std::fs::write(
        results_dir.join("arch.json"),
        serde_json::to_string_pretty(&results)?,
    )
    .context("writing arch.json")?;
    std::fs::write(results_dir.join("arch.md"), &md).ok();
    Ok(md)
}

fn render(results: &[ArchResult]) -> String {
    let mut s = String::new();
    s.push_str("# Architecture-graph foreignness — real-infra validation\n\n");
    s.push_str("Fire = a hunk introduces a reversal/sink-out internal edge vs the fit graph.\n");
    s.push_str("over-fire = real clean commits firing; catch = popularity-weighted coverage.\n\n");
    s.push_str("| corpus | lang | layers | edges | commits | fires | over-fire | catch |\n");
    s.push_str("|---|---|--:|--:|--:|--:|--:|--:|\n");
    let (mut of_sum, mut catch_sum, mut fires_sum, mut commits_sum) = (0.0, 0.0, 0usize, 0usize);
    let mut worst_of = 0.0f64;
    for r in results {
        s.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {:.1}% | {:.0}% |\n",
            r.corpus, r.language, r.layers, r.edges, r.commits, r.fires, r.over_fire, r.catch
        ));
        of_sum += r.over_fire;
        catch_sum += r.catch;
        fires_sum += r.fires;
        commits_sum += r.commits;
        worst_of = worst_of.max(r.over_fire);
    }
    let n = results.len().max(1) as f64;
    let agg_of = if commits_sum > 0 {
        100.0 * fires_sum as f64 / commits_sum as f64
    } else {
        0.0
    };
    s.push_str(&format!(
        "\n**MEAN** over-fire {:.1}% · catch {:.0}%  |  **aggregate** {}/{} commits = {:.2}% · **worst** {:.1}%\n",
        of_sum / n,
        catch_sum / n,
        fires_sum,
        commits_sum,
        agg_of,
        worst_of,
    ));
    s.push_str(
        "\n(gatable ⇒ over-fire ≤5% on every corpus. catch is a coverage estimate, \
         not injected-fixture recall.)\n",
    );
    s
}
