//! The `superseded` rule's check-time scan — flag newly-written code that
//! uses a pattern this repo has been replacing (a mined supersession) or has
//! declared migrated away from (`[[migration]]`).
//!
//! Fires on added hunk lines only, once per pattern per changeset (using a
//! replaced dependency across ten files is one decision, like the
//! foreign-import dedup). Evidence is the mining observation itself — real
//! commits, dates, counts — or the declared migration's reason.

use std::collections::{HashMap, HashSet};

use argot_engine::check::render::{paint, C_DIM};
use argot_engine::check::PatchBatch;
use argot_engine::config::{MigrationKind, MigrationRule};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::rules::{Registry, RuleSettings, Severity};
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::adapters::{Language, LanguageAdapter};
use argot_lang::callees::non_none_callees;
use argot_lang::ext::{ext_to_lang, ext_to_lang_ctx, extension};
use argot_lang::text::splitlines;

use crate::scoring::supersede::{Supersession, SupersessionKind};

/// The finding's evidence: one pre-rendered line (mined observation or
/// declared reason) plus the superseded pattern as the machine `symbol`.
struct SupersededEvidence {
    line: String,
    old: String,
}

impl RenderEvidence for SupersededEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        vec![paint(&format!("    ↳ {}", self.line), C_DIM, use_color)]
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        vec![format!("↳ {}", self.line)]
    }

    fn symbol(&self) -> Option<String> {
        Some(self.old.clone())
    }
}

/// One pattern the scan watches for, whichever surface produced it.
struct Watch<'a> {
    old: &'a str,
    kind: SupersessionKind,
    evidence: String,
}

fn mined_evidence(s: &Supersession) -> String {
    format!(
        "this repo replaced '{}' with '{}' — {} commits across {} files ({}..{}, e.g. {})",
        s.old, s.new, s.commits, s.files, s.first, s.last, s.example_commit
    )
}

fn declared_evidence(m: &MigrationRule) -> String {
    format!("argot.toml migration to '{}' — {}", m.to, m.reason)
}

fn migration_kind(kind: MigrationKind) -> SupersessionKind {
    match kind {
        MigrationKind::Import => SupersessionKind::Import,
        MigrationKind::Callee => SupersessionKind::Callee,
    }
}

/// Scan the changeset's added hunks for superseded patterns. `supersessions`
/// is the fitted model's per-language mined list; `migrations` the declared
/// `[[migration]]` set (language-blind — a specifier only matches where it
/// appears). Returns at most one finding per pattern per run.
#[allow(clippy::too_many_arguments)]
pub(crate) fn superseded_findings(
    batches: &[PatchBatch],
    supersessions: &HashMap<String, Vec<Supersession>>,
    migrations: &[MigrationRule],
    filter_adapters: &HashMap<String, Box<dyn LanguageAdapter>>,
    mute_rules: &[SuppressionRule],
    registry: &Registry,
    settings: &RuleSettings,
    header_cpp: bool,
) -> Vec<Finding> {
    if settings.severity_of_reason("superseded") == Severity::Off {
        return Vec::new();
    }
    let mut findings: Vec<Finding> = Vec::new();
    let mut alerted: HashSet<String> = HashSet::new();

    for batch in batches {
        let ext = extension(&batch.file_path);
        let Some(lang) = ext_to_lang_ctx(&ext, header_cpp) else {
            continue;
        };
        let Some(adapter) = filter_adapters.get(lang) else {
            continue;
        };
        let mut watches: Vec<Watch<'_>> = Vec::new();
        if let Some(mined) = supersessions.get(lang) {
            watches.extend(mined.iter().map(|s| Watch {
                old: &s.old,
                kind: s.kind,
                evidence: mined_evidence(s),
            }));
        }
        watches.extend(migrations.iter().map(|m| Watch {
            old: &m.from,
            kind: migration_kind(m.kind),
            evidence: declared_evidence(m),
        }));
        if watches.is_empty() {
            continue;
        }

        let file_source = String::from_utf8_lossy(&batch.content).into_owned();
        let file_lines = splitlines(&file_source);
        let n_lines = file_lines.len();
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

        for hunk in &batch.hunks {
            let hs = (hunk.new_start as usize).saturating_sub(1);
            if hs >= n_lines {
                continue;
            }
            let he = (hs + hunk.new_lines as usize).min(n_lines);
            let hunk_content = file_lines[hs..he].join("\n");

            let hunk_imports: HashSet<String> = adapter.extract_imports(&hunk_content);
            let hunk_callees: HashSet<String> = Language::from_scoring_name(lang)
                .map(|l| non_none_callees(&hunk_content, l).into_iter().collect())
                .unwrap_or_default();

            for watch in &watches {
                let present = match watch.kind {
                    SupersessionKind::Import => hunk_imports.contains(watch.old),
                    SupersessionKind::Callee => hunk_callees.contains(watch.old),
                };
                if !present || !alerted.insert(watch.old.to_string()) {
                    continue;
                }
                let offset = file_lines[hs..he]
                    .iter()
                    .position(|l| l.contains(watch.old))
                    .unwrap_or(0);
                let line = hs + offset + 1;
                let hash = hit_hash(&batch.file_path, "superseded", &hunk_content);
                let suppressed_by = suppressions.classify("superseded", &hash, line, line);
                findings.push(Finding {
                    score: 1.0,
                    file_path: batch.file_path.clone(),
                    line,
                    line_end: line,
                    source: batch.source.clone(),
                    reason: "superseded".to_string(),
                    flagged: true,
                    threshold: 0.5,
                    hunk_content: hunk_content.clone(),
                    evidence: Some(Box::new(SupersededEvidence {
                        line: watch.evidence.clone(),
                        old: watch.old.to_string(),
                    })),
                    hash,
                    suppressed_by,
                });
            }
        }
    }
    findings
}

/// The superseded patterns one hunk uses — the single-hunk variant of the
/// changeset scan, for surfaces that score agent-supplied hunks (the MCP
/// server). Returns `(old, new, evidence)` per match.
pub(crate) fn hunk_matches(
    lang: &str,
    hunk_content: &str,
    supersessions: &[Supersession],
    migrations: &[MigrationRule],
) -> Vec<(String, String, String)> {
    let Some(adapter) = argot_lang::adapters::adapter_for(lang) else {
        return Vec::new();
    };
    let imports = adapter.extract_imports(hunk_content);
    let callees: HashSet<String> = Language::from_scoring_name(lang)
        .map(|l| non_none_callees(hunk_content, l).into_iter().collect())
        .unwrap_or_default();
    let uses = |kind: SupersessionKind, old: &str| match kind {
        SupersessionKind::Import => imports.contains(old),
        SupersessionKind::Callee => callees.contains(old),
    };
    let mut out = Vec::new();
    for s in supersessions {
        if uses(s.kind, &s.old) {
            out.push((s.old.clone(), s.new.clone(), mined_evidence(s)));
        }
    }
    for m in migrations {
        if uses(migration_kind(m.kind), &m.from) {
            out.push((m.from.clone(), m.to.clone(), declared_evidence(m)));
        }
    }
    out
}

#[cfg(test)]
mod tests;
