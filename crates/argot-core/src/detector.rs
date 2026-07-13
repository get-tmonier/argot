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
use crate::scoring::adapters::LanguageAdapter;
use crate::suppress::SuppressionRule;

/// Everything a detection pass may consult, engine-owned and shared across
/// detectors. Detectors read the changeset and write findings + diagnostics;
/// only the base (voice) detector fills [`CheckContext::scan`].
pub(crate) struct CheckContext<'a> {
    /// The scoped, filtered changeset batches (post-images + hunk spans).
    pub batches: &'a [crate::check::PatchBatch],
    /// The full check invocation (repo path, mode, argot dir, …). The
    /// integrity pass re-derives two-sided changesets from it.
    #[cfg_attr(not(feature = "integrity"), allow(dead_code))]
    pub args: &'a crate::check::CheckArgs,
    /// Per-language adapters for comment prefixes / parsing.
    pub filter_adapters: &'a HashMap<String, Box<dyn LanguageAdapter>>,
    /// Active `[[mute]]` rules.
    pub mute_rules: &'a [SuppressionRule],
    /// `[detect]` configuration (read by the semantic pass).
    #[cfg_attr(not(feature = "semantic"), allow(dead_code))]
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
pub(crate) struct ScanReport {
    pub hunk_count: usize,
    pub files_scanned: Vec<FileScan>,
}

/// One rule group's detection pass.
pub(crate) trait Detector {
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
pub(crate) struct RegisteredDetector<'a> {
    pub detector: Box<dyn Detector + 'a>,
    /// Position in the pass execution order (stderr interleave parity).
    pub execution_rank: usize,
    /// Position in the finding merge order (stdout parity).
    pub merge_rank: usize,
}

/// Run every registered detector by ascending `execution_rank`, then merge
/// their findings by ascending `merge_rank`.
pub(crate) fn run_detectors(
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
