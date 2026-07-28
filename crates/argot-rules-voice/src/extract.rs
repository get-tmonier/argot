//! Dataset extraction.
//!
//! Walks git history, tokenises each changed hunk plus up to
//! [`CONTEXT_LINES`] of surrounding context, and writes one JSON
//! [`HunkRecord`](argot_lang::dataset::HunkRecord) per line.

use anyhow::Result;
use argot_engine::git_walk::{open_repo, resolve_shas, walk_commits, walk_repo, WalkItem};
use argot_lang::dataset::HunkRecord;
use argot_lang::text::splitlines;
use argot_lang::tokenize::{language_for_path_ctx, tokenize_lines};
use std::collections::HashSet;
use std::io::Write;
use std::ops::ControlFlow;

/// Lines of context captured on each side of a hunk.
pub const CONTEXT_LINES: usize = 50;

/// Extraction errors that map to specific CLI exit codes / messages.
#[derive(Debug, thiserror::Error)]
pub enum ExtractError {
    #[error("no commits found for ref {0:?}")]
    NoCommitsForRef(String),
}

/// Result of a completed extraction.
#[derive(Debug, Clone, Copy)]
pub struct ExtractStats {
    pub count: usize,
    pub limit_reached: bool,
}

/// Extract records from `repo_path` (optionally scoped to `reference`) and
/// write them as JSONL to `out`. Returns record counts.
///
/// Topological walk, single-parent commits only, hunk bounds clamped to the
/// post-image, context of [`CONTEXT_LINES`] each side.
pub fn write_dataset<W: Write>(
    repo_path: &str,
    reference: Option<&str>,
    limit: Option<usize>,
    out: &mut W,
) -> Result<ExtractStats> {
    let filter: Option<HashSet<String>> = match reference {
        Some(r) if !r.is_empty() => {
            let repo = open_repo(repo_path)?;
            let shas = resolve_shas(&repo, r).unwrap_or_default();
            if shas.is_empty() {
                return Err(ExtractError::NoCommitsForRef(r.to_string()).into());
            }
            Some(shas)
        }
        _ => None,
    };

    let mut count = 0usize;
    let mut limit_reached = false;

    // `.h` and `.inc` route by what the repo itself writes — decided once here
    // so the dataset labels match how calibrate/check route.
    let repo_langs = argot_engine::corpus::repo_langs(std::path::Path::new(repo_path));

    {
        let mut visit = |item: WalkItem| -> Result<ControlFlow<()>> {
            let lang = match language_for_path_ctx(&item.file_path, repo_langs) {
                Some(l) => l,
                None => return Ok(ControlFlow::Continue(())),
            };
            let source = String::from_utf8_lossy(&item.post_blob);
            let lines = splitlines(&source);
            let nlines = lines.len() as i64;

            for hunk in &item.hunks {
                let hunk_start = hunk.new_start as i64 - 1; // 0-indexed
                let hunk_end = hunk_start + hunk.new_lines as i64;
                if hunk_start < 0 || hunk_end > nlines {
                    continue;
                }
                let hunk_start = hunk_start as usize;
                let hunk_end = hunk_end as usize;

                let before_start = hunk_start.saturating_sub(CONTEXT_LINES);
                let after_start = hunk_end;

                let context_before = tokenize_lines(&lines, lang, before_start, hunk_start);
                let hunk_tokens = tokenize_lines(&lines, lang, hunk_start, hunk_end);
                let context_after = tokenize_lines(
                    &lines,
                    lang,
                    after_start,
                    (after_start + CONTEXT_LINES).min(lines.len()),
                );

                let record = HunkRecord {
                    commit_sha: item.commit_id.clone(),
                    file_path: item.file_path.clone(),
                    language: lang,
                    hunk_start_line: hunk_start,
                    hunk_end_line: hunk_end,
                    context_before,
                    hunk_tokens,
                    context_after,
                    parent_sha: item.parent_id.clone(),
                    author_date_iso: item.author_time.to_string(),
                };

                out.write_all(argot_engine::json::to_string(&record)?.as_bytes())?;
                out.write_all(b"\n")?;
                count += 1;

                if let Some(l) = limit {
                    if count >= l {
                        limit_reached = true;
                        break;
                    }
                }
            }

            if limit_reached {
                Ok(ControlFlow::Break(()))
            } else {
                Ok(ControlFlow::Continue(()))
            }
        };

        match &filter {
            Some(shas) => walk_commits(repo_path, shas, &mut visit)?,
            None => walk_repo(repo_path, &mut visit)?,
        }
    }

    Ok(ExtractStats {
        count,
        limit_reached,
    })
}
