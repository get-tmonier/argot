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
//! Resolves internal imports across the supported languages (see `corpus_lang`);
//! a corpus with no resolver (or a polyglot `multi` repo) produces an empty graph
//! and is skipped — a graceful no-op.

use std::path::Path;

use anyhow::{Context, Result};
use argot_core::config::ArgotConfig;
use argot_core::scoring::adapters::Language;
use argot_core::scoring::arch_graph::RepoLayering;
use argot_core::train::{collect_source_files, is_corpus_source};
use serde::Serialize;

use crate::holdout::{ensure_full_history, plan_holdout};
use crate::production::git_stdout;
use crate::run::{ensure_clone, ensure_sha_checked_out, sync_corpus_config};
use crate::targets::Target;

const DEFAULT_WINDOW: usize = 150;

/// The corpus's primary language, mapped to a resolver. `None` = no resolver yet
/// (skip the corpus) or a polyglot `multi` repo.
fn corpus_lang(target: &Target) -> Option<Language> {
    Some(match target.language.as_str() {
        "python" => Language::Python,
        "go" => Language::Go,
        "typescript" => Language::Typescript,
        "javascript" => Language::Javascript,
        "rust" => Language::Rust,
        "java" => Language::Java,
        "php" => Language::Php,
        "csharp" => Language::CSharp,
        "ruby" => Language::Ruby,
        "c" => Language::C,
        "cpp" => Language::Cpp,
        _ => return None,
    })
}

/// Source-file extensions for a language, plus manifest files fit needs for
/// context (go.mod for Go's module path).
fn lang_files(language: Language) -> (&'static [&'static str], &'static [&'static str]) {
    match language {
        Language::Python => (&["py"], &[]),
        Language::Go => (&["go"], &["go.mod"]),
        Language::Typescript => (&["ts", "tsx"], &[]),
        Language::Javascript => (&["js", "jsx", "mjs", "cjs"], &[]),
        Language::Rust => (&["rs"], &[]),
        Language::Java => (&["java"], &[]),
        Language::Php => (&["php"], &[]),
        Language::CSharp => (&["cs"], &[]),
        Language::Ruby => (&["rb"], &[]),
        Language::C => (&["c", "h"], &[]),
        Language::Cpp => (&["cc", "cpp", "cxx", "hpp", "hh", "h"], &[]),
    }
}

fn has_ext(path: &str, exts: &[&str]) -> bool {
    path.rsplit('.').next().is_some_and(|e| exts.contains(&e))
}

fn show(repo: &Path, sha: &str, path: &str) -> String {
    git_stdout(repo, &["show", &format!("{sha}:{path}")]).unwrap_or_default()
}

#[derive(Serialize)]
struct ArchResult {
    corpus: String,
    language: String,
    layers: usize,
    /// Layers that map to a real source file — the authorable-violation space.
    host_layers: usize,
    edges: usize,
    commits: usize,
    fires: usize,
    over_fire: f64,
    catch: f64,
    /// Authored-fixture recall: violations caught / valid, control fires / total.
    violations: usize,
    v_caught: usize,
    v_invalid: usize,
    controls: usize,
    c_fired: usize,
}

/// Authored architectural-violation fixtures (`arch_violations.yaml`): a real
/// import line added to a real host file, scored through the actual resolver.
#[derive(serde::Deserialize, Default)]
struct ArchFixtures {
    #[serde(default)]
    violations: Vec<ArchFix>,
    #[serde(default)]
    controls: Vec<ArchFix>,
}

#[derive(serde::Deserialize)]
struct ArchFix {
    host_file: String,
    import_line: String,
}

/// Outcome of scoring one fixture through the real resolver.
enum FixOutcome {
    /// The edge fires (reversal/near-sink) — a violation caught.
    Fired,
    /// The edge is 0-usage but not a reversal/sink (forward-cross) — a real miss.
    NovelMissed,
    /// The edge already exists in the repo — the fixture is NOT 0-usage (invalid),
    /// e.g. the source layer already imports the target via a relative import the
    /// author's absolute grep missed. Excluded from recall.
    Attested,
    /// The import didn't resolve to an internal cross-layer edge (bad fixture).
    NoEdge,
}

