//! The test-integrity pass: diffs each changeset's test files into gaming
//! events (`test-deleted` / `test-disabled` / `test-weakened`), gated by the
//! repo's own learned event gates. Group `integrity`.

use argot_engine::check::render::{paint, C_DIM};
use argot_engine::check::CheckArgs;
use argot_engine::detector::{CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::rules;
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::adapters::LanguageAdapter;
use argot_lang::ext::{ext_to_lang, extension};
use std::collections::HashMap;

#[cfg(test)]
mod tests;

/// The integrity pass's changesets: the engine's two-sided collection,
/// scoped to files whose language has a test inventory (path filter applied
/// before any blob read). Uses the **per-commit** variant so a `base..head`
/// range (e.g. `argot audit`) is reasoned about one accepted unit at a time —
/// the "did this change also touch production?" gate must see each commit
/// separately, never the whole window's union.
fn integrity_changesets(args: &CheckArgs) -> Vec<(String, Vec<crate::model::FileChange>)> {
    argot_engine::check::two_sided::collect_two_sided_per_commit(args, &|path| {
        crate::test_inventory::language_for_path(path).is_some()
    })
}

/// The rendered evidence of a test-integrity finding — the gamed test and the
/// co-changed production source, plus the affected test's name (`None` for
/// whole-file events) surfaced as `HitRecord.symbol` so consumers can act on
/// the name (e.g. audit attributing a deleted test to the commit whose diff
/// dropped it) without parsing evidence text.
struct IntegrityEvidence {
    line: String,
    symbol: Option<String>,
}

impl RenderEvidence for IntegrityEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        vec![paint(&format!("    ↳ {}", self.line), C_DIM, use_color)]
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        vec![format!("↳ {}", self.line)]
    }

    fn symbol(&self) -> Option<String> {
        self.symbol.clone()
    }
}
/// The test-integrity pass — additive `Finding`s from diffing both sides of the
/// change's test files into gaming events, gated by the repo's own learned
/// event gates (`.argot/integrity.json`). Runs beside the statistical
/// scorers; a graceful no-op when the changeset carries no tests. Reasons
/// `test_deleted` / `test_disabled` / `test_weakened`.
pub(super) fn integrity_hits(
    args: &CheckArgs,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    registry: &argot_engine::rules::Registry,
    settings: &argot_engine::rules::RuleSettings,
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::model::{changeset_events, IntegrityModel, INTEGRITY_FILE};

    let model = match std::fs::read_to_string(args.argot_dir.join(INTEGRITY_FILE)) {
        Ok(raw) => match IntegrityModel::from_json(&raw) {
            Some(m) => m,
            None => {
                stderr.push_str(
                    "[argot] integrity gates unreadable — run `argot fit` to restore the test-integrity rules\n",
                );
                return Vec::new();
            }
        },
        // No artifact (an older fit): the built-in default gates apply.
        Err(_) => IntegrityModel::permissive(),
    };

    let mut hits = Vec::new();
    for (source, files) in integrity_changesets(args) {
        for ev in changeset_events(&files) {
            if !model.enabled(ev.kind) {
                continue;
            }
            let reason = ev.kind.reason();
            let hash = hit_hash(&ev.file, reason, &ev.hash_content());
            // Display body: the post-image line the event anchors to (the
            // hash above never depends on it).
            let hunk_content = files
                .iter()
                .find(|f| f.path == ev.file)
                .and_then(|f| f.new.as_deref())
                .and_then(|src| src.lines().nth(ev.line.saturating_sub(1)))
                .unwrap_or_default()
                .to_string();
            let suppressed_by = {
                let new_side = files
                    .iter()
                    .find(|f| f.path == ev.file)
                    .and_then(|f| f.new.as_deref());
                let suppressions = FileSuppressions::parse(
                    &ev.file,
                    new_side.unwrap_or_default(),
                    new_side.and(
                        ext_to_lang(&extension(&ev.file))
                            .and_then(|l| filter_adapters.get(l))
                            .map(|a| a.line_comment_prefix()),
                    ),
                    mute_rules,
                    false,
                    registry,
                    settings,
                );
                suppressions.classify(reason, &hash, ev.line, ev.line)
            };
            hits.push(Finding {
                score: 1.0,
                file_path: ev.file.clone(),
                line: ev.line,
                line_end: ev.line,
                source: source.clone(),
                reason: reason.to_string(),
                flagged: true,
                threshold: 0.5,
                hunk_content,
                evidence: Some(Box::new(IntegrityEvidence {
                    line: ev.evidence(),
                    symbol: (!ev.test_name.is_empty()).then(|| ev.test_name.clone()),
                })),
                hash,
                suppressed_by,
            });
        }
    }
    hits
}
/// The integrity group's detection pass.
pub struct IntegrityDetector;

impl Detector for IntegrityDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_INTEGRITY
    }

    fn timing_label(&self) -> &'static str {
        "check: integrity pass"
    }

    /// Test-integrity gates (`.argot/integrity.json`), a sibling of
    /// scorer-config.json so the base config is byte-for-byte unchanged. A
    /// mini-replay over the repo's accepted-history window measures each
    /// gaming event's natural rate and disables the classes this repo's
    /// normal development trips too often (FP-first; see the module docs of
    /// `scoring::integrity`).
    fn fit(&mut self, ctx: &argot_engine::detector::FitContext<'_>) {
        // Self-gated: an off group writes no artifact and pays no cost.
        if !self.enabled(ctx.settings) {
            return;
        }
        let _t = argot_engine::timing::phase("calibrate: integrity mini-replay");
        use crate::model::{fit_model, INTEGRITY_FILE};
        if let Some(model) = fit_model(ctx.repo_dir, ctx.repo_sha) {
            let path = ctx.output.with_file_name(INTEGRITY_FILE);
            if let Err(e) = argot_engine::artifact::write_atomic(&path, model.to_json().as_bytes())
            {
                eprintln!("argot: writing integrity gates failed: {e}");
            }
        }
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        integrity_hits(
            ctx.args,
            ctx.filter_adapters,
            ctx.mute_rules,
            ctx.registry,
            ctx.settings,
            ctx.stderr,
        )
    }
}
