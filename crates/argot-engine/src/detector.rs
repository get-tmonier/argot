//! The detector contract — how a rule group plugs into `check`.
//!
//! Each rule group (voice, semantic, architecture, integrity, …) implements
//! [`Detector`] and is registered in one place; the engine loop in
//! [`crate::check::run_check`] drives every registered detector through the
//! same lifecycle: gate on the group's rule settings, run inside a timing
//! phase, classify suppressions, merge findings. Adding or deleting a group
//! never touches the loop.

use std::collections::HashMap;

use crate::config::DetectConfig;
use crate::finding::Finding;
use crate::output::FileScan;
use crate::rules::RuleSettings;
use crate::suppress::SuppressionRule;
use argot_lang::adapters::LanguageAdapter;

/// Everything a detection pass may consult, engine-owned and shared across
/// detectors. Detectors read the changeset and write findings + diagnostics;
/// only the base (voice) detector fills [`CheckContext::scan`].
pub struct CheckContext<'a> {
    /// The scoped, filtered changeset batches (post-images + hunk spans).
    pub batches: &'a [crate::check::PatchBatch],
    /// The full check invocation (repo path, mode, argot dir, …). The
    /// integrity pass re-derives two-sided changesets from it.
    pub args: &'a crate::check::CheckArgs,
    /// Per-language adapters for comment prefixes / parsing.
    pub filter_adapters: &'a HashMap<String, Box<dyn LanguageAdapter>>,
    /// Active `[[mute]]` rules.
    pub mute_rules: &'a [SuppressionRule],
    /// `[detect]` configuration (read by the semantic pass).
    pub detect: &'a DetectConfig,
    /// Repo-wide `.h` → C/C++ routing majority.
    pub header_cpp: bool,
    /// Resolved rule severities.
    pub settings: &'a RuleSettings,
    /// Ordered stderr sink.
    pub stderr: &'a mut String,
    /// Scan statistics for the report meta — filled by the base detector.
    pub scan: &'a mut ScanReport,
}

/// What the base scan covered (drives `files scanned` in the report meta).
#[derive(Default)]
pub struct ScanReport {
    pub hunk_count: usize,
    pub files_scanned: Vec<FileScan>,
}

/// The base model's identity and coverage, provided by exactly one detector
/// after [`Detector::load`]. The engine derives everything model-shaped from
/// this — batch scoping, the report's model line, the freshness warning —
/// without holding any scorer type itself.
#[derive(Debug, Clone)]
pub struct BaseModelInfo {
    /// Combined fit-time model fingerprint (the manifest's `model_hash`).
    pub model_hash: String,
    /// Repo SHA the model was fitted at (`None` for configs predating it).
    pub fit_sha: Option<String>,
    /// File extensions the fitted model covers (batch scoping).
    pub language_extensions: std::collections::HashSet<String>,
    /// Languages with a fitted model (the unfitted-language warning).
    pub fitted_languages: std::collections::HashSet<String>,
}

/// Everything a fit-time hook may consult. The voice model's own fit IS
/// `run_calibrate`; this context serves the additive groups' artifacts,
/// written as `.argot/` siblings of `scorer-config.json` so the base config
/// stays byte-for-byte unchanged whatever is compiled in.
pub struct FitContext<'a> {
    pub repo_dir: &'a std::path::Path,
    /// The `scorer-config.json` output path — sibling artifacts derive their
    /// paths via `with_file_name`.
    pub output: &'a std::path::Path,
    /// The repo SHA this fit reflects (stamped into each artifact).
    pub repo_sha: &'a str,
    /// Effective `[rules]` severities — fit hooks self-gate on their group so
    /// an off group produces no artifact and pays no cost (and can explain
    /// the skip, which a loop-level gate could not).
    pub settings: &'a RuleSettings,
}

/// One language's corpus, observed mid-calibration. The base fit reads every
/// language's sources exactly once; a detector whose artifact derives from
/// the same corpus (e.g. an embedding index) hooks
/// [`Detector::fit_language`] instead of re-reading the tree — one read,
/// and the fit diagnostics keep their per-language interleave.
pub struct FitLanguageContext<'a> {
    /// The scoring language name (`"python"`, …).
    pub language: &'a str,
    /// Absolute path + source of every filtered corpus file of this language
    /// (data-dominant and auto-generated files already excluded).
    pub files: &'a [(std::path::PathBuf, String)],
    /// The language's adapter.
    pub adapter: &'a dyn argot_lang::adapters::LanguageAdapter,
    /// The resolved path-suppression set (the same set calibration samples
    /// from — lock-step principle).
    pub suppressions: &'a crate::suppress::PathSuppressions,
    pub repo_dir: &'a std::path::Path,
}

