//! The base statistical pass (the voice group): dispatches each hunk to its
//! per-language `SequentialImportBpeScorer`.

use super::load::{load_scorers, Loaded, SliceEntry};
use crate::scoring::adapters::LanguageAdapter;
use crate::scoring::evidence::types::Evidence;
use crate::scoring::evidence::{evidence_caret_spans, evidence_lines_of_interest, format_evidence};
use crate::scoring::sequential::SequentialImportBpeScorer;
use argot_engine::check::PatchBatch;
use argot_engine::detector::{BaseModelInfo, CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::output::FileScan;
use argot_engine::rules::{self, RuleSettings};
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::ext::{ext_to_lang, ext_to_lang_ctx, extension};
use argot_lang::text::splitlines;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

/// The statistical voice evidence renders through its existing formatters —
/// the exact strings the pre-contract render paths produced. The impl lives
/// with the voice slice (its `Evidence` type), not the contract.
impl RenderEvidence for Evidence {
    fn human(&self, use_color: bool, hunk_start_line: usize) -> Vec<String> {
        format_evidence(self, use_color, hunk_start_line)
    }

    fn machine(&self, hunk_start_line: usize) -> Vec<String> {
        format_evidence(self, false, hunk_start_line)
            .into_iter()
            .map(|l| l.trim().to_string())
            .collect()
    }

    fn foreign_specifiers(&self) -> Vec<String> {
        match self {
            Evidence::Import(imp) => imp.foreign_specifiers.clone(),
            _ => Vec::new(),
        }
    }

    fn lines_of_interest(&self) -> HashSet<usize> {
        evidence_lines_of_interest(Some(self))
    }

    fn caret_spans(&self) -> HashMap<usize, Vec<argot_engine::finding::SourceSpan>> {
        evidence_caret_spans(Some(self))
    }
}

/// The slice threshold that applies to `rel_path` for `lang`, if any (first
/// matching slice wins — most-specific specs should be listed first at fit).
fn slice_threshold(
    slices: &HashMap<String, Vec<SliceEntry>>,
    lang: &str,
    rel_path: &str,
) -> Option<f64> {
    slices.get(lang)?.iter().find_map(|s| {
        if s.paths
            .iter()
            .any(|p| rel_path == p || rel_path.starts_with(p))
        {
            Some(s.threshold)
        } else {
            None
        }
    })
}
/// One batch's hunks, scored but not yet merged. The per-changeset
/// novel-import dedup is order-dependent — the first appearance of a foreign
/// module alerts and the rest are folded into it — so it cannot happen while
/// batches are scored concurrently. Everything that depends on batch order
/// waits for [`score_patches`]'s serial merge.
#[derive(Default)]
struct BatchScored {
    /// Emitted verbatim when the batch had no scorer for its extension.
    skip_note: Option<String>,
    /// Suppression-surface warnings, in file order; deduped across batches on
    /// merge, as they were when the loop was serial.
    warnings: Vec<String>,
    /// Hunks that reached the scorer, plus those skipped for being out of the
    /// file's line range — both counted toward the scan statistics.
    counted: usize,
    /// Hunks that would have fired but were larger than
    /// [`MAX_SCORED_HUNK_LINES`]. Reported, never silent: a hunk not judged
    /// must not look like a hunk judged clean.
    oversized_hunks: usize,
    hunks: Vec<HunkScored>,
}

/// One scored hunk, carrying what the merge needs to finish it.
struct HunkScored {
    score: f64,
    line: usize,
    line_end: usize,
    reason: String,
    /// Before the novel-import dedup, which the merge applies.
    flagged: bool,
    threshold: f64,
    hunk_content: String,
    evidence: Option<crate::scoring::evidence::types::Evidence>,
    hash: String,
    suppressed_by: Option<argot_engine::finding::SuppressedBy>,
    foreign_import_modules: Vec<String>,
}

/// Score one file's hunks. Pure with respect to the changeset: it reads the
/// shared scorers by `&` and touches nothing another batch can see, which is
/// what lets [`score_patches`] run these concurrently.
#[allow(clippy::too_many_arguments)]
fn score_batch(
    batch: &PatchBatch,
    scorers: &HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    slices: &HashMap<String, Vec<SliceEntry>>,
    new_file_thresholds: &HashMap<String, f64>,
    fit_corpus_files: &HashSet<String>,
    mute_rules: &[SuppressionRule],
    registry: &argot_engine::rules::Registry,
    settings: &argot_engine::rules::RuleSettings,
    header_cpp: bool,
) -> BatchScored {
    let mut counted = 0usize;
    let mut oversized_hunks = 0usize;
    let mut hunks: Vec<HunkScored> = Vec::new();

    let ext = extension(&batch.file_path);
    let scorer = match ext_to_lang_ctx(&ext, header_cpp).and_then(|l| scorers.get(l)) {
        Some(s) => s,
        None => {
            return BatchScored {
                skip_note: Some(format!(
                    "[argot] skipping {}: no scorer for extension '{}'\n",
                    batch.file_path, ext
                )),
                ..BatchScored::default()
            };
        }
    };

    let file_source = String::from_utf8_lossy(&batch.content).into_owned();
    let file_lines = splitlines(&file_source);
    let n_lines = file_lines.len() as i64;

    // The file's suppression surfaces, resolved once from the same content
    // that gets scored (inline comments use the language's comment token).
    let suppressions = FileSuppressions::parse(
        &batch.file_path,
        &file_source,
        ext_to_lang(&ext)
            .and_then(|l| filter_adapters.get(l))
            .map(|a| a.line_comment_prefix()),
        mute_rules,
        batch.ignored_by_pattern,
        registry,
        settings,
    );
    let warnings: Vec<String> = suppressions
        .warnings()
        .iter()
        .map(|w| format!("[argot] {}:{}: {}\n", batch.file_path, w.line, w.message))
        .collect();

    for hunk in &batch.hunks {
        counted += 1;
        let hunk_start = hunk.new_start as i64 - 1;
        let hunk_end = hunk_start + hunk.new_lines as i64;
        if hunk_start < 0 || hunk_start >= n_lines {
            continue;
        }
        let hs = hunk_start as usize;
        // Clamp the end to the file: git's post-image line count can exceed
        // `splitlines`' when the last line has no trailing newline ("\ No
        // newline at end of file"). Dropping the whole hunk there missed an
        // import appended at the very bottom of a file (B6).
        let he = hunk_end.min(n_lines) as usize;
        let hunk_content = file_lines[hs..he].join("\n");
        // file_path routes the hunk to its fit-time cluster (falling back
        // to Jaccard-nearest for files the model has never seen) — the
        // same signal surface calibration hunks scored against, so the
        // threshold and the check path see one score distribution.
        let scored = scorer.score_hunk(
            &hunk_content,
            Some(&file_source),
            Some(hs + 1),
            Some(he),
            Some(Path::new(&batch.file_path)),
        );
        let line = hunk.new_start as usize;
        let line_end = (hunk.new_start + hunk.new_lines).saturating_sub(1) as usize;
        let reason = scored.reason.as_str().to_string();
        let lang = ext_to_lang(&ext);
        // New-file dispatch takes precedence: a hunk whose file was absent
        // from the fit corpus is judged against the (higher) new-file
        // threshold — a new file gets full unattested-callee mass with no
        // cluster routing, a systematically higher distribution than an edit
        // to a known file (issue #92 new-file flooding). Foreign imports
        // still fire regardless of threshold. Falls through to per-slice /
        // whole-repo dispatch for known files, or configs without the field.
        let is_new_file = if fit_corpus_files.is_empty() {
            // Config predates the corpus_files snapshot: fall back to cluster
            // membership (misclassifies data-dominant known files).
            !scorer.is_fit_file(Path::new(&batch.file_path))
        } else {
            !fit_corpus_files.contains(&batch.file_path)
        };
        let new_file_threshold = lang.and_then(|l| {
            is_new_file
                .then(|| new_file_thresholds.get(l).copied())
                .flatten()
        });
        // A `none`-reason hunk fired no stage: its call-receiver
        // contribution was *not* gated (the hunk reaches nothing foreign),
        // so it must not count toward the new-file / slice threshold —
        // otherwise a new file of the repo's own code (its own unattested
        // callees) is flagged on exactly the signal the hunk-level
        // foreign-reach gate already rejected. Judge it on token surprise
        // alone. Firing reasons (import/bpe/call_receiver) already carry a
        // gated score in `scored.score`.
        let new_score = if reason == "none" {
            scored.stages.bpe_score
        } else {
            scored.score
        };
        // A `[exclude].check-only` file (tests, by default) never entered
        // the corpus, so no threshold here was calibrated on phrasing like
        // its own — re-deciding against one would judge test style by
        // production's distribution. Only the import verdict survives, and
        // that one is a membership test the fit did learn for this scope.
        let (flagged, threshold) = if scorer.is_check_only_file(Path::new(&batch.file_path)) {
            (reason == "import", scored.threshold)
        } else {
            match new_file_threshold {
                Some(t) => (reason == "import" || new_score >= t, t),
                None => match lang.and_then(|l| slice_threshold(slices, l, &batch.file_path)) {
                    Some(t) => (reason == "import" || new_score >= t, t),
                    None => (scored.flagged, scored.threshold),
                },
            }
        };
        // A rewrite of a whole file is not one pattern being introduced. Past
        // [`MAX_SCORED_HUNK_LINES`] the hunk holds most of the file's
        // vocabulary, so something in it is always unfamiliar and the verdict
        // says more about the hunk's size than about the code. New files are
        // exempt: there the whole file legitimately *is* the change, and the
        // new-file threshold above already judges it on its own distribution.
        let oversized = is_oversized(is_new_file, line, line_end);
        if oversized && flagged {
            oversized_hunks += 1;
        }
        let flagged = flagged && !oversized;
        // Per-changeset novel-import dedup: an import alert whose foreign
        // modules were all already alerted in this run is the same decision
        // seen again (one dependency spread across a migration). Alert on
        // the first appearance; dedup the repeats. A hunk that adds a
        // genuinely new foreign module still fires.
        let hash = hit_hash(&batch.file_path, &reason, &hunk_content);
        let suppressed_by = suppressions.classify(&reason, &hash, line, line_end);
        hunks.push(HunkScored {
            score: scored.score,
            line,
            line_end,
            reason,
            flagged,
            threshold,
            hunk_content,
            evidence: scored.evidence,
            hash,
            suppressed_by,
            foreign_import_modules: scored.foreign_import_modules,
        });
    }
    BatchScored {
        skip_note: None,
        warnings,
        counted,
        oversized_hunks,
        hunks,
    }
}

/// Past this many lines a hunk stops being a reviewable unit and starts being
/// the file. A whole-file rewrite — a comment reshuffle, a reformat, a licence
/// header sweep — then holds most of the file's vocabulary, so something in it
/// is always unfamiliar and the verdict reports the hunk's size rather than the
/// code.
///
/// Measured over the 36 benchmark corpora: capping existing-file hunks here
/// removes **73 of 471 false alarms (15,5 %)** across ten corpora, and costs
/// nothing measurable — the largest fixture in the whole catalogue is **80
/// lines** (n=977, median 13, p99 59), so no fixture is even close. uos alone
/// gives back 47, from commits like "Comment reordered for all the functions"
/// (2 564 insertions / 2 547 deletions in one file, scored as a single hunk).
///
/// New files are exempt: there the whole file legitimately is the change.
const MAX_SCORED_HUNK_LINES: usize = 100;

/// Whether a hunk is too large to judge — see [`MAX_SCORED_HUNK_LINES`]. A new
/// file is never oversized: there the whole file legitimately is the change.
fn is_oversized(is_new_file: bool, line: usize, line_end: usize) -> bool {
    !is_new_file && line_end.saturating_sub(line) + 1 > MAX_SCORED_HUNK_LINES
}

/// Score each hunk, dispatching per language (`_score_patches`). Applies the
/// inline-comment and `[[mute]]` surfaces per hit (path-level `[exclude].paths`
/// suppression arrives pre-marked on the batch). Returns
/// `(hits, hunk_count, per-file hunk counts)`.
///
/// Batches are scored **in parallel** and merged **in batch order**, so the
/// order-dependent parts — the novel-import dedup, the warning dedup, and the
/// order of `hits` itself — are byte-identical to the serial loop this
/// replaced. Scoring a 921-file changeset used one core of eleven.
#[allow(clippy::too_many_arguments)]
fn score_patches(
    patches: &[PatchBatch],
    scorers: &HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    slices: &HashMap<String, Vec<SliceEntry>>,
    new_file_thresholds: &HashMap<String, f64>,
    fit_corpus_files: &HashSet<String>,
    mute_rules: &[SuppressionRule],
    registry: &argot_engine::rules::Registry,
    settings: &argot_engine::rules::RuleSettings,
    header_cpp: bool,
    stderr: &mut String,
) -> (Vec<Finding>, usize, Vec<FileScan>) {
    let scored: Vec<BatchScored> = argot_engine::par::par_map_indexed(patches.len(), |i| {
        score_batch(
            &patches[i],
            scorers,
            filter_adapters,
            slices,
            new_file_thresholds,
            fit_corpus_files,
            mute_rules,
            registry,
            settings,
            header_cpp,
        )
    });

    let mut hits: Vec<Finding> = Vec::new();
    let mut hunk_count = 0usize;
    let mut file_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut warned: HashSet<String> = HashSet::new();
    // Per-changeset novel-import dedup: foreign top-level modules that have
    // already raised an import alert in this check run. The same new dependency
    // added across many files of one change (a mechanical migration) is one
    // decision — alert on its first appearance, dedup the rest.
    let mut alerted_foreign_modules: HashSet<String> = HashSet::new();
    let mut deduped_import_alerts: usize = 0;
    let mut oversized_total: usize = 0;

    for (batch, out) in patches.iter().zip(scored) {
        if let Some(note) = out.skip_note {
            stderr.push_str(&note);
            continue;
        }
        for msg in out.warnings {
            if warned.insert(msg.clone()) {
                stderr.push_str(&msg);
            }
        }
        hunk_count += out.counted;
        oversized_total += out.oversized_hunks;
        if out.counted > 0 {
            *file_counts.entry(batch.file_path.clone()).or_insert(0) += out.counted;
        }
        for h in out.hunks {
            let mut flagged = h.flagged;
            if flagged && h.reason == "import" && !h.foreign_import_modules.is_empty() {
                if h.foreign_import_modules
                    .iter()
                    .all(|m| alerted_foreign_modules.contains(m))
                {
                    flagged = false;
                    deduped_import_alerts += 1;
                } else {
                    alerted_foreign_modules.extend(h.foreign_import_modules.iter().cloned());
                }
            }
            hits.push(Finding {
                score: h.score,
                file_path: batch.file_path.clone(),
                line: h.line,
                line_end: h.line_end,
                source: batch.source.clone(),
                reason: h.reason,
                flagged,
                threshold: h.threshold,
                hunk_content: h.hunk_content,
                evidence: h.evidence.map(|e| Box::new(e) as Box<dyn RenderEvidence>),
                hash: h.hash,
                suppressed_by: h.suppressed_by,
            });
        }
    }
    if deduped_import_alerts > 0 {
        stderr.push_str(&format!(
            "[argot] {deduped_import_alerts} repeat novel-import alert(s) deduped \
             (same dependency across the change)\n"
        ));
    }
    if oversized_total > 0 {
        stderr.push_str(&format!(
            "[argot] {oversized_total} hunk(s) over {MAX_SCORED_HUNK_LINES} lines were not \
             judged — that much at once is a rewrite, not one pattern being introduced, \
             and holds most of the file's vocabulary. Review those by hand.\n"
        ));
    }

    let files_scanned = file_counts
        .into_iter()
        .map(|(path, hunks)| FileScan { path, hunks })
        .collect();
    (hits, hunk_count, files_scanned)
}
/// The base statistical pass (the voice group) as a detector. Owns the
/// loaded per-language model state (filled by [`Detector::load`]); the only
/// detector that provides [`argot_engine::detector::BaseModelInfo`] and fills
/// [`argot_engine::detector::ScanReport`].
pub struct VoiceDetector {
    loaded: Option<Loaded>,
    info: Option<BaseModelInfo>,
}

impl Default for VoiceDetector {
    fn default() -> Self {
        Self::new()
    }
}

impl VoiceDetector {
    pub fn new() -> Self {
        VoiceDetector {
            loaded: None,
            info: None,
        }
    }
}

impl Detector for VoiceDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_VOICE
    }

    fn timing_label(&self) -> &'static str {
        "check: score patches (statistical)"
    }

    /// Always runs: it owns the scan statistics (hunk/file counts in the
    /// report meta), and internal reasons (`none` under `--threshold`) have
    /// no rule to gate on. Off-rule findings are dropped by the engine.
    fn enabled(&self, _settings: &RuleSettings) -> bool {
        true
    }

    /// Loads the fit-time model snapshot (`scorer-config.json` v3). A failure
    /// here fails the whole check — the base model is mandatory.
    fn load(&mut self, ctx: &argot_engine::detector::LoadContext<'_>) -> Result<(), (String, i32)> {
        let check_only: Vec<String> = ctx
            .path_suppressions
            .check_only_patterns()
            .into_iter()
            .map(str::to_string)
            .collect();
        let loaded = load_scorers(ctx.argot_dir, ctx.detect, &check_only)?;
        self.info = Some(BaseModelInfo {
            model_hash: loaded.model_hash.clone(),
            fit_sha: loaded.fit_sha.clone(),
            language_extensions: loaded.language_extensions.clone(),
            fitted_languages: loaded.scorers.keys().cloned().collect(),
        });
        self.loaded = Some(loaded);
        Ok(())
    }

    fn base_info(&self) -> Option<&BaseModelInfo> {
        self.info.as_ref()
    }

    fn model_facts(&self) -> Option<std::sync::Arc<dyn argot_engine::detector::ModelFacts>> {
        self.loaded
            .as_ref()
            .map(|l| l.facts.clone() as std::sync::Arc<dyn argot_engine::detector::ModelFacts>)
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        let loaded = self
            .loaded
            .as_mut()
            .expect("VoiceDetector::check before load()");
        // Changeset-wide local bindings: names any file in this change
        // defines. A change that calls what it also defines (a new feature
        // naming its own components) is new code, not foreign voice; only
        // callees neither the corpus nor the changeset knows keep
        // contributing.
        let mut changeset_bindings: HashMap<&'static str, HashSet<String>> = HashMap::new();
        // …and the modules it declares. A file carrying `unit foo` makes
        // `uses foo` elsewhere in the same change a reference to the repo's own
        // new module rather than to an unknown dependency.
        let mut changeset_modules: HashMap<&'static str, HashSet<String>> = HashMap::new();
        for b in ctx.batches {
            let ext = extension(&b.file_path);
            let Some(lang) = ext_to_lang(&ext) else {
                continue;
            };
            let Some(adapter) = ctx.filter_adapters.get(lang) else {
                continue;
            };
            let source = String::from_utf8_lossy(&b.content);
            changeset_bindings
                .entry(lang)
                .or_default()
                .extend(adapter.callable_definitions(&source));
            if let Some(module) = adapter.declared_module(&source) {
                changeset_modules.entry(lang).or_default().insert(module);
            }
        }
        for (lang, bindings) in changeset_bindings {
            if let Some(scorer) = loaded.scorers.get_mut(lang) {
                scorer.set_changeset_bindings(bindings);
            }
        }
        for (lang, modules) in changeset_modules {
            if let Some(scorer) = loaded.scorers.get_mut(lang) {
                scorer.attest_changeset_modules(modules);
            }
        }

        // Declared migrations (`[[migration]]`) widen the attestation the
        // same way mined supersessions did at load — the pattern the repo
        // declared it is moving *to* must never read as foreign, and that
        // must hold without a refit.
        if !ctx.migrations.is_empty() {
            let imports: Vec<String> = ctx
                .migrations
                .iter()
                .filter(|m| m.kind == argot_engine::config::MigrationKind::Import)
                .map(|m| m.to.clone())
                .collect();
            let callees: Vec<String> = ctx
                .migrations
                .iter()
                .filter(|m| m.kind == argot_engine::config::MigrationKind::Callee)
                .map(|m| m.to.clone())
                .collect();
            for scorer in loaded.scorers.values_mut() {
                scorer.attest_replacements(&imports, &callees);
            }
        }

        let (mut hits, hunk_count, files_scanned) = score_patches(
            ctx.batches,
            &loaded.scorers,
            ctx.filter_adapters,
            &loaded.slices,
            &loaded.new_file_thresholds,
            &loaded.fit_corpus_files,
            ctx.mute_rules,
            ctx.registry,
            ctx.settings,
            ctx.header_cpp,
            ctx.stderr,
        );
        ctx.scan.hunk_count = hunk_count;
        ctx.scan.files_scanned = files_scanned;
        hits.extend(crate::superseded::superseded_findings(
            ctx.batches,
            &loaded.supersessions,
            ctx.migrations,
            ctx.filter_adapters,
            ctx.mute_rules,
            ctx.registry,
            ctx.settings,
            ctx.header_cpp,
        ));
        hits
    }
}

#[cfg(test)]
mod tests;
