//! The guardrail's self-protection — the `rule-tampered` rule (group
//! `governance`).
//!
//! A locked rule is only as strong as the files that define it. This pass
//! reads both sides of the changeset for the sensitive surfaces —
//! `argot.toml` and `.argot/rules/<name>/` — and fires an **error** finding
//! (pinned severity, unsuppressable) when the change itself weakens a rule
//! that was locked *before* the change:
//!
//! - the lock removed, or the locked rule's committed severity weakened;
//! - a `[[mute]]` added targeting a locked rule;
//! - a locked custom rule's manifest or script edited or deleted.
//!
//! Tamper-evident, not tamper-proof: an agent can touch the alarm, but
//! touching the alarm *is* the alarm. The only quiet way to relax a locked
//! rule is a committed `argot.toml` diff a human reviews.

use super::two_sided::{collect_two_sided, FileChange};
use super::CheckArgs;
use crate::finding::{Finding, RenderEvidence};
use crate::rules::Severity;
use crate::suppress::hit_hash;

/// The committed config file this pass watches.
const CONFIG_FILE: &str = "argot.toml";
/// The custom-rules directory prefix (repo-relative).
const RULES_DIR_PREFIX: &str = ".argot/rules/";

/// Rendered evidence: one pre-formatted line naming what was weakened.
struct TamperEvidence(String);

impl RenderEvidence for TamperEvidence {
    fn human(&self, use_color: bool, _hunk_start_line: usize) -> Vec<String> {
        vec![super::render::paint(
            &format!("    ↳ {}", self.0),
            super::render::C_DIM,
            use_color,
        )]
    }

    fn machine(&self, _hunk_start_line: usize) -> Vec<String> {
        vec![format!("↳ {}", self.0)]
    }
}

/// Locks + per-selector committed severities + rule-scoped mute counts,
/// parsed from one side of `argot.toml`.
#[derive(Default)]
struct LockState {
    /// selector → committed severity string ("error" when unspecified).
    locked: std::collections::HashMap<String, String>,
    /// mute-rule selector → count of `[[mute]]` entries scoped to it.
    mutes: std::collections::HashMap<String, usize>,
}

fn lock_state(toml_text: &str) -> LockState {
    let mut state = LockState::default();
    let Ok(doc) = toml_text.parse::<toml::Table>() else {
        return state;
    };
    if let Some(rules) = doc.get("rules").and_then(|r| r.as_table()) {
        for (key, value) in rules {
            if let Some(t) = value.as_table() {
                if t.get("locked").and_then(|l| l.as_bool()) == Some(true) {
                    let sev = t
                        .get("severity")
                        .and_then(|s| s.as_str())
                        .unwrap_or("error")
                        .to_string();
                    state.locked.insert(key.clone(), sev);
                }
            }
        }
    }
    if let Some(mutes) = doc.get("mute").and_then(|m| m.as_array()) {
        for entry in mutes {
            if let Some(rule) = entry.get("rule").and_then(|r| r.as_str()) {
                *state.mutes.entry(rule.to_string()).or_insert(0) += 1;
            }
        }
    }
    state
}

/// Does `selector` (a lock key) cover `mute_selector`? Exact match, or the
/// lock is a group covering the muted rule.
fn lock_covers(lock: &str, selector: &str) -> bool {
    if lock == selector {
        return true;
    }
    // A group lock covers its rules. For built-ins that's the registry group;
    // for the `custom` group, any selector that isn't a built-in rule or a
    // known group name is a custom rule the group lock owns.
    if lock == crate::rules::GROUP_CUSTOM {
        return crate::rules::rule_named(selector).is_none() && !crate::rules::is_group(selector);
    }
    crate::rules::rule_named(selector).is_some_and(|r| r.group == lock)
}

/// How much weaker is `new` than `old`? (`error` > `warn` > `off`.)
fn weakened(old: &str, new: &str) -> bool {
    let rank = |s: &str| match Severity::parse(s) {
        Some(Severity::Error) => 2,
        Some(Severity::Warn) => 1,
        Some(Severity::Off) => 0,
        None => 2, // unparseable reads as strict — never a false tamper
    };
    rank(new) < rank(old)
}

fn finding(path: &str, line: usize, body: String, evidence: String) -> Finding {
    let hash = hit_hash(path, "rule_tampered", &body);
    Finding {
        score: 1.0,
        file_path: path.to_string(),
        line,
        line_end: line,
        source: "workdir".to_string(),
        reason: "rule_tampered".to_string(),
        flagged: true,
        threshold: 0.5,
        hunk_content: body,
        evidence: Some(Box::new(TamperEvidence(evidence))),
        hash,
        suppressed_by: None, // pinned: never suppressable
    }
}

