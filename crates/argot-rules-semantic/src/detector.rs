//! The semantic pass (`--features semantic`): F1 reinvention + F2 placement
//! findings from the per-repo embedding index (`.argot/semantic-index.json`),
//! plus F4 nearest-code evidence. Group `semantic`.

use argot_engine::check::render::{paint, C_DIM};
use argot_engine::check::PatchBatch;
use argot_engine::config::DetectConfig;
use argot_engine::detector::{CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::rules;
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::adapters::LanguageAdapter;
use argot_lang::ext::{ext_to_lang, ext_to_lang_ctx, extension};
use std::collections::{HashMap, HashSet};
use std::path::Path;

#[cfg(test)]
mod tests;

/// The nearest-existing-code evidence attached to a semantic finding (F4). Held
/// as structured data so every output format renders it its own way.
#[derive(Debug, Clone)]
pub(super) enum SemanticHitEvidence {
    /// F1 reinvention: the existing function this one duplicates.
    Redundant {
        nearest_symbol: String,
        nearest_path: String,
        nearest_line: usize,
        similarity: f32,
    },
    /// F2 placement: the area this function looks like it belongs in.
    Misplaced {
        neighbor_area: String,
        actual_area: String,
        /// Nearest peers (symbol, path:line) that voted for `neighbor_area`.
        peers: Vec<(String, String, usize)>,
    },
}
/// The semantic group's detection pass. Skipped whole when both semantic
/// rules are off: no index load, no model download, no cost.
///
/// Fit side: the index build observes the calibration loop's own corpus
/// reads through [`Detector::fit_begin`] (embedder + prior artifact + cache
/// acquisition), [`Detector::fit_language`] (embed one language's corpus,
/// incremental vector reuse), and [`Detector::fit`] (artifact write) — one
/// corpus read, per-language diagnostics kept in order, and no dependency
/// from the base calibration onto this slice.
pub struct SemanticDetector {
    embedder: Option<crate::embedder::Embedder>,
    embed_cache: Option<crate::embed_cache::EmbedCache>,
    prior_artifact: Option<crate::index::SemanticArtifact>,
    /// The artifact under construction (`Some` once `fit_begin` ran).
    artifact: Option<crate::index::SemanticArtifact>,
}

impl Default for SemanticDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl SemanticDetector {
    pub fn new() -> Self {
        SemanticDetector {
            embedder: None,
            embed_cache: None,
            prior_artifact: None,
            artifact: None,
        }
    }
}

impl Detector for SemanticDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_SEMANTIC
    }

    fn timing_label(&self) -> &'static str {
        "check: semantic pass"
    }

    /// Acquire the embedder once for the whole fit (lazy model load /
    /// one-time fetch), the machine-wide embed cache, and the previous fit's
    /// index for incremental vector reuse. `None` embedder (offline, load
    /// failure, or the group off in `[rules]`) degrades to no-semantic-index
    /// — loudly, never silently.
    fn fit_begin(&mut self, ctx: &argot_engine::detector::FitContext<'_>) {
        self.artifact = Some(crate::index::SemanticArtifact::new(
            ctx.repo_sha.to_string(),
        ));
        self.embedder = if !ctx.settings.group_enabled(rules::GROUP_SEMANTIC) {
            // Both semantic rules are off in [rules]: no model, no index, no cost.
            eprintln!("argot: semantic rules are off in [rules] — skipping the semantic index");
            None
        } else {
            let _t = argot_engine::timing::phase("calibrate: embedder load");
            match crate::embedder::Embedder::ready() {
                Ok(e) => e,
                Err(e) => {
                    // Degrade to no-semantic-index, but NEVER silently: a load failure
                    // (GPU memory pressure, corrupt model) must be visible, or a bench
                    // records "0 recall" where the truth is "no embedder".
                    eprintln!(
                        "argot: semantic model failed to load — skipping semantic index: {e:#}"
                    );
                    None
                }
            }
        };
        // The machine-wide embed cache: a fresh clone or audit worktree of an
        // already-seen repo reuses vectors across checkouts, not just within one.
        self.embed_cache = if self.embedder.is_some() {
            let _t = argot_engine::timing::phase("calibrate: embed-cache load");
            crate::embed_cache::EmbedCache::open_current()
        } else {
            None
        };
        // The previous fit's index, when it exists and was built by THIS model:
        // unchanged functions reuse their vectors (incremental refresh), so a
        // routine refit embeds only what actually changed. A stale/other-model
        // artifact yields no reuse — full re-embed, never mixed spaces.
        self.prior_artifact = {
            let _t = argot_engine::timing::phase("calibrate: semantic prior-artifact load");
            std::fs::read_to_string(ctx.output.with_file_name(crate::SEMANTIC_INDEX_FILE))
                .ok()
                .and_then(|raw| crate::index::SemanticArtifact::from_json_str(&raw).ok())
                .filter(|a| a.validate_current().is_ok())
        };
    }

    /// Semantic index for this language: embed every corpus function (the
    /// same `callable_bodies` extraction check uses on the diff). Skipped
    /// silently when no embedder is available — never load-bearing.
    fn fit_language(&mut self, lang: &argot_engine::detector::FitLanguageContext<'_>) {
        let Some(emb) = self.embedder.as_ref() else {
            return;
        };
        let name = lang.language;
        let t_ext = argot_engine::timing::phase(format!("calibrate[{name}]: semantic fn-extract"));
        let mut funcs = Vec::new();
        for (path, source) in lang.files {
            // Mirror the check-time candidate scope (`batch.ignored_by_pattern`):
            // never index code the check would skip — tests, docs, examples,
            // scripts, migrations, generated (`argot:recommended` + `[exclude]`).
            // A reinvention target must be *canonical* repo code, not a tutorial
            // or fixture; indexing those both mis-points findings and inflates
            // self-similarity on duplication-heavy example trees.
            if lang.suppressions.is_suppressed_abs(path, lang.repo_dir) {
                continue;
            }
            let rel = argot_engine::corpus::rel_to_repo(path, lang.repo_dir);
            funcs.extend(crate::index::functions_in_file(lang.adapter, &rel, source));
        }
        t_ext.done();
        if funcs.is_empty() {
            return;
        }
        let t_prior =
            argot_engine::timing::phase(format!("calibrate[{name}]: semantic prior-index load"));
        let prior_index = self
            .prior_artifact
            .as_ref()
            .and_then(|a| a.load(name).ok().flatten())
            .map(|li| li.index);
        t_prior.done();
        eprintln!(
            "argot: building semantic index for {name} ({} functions)…",
            funcs.len()
        );
        let mut t_embed = argot_engine::timing::phase(String::new());
        match crate::index::SemanticIndex::build_with_reuse(
            emb,
            &funcs,
            prior_index.as_ref(),
            self.embed_cache.as_ref(),
        ) {
            Ok((idx, reused)) if !idx.is_empty() => {
                t_embed.relabel(format!(
                    "calibrate[{name}]: semantic embed ({} fns, {} prior-reused, {} cache-reused)",
                    funcs.len(),
                    reused.from_prior,
                    reused.from_cache
                ));
                t_embed.done();
                if reused.total() > 0 {
                    eprintln!(
                        "argot: {name}: reused {} of {} embeddings ({} from the previous fit, {} from the machine cache)",
                        reused.total(),
                        funcs.len(),
                        reused.from_prior,
                        reused.from_cache
                    );
                }
                // Self-calibrate the placement sense on the fresh index
                // (adaptive areas, entangled merges, vote parameters —
                // or disabled when the repo's areas aren't separable).
                let t_plc =
                    argot_engine::timing::phase(format!("calibrate[{name}]: placement calibrate"));
                let placement = crate::placement::calibrate_placement(&idx);
                t_plc.done();
                // Self-calibrate the reinvention sense: mini-replay of
                // the functions added over the recent window against the
                // older code (conservative mode for repos practicing
                // systematic parallel implementation).
                let t_rec = argot_engine::timing::phase(format!(
                    "calibrate[{name}]: reinvention calibrate (recent-fns replay)"
                ));
                let recent = recent_semantic_functions(
                    lang.repo_dir,
                    &funcs,
                    lang.adapter,
                    SEMANTIC_CALIBRATION_WINDOW,
                );
                let reinvention = crate::redundant::calibrate_reinvention(&idx, &recent);
                t_rec.done();
                let t_ins =
                    argot_engine::timing::phase(format!("calibrate[{name}]: semantic serialize"));
                self.artifact.as_mut().expect("fit_begin ran").insert(
                    name,
                    &idx,
                    placement,
                    reinvention,
                );
                t_ins.done();
            }
            Ok(_) => {}
            // `{e:#}` prints anyhow's full cause chain — the inner
            // llama.cpp/tokenizer detail, not just the outer context.
            Err(e) => eprintln!("argot: semantic index for {name} failed: {e:#}"),
        }
    }

    /// Semantic index artifact, alongside scorer-config.json. Kept in its own
    /// file so scorer-config.json (and its model hash) stay byte-for-byte
    /// unchanged whether or not the semantic layer is compiled in.
    fn fit(&mut self, ctx: &argot_engine::detector::FitContext<'_>) {
        let Some(artifact) = self.artifact.as_ref() else {
            return;
        };
        if artifact.is_empty() {
            return;
        }
        let _t = argot_engine::timing::phase("calibrate: semantic artifact write");
        let sem_path = ctx.output.with_file_name(crate::SEMANTIC_INDEX_FILE);
        match artifact.to_json_string() {
            Ok(sem_json) => {
                if let Err(e) = argot_engine::artifact::write_atomic(&sem_path, sem_json.as_bytes())
                {
                    eprintln!("argot: writing semantic index failed: {e}");
                }
            }
            Err(e) => eprintln!("argot: serializing semantic index failed: {e}"),
        }
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        semantic_hits(
            ctx.batches,
            &ctx.args.argot_dir,
            ctx.filter_adapters,
            ctx.mute_rules,
            ctx.detect,
            ctx.header_cpp,
            ctx.stderr,
        )
    }
}
/// The semantic pass (F1 reinvention, F2 placement) — additive `Finding`s from
/// the per-repo embedding index. It runs *alongside* the
/// statistical scorers, never through them: it reads `.argot/semantic-index.json`
/// plus the embedder, finds the functions the diff *defines*, and flags any that
/// reinvent existing code. Returns extra hits to merge into the report. Empty
/// (a clean graceful degrade) when the index or model is unavailable, so the
/// base guardrail is entirely unaffected.
fn semantic_hits(
    patches: &[PatchBatch],
    argot_dir: &Path,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    detect: &DetectConfig,
    header_cpp: bool,
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::embedder::Embedder;
    use crate::index::{functions_in_file, FunctionRef, LoadedIndex, SemanticArtifact};
    use crate::placement::PlacementScorer;
    use crate::redundant::RedundantScorer;
    use crate::SEMANTIC_INDEX_FILE;

    // Load the fit-time index artifact; its absence just means no semantic layer.
    let t_art = argot_engine::timing::phase("check: semantic artifact read+parse");
    let Ok(raw) = std::fs::read_to_string(argot_dir.join(SEMANTIC_INDEX_FILE)) else {
        return Vec::new();
    };
    let artifact = match SemanticArtifact::from_json_str(&raw) {
        Ok(a) => a,
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic index unreadable: {e}\n"));
            return Vec::new();
        }
    };
    t_art.done();
    // A stale index (older format, different embedding model) must never be
    // queried — its cosines would be silently wrong. Loud skip + rebuild hint.
    if let Err(reason) = artifact.validate_current() {
        stderr.push_str(&format!(
            "[argot] semantic index {reason} — run `argot fit` to rebuild; \
             redundant/misplaced checks skipped this run\n"
        ));
        return Vec::new();
    }

    // Gather the functions this diff defines: a function whose definition line is
    // among the diff's added lines is newly added (its whole body, incl. the def,
    // is in an added hunk) — the reinvention candidates.
    let t_cand = argot_engine::timing::phase("check: semantic candidate extract");
    let mut candidates: Vec<(usize, &'static str, FunctionRef)> = Vec::new();
    for (bi, batch) in patches.iter().enumerate() {
        if batch.ignored_by_pattern {
            continue;
        }
        let ext = extension(&batch.file_path);
        let Some(lang) = ext_to_lang_ctx(&ext, header_cpp) else {
            continue;
        };
        let Some(adapter) = filter_adapters.get(lang) else {
            continue;
        };
        let source = String::from_utf8_lossy(&batch.content);
        // Mirror the index scope (calibration's `filtered`): a data-dominant or
        // auto-generated file (unicode tables, transpiled output, generated stubs)
        // is not authored voice — its functions are neither reinvention candidates
        // nor placement candidates. Skips the F2 over-fire clean-commit measurement
        // caught on generated data modules (e.g. rich/_unicode_data).
        if adapter.is_data_dominant(&source, detect.data_threshold)
            || adapter.is_auto_generated(&source, &detect.generated_markers)
        {
            continue;
        }
        let mut added: HashSet<usize> = HashSet::new();
        for h in &batch.hunks {
            for l in h.new_start..(h.new_start + h.new_lines) {
                added.insert(l as usize);
            }
        }
        for f in functions_in_file(adapter.as_ref(), &batch.file_path, &source) {
            if added.contains(&f.line) {
                candidates.push((bi, lang, f));
            }
        }
    }
    if candidates.is_empty() {
        return Vec::new();
    }
    t_cand.done();

    // Load only the indices we actually need.
    let t_idx = argot_engine::timing::phase("check: semantic index decode");
    let mut loaded: HashMap<&'static str, LoadedIndex> = HashMap::new();
    for (_, lang, _) in &candidates {
        if loaded.contains_key(lang) {
            continue;
        }
        match artifact.load(lang) {
            Ok(Some(li)) => {
                loaded.insert(lang, li);
            }
            Ok(None) => {}
            Err(e) => stderr.push_str(&format!("[argot] semantic index for {lang}: {e}\n")),
        }
    }
    candidates.retain(|(_, lang, _)| loaded.contains_key(lang));
    if candidates.is_empty() {
        return Vec::new();
    }
    t_idx.done();

    // Acquire the embedder once; unavailable model → degrade (no semantic hits).
    let t_model = argot_engine::timing::phase("check: semantic embedder load");
    let embedder = match Embedder::ready() {
        Ok(Some(e)) => e,
        Ok(None) => {
            stderr.push_str(
                "[argot] semantic model unavailable — redundant/misplaced checks skipped this run\n",
            );
            return Vec::new();
        }
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic model load failed: {e}\n"));
            return Vec::new();
        }
    };

    t_model.done();

    // Embed all candidate functions in one batch, serving any the machine-wide
    // cache already holds (e.g. functions a fit of this repo indexed at HEAD).
    let t_embed =
        argot_engine::timing::phase(format!("check: semantic embed ({} fns)", candidates.len()));
    let embed_cache = crate::embed_cache::EmbedCache::open_current();
    let texts: Vec<&str> = candidates.iter().map(|(_, _, f)| f.text.as_str()).collect();
    let vecs = match crate::embed_cache::embed_with_cache(&embedder, &texts, embed_cache.as_ref()) {
        Ok(v) => v,
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic embedding failed: {e}\n"));
            return Vec::new();
        }
    };
    t_embed.done();
    let _t_score = argot_engine::timing::phase("check: semantic score candidates");

    // Dev-only feature capture (`ARGOT_SEM_DUMP=<path>`): append one JSON line
    // per candidate — its structural features, nearest neighbours and the fire
    // outcome — so bench sweeps can re-evaluate rule variants offline against a
    // saved index copy without re-running fit/check. Inert without the env var.
    let dump_path = std::env::var_os("ARGOT_SEM_DUMP");
    let mut dump_lines: Vec<String> = Vec::new();

    // Scorer construction is per-language, never per-candidate:
    // `RedundantScorer::new` builds corpus-wide IDF/DF tables over the whole
    // index — rebuilt for every candidate it dominated the check phase
    // (~35 ms × every diff-defined function on a 25k-entry index).
    let scorers: HashMap<&'static str, (RedundantScorer, PlacementScorer)> = loaded
        .iter()
        .map(|(lang, li)| {
            (
                *lang,
                (
                    RedundantScorer::new(&li.index, &li.reinvention),
                    PlacementScorer::new(&li.index, &li.placement),
                ),
            )
        })
        .collect();

    // Evaluate all candidates in parallel: the scorers are read-only, each
    // candidate is independent, and results come back in candidate order with
    // F1-before-F2 preserved per candidate — element-for-element identical to
    // the sequential loop.
    let evals = argot_engine::par::par_map_indexed(candidates.len(), |i| {
        let (_, lang, f) = &candidates[i];
        let (redundant, placement) = &scorers[lang];
        let found = redundant.evaluate(f, &vecs[i]);
        // F2 placement is consulted only when F1 didn't claim the function.
        let mis = if found.is_none() {
            placement.evaluate(f, &vecs[i])
        } else {
            None
        };
        (found, mis)
    });

    let mut hits = Vec::new();
    for (((bi, lang, f), vec), (found, mis)) in candidates.iter().zip(&vecs).zip(evals) {
        let li = &loaded[lang];
        let batch = &patches[*bi];
        let mut fired: Option<&'static str> = None;
        // F1 first: a duplicate isn't "misplaced", it's "redundant" — the
        // stronger signal wins, one finding per function.
        if let Some(found) = found {
            fired = Some("redundant");
            let similarity = found.similarity;
            hits.push(build_semantic_hit(
                batch,
                f,
                "redundant",
                similarity as f64,
                crate::redundant::MIN_SIMILARITY_TO_FIRE as f64,
                SemanticHitEvidence::Redundant {
                    nearest_symbol: found.nearest_symbol,
                    nearest_path: found.nearest_path,
                    nearest_line: found.nearest_line,
                    similarity,
                },
                filter_adapters,
                mute_rules,
            ));
        }
        // F2 placement (only when F1 didn't already claim the function).
        if fired.is_none() {
            if let Some(m) = mis {
                fired = Some("misplaced");
                let score = (m.expected_fraction - m.in_area_fraction).max(0.0) as f64;
                hits.push(build_semantic_hit(
                    batch,
                    f,
                    "misplaced",
                    score,
                    m.expected_fraction as f64,
                    SemanticHitEvidence::Misplaced {
                        neighbor_area: m.neighbor_area,
                        actual_area: m.actual_area,
                        peers: m.peers,
                    },
                    filter_adapters,
                    mute_rules,
                ));
            }
        }
        if dump_path.is_some() {
            dump_lines.push(dump_semantic_candidate(lang, f, vec, li, fired));
        }
    }
    if let (Some(p), false) = (dump_path, dump_lines.is_empty()) {
        use std::io::Write as _;
        if let Ok(mut fh) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&p)
        {
            let _ = writeln!(fh, "{}", dump_lines.join("\n"));
        }
    }
    hits
}
/// One JSON line for the `ARGOT_SEM_DUMP` capture: the candidate's identity and
/// structural features, its f16 embedding, its top nearest index neighbours
/// (unfiltered — offline analysis applies its own same-file / cross-file rules
/// by joining `entry_index` with a saved copy of the index), and the production
/// fire outcome (to validate offline re-implementations against the binary).
fn dump_semantic_candidate(
    lang: &str,
    f: &crate::index::FunctionRef,
    vec: &[f32],
    li: &crate::index::LoadedIndex,
    fired: Option<&str>,
) -> String {
    use base64::Engine as _;
    /// Neighbours captured per candidate — enough headroom over the check-time
    /// k=10 for offline variants to re-filter (same-file, deeper k) losslessly.
    const DUMP_NEIGHBORS: usize = 40;
    let neighbors: Vec<serde_json::Value> = li
        .index
        .nearest(vec, DUMP_NEIGHBORS, |_| true)
        .iter()
        .map(|n| serde_json::json!([n.entry_index, n.cosine]))
        .collect();
    let vec_f16: Vec<u8> = vec
        .iter()
        .flat_map(|x| half::f16::from_f32(*x).to_le_bytes())
        .collect();
    serde_json::json!({
        "lang": lang,
        "path": f.path,
        "symbol": f.symbol,
        "line": f.line,
        "body_lines": f.text.lines().count(),
        "callees": f.callees,
        "subtokens": f.subtokens,
        "vec_b64": base64::engine::general_purpose::STANDARD.encode(vec_f16),
        "neighbors": neighbors,
        "fired": fired,
    })
    .to_string()
}
/// Build one semantic `Finding`, applying the mute + inline suppression
/// surfaces exactly as base hits do. `reason` is `"redundant"` / `"misplaced"`.
#[allow(clippy::too_many_arguments)]
fn build_semantic_hit(
    batch: &PatchBatch,
    f: &crate::index::FunctionRef,
    reason: &str,
    score: f64,
    threshold: f64,
    sem: SemanticHitEvidence,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
) -> Finding {
    let hunk_content = f.text.clone();
    let hash = hit_hash(&batch.file_path, reason, &hunk_content);
    let source = String::from_utf8_lossy(&batch.content);
    let suppressions = FileSuppressions::parse(
        &batch.file_path,
        &source,
        ext_to_lang(&extension(&batch.file_path))
            .and_then(|l| filter_adapters.get(l))
            .map(|a| a.line_comment_prefix()),
        mute_rules,
        false,
    );
    let suppressed_by = suppressions.classify(reason, &hash, f.line, f.end_line);
    Finding {
        score,
        file_path: batch.file_path.clone(),
        line: f.line,
        line_end: f.end_line,
        source: batch.source.clone(),
        reason: reason.to_string(),
        flagged: true,
        threshold,
        hunk_content,
        evidence: Some(Box::new(sem)),
        hash,
        suppressed_by,
    }
}
impl RenderEvidence for SemanticHitEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        format_semantic_evidence(self, use_color)
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        format_semantic_evidence(self, false)
            .into_iter()
            .map(|l| l.trim().to_string())
            .collect()
    }

    fn similarity(&self) -> Option<f32> {
        match self {
            SemanticHitEvidence::Redundant { similarity, .. } => Some(*similarity),
            SemanticHitEvidence::Misplaced { .. } => None,
        }
    }
}
/// Render the nearest-existing-code evidence for a semantic finding as `↳` lines
/// (F4 — retrieval + template, no LLM).
pub(super) fn format_semantic_evidence(sem: &SemanticHitEvidence, use_color: bool) -> Vec<String> {
    match sem {
        SemanticHitEvidence::Redundant {
            nearest_symbol,
            nearest_path,
            nearest_line,
            similarity,
        } => {
            let body = format!(
                "    ↳ duplicates {nearest_symbol} ({nearest_path}:{nearest_line}) — similarity {similarity:.2}"
            );
            vec![paint(&body, C_DIM, use_color)]
        }
        SemanticHitEvidence::Misplaced {
            neighbor_area,
            actual_area,
            peers,
        } => {
            let filed = if actual_area.is_empty() {
                "the repo root".to_string()
            } else {
                actual_area.clone()
            };
            let head = format!("    ↳ looks like {neighbor_area} code filed under {filed}");
            let mut lines = vec![paint(&head, C_DIM, use_color)];
            if let Some((sym, path, line)) = peers.first() {
                let peer = format!("      nearest peer: {sym} ({path}:{line})");
                lines.push(paint(&peer, C_DIM, use_color));
            }
            lines
        }
    }
}

