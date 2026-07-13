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
/// Score each hunk, dispatching per language (`_score_patches`). Applies the
/// inline-comment and `[[mute]]` surfaces per hit (path-level `[exclude].paths`
/// suppression arrives pre-marked on the batch). Returns
/// `(hits, hunk_count, per-file hunk counts)`.
#[allow(clippy::too_many_arguments)]
fn score_patches(
    patches: &[PatchBatch],
    scorers: &mut HashMap<String, SequentialImportBpeScorer>,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    slices: &HashMap<String, Vec<SliceEntry>>,
    new_file_thresholds: &HashMap<String, f64>,
    fit_corpus_files: &HashSet<String>,
    mute_rules: &[SuppressionRule],
    registry: &argot_engine::rules::Registry,
    header_cpp: bool,
    stderr: &mut String,
) -> (Vec<Finding>, usize, Vec<FileScan>) {
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

    for batch in patches {
        let ext = extension(&batch.file_path);
        let scorer = match ext_to_lang_ctx(&ext, header_cpp).and_then(|l| scorers.get_mut(l)) {
            Some(s) => s,
            None => {
                stderr.push_str(&format!(
                    "[argot] skipping {}: no scorer for extension '{}'\n",
                    batch.file_path, ext
                ));
                continue;
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
        );
        for w in suppressions.warnings() {
            let msg = format!("[argot] {}:{}: {}\n", batch.file_path, w.line, w.message);
            if warned.insert(msg.clone()) {
                stderr.push_str(&msg);
            }
        }

        for hunk in &batch.hunks {
            hunk_count += 1;
            *file_counts.entry(batch.file_path.clone()).or_insert(0) += 1;
            let hunk_start = hunk.new_start as i64 - 1;
            let hunk_end = hunk_start + hunk.new_lines as i64;
            if hunk_start < 0 || hunk_end > n_lines {
                continue;
            }
            let hs = hunk_start as usize;
            let he = hunk_end as usize;
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
            let (mut flagged, threshold) = match new_file_threshold {
                Some(t) => (reason == "import" || new_score >= t, t),
                None => match lang.and_then(|l| slice_threshold(slices, l, &batch.file_path)) {
                    Some(t) => (reason == "import" || new_score >= t, t),
                    None => (scored.flagged, scored.threshold),
                },
            };
            // Per-changeset novel-import dedup: an import alert whose foreign
            // modules were all already alerted in this run is the same decision
            // seen again (one dependency spread across a migration). Alert on
            // the first appearance; dedup the repeats. A hunk that adds a
            // genuinely new foreign module still fires.
            if flagged && reason == "import" && !scored.foreign_import_modules.is_empty() {
                if scored
                    .foreign_import_modules
                    .iter()
                    .all(|m| alerted_foreign_modules.contains(m))
                {
                    flagged = false;
                    deduped_import_alerts += 1;
                } else {
                    alerted_foreign_modules.extend(scored.foreign_import_modules.iter().cloned());
                }
            }
            let hash = hit_hash(&batch.file_path, &reason, &hunk_content);
            let suppressed_by = suppressions.classify(&reason, &hash, line, line_end);
            hits.push(Finding {
                score: scored.score,
                file_path: batch.file_path.clone(),
                line,
                line_end,
                source: batch.source.clone(),
                reason,
                flagged,
                threshold,
                hunk_content,
                evidence: scored
                    .evidence
                    .map(|e| Box::new(e) as Box<dyn RenderEvidence>),
                hash,
                suppressed_by,
            });
        }
    }
    if deduped_import_alerts > 0 {
        stderr.push_str(&format!(
            "[argot] {deduped_import_alerts} repeat novel-import alert(s) deduped \
             (same dependency across the change)\n"
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
    fn load(
        &mut self,
        argot_dir: &Path,
        detect: &argot_engine::config::DetectConfig,
    ) -> Result<(), (String, i32)> {
        let loaded = load_scorers(argot_dir, detect)?;
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
        }
        for (lang, bindings) in changeset_bindings {
            if let Some(scorer) = loaded.scorers.get_mut(lang) {
                scorer.set_changeset_bindings(bindings);
            }
        }

        let (hits, hunk_count, files_scanned) = score_patches(
            ctx.batches,
            &mut loaded.scorers,
            ctx.filter_adapters,
            &loaded.slices,
            &loaded.new_file_thresholds,
            &loaded.fit_corpus_files,
            ctx.mute_rules,
            ctx.registry,
            ctx.header_cpp,
            ctx.stderr,
        );
        ctx.scan.hunk_count = hunk_count;
        ctx.scan.files_scanned = files_scanned;
        hits
    }
}
