//! The architecture-graph pass: flags an internal module-dependency import
//! that reverses the repo's learned layer direction, closes a cycle, or
//! leaves a (near-)sink. Group `architecture`, reason `layering`.

use argot_engine::check::render::{paint, C_DIM};
use argot_engine::check::PatchBatch;
use argot_engine::detector::{CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::rules;
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::adapters::LanguageAdapter;
use argot_lang::ext::{ext_to_lang, extension};
use std::collections::HashMap;
use std::path::Path;

#[cfg(test)]
mod tests;

/// The architecture-graph pass — additive `Finding`s from the per-repo
/// module-dependency graph (`.argot/layering.json`). For each changed file it
/// takes the ADDED lines, resolves the internal import edges they introduce, and
/// flags any that reverse an established layer direction or leave a (near-)sink —
/// a boundary the repo never crosses. Runs alongside the statistical scorers,
/// never through them; empty (graceful degrade) when the graph is absent, so the
/// base guardrail is entirely unaffected. Reason code `layering`.
fn arch_hits(
    patches: &[PatchBatch],
    argot_dir: &Path,
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    registry: &argot_engine::rules::Registry,
    settings: &argot_engine::rules::RuleSettings,
    stderr: &mut String,
) -> Vec<Finding> {
    use crate::graph::{RepoLayering, LAYERING_FILE};
    let Ok(raw) = std::fs::read_to_string(argot_dir.join(LAYERING_FILE)) else {
        return Vec::new();
    };
    let Some(graph) = RepoLayering::from_json(&raw) else {
        stderr.push_str("[argot] layering graph unreadable\n");
        return Vec::new();
    };
    let mut hits = Vec::new();
    for batch in patches {
        if batch.ignored_by_pattern || !batch.file_path.ends_with(".py") {
            continue; // v1: Python resolver only
        }
        let source = String::from_utf8_lossy(&batch.content);
        let lines: Vec<&str> = source.lines().collect();
        // Concatenate the ADDED lines (1-indexed) — the imports the diff introduces.
        let mut added = String::new();
        let mut first_line = 0usize;
        for h in &batch.hunks {
            for l in h.new_start..(h.new_start + h.new_lines) {
                if let Some(t) = lines.get((l as usize).saturating_sub(1)) {
                    if first_line == 0 {
                        first_line = l as usize;
                    }
                    added.push_str(t);
                    added.push('\n');
                }
            }
        }
        if added.is_empty() {
            continue;
        }
        // Fire if the added imports create a novel reversal/sink-out edge —
        // and keep that edge: the evidence line names the direction it breaks.
        let Some((edge, violation)) = graph
            .file_edges(&batch.file_path, &added)
            .iter()
            .find_map(|e| graph.classify(e).map(|v| (e.clone(), v)))
        else {
            continue;
        };
        let hunk_content = added.clone();
        let hash = hit_hash(&batch.file_path, "layering", &hunk_content);
        let suppressions = FileSuppressions::parse(
            &batch.file_path,
            &source,
            ext_to_lang(&extension(&batch.file_path))
                .and_then(|l| filter_adapters.get(l))
                .map(|a| a.line_comment_prefix()),
            mute_rules,
            false, // ignored-by-pattern batches were skipped above
            registry,
            settings,
        );
        let suppressed_by = suppressions.classify("layering", &hash, first_line, first_line);
        hits.push(Finding {
            score: 1.0,
            file_path: batch.file_path.clone(),
            line: first_line,
            line_end: first_line,
            source: batch.source.clone(),
            reason: "layering".to_string(),
            flagged: true,
            threshold: 0.5,
            hunk_content,
            evidence: Some(Box::new(ArchEvidence(arch_evidence(&edge, violation)))),
            hash,
            suppressed_by,
        });
    }
    hits
}
/// The rendered evidence of a `layering` finding — one pre-formatted line
/// naming the established direction the novel edge violates.
struct ArchEvidence(String);

impl RenderEvidence for ArchEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        vec![paint(&format!("    ↳ {}", self.0), C_DIM, use_color)]
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        vec![format!("↳ {}", self.0)]
    }
}
/// The architecture group's detection pass.
pub struct ArchDetector;

impl Detector for ArchDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_ARCHITECTURE
    }

    fn timing_label(&self) -> &'static str {
        "check: arch pass"
    }

    /// Architecture-graph artifact (`.argot/layering.json`), a sibling of
    /// scorer-config.json so the base config is byte-for-byte unchanged
    /// whether or not the layer is compiled in. Built from the same
    /// voice-file collection production fits on (config-respecting) —
    /// Python only in v1; other languages simply produce no graph.
    fn fit(&mut self, ctx: &argot_engine::detector::FitContext<'_>) {
        // Self-gated: an off group writes no artifact and pays no cost.
        if !self.enabled(ctx.settings) {
            return;
        }
        let _t = argot_engine::timing::phase("calibrate: arch graph");
        use crate::graph::{RepoLayering, LAYERING_FILE};
        use argot_lang::adapters::Language;
        let files = argot_engine::corpus::collect_source_files(ctx.repo_dir);
        let mut sources: Vec<(String, String)> = Vec::new();
        for abs in &files {
            if abs.extension().and_then(|e| e.to_str()) != Some("py") {
                continue;
            }
            if let (Ok(rel), Ok(src)) =
                (abs.strip_prefix(ctx.repo_dir), std::fs::read_to_string(abs))
            {
                sources.push((rel.to_string_lossy().replace('\\', "/"), src));
            }
        }
        let graph = RepoLayering::fit(
            sources.iter().map(|(p, s)| (p.as_str(), s.as_str())),
            Language::Python,
        );
        if graph.edge_count() > 0 {
            let path = ctx.output.with_file_name(LAYERING_FILE);
            if let Err(e) =
                argot_engine::artifact::write_atomic(&path, graph.to_json(ctx.repo_sha).as_bytes())
            {
                eprintln!("argot: writing layering graph failed: {e}");
            }
        } else if sources.is_empty() {
            // Make the abstention visible so a quiet `layering` isn't mistaken
            // for a clean bill of health.
            eprintln!(
                "argot: layering rule (v1) analyzes Python only — no Python source found, \
                 so it will not run for this repo."
            );
        } else {
            eprintln!(
                "argot: layering found no confident layer order (a flat, or facade/barrel-heavy, \
                 module graph) — the rule will not run for this repo."
            );
        }
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        arch_hits(
            ctx.batches,
            &ctx.args.argot_dir,
            ctx.filter_adapters,
            ctx.mute_rules,
            ctx.registry,
            ctx.settings,
            ctx.stderr,
        )
    }
}
/// The evidence line for a `layering` finding: name the established direction
/// the novel edge `(a, b)` breaks, in the repo's own module vocabulary.
pub(super) fn arch_evidence(
    edge: &crate::graph::Edge,
    violation: crate::graph::Violation,
) -> String {
    use crate::graph::Violation;
    let (a, b) = edge;
    match violation {
        Violation::Reversal => {
            format!("{b} → {a} is this repo's direction — this import reverses it")
        }
        Violation::TransitiveReversal => format!(
            "{b} already depends on {a} — this import closes a cycle against the repo's layering"
        ),
        Violation::SinkOut => {
            format!("{a} is a module this repo never imports out of — this import leaves it")
        }
    }
}
