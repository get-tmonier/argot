//! The scripted rule group's detection pass.
//!
//! Discovery happens in the [`Detector::vocabulary`] lifecycle step (before
//! config validation, so custom `[rules]` keys resolve); scripts compile in
//! [`Detector::load`]; [`Detector::check`] runs each rule over each in-scope
//! file. A rule that trips the sandbox (operation cap, wall clock, runtime
//! error) is disabled for the rest of the run with one diagnostic — one bad
//! rule never takes down the check.

use crate::host;
use crate::manifest::ScriptRule;
use argot_engine::detector::{CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::rules::{self, CustomRule};
use argot_engine::suppress::{hit_hash, FileSuppressions};
use argot_lang::ext::{ext_to_lang_ctx, extension};
use std::collections::HashSet;

/// One scripted rule's rendered evidence: the lines its `report_span` opts
/// carried, plus the optional symbol.
struct ScriptEvidence {
    lines: Vec<String>,
    symbol: Option<String>,
}

impl RenderEvidence for ScriptEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        self.lines
            .iter()
            .map(|l| {
                argot_engine::check::render::paint(
                    &format!("    ↳ {l}"),
                    argot_engine::check::render::C_DIM,
                    use_color,
                )
            })
            .collect()
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        self.lines.iter().map(|l| format!("↳ {l}")).collect()
    }

    fn symbol(&self) -> Option<String> {
        self.symbol.clone()
    }
}

/// The scripted rule group (`custom`): community rules from `.argot/rules/`.
#[derive(Default)]
pub struct ScriptDetector {
    rules: Vec<ScriptRule>,
    /// Compiled ASTs, one per rule (indices match `rules`).
    compiled: Vec<Option<rhai::AST>>,
    /// Rules disabled for this run (sandbox trips, runtime errors).
    disabled: HashSet<String>,
}

impl ScriptDetector {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Detector for ScriptDetector {
    fn group(&self) -> &'static str {
        rules::GROUP_CUSTOM
    }

