//! The test-integrity pass: diffs each changeset's test files into gaming
//! events (`test-deleted` / `test-disabled` / `test-weakened`), gated by the
//! repo's own learned event gates. Group `integrity`.

use argot_engine::check::render::{paint, C_DIM};
use argot_engine::check::CheckArgs;
use argot_engine::detector::{CheckContext, Detector};
use argot_engine::finding::{Finding, RenderEvidence};
use argot_engine::git_walk::{open_repo, resolve_shas};
use argot_engine::rules;
use argot_engine::suppress::{hit_hash, FileSuppressions, SuppressionRule};
use argot_lang::adapters::LanguageAdapter;
use argot_lang::ext::{ext_to_lang, extension};
use git2::DiffFindOptions;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::Path;

#[cfg(test)]
mod tests;

/// Collect the test-integrity pass's changesets: both sides of every changed
/// source file (renames resolved) **including deletions**, which the scoring
/// `PatchBatch` path never carries. Mirrors `collect_patches`' mode dispatch;
/// an explicit commit set yields one changeset per commit so the event
/// refinements reason about each accepted unit separately. Every changeset is
/// labelled with its display source (`workdir` / `staged` / short SHA).
fn integrity_changesets(args: &CheckArgs) -> Vec<(String, Vec<crate::model::FileChange>)> {
    use crate::model::FileChange;
    use crate::test_inventory::language_for_path;

    const MAX_BLOB: usize = 400_000;

    fn tree_text(repo: &git2::Repository, tree: &git2::Tree, path: &str) -> Option<String> {
        let entry = tree.get_path(Path::new(path)).ok()?;
        let blob = repo.find_blob(entry.id()).ok()?;
        (blob.size() <= MAX_BLOB).then(|| String::from_utf8_lossy(blob.content()).to_string())
    }
    fn workdir_text(repo: &git2::Repository, path: &str) -> Option<String> {
        let full = repo.workdir()?.join(path);
        let data = fs::read(&full).ok()?;
        (data.len() <= MAX_BLOB).then(|| String::from_utf8_lossy(&data).to_string())
    }
    fn index_text(repo: &git2::Repository, path: &str) -> Option<String> {
        let index = repo.index().ok()?;
        let entry = index.get_path(Path::new(path), 0)?;
        let blob = repo.find_blob(entry.id).ok()?;
        (blob.size() <= MAX_BLOB).then(|| String::from_utf8_lossy(blob.content()).to_string())
    }
    fn changes_from_diff(
        diff: &mut git2::Diff,
        old_side: &dyn Fn(&str) -> Option<String>,
        new_side: &dyn Fn(&str) -> Option<String>,
    ) -> Vec<FileChange> {
        let _ = diff.find_similar(Some(&mut DiffFindOptions::new()));
        let mut out = Vec::new();
        for d in diff.deltas() {
            let new_path = d
                .new_file()
                .path()
                .and_then(|p| p.to_str())
                .map(str::to_string);
            let old_path = d
                .old_file()
                .path()
                .and_then(|p| p.to_str())
                .map(str::to_string);
            let path = new_path
                .clone()
                .or_else(|| old_path.clone())
                .unwrap_or_default();
            if language_for_path(&path).is_none() {
                continue;
            }
            let old = match d.status() {
                git2::Delta::Added | git2::Delta::Untracked => None,
                _ => old_path.as_deref().and_then(old_side),
            };
            let new = match d.status() {
                git2::Delta::Deleted => None,
                _ => new_path.as_deref().and_then(new_side),
            };
            if old.is_none() && new.is_none() {
                continue;
            }
            out.push(FileChange { path, old, new });
        }
        out
    }
    fn one(source: &str, cs: Vec<FileChange>) -> Vec<(String, Vec<FileChange>)> {
        if cs.is_empty() {
            Vec::new()
        } else {
            vec![(source.to_string(), cs)]
        }
    }

    let repo_path = args.repo_path.as_str();
    let Ok(repo) = open_repo(repo_path) else {
        return Vec::new();
    };
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    let ref_nonempty = !args.reference.is_empty();

    let per_commit = |shas: &HashSet<String>| -> Vec<(String, Vec<FileChange>)> {
        let mut out = Vec::new();
        for sha in shas {
            let Ok(oid) = git2::Oid::from_str(sha) else {
                continue;
            };
            let Ok(commit) = repo.find_commit(oid) else {
                continue;
            };
            if commit.parent_count() != 1 {
                continue;
            }
            let Ok(parent_tree) = commit.parent(0).and_then(|p| p.tree()) else {
                continue;
            };
            let Ok(tree) = commit.tree() else {
                continue;
            };
            let Ok(mut diff) = repo.diff_tree_to_tree(Some(&parent_tree), Some(&tree), None) else {
                continue;
            };
            let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &parent_tree, p), &|p| {
                tree_text(&repo, &tree, p)
            });
            if !cs.is_empty() {
                let short: String = sha.chars().take(7).collect();
                out.push((short, cs));
            }
        }
        out
    };

    if commit_set {
        let Ok(shas) = resolve_shas(&repo, args.commit.as_deref().unwrap_or_default()) else {
            return Vec::new();
        };
        return per_commit(&shas);
    }
    if ref_nonempty {
        let reference = args.reference.as_str();
        if let Some((base_raw, head_raw)) = reference.split_once("..") {
            let base = if base_raw.is_empty() {
                "HEAD"
            } else {
                base_raw
            };
            let head_trimmed = head_raw.trim_start_matches('.');
            let head = if head_trimmed.is_empty() {
                "HEAD"
            } else {
                head_trimmed
            };
            let Ok(base_c) = repo.revparse_single(base).and_then(|o| o.peel_to_commit()) else {
                return Vec::new();
            };
            let Ok(head_c) = repo.revparse_single(head).and_then(|o| o.peel_to_commit()) else {
                return Vec::new();
            };
            let base_id = repo
                .merge_base(base_c.id(), head_c.id())
                .unwrap_or_else(|_| base_c.id());
            let Ok(base_tree) = repo.find_commit(base_id).and_then(|c| c.tree()) else {
                return Vec::new();
            };
            let Ok(head_tree) = head_c.tree() else {
                return Vec::new();
            };
            let Ok(mut diff) = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)
            else {
                return Vec::new();
            };
            let short: String = head_c.id().to_string().chars().take(7).collect();
            let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &base_tree, p), &|p| {
                tree_text(&repo, &head_tree, p)
            });
            return one(&short, cs);
        }
        // Bare ref: the net view merge-base(ref, HEAD) → working tree.
        let Ok(base_c) = repo
            .revparse_single(reference)
            .and_then(|o| o.peel_to_commit())
        else {
            return Vec::new();
        };
        let base_id = repo
            .head()
            .ok()
            .and_then(|h| h.target())
            .and_then(|h| repo.merge_base(base_c.id(), h).ok())
            .unwrap_or_else(|| base_c.id());
        let Ok(base_tree) = repo.find_commit(base_id).and_then(|c| c.tree()) else {
            return Vec::new();
        };
        let mut opts = git2::DiffOptions::new();
        opts.include_untracked(true).recurse_untracked_dirs(true);
        let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&base_tree), Some(&mut opts))
        else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &base_tree, p), &|p| {
            workdir_text(&repo, p)
        });
        return one("workdir", cs);
    }
    if args.staged {
        let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) else {
            return Vec::new();
        };
        let Ok(index) = repo.index() else {
            return Vec::new();
        };
        let Ok(mut diff) = repo.diff_tree_to_index(Some(&head_tree), Some(&index), None) else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &head_tree, p), &|p| {
            index_text(&repo, p)
        });
        return one("staged", cs);
    }
    if args.unstaged {
        let Ok(index) = repo.index() else {
            return Vec::new();
        };
        let Ok(mut diff) = repo.diff_index_to_workdir(Some(&index), None) else {
            return Vec::new();
        };
        let cs = changes_from_diff(&mut diff, &|p| index_text(&repo, p), &|p| {
            workdir_text(&repo, p)
        });
        return one("workdir", cs);
    }
    let Ok(head_tree) = repo.head().and_then(|h| h.peel_to_tree()) else {
        return Vec::new();
    };
    let mut opts = git2::DiffOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let Ok(mut diff) = repo.diff_tree_to_workdir_with_index(Some(&head_tree), Some(&mut opts))
    else {
        return Vec::new();
    };
    let cs = changes_from_diff(&mut diff, &|p| tree_text(&repo, &head_tree, p), &|p| {
        workdir_text(&repo, p)
    });
    one("workdir", cs)
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
            ctx.stderr,
        )
    }
}