/// History window (first-parent commits) for the semantic self-calibrations —
/// mirrors the clean-commit bench window.
const SEMANTIC_CALIBRATION_WINDOW: usize = 150;

/// Which of `funcs` (index order) were ADDED within the recent history window:
/// their file is new since the window commit, or the file changed and the old
/// version (parsed with the same adapter — names only, no embedding) did not
/// define their symbol. Empty history / any git failure → all-false (the
/// reinvention calibration then keeps the standard rule).
fn recent_semantic_functions(
    repo_dir: &Path,
    funcs: &[crate::index::FunctionRef],
    adapter: &dyn LanguageAdapter,
    window: usize,
) -> Vec<bool> {
    let none = vec![false; funcs.len()];
    let Ok(repo) = git2::Repository::open(repo_dir) else {
        return none;
    };
    let Ok(head) = repo.head().and_then(|h| h.peel_to_commit()) else {
        return none;
    };
    // Walk `window` first-parent steps back (the bench's HEAD~window).
    let mut old = head.clone();
    for _ in 0..window {
        match old.parent(0) {
            Ok(p) => old = p,
            Err(_) => return none, // history shorter than the window
        }
    }
    let (Ok(old_tree), Ok(head_tree)) = (old.tree(), head.tree()) else {
        return none;
    };
    let Ok(diff) = repo.diff_tree_to_tree(Some(&old_tree), Some(&head_tree), None) else {
        return none;
    };
    let mut changed: std::collections::HashSet<String> = std::collections::HashSet::new();
    diff.foreach(
        &mut |delta, _| {
            if let Some(p) = delta.new_file().path().and_then(|p| p.to_str()) {
                changed.insert(p.replace('\\', "/"));
            }
            true
        },
        None,
        None,
        None,
    )
    .ok();
    // Old symbol sets for changed files that already existed at the old commit.
    let mut old_syms: std::collections::HashMap<String, std::collections::HashSet<String>> =
        std::collections::HashMap::new();
    for path in funcs.iter().map(|f| f.path.as_str()) {
        if !changed.contains(path) || old_syms.contains_key(path) {
            continue;
        }
        let syms = old_tree
            .get_path(Path::new(path))
            .ok()
            .and_then(|e| e.to_object(&repo).ok())
            .and_then(|o| o.into_blob().ok())
            .map(|b| String::from_utf8_lossy(b.content()).into_owned())
            .map(|src| {
                crate::index::functions_in_file(adapter, path, &src)
                    .into_iter()
                    .map(|f| f.symbol)
                    .collect::<std::collections::HashSet<String>>()
            })
            .unwrap_or_default(); // absent at the old commit → new file → empty set
        old_syms.insert(path.to_string(), syms);
    }
    funcs
        .iter()
        .map(|f| {
            changed.contains(&f.path)
                && !old_syms
                    .get(&f.path)
                    .map(|s| s.contains(&f.symbol))
                    .unwrap_or(false)
        })
        .collect()
}