    fn timing_label(&self) -> &'static str {
        "check: scripted rules pass"
    }

    fn wants_unscored_files(&self) -> bool {
        // Only if some rule has `files` globs — otherwise every rule is
        // language-gated and the extra collection is wasted.
        self.rules.iter().any(|r| !r.files.is_empty())
    }

    /// Discover `.argot/rules/` and contribute the custom vocabulary. Also
    /// remembers the rules for `check` — discovery reads each manifest and
    /// script exactly once per run.
    fn vocabulary(
        &mut self,
        argot_dir: &std::path::Path,
        warnings: &mut Vec<String>,
    ) -> Vec<CustomRule> {
        self.rules = crate::discover::discover(argot_dir, warnings);
        self.rules.iter().map(|r| r.custom_rule()).collect()
    }

    /// Compile every discovered script once; a rule that fails to compile is
    /// disabled with a diagnostic (its vocabulary entry stays — the name is
    /// still addressable in config, it just produces nothing this run).
    fn load(
        &mut self,
        _argot_dir: &std::path::Path,
        _detect: &argot_engine::config::DetectConfig,
    ) -> Result<(), (String, i32)> {
        self.compiled = self
            .rules
            .iter()
            .map(|rule| match host::compile(&rule.script) {
                Ok(ast) => Some(ast),
                Err(e) => {
                    self.disabled.insert(rule.name.clone());
                    eprintln!(
                        "argot: custom rule {}: script failed to compile — rule disabled: {e}",
                        rule.name
                    );
                    None
                }
            })
            .collect();
        Ok(())
    }

    fn check(&mut self, ctx: &mut CheckContext<'_>) -> Vec<Finding> {
        if self.rules.is_empty() {
            return Vec::new();
        }
        let changeset_paths: Vec<String> =
            ctx.batches.iter().map(|b| b.file_path.clone()).collect();
        // Pre-images for `file.old_text` / `ts_query_old`, keyed by
        // (source label, path) — the same labels the batches carry. A miss
        // (label mismatch, unresolvable side) degrades to no old side.
        let old_sides: std::collections::HashMap<(String, String), String> =
            argot_engine::check::two_sided::collect_two_sided(ctx.args, &|path| {
                let language = ext_to_lang_ctx(&extension(path), ctx.header_cpp);
                self.rules.iter().any(|r| r.covers_file(path, language))
            })
            .into_iter()
            .flat_map(|(source, files)| {
                files
                    .into_iter()
                    .filter_map(move |f| f.old.map(|old| ((source.clone(), f.path), old)))
            })
            .collect();
        let mut findings = Vec::new();
        // Scored batches plus the unscored ones (unsupported extensions —
        // `.env`, CI configs…): a rule's `files` globs may claim the latter.
        for batch in ctx.batches.iter().chain(ctx.extra_batches) {
            let language = ext_to_lang_ctx(&extension(&batch.file_path), ctx.header_cpp);
            let source = String::from_utf8_lossy(&batch.content).into_owned();
            let hunk_ranges: Vec<(usize, usize)> = batch
                .hunks
                .iter()
                .map(|h| {
                    let start = h.new_start as usize;
                    (
                        start,
                        (h.new_start + h.new_lines).saturating_sub(1) as usize,
                    )
                })
                .collect();
            let file = host::FileInput {
                path: &batch.file_path,
                language: language.unwrap_or(""),
                new_text: &source,
                old_text: old_sides
                    .get(&(batch.source.clone(), batch.file_path.clone()))
                    .map(String::as_str),
                hunks: &hunk_ranges,
            };
            // The file's suppression surfaces, once per file (shared by every
            // rule's findings on it).
            let suppressions = FileSuppressions::parse(
                &batch.file_path,
                &source,
                language
                    .and_then(|l| ctx.filter_adapters.get(l))
                    .map(|a| a.line_comment_prefix()),
                ctx.mute_rules,
                batch.ignored_by_pattern,
                ctx.registry,
                ctx.settings,
            );
            for (i, rule) in self.rules.iter().enumerate() {
                if self.disabled.contains(&rule.name)
                    || !rule.covers_file(&batch.file_path, language)
                {
                    continue;
                }
                let reason = format!("{}{}", rules::CUSTOM_REASON_PREFIX, rule.name);
                // The rule may be individually `off` while the group runs.
                if ctx.settings.severity_of_reason(&reason) == rules::Severity::Off {
                    continue;
                }
                let Some(ast) = self.compiled.get(i).and_then(|a| a.as_ref()) else {
                    continue;
                };
                match host::run_on_file(ast, &file, changeset_paths.clone(), ctx.facts.clone()) {
                    Ok(reports) => {
                        for r in reports {
                            // The finding body is the reported source span
                            // (like every other rule); the script's message
                            // leads the evidence lines. Hashing the span —
                            // not the message — keeps mutes stable across
                            // rule-message rewording.
                            let hunk_content = source
                                .lines()
                                .skip(r.line.saturating_sub(1))
                                .take(r.line_end.saturating_sub(r.line) + 1)
                                .collect::<Vec<_>>()
                                .join("\n");
                            let hash = hit_hash(&batch.file_path, &reason, &hunk_content);
                            let suppressed_by =
                                suppressions.classify(&reason, &hash, r.line, r.line_end);
                            let mut lines = vec![r.message];
                            lines.extend(r.evidence);
                            findings.push(Finding {
                                score: 1.0,
                                file_path: batch.file_path.clone(),
                                line: r.line,
                                line_end: r.line_end,
                                source: batch.source.clone(),
                                reason: reason.clone(),
                                flagged: true,
                                threshold: 0.5,
                                hunk_content,
                                evidence: Some(Box::new(ScriptEvidence {
                                    lines,
                                    symbol: r.symbol,
                                })),
                                hash,
                                suppressed_by,
                            });
                        }
                    }
                    Err(e) => {
                        self.disabled.insert(rule.name.clone());
                        ctx.stderr.push_str(&format!(
                            "[argot] custom rule {}: {} — rule disabled for this run\n",
                            rule.name,
                            e.trim_end()
                        ));
                    }
                }
            }
        }
        findings
    }
}

#[cfg(test)]
mod tests;