fn fixture_outcome(graph: &RepoLayering, fix: &ArchFix) -> FixOutcome {
    let edges = graph.file_edges(&fix.host_file, &fix.import_line);
    if edges.is_empty() {
        return FixOutcome::NoEdge;
    }
    // fires if ANY edge fires; else if ANY edge is attested → Attested; else missed.
    if edges.iter().any(|e| graph.classify(e).is_some()) {
        FixOutcome::Fired
    } else if edges.iter().any(|e| graph.contains_edge(e)) {
        FixOutcome::Attested
    } else {
        FixOutcome::NovelMissed
    }
}

/// Fit a `language` layering graph from a SHA's tree, using the SAME voice-file
/// collection production uses (`train::collect_source_files` — honors
/// EXCLUDE_DIRS, `.argotignore`, `[exclude].paths`) plus the manifest files the
/// resolver needs (go.mod). No hardcoded scaffolding paths.
fn graph_at(repo: &Path, sha: &str, language: Language) -> Result<RepoLayering> {
    ensure_sha_checked_out(repo, sha)?;
    let (exts, manifests) = lang_files(language);
    let mut sources: Vec<(String, String)> = Vec::new();
    for abs in &collect_source_files(repo) {
        if let Ok(rel) = abs.strip_prefix(repo) {
            let rel = rel.to_string_lossy().replace('\\', "/");
            if has_ext(&rel, exts) {
                if let Ok(src) = std::fs::read_to_string(abs) {
                    sources.push((rel, src));
                }
            }
        }
    }
    // Manifest files (go.mod) live outside the source-extension set — read them
    // from disk directly so the resolver can derive the module path.
    for m in manifests {
        if let Ok(src) = std::fs::read_to_string(repo.join(m)) {
            sources.push((m.to_string(), src));
        }
    }
    Ok(RepoLayering::fit(
        sources.iter().map(|(p, s)| (p.as_str(), s.as_str())),
        language,
    ))
}