/// The committed `argot.toml` at HEAD (the lock authority when the config
/// file itself isn't part of the change being checked).
fn head_config(repo_path: &str) -> Option<String> {
    let repo = crate::git_walk::open_repo(repo_path).ok()?;
    let tree = repo.head().ok()?.peel_to_tree().ok()?;
    let entry = tree.get_path(std::path::Path::new(CONFIG_FILE)).ok()?;
    let blob = repo.find_blob(entry.id()).ok()?;
    Some(String::from_utf8_lossy(blob.content()).to_string())
}

/// 1-indexed line of the first occurrence of `needle` in `text` (1 fallback).
fn line_of(text: &str, needle: &str) -> usize {
    text.lines()
        .position(|l| l.contains(needle))
        .map(|i| i + 1)
        .unwrap_or(1)
}

/// The tamper pass. Reads both sides of the sensitive surfaces in the same
/// changeset `check` is judging; the OLD side's lock set is the authority —
/// what this change found locked, this change must not weaken.
pub(crate) fn tamper_findings(args: &CheckArgs) -> Vec<Finding> {
    let changesets = collect_two_sided(args, &|path| {
        path == CONFIG_FILE || path.starts_with(RULES_DIR_PREFIX)
    });
    // The lock authority when the config file isn't itself in the diff:
    // the repo's committed (HEAD) argot.toml — an edit to a locked custom
    // rule's script must be caught even in a code-only change.
    let head_locks = head_config(&args.repo_path).map(|t| lock_state(&t));

    let mut findings = Vec::new();
    for (source, files) in changesets {
        let config_change = files.iter().find(|f| f.path == CONFIG_FILE);
        let old_state = config_change
            .and_then(|f| f.old.as_deref())
            .map(lock_state)
            .or_else(|| {
                head_locks.as_ref().map(|s| LockState {
                    locked: s.locked.clone(),
                    mutes: s.mutes.clone(),
                })
            })
            .unwrap_or_default();
        if let Some(FileChange {
            old: Some(old),
            new,
            ..
        }) = config_change
        {
            let new_text = new.as_deref().unwrap_or_default();
            let new_state = lock_state(new_text);
            for (selector, old_sev) in &old_state.locked {
                match new_state.locked.get(selector) {
                    None => findings.push(finding(
                        CONFIG_FILE,
                        line_of(old, selector),
                        format!("[rules] {selector}: lock removed"),
                        format!(
                            "'{selector}' was locked before this change — unlocking is a \
                             reviewable decision, not a fix"
                        ),
                    )),
                    Some(new_sev) if weakened(old_sev, new_sev) => findings.push(finding(
                        CONFIG_FILE,
                        line_of(new_text, selector),
                        format!("[rules] {selector}: locked severity {old_sev} → {new_sev}"),
                        format!(
                            "'{selector}' is locked — its severity only moves in a reviewed diff"
                        ),
                    )),
                    _ => {}
                }
                let old_mutes: usize = old_state
                    .mutes
                    .iter()
                    .filter(|(s, _)| lock_covers(selector, s))
                    .map(|(_, n)| n)
                    .sum();
                let new_mutes: usize = new_state
                    .mutes
                    .iter()
                    .filter(|(s, _)| lock_covers(selector, s))
                    .map(|(_, n)| n)
                    .sum();
                if new_mutes > old_mutes {
                    findings.push(finding(
                        CONFIG_FILE,
                        line_of(new_text, "[[mute]]"),
                        format!("[[mute]] added on locked rule '{selector}'"),
                        format!("mutes are refused on locked rules — '{selector}' stays enforced"),
                    ));
                }
            }
        }
        // A locked custom rule's own files: any edit or deletion while the
        // OLD side's config locked it.
        for f in &files {
            let Some(rest) = f.path.strip_prefix(RULES_DIR_PREFIX) else {
                continue;
            };
            let Some(rule_name) = rest.split('/').next() else {
                continue;
            };
            let was_locked = old_state
                .locked
                .keys()
                .any(|l| l == rule_name || l == crate::rules::GROUP_CUSTOM);
            if !was_locked || f.old.is_none() {
                continue; // not locked before, or a brand-new file (authoring)
            }
            if f.new.as_deref() != f.old.as_deref() {
                findings.push(finding(
                    &f.path,
                    1,
                    format!(
                        "locked custom rule '{rule_name}': {} {}",
                        if f.new.is_none() { "deleted" } else { "edited" },
                        f.path
                    ),
                    format!(
                        "'{rule_name}' is locked — changing its definition alongside code \
                         is the move this rule exists to catch"
                    ),
                ));
            }
        }
        // Per-commit changesets label their source; keep it.
        for finding in findings.iter_mut().rev() {
            if finding.source == "workdir" && source != "workdir" {
                finding.source = source.clone();
            } else {
                break;
            }
        }
    }
    findings
}

#[cfg(test)]
mod tests;
