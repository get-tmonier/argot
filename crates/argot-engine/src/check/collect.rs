//! Git patch collection for the requested `check` mode (commit / range /
//! workdir / staged / unstaged).

use super::{extension, CheckArgs, CheckOutcome, PatchBatch};
use crate::git_walk::{
    open_repo, resolve_shas, walk_commits, HunkSpan, WalkItem, SUPPORTED_EXTENSIONS,
};
use crate::suppress::{fnmatch, PathScope, PathSuppressions};
use crate::text::splitlines;
use git2::{DiffFindOptions, Patch, Status, StatusOptions};
use std::collections::HashSet;
use std::fs;
use std::ops::ControlFlow;
use std::path::Path;

fn is_supported_ext(file_path: &str) -> bool {
    SUPPORTED_EXTENSIONS.contains(&extension(file_path).as_str())
}
/// Scope decision for one patch batch, against the resolved path-suppression
/// set (recommended built-ins + `argot.toml [exclude].paths` — the same set calibration
/// samples from; lock-step principle).
pub(super) enum BatchScope {
    /// In scope: score and report normally.
    Score,
    /// In scope but matched by a user `[exclude].paths` pattern: score it so the
    /// suppression is countable, then drop its hits from output.
    ScoreSuppressed,
    /// Out of scope (wrong language, recommended exclusion, data-dominant):
    /// silently dropped, exactly as before suppressions existed.
    Drop,
}
/// Port of `_is_out_of_scope`, split so user-ignored files stay countable:
/// wrong language / recommended-set path → `Drop` (silent, as always); user
/// `[exclude].paths` match → `ScoreSuppressed`. Data-heavy files are NOT dropped
/// here: data scope is row-granular inside the scorer (a planted code hunk in
/// a data-dominant file must still be judged; its data-row hunks are skipped
/// per hunk).
pub(super) fn batch_scope(
    file_path: &str,
    language_extensions: &HashSet<String>,
    path_suppressions: &PathSuppressions,
) -> BatchScope {
    let ext = extension(file_path);
    if !language_extensions.contains(&ext) {
        return BatchScope::Drop;
    }
    match path_suppressions.classify(file_path) {
        PathScope::Recommended => BatchScope::Drop,
        PathScope::UserIgnored => BatchScope::ScoreSuppressed,
        PathScope::InScope => BatchScope::Score,
    }
}
/// `--exclude` overrides `--only`; empty `only` means "no restriction"
/// (`_apply_filters`).
pub(super) fn passes_filters(fp: &str, only: &[String], exclude: &[String]) -> bool {
    if exclude.iter().any(|pat| fnmatch(fp, pat)) {
        return false;
    }
    if !only.is_empty() && !only.iter().any(|pat| fnmatch(fp, pat)) {
        return false;
    }
    true
}
/// Languages present in the change that argot supports but the current fit
/// has no model for (fitted before the language appeared in the repo).
pub(super) fn patches_langs_without_model(
    patches: &[PatchBatch],
    fitted_languages: &HashSet<String>,
) -> Vec<&'static str> {
    patches
        .iter()
        .filter_map(|b| crate::check::ext_to_lang(&extension(&b.file_path)))
        .filter(|lang| !fitted_languages.contains(*lang))
        .collect()
}
/// Yield batches for committed changes (`_committed_patches`), source = 7-char SHA.
fn committed_patches(repo_path: &str, shas: &HashSet<String>) -> anyhow::Result<Vec<PatchBatch>> {
    let mut out = Vec::new();
    walk_commits(repo_path, shas, |item: WalkItem| {
        let short: String = item.commit_id.chars().take(7).collect();
        out.push(PatchBatch {
            file_path: item.file_path,
            content: item.post_blob,
            hunks: item.hunks,
            source: short,
            ignored_by_pattern: false,
        });
        Ok(ControlFlow::Continue(()))
    })?;
    Ok(out)
}
/// Net diff of a `base..head` range, scored as one changeset — the changes
/// `head` introduces relative to `base` (merge-base → head, matching a pull
/// request's diff), *not* each intervening commit. So when a later commit in the
/// range reverts or rewrites an earlier one (e.g. a fix that drops a foreign
/// import a prior commit added), the range shows only the net result — a fix
/// commit clears the flag, exactly as a reviewer reading the PR's files would
/// expect. Content is the file as `head` leaves it; source = head's short SHA.
pub(super) fn net_range_patches(
    repo_path: &str,
    base_ref: &str,
    head_ref: &str,
) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let base_commit = repo.revparse_single(base_ref)?.peel_to_commit()?;
    let head_commit = repo.revparse_single(head_ref)?.peel_to_commit()?;
    // Merge-base → head is what `head` adds since diverging from `base`, so a
    // base that advanced past the branch point doesn't show as spurious changes.
    let base_id = repo
        .merge_base(base_commit.id(), head_commit.id())
        .unwrap_or_else(|_| base_commit.id());
    let base_tree = repo.find_commit(base_id)?.tree()?;
    let head_tree = head_commit.tree()?;
    let mut diff = repo.diff_tree_to_tree(Some(&base_tree), Some(&head_tree), None)?;
    diff.find_similar(Some(&mut DiffFindOptions::new()))?;
    let short: String = head_commit.id().to_string().chars().take(7).collect();
    let mut out = Vec::new();
    for idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(idx) {
            Some(d) => d,
            None => continue,
        };
        let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let patch = match Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        if patch.num_hunks() == 0 {
            continue;
        }
        let hunks = hunks_from_patch(&patch)?;
        // Post-state content: the file as it stands at `head` (deleted → skip).
        let content = match head_tree
            .get_path(Path::new(&file_path))
            .ok()
            .and_then(|e| repo.find_blob(e.id()).ok())
        {
            Some(b) => b.content().to_vec(),
            None => continue,
        };
        out.push(PatchBatch {
            file_path,
            content,
            hunks,
            source: short.clone(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}
fn hunks_from_patch(patch: &Patch) -> anyhow::Result<Vec<HunkSpan>> {
    let n = patch.num_hunks();
    let mut hunks = Vec::with_capacity(n);
    for h in 0..n {
        let (hunk, _lines) = patch.hunk(h)?;
        hunks.push(HunkSpan {
            new_start: hunk.new_start(),
            new_lines: hunk.new_lines(),
        });
    }
    Ok(hunks)
}
/// Unstaged changes vs the index (`_modified_patches`, source="workdir").
fn modified_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let index = repo.index()?;
    let mut diff = match repo.diff_index_to_workdir(Some(&index), None) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    diff.find_similar(Some(&mut DiffFindOptions::new()))?;
    let workdir = match repo.workdir() {
        Some(w) => w.to_path_buf(),
        None => return Ok(Vec::new()),
    };
    let mut out = Vec::new();
    for idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(idx) {
            Some(d) => d,
            None => continue,
        };
        let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let patch = match Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        if patch.num_hunks() == 0 {
            continue;
        }
        let hunks = hunks_from_patch(&patch)?;
        let full = workdir.join(&file_path);
        if !full.exists() {
            continue;
        }
        let content = fs::read(&full)?;
        out.push(PatchBatch {
            file_path,
            content,
            hunks,
            source: "workdir".to_string(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}
/// Staged changes vs HEAD (`_staged_patches`, source="staged"). Content from
/// the index blob.
fn staged_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let index = repo.index()?;
    let head_tree = match repo.head().and_then(|h| h.peel_to_tree()) {
        Ok(t) => t,
        Err(_) => return Ok(Vec::new()),
    };
    let mut diff = match repo.diff_tree_to_index(Some(&head_tree), Some(&index), None) {
        Ok(d) => d,
        Err(_) => return Ok(Vec::new()),
    };
    diff.find_similar(Some(&mut DiffFindOptions::new()))?;
    let mut out = Vec::new();
    for idx in 0..diff.deltas().len() {
        let delta = match diff.get_delta(idx) {
            Some(d) => d,
            None => continue,
        };
        let file_path = match delta.new_file().path().and_then(|p| p.to_str()) {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let patch = match Patch::from_diff(&diff, idx)? {
            Some(p) => p,
            None => continue,
        };
        if patch.num_hunks() == 0 {
            continue;
        }
        let hunks = hunks_from_patch(&patch)?;
        let entry = match index.get_path(Path::new(&file_path), 0) {
            Some(e) => e,
            None => continue,
        };
        let blob = match repo.find_blob(entry.id) {
            Ok(b) => b,
            Err(_) => continue,
        };
        out.push(PatchBatch {
            file_path,
            content: blob.content().to_vec(),
            hunks,
            source: "staged".to_string(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}
/// Untracked supported files (`_untracked_patches`, source="untracked"). One
/// synthetic full-file hunk each.
fn untracked_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let repo = open_repo(repo_path)?;
    let workdir = match repo.workdir() {
        Some(w) => w.to_path_buf(),
        None => return Ok(Vec::new()),
    };
    let mut opts = StatusOptions::new();
    opts.include_untracked(true).recurse_untracked_dirs(true);
    let statuses = repo.statuses(Some(&mut opts))?;
    let mut out = Vec::new();
    for entry in statuses.iter() {
        if !entry.status().contains(Status::WT_NEW) {
            continue;
        }
        let file_path = match entry.path() {
            Some(p) => p.to_string(),
            None => continue,
        };
        if !is_supported_ext(&file_path) {
            continue;
        }
        let full = workdir.join(&file_path);
        if !full.exists() {
            continue;
        }
        let content = fs::read(&full)?;
        let source = String::from_utf8_lossy(&content);
        let line_count = splitlines(&source).len();
        if line_count == 0 {
            continue;
        }
        out.push(PatchBatch {
            file_path,
            content,
            hunks: vec![HunkSpan {
                new_start: 1,
                new_lines: line_count as u32,
            }],
            source: "untracked".to_string(),
            ignored_by_pattern: false,
        });
    }
    Ok(out)
}
fn chain_workdir_patches(repo_path: &str) -> anyhow::Result<Vec<PatchBatch>> {
    let mut out = modified_patches(repo_path)?;
    out.extend(staged_patches(repo_path)?);
    out.extend(untracked_patches(repo_path)?);
    Ok(out)
}
/// Collect patches for the requested mode (`main()` mode dispatch). On a
/// mode-specific early exit returns the finished outcome.
pub(super) fn collect_patches(args: &CheckArgs) -> Result<(Vec<PatchBatch>, String), CheckOutcome> {
    let repo_path = args.repo_path.as_str();
    let commit_set = args
        .commit
        .as_deref()
        .map(|c| !c.is_empty())
        .unwrap_or(false);
    let ref_nonempty = !args.reference.is_empty();

    if commit_set {
        let commit = args.commit.as_deref().unwrap();
        let repo =
            open_repo(repo_path).map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        let shas = resolve_shas(&repo, commit)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        if shas.is_empty() {
            return Err(CheckOutcome::err(
                format!("No commits found for '{commit}'\n"),
                2,
            ));
        }
        let patches = committed_patches(repo_path, &shas)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        let short: String = commit.chars().take(8).collect();
        return Ok((patches, format!("1 commit ({short})")));
    }

    if ref_nonempty {
        let reference = args.reference.as_str();
        let repo =
            open_repo(repo_path).map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        if let Some((base_raw, head_raw)) = reference.split_once("..") {
            // Score the *net* diff (merge-base → head), not each commit in the
            // range: a PR's voice check must match what a reviewer sees in the
            // files, so a fix commit clears an earlier commit's flag. Handles
            // both `a..b` and `a...b` (leading '.' of a three-dot range trimmed);
            // an empty side defaults to HEAD.
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
            let patches = net_range_patches(repo_path, base, head)
                .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
            if patches.is_empty() {
                // Note: exit 0 (not 2) for an empty net diff.
                return Err(CheckOutcome::err(
                    format!("No changes in range '{reference}'\n"),
                    0,
                ));
            }
            return Ok((patches, format!("net diff ({reference})")));
        }
        // Bare ref: <ref>..HEAD commits plus full workdir. Validate the ref
        // first — otherwise `resolve_shas` treats an unknown start as "since the
        // beginning of history" and silently scores the whole tree as if clean.
        if repo.revparse_single(reference).is_err() {
            return Err(CheckOutcome::err(
                format!("error: unknown revision '{reference}' — not a commit, branch, or tag.\n"),
                2,
            ));
        }
        let shas = resolve_shas(&repo, &format!("{reference}..HEAD"))
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 2))?;
        let workdir = chain_workdir_patches(repo_path)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        if !shas.is_empty() {
            let mut patches = committed_patches(repo_path, &shas)
                .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
            patches.extend(workdir);
            return Ok((
                patches,
                format!("workdir + {} commit(s) since {reference}", shas.len()),
            ));
        }
        return Ok((workdir, format!("workdir (no commits since {reference})")));
    }

    if args.staged {
        let patches =
            staged_patches(repo_path).map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        return Ok((patches, "staged changes".to_string()));
    }
    if args.unstaged {
        let patches = modified_patches(repo_path)
            .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
        return Ok((patches, "unstaged changes".to_string()));
    }

    let patches = chain_workdir_patches(repo_path)
        .map_err(|e| CheckOutcome::err(format!("error: {e}\n"), 1))?;
    Ok((patches, "workdir".to_string()))
}
