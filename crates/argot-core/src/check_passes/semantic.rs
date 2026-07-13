//! The semantic pass (`--features semantic`): F1 reinvention + F2 placement
//! findings from the per-repo embedding index (`.argot/semantic-index.json`),
//! plus F4 nearest-code evidence. Group `semantic`.

use crate::scoring::adapters::LanguageAdapter;
use argot_engine::check::render::{paint, C_DIM};
use argot_engine::check::PatchBatch;
use argot_engine::config::DetectConfig;
use argot_engine::detector::{CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::rules;
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::ext::{ext_to_lang, ext_to_lang_ctx, extension};
use std::collections::{HashMap, HashSet};
use std::path::Path;

/// The nearest-existing-code evidence attached to a semantic finding (F4). Held
/// as structured data so every output format renders it its own way.
#[cfg(feature = "semantic")]
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
/// Its fit-time index build stays integrated in `run_calibrate` (not the
/// [`Detector::fit`] hook): the embedding pass shares the calibration loop's
/// per-language corpus reads, the one loaded embedder, and the prior
/// artifact's incremental vector reuse. A standalone hook would re-read the
/// corpus and reorder fit diagnostics for no deletion value — revisit when
/// the slice moves to its own crate.
#[cfg(feature = "semantic")]
pub(crate) struct SemanticDetector;

#[cfg(feature = "semantic")]
impl Detector for SemanticDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_SEMANTIC
    }

    fn timing_label(&self) -> &'static str {
        "check: semantic pass"
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
#[cfg(feature = "semantic")]
fn semantic_hits(
    patches: &[PatchBatch],
    argot_dir: &Path,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    detect: &DetectConfig,
    header_cpp: bool,
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::scoring::semantic::embedder::Embedder;
    use crate::scoring::semantic::index::{
        functions_in_file, FunctionRef, LoadedIndex, SemanticArtifact,
    };
    use crate::scoring::semantic::placement::PlacementScorer;
    use crate::scoring::semantic::redundant::RedundantScorer;
    use crate::scoring::semantic::SEMANTIC_INDEX_FILE;

    // Load the fit-time index artifact; its absence just means no semantic layer.
    let t_art = crate::timing::phase("check: semantic artifact read+parse");
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
    let t_cand = crate::timing::phase("check: semantic candidate extract");
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
    let t_idx = crate::timing::phase("check: semantic index decode");
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
    let t_model = crate::timing::phase("check: semantic embedder load");
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
    let t_embed = crate::timing::phase(format!("check: semantic embed ({} fns)", candidates.len()));
    let embed_cache = crate::scoring::semantic::embed_cache::EmbedCache::open_current();
    let texts: Vec<&str> = candidates.iter().map(|(_, _, f)| f.text.as_str()).collect();
    let vecs = match crate::scoring::semantic::embed_cache::embed_with_cache(
        &embedder,
        &texts,
        embed_cache.as_ref(),
    ) {
        Ok(v) => v,
        Err(e) => {
            stderr.push_str(&format!("[argot] semantic embedding failed: {e}\n"));
            return Vec::new();
        }
    };
    t_embed.done();
    let _t_score = crate::timing::phase("check: semantic score candidates");

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
    let evals = crate::par::par_map_indexed(candidates.len(), |i| {
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
                crate::scoring::semantic::redundant::MIN_SIMILARITY_TO_FIRE as f64,
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
#[cfg(feature = "semantic")]
fn dump_semantic_candidate(
    lang: &str,
    f: &crate::scoring::semantic::index::FunctionRef,
    vec: &[f32],
    li: &crate::scoring::semantic::index::LoadedIndex,
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
#[cfg(feature = "semantic")]
#[allow(clippy::too_many_arguments)]
fn build_semantic_hit(
    batch: &PatchBatch,
    f: &crate::scoring::semantic::index::FunctionRef,
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
#[cfg(feature = "semantic")]
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
#[cfg(feature = "semantic")]
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