/// One rule group's detection pass.
pub trait Detector {
    /// The group this detector's rules belong to (gates the whole pass).
    fn group(&self) -> &'static str;

    /// The timing-phase label (`ARGOT_TIMING` diagnostics).
    fn timing_label(&self) -> &'static str;

    /// Whether the pass runs at all. Default: at least one rule in the group
    /// is not `off` — skipping an off group avoids its whole cost (index
    /// load, model download). The base detector always runs: it owns the scan
    /// statistics, and off-rule findings are dropped by the engine afterward.
    fn enabled(&self, settings: &RuleSettings) -> bool {
        settings.group_enabled(self.group())
    }

    /// Fit-time lifecycle, before the per-language calibration loop: acquire
    /// whatever the group's fit needs once (a model load, a prior artifact).
    /// Self-gated (consult `ctx.settings`); failures degrade loudly, never
    /// fail the fit. Default no-op.
    fn fit_begin(&mut self, _ctx: &FitContext<'_>) {}

    /// Fit-time observation of one language's corpus, inside the calibration
    /// loop — see [`FitLanguageContext`]. Default no-op.
    fn fit_language(&mut self, _lang: &FitLanguageContext<'_>) {}

    /// Fit-time hook, after the base model is written: build/write this
    /// group's artifact(s) as `.argot/` sibling files. Self-gated; default
    /// no-op — a group with no learned state skips it. Failures degrade
    /// loudly (stderr) but never fail the fit; the base statistical model
    /// must land regardless.
    fn fit(&mut self, _ctx: &FitContext<'_>) {}

    /// Check-time lifecycle, before the changeset is collected: load this
    /// group's model state. `Err((message, exit_code))` fails the whole check
    /// (the base model is mandatory); additive groups instead degrade inside
    /// [`Detector::check`] (missing artifact → no findings) so the base
    /// guardrail always runs. Default no-op.
    fn load(
        &mut self,
        _argot_dir: &std::path::Path,
        _detect: &crate::config::DetectConfig,
    ) -> Result<(), (String, i32)> {
        Ok(())
    }

    /// The base model's identity + coverage — implemented by exactly one
    /// registered detector (the base pass). The engine refuses to run
    /// without it.
    fn base_info(&self) -> Option<&BaseModelInfo> {
        None
    }

    /// Run the pass and return raw findings (suppression already classified
    /// per finding by the pass via [`crate::suppress::FileSuppressions`]).
    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding>;
}

/// One registered detector plus its two order ranks.
///
/// **Order table (parity-critical).** `execution_rank` orders the passes —
/// additive passes first, the base (voice) pass last — preserving the stderr
/// interleave. `merge_rank` orders findings in the report — the base pass
/// first, then the additive passes — preserving stdout. Both projections come
/// from the registration site; change them only with the check goldens.
pub struct RegisteredDetector<'a> {
    pub detector: Box<dyn Detector + 'a>,
    /// Position in the pass execution order (stderr interleave parity).
    pub execution_rank: usize,
    /// Position in the finding merge order (stdout parity).
    pub merge_rank: usize,
}

/// Run every registered detector by ascending `execution_rank`, then merge
/// their findings by ascending `merge_rank`.
pub fn run_detectors(
    detectors: &mut [RegisteredDetector<'_>],
    ctx: &mut CheckContext<'_>,
) -> Vec<Finding> {
    detectors.sort_by_key(|reg| reg.execution_rank);
    let mut collected: Vec<(usize, Vec<Finding>)> = Vec::new();
    for reg in detectors.iter_mut() {
        if !reg.detector.enabled(ctx.settings) {
            continue;
        }
        let t = crate::timing::phase(reg.detector.timing_label());
        let findings = reg.detector.check(ctx);
        t.done();
        collected.push((reg.merge_rank, findings));
    }
    collected.sort_by_key(|(rank, _)| *rank);
    collected.into_iter().flat_map(|(_, f)| f).collect()
}