fn run_corpus(target: &Target, data_dir: &Path, catalogs_dir: &Path) -> Result<Option<ArchResult>> {
    let Some(language) = corpus_lang(target) else {
        return Ok(None); // no resolver for this language yet
    };
    let (exts, _) = lang_files(language);
    let repo = ensure_clone(data_dir, &target.name, &target.url)?;
    let head = target.prs[0].sha.clone();
    ensure_full_history(&repo)?;
    // Install the per-corpus argot.toml (mute system) so exclusions come from
    // config, never hardcoded — exactly how the base bench fits each corpus.
    sync_corpus_config(catalogs_dir, &target.name, &repo)?;
    let suppressions = ArgotConfig::load(&repo).path_suppressions();
    // Catch is measured on the HEAD graph — production fits on the CURRENT repo,
    // not the holdout fit-SHA (an old, thin tree that understates coverage).
    let head_graph = graph_at(&repo, &head, language)?;
    // Host-mapped layers = layers a real HEAD file lives in (the authorable-violation
    // space). MUST be computed now, while the working tree is still checked out at
    // HEAD — the holdout `graph_at(fit_sha)` below moves the tree to an older sha.
    let host_layers: std::collections::HashSet<String> = collect_source_files(&repo)
        .iter()
        .filter_map(|abs| abs.strip_prefix(&repo).ok())
        .map(|rel| rel.to_string_lossy().replace('\\', "/"))
        .filter(|rel| has_ext(rel, exts))
        .filter_map(|rel| head_graph.layer_of(&rel))
        .collect();
    let window = target.holdout_window.unwrap_or(DEFAULT_WINDOW);
    let (fit_sha, replay) = plan_holdout(&repo, &head, window)?;
    // FP is measured on the holdout graph (fit @ HEAD~window) — a real temporal split.
    let graph = graph_at(&repo, &fit_sha, language)?;
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
            if !has_ext(path, exts) || !is_corpus_source(path, &suppressions) {
                continue;
            }
            let cur = show(&repo, sha, path);
            if cur.is_empty() {
                continue;
            }
            let parent = show(&repo, &format!("{sha}~1"), path);
            let cur_e = graph.file_edges(path, &cur);
            let par_e = graph.file_edges(path, &parent);
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

    // catch (popularity-weighted coverage) — on the HEAD graph (production reality).
    // Restrict the SOURCE layer `a` to layers that actually map to a real source
    // file (a hunk can only introduce a violation FROM a file that exists). Without
    // this, the estimate counts (a→b) pairs whose `a` is a target-only namespace
    // layer no file lives in — inflating catch where layer ≠ directory (notably C#,
    // where every source can collapse to one bucket). `host_layers` records how many
    // layers are authorable so the render can flag a thin real-violation space.
    let layers: Vec<String> = head_graph.layers().into_iter().collect();
    let mut num = 0f64;
    let mut den = 0f64;
    for a in &layers {
        if !host_layers.contains(a) {
            continue; // no source file lives in this layer — not authorable
        }
        for b in &layers {
            if a == b {
                continue;
            }
            let mass = head_graph.in_mass(b);
            if mass == 0 || head_graph.contains_edge(&(a.clone(), b.clone())) {
                continue;
            }
            den += mass as f64;
            if head_graph.classify(&(a.clone(), b.clone())).is_some() {
                num += mass as f64;
            }
        }
    }
    let catch = if den > 0.0 { 100.0 * num / den } else { 0.0 };

    // Authored-fixture recall (real recall, not the coverage estimate): score each
    // violation/control through the resolver on the HEAD graph.
    let fx_path = catalogs_dir.join(&target.name).join("arch_violations.yaml");
    let fx: ArchFixtures = std::fs::read_to_string(&fx_path)
        .ok()
        .and_then(|s| serde_yaml::from_str(&s).ok())
        .unwrap_or_default();
    let (mut v_caught, mut v_missed, mut v_invalid) = (0usize, 0usize, 0usize);
    for f in &fx.violations {
        match fixture_outcome(&head_graph, f) {
            FixOutcome::Fired => v_caught += 1,
            FixOutcome::NovelMissed => {
                v_missed += 1;
                eprintln!("  [{}] MISS (novel forward): {}", target.name, f.host_file);
            }
            FixOutcome::Attested => {
                v_invalid += 1;
                eprintln!(
                    "  [{}] INVALID (edge already attested — not 0-usage): {} :: {}",
                    target.name, f.host_file, f.import_line
                );
            }
            FixOutcome::NoEdge => {
                v_invalid += 1;
                eprintln!(
                    "  [{}] INVALID (no internal edge): {}",
                    target.name, f.host_file
                );
            }
        }
    }
    let c_fired = fx
        .controls
        .iter()
        .filter(|f| matches!(fixture_outcome(&head_graph, f), FixOutcome::Fired))
        .count();
    let _ = v_missed;

    Ok(Some(ArchResult {
        corpus: target.name.clone(),
        language: target.language.clone(),
        layers: layers.len(),
        // Host-mapped layers that also participate in the edge graph — the
        // authorable AND connected space (a file-only layer with no edges can't be
        // a violation source). Intersecting with `layers` keeps the ratio ≤ 1.
        host_layers: layers.iter().filter(|l| host_layers.contains(*l)).count(),
        edges: head_graph.edge_count(),
        commits,
        fires,
        over_fire,
        catch,
        violations: fx.violations.len(),
        v_caught,
        v_invalid,
        controls: fx.controls.len(),
        c_fired,
    }))
}

pub fn run_arch(
    targets: &[Target],
    data_dir: &Path,
    catalogs_dir: &Path,
    results_dir: &Path,
) -> Result<String> {
    let mut results: Vec<ArchResult> = Vec::new();
    for t in targets {
        match run_corpus(t, data_dir, catalogs_dir) {
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

/// `--mode arch-candidates`: for each corpus, dump the resolver's GENUINE 0-usage
/// reversal/sink candidate edges (with a sample host file per source layer), so
/// fixtures target edges the resolver actually sees as 0-usage — closing the gap
/// where text-`git grep` misses relative/grouped imports the resolver counts.
pub fn run_arch_candidates(
    targets: &[Target],
    data_dir: &Path,
    catalogs_dir: &Path,
    results_dir: &Path,
) -> Result<String> {
    use argot_core::scoring::arch_graph::Violation;
    use std::collections::HashMap;
    std::fs::create_dir_all(results_dir).ok();
    let mut summary =
        String::from("# Architecture-graph — resolver-grounded 0-usage candidates\n\n");
    for t in targets {
        let Some(language) = corpus_lang(t) else {
            continue;
        };
        let repo = ensure_clone(data_dir, &t.name, &t.url)?;
        let head = t.prs[0].sha.clone();
        ensure_full_history(&repo)?;
        sync_corpus_config(catalogs_dir, &t.name, &repo)?;
        let graph = graph_at(&repo, &head, language)?;
        if graph.edge_count() == 0 {
            continue;
        }
        // sample host file per source layer (first seen)
        let (exts, _) = lang_files(language);
        let mut host: HashMap<String, String> = HashMap::new();
        for abs in collect_source_files(&repo) {
            if let Ok(rel) = abs.strip_prefix(&repo) {
                let rel = rel.to_string_lossy().replace('\\', "/");
                if has_ext(&rel, exts) {
                    if let Some(l) = graph.layer_of(&rel) {
                        host.entry(l).or_insert(rel);
                    }
                }
            }
        }
        // Enumerate 0-usage cross-layer candidates whose SOURCE layer has a real
        // host file, with a resolver-VERIFIED import line (so a fixture built from
        // a row is valid by construction — no text-grep gap). Both catchable
        // (reversal/transitive/sink) and `forward` (not-a-violation) pools are
        // emitted: forward rows author controls + realistic non-catchable misses,
        // keeping any authored recall non-circular.
        let layers: Vec<String> = graph.layers().into_iter().collect();
        // (kind, src, tgt, popularity, host_file, verified import line)
        let mut cands: Vec<(&'static str, &String, &String, u32, String, String)> = Vec::new();
        for a in &layers {
            let Some(host_file) = host.get(a) else {
                continue; // no source file to host the added import
            };
            for b in &layers {
                if a == b || graph.contains_edge(&(a.clone(), b.clone())) {
                    continue;
                }
                if graph.in_mass(b) == 0 {
                    continue; // unpopular target — not a realistic violation
                }
                // `__root__` (files directly under the package root) is not a nameable
                // import target for prefix-import languages — `from pkg.__root__ import`
                // is bogus even though it resolves. Skip it as a target there.
                if b == "__root__"
                    && !matches!(
                        language,
                        Language::Typescript
                            | Language::Javascript
                            | Language::Ruby
                            | Language::C
                            | Language::Cpp
                    )
                {
                    continue;
                }
                let kind = match graph.classify(&(a.clone(), b.clone())) {
                    Some(Violation::Reversal) => "reversal",
                    Some(Violation::TransitiveReversal) => "transitive_reversal",
                    Some(Violation::SinkOut) => "sink_out",
                    None => "forward",
                };
                let Some(import_line) =
                    graph.example_import(host_file, b, host.get(b).map(|s| s.as_str()))
                else {
                    continue; // can't synthesize a resolvable import
                };
                cands.push((kind, a, b, graph.in_mass(b), host_file.clone(), import_line));
            }
        }
        cands.sort_by_key(|c| std::cmp::Reverse(c.3));
        let host_layers = layers.iter().filter(|l| host.contains_key(*l)).count();
        let n_catch = cands.iter().filter(|c| c.0 != "forward").count();
        let n_fwd = cands.len() - n_catch;
        let mut out = format!(
            "# {} ({}) — resolver-verified 0-usage fixture candidates\n\n\
             {} layers · {} host-mapped · {} catchable + {} forward candidates \
             (each import line VERIFIED to create the src→tgt edge)\n\n",
            t.name,
            t.language,
            layers.len(),
            host_layers,
            n_catch,
            n_fwd
        );
        out.push_str(
            "| kind | src | tgt | pop | host_file | import_line |\n|---|---|---|--:|---|---|\n",
        );
        for (kind, a, b, pop, hf, line) in cands.iter().take(150) {
            out.push_str(&format!(
                "| {} | {} | {} | {} | `{}` | `{}` |\n",
                kind,
                a,
                b,
                pop,
                hf,
                line.replace('\n', "\\n")
            ));
        }
        std::fs::write(results_dir.join(format!("candidates-{}.md", t.name)), &out).ok();
        summary.push_str(&format!(
            "- {}: {} catchable + {} forward verified candidates · {}/{} layers host-mapped\n",
            t.name,
            n_catch,
            n_fwd,
            host_layers,
            layers.len()
        ));
    }
    std::fs::write(results_dir.join("candidates.md"), &summary).ok();
    Ok(summary)
}

fn render(results: &[ArchResult]) -> String {
    let mut s = String::new();
    s.push_str("# Architecture-graph foreignness — real-infra validation\n\n");
    s.push_str("Fire = a hunk introduces a reversal/(near-)sink-out internal edge.\n");
    s.push_str(
        "over-fire = real clean commits firing (fit @ HEAD~window); catch = \
         popularity-weighted coverage over the HEAD graph, restricted to SOURCE \
         layers with a real file (authorable violations only).\n",
    );
    s.push_str(
        "`layers` = host-mapped / total; a low ratio means the resolver splits \
         source and target vocabularies (layer ≠ directory) — a thin authorable \
         space and an unreliable catch.\n\n",
    );
    s.push_str(
        "| corpus | lang | layers | edges | commits | over-fire | catch(cov) | \
         **recall(fixtures)** | ctrl-FP |\n",
    );
    s.push_str("|---|---|--:|--:|--:|--:|--:|--:|--:|\n");
    let (mut of_sum, mut catch_sum, mut fires_sum, mut commits_sum) = (0.0, 0.0, 0usize, 0usize);
    let mut worst_of = 0.0f64;
    let (mut vc, mut vt, mut cf, mut ct) = (0usize, 0usize, 0usize, 0usize);
    for r in results {
        let valid = r.violations.saturating_sub(r.v_invalid);
        let recall = if valid > 0 {
            format!(
                "{}/{} = {:.0}%{}",
                r.v_caught,
                valid,
                100.0 * r.v_caught as f64 / valid as f64,
                if r.v_invalid > 0 {
                    format!(" ({}inv)", r.v_invalid)
                } else {
                    String::new()
                }
            )
        } else {
            "—".to_string()
        };
        let ctrl = if r.controls > 0 {
            format!("{}/{}", r.c_fired, r.controls)
        } else {
            "—".to_string()
        };
        s.push_str(&format!(
            "| {} | {} | {}/{} | {} | {} | {:.1}% | {:.0}% | {} | {} |\n",
            r.corpus,
            r.language,
            r.host_layers,
            r.layers,
            r.edges,
            r.commits,
            r.over_fire,
            r.catch,
            recall,
            ctrl
        ));
        of_sum += r.over_fire;
        catch_sum += r.catch;
        fires_sum += r.fires;
        commits_sum += r.commits;
        worst_of = worst_of.max(r.over_fire);
        vc += r.v_caught;
        vt += r.violations.saturating_sub(r.v_invalid); // valid violations only
        cf += r.c_fired;
        ct += r.controls;
    }
    let n = results.len().max(1) as f64;
    let agg_of = if commits_sum > 0 {
        100.0 * fires_sum as f64 / commits_sum as f64
    } else {
        0.0
    };
    s.push_str(&format!(
        "\n**MEAN** over-fire {:.1}% · catch(cov) {:.0}%  |  **holdout** {}/{} commits = {:.2}% (worst {:.1}%)\n",
        of_sum / n,
        catch_sum / n,
        fires_sum,
        commits_sum,
        agg_of,
        worst_of,
    ));
    if vt > 0 {
        s.push_str(&format!(
            "**FIXTURE recall** {}/{} = {:.0}% caught · **control-FP** {}/{} = {:.1}%  (real recall on authored violations)\n",
            vc, vt, 100.0 * vc as f64 / vt as f64, cf, ct, 100.0 * cf as f64 / ct.max(1) as f64,
        ));
    }
    s.push_str(
        "\n(gatable ⇒ over-fire ≤5% every corpus. catch(cov) = popularity-weighted coverage \
         estimate; recall(fixtures) = real recall on authored 0-usage-verified violations.)\n",
    );
    s
}
