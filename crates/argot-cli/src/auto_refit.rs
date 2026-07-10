//! Background auto-refit — the voice model keeps itself fresh, on **accepted
//! history only**.
//!
//! A fit is a snapshot: as the repo merges new dependencies and modules, a
//! stale model reads its own accepted code as foreign (a month of drift once
//! measured ~14× the hit volume of a fresh fit). Nudging the user to re-run
//! `argot fit` makes freshness a chore; this module makes it automatic with
//! the same zero-latency shape as the update check:
//!
//! At the end of a `check`, when accepted history — the default-branch line,
//! per `[fit] refresh-from` — has gained **≥ `[fit] refresh-after` commits
//! touching in-scope source** since the fit (or the fit is > 7 days old with
//! any such drift), spawn a detached `argot background-refit` child and say
//! so in one dim line. The check that noticed still used the old model — no
//! added latency — and the next check scores against the fresh one.
//!
//! What the refit learns is as guarded as when it runs: it fits **at the
//! accepted anchor in a throwaway worktree** whenever HEAD isn't that anchor
//! or the tree is dirty — a feature branch's own commits and uncommitted
//! edits are the code argot is judging, and must never become the voice it
//! judges against. The semantic index seeds from the current fit, so a
//! routine refresh costs seconds, not a full re-embed.
//!
//! Guard-rails: at most one attempt per 24 h (state file), a lock so two
//! checks can't race two fits, skipped in CI (the Action refits per base
//! advance already), and opt-out via `[fit] auto-refresh = false`.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Age threshold: a week-old fit with any drift at all also refreshes.
const REFIT_AGE: Duration = Duration::from_secs(7 * 24 * 60 * 60);
/// At most one background attempt per this window (guards failure loops).
const ATTEMPT_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);
/// A lock older than this is a crashed refit, not a running one.
const LOCK_STALE: Duration = Duration::from_secs(60 * 60);

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// The fit's identity as recorded in `scorer-config.json`: the SHA it was
/// fitted at and its UTC timestamp (every language shares them).
fn fit_identity(argot_dir: &Path) -> Option<(String, String)> {
    let bytes = std::fs::read(argot_dir.join("scorer-config.json")).ok()?;
    let v: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    v.get("languages")?.as_object()?.values().find_map(|lc| {
        let cal = lc.get("calibration")?;
        Some((
            cal.get("repo_sha")?.as_str()?.to_string(),
            cal.get("timestamp_utc")?.as_str()?.to_string(),
        ))
    })
}

/// Parse the fit timestamp's date and return its age in days (calendar-level
/// precision is plenty for a 7-day bar). `None` on any parse trouble.
fn fit_age_days(timestamp_utc: &str, today: &str) -> Option<i64> {
    crate::days_since_fit(timestamp_utc, today)
}

fn state_path(argot_dir: &Path) -> PathBuf {
    argot_dir.join("auto-refit.json")
}

fn lock_path(argot_dir: &Path) -> PathBuf {
    argot_dir.join("auto-refit.lock")
}

fn read_state(argot_dir: &Path) -> serde_json::Value {
    std::fs::read_to_string(state_path(argot_dir))
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_else(|| serde_json::json!({}))
}

fn last_attempt(argot_dir: &Path) -> u64 {
    read_state(argot_dir)
        .get("last_attempt")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
}

/// Did the last background refit fail? (Cleared by the next success.)
fn last_failed(argot_dir: &Path) -> bool {
    read_state(argot_dir)
        .get("last_failed")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

fn record_attempt(argot_dir: &Path) {
    let mut state = read_state(argot_dir);
    state["last_attempt"] = serde_json::json!(now_secs());
    let _ = std::fs::write(state_path(argot_dir), state.to_string());
}

fn record_result(argot_dir: &Path, failed: bool) {
    let mut state = read_state(argot_dir);
    state["last_failed"] = serde_json::json!(failed);
    let _ = std::fs::write(state_path(argot_dir), state.to_string());
}

/// Are we on a CI runner? `CI` covers GitHub/GitLab/CircleCI/Travis/Buildkite;
/// Jenkins, TeamCity, and Azure Pipelines don't set it, so their own markers
/// are checked too — an ephemeral runner must never get an unsolicited
/// background fit (the job may kill it mid-write).
pub fn is_ci() -> bool {
    ["CI", "JENKINS_URL", "TEAMCITY_VERSION", "TF_BUILD"]
        .iter()
        .any(|k| std::env::var_os(k).is_some())
}

/// End-of-check hook: spawn a detached refit when the model has drifted.
/// Never blocks, never errors — a missed refresh just tries again tomorrow.
pub fn maybe_refit(repo: &Path, argot_dir: &Path, today: &str, quiet: bool) {
    // CI refits per base advance (the Action) or explicit `argot fit` steps;
    // a runner must never burn minutes on an unsolicited background fit.
    if is_ci() {
        return;
    }
    let config = argot_core::config::ArgotConfig::load(repo);
    if !config.fit_auto_refresh {
        return;
    }
    let Some((fit_sha, timestamp)) = fit_identity(argot_dir) else {
        return; // no fit yet — nothing to refresh
    };
    // Staleness is measured on ACCEPTED history (default-branch line unless
    // `[fit] refresh-from` says otherwise) and counts only commits touching
    // in-scope source — a feature branch's own commits and docs-only churn
    // never age the voice. The count stops at the threshold, so the fresh
    // common case costs one commit-graph query, no tree diffs.
    let stale_after = config.fit_refresh_after;
    let Some(behind) = argot_core::check::accepted_source_commits_behind(
        &repo.to_string_lossy(),
        &fit_sha,
        &config,
        stale_after,
    ) else {
        return; // unresolvable history (shallow clone, rewritten) — leave it be
    };
    // Two staleness axes: accepted history moved past the fit, OR the fit no
    // longer reflects the configuration (the user/skill edited
    // [exclude]/[detect] — recalibration completes itself instead of waiting
    // for a manual fit).
    let config_changed = argot_core::health::read(argot_dir)
        .map(|h| {
            !h.config_fingerprint.is_empty()
                && h.config_fingerprint != argot_core::health::config_fingerprint(&config)
        })
        .unwrap_or(false);
    let age_days = fit_age_days(&timestamp, today).unwrap_or(0);
    let history_stale =
        behind >= stale_after || (age_days >= (REFIT_AGE.as_secs() / 86_400) as i64 && behind >= 1);
    if !history_stale && !config_changed {
        return;
    }
    // A failing background refit must not loop silently forever: after a
    // failed attempt, hand the wheel back with a visible note instead of
    // re-spawning into the same wall.
    if last_failed(argot_dir) {
        if !quiet {
            eprintln!(
                "\x1b[2margot: the last background refit failed — run `argot fit` to see why\x1b[0m"
            );
        }
        return;
    }
    if now_secs().saturating_sub(last_attempt(argot_dir)) < ATTEMPT_INTERVAL.as_secs() {
        return;
    }
    record_attempt(argot_dir);

    let Ok(exe) = std::env::current_exe() else {
        return;
    };
    let spawned = std::process::Command::new(exe)
        .arg("background-refit")
        .arg("--repo")
        .arg(repo)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .is_ok();
    if spawned && !quiet {
        let reason = if config_changed {
            "argot.toml changed since the fit".to_string()
        } else {
            // `behind` stops counting at the threshold — honest "+" past it.
            let plus = if behind >= stale_after { "+" } else { "" };
            format!("voice model is {behind}{plus} accepted source commit(s) behind")
        };
        eprintln!(
            "\x1b[2margot: {reason} — refitting in the background; your next check uses \
             the fresh voice ([fit] auto-refresh = false to disable)\x1b[0m"
        );
    }
}

/// The hidden `argot background-refit` body: take the lock, fit, release.
/// Always exits 0 — its stdio is detached; failures surface on the next
/// foreground fit or check.
pub fn run_background_refit(repo: &Path) {
    let argot_dir = repo.join(".argot");
    let lock = lock_path(&argot_dir);

    // One refit at a time: create_new fails when a live lock exists; a lock
    // older than LOCK_STALE is a crashed run and is replaced.
    let acquire = || {
        std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock)
    };
    if acquire().is_err() {
        let fresh = std::fs::metadata(&lock)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|t| t.elapsed().ok())
            .is_some_and(|age| age < LOCK_STALE);
        if fresh {
            return; // another refit is running
        }
        let _ = std::fs::remove_file(&lock);
        if acquire().is_err() {
            return;
        }
    }

    let failed = refit_accepted(repo).is_err();
    record_result(&argot_dir, failed);
    let _ = std::fs::remove_file(&lock);
}

/// Fit the voice from accepted history. In place only when HEAD *is* the
/// anchor and the tree is clean (the fast common case on the default branch);
/// otherwise in a throwaway worktree at the anchor, publishing the artifacts
/// back — unmerged branch commits and uncommitted edits never train the voice.
fn refit_accepted(repo: &Path) -> Result<(), ()> {
    let repo_s = repo.to_string_lossy().into_owned();
    let config = argot_core::config::ArgotConfig::load(repo);
    let anchor = argot_core::check::freshness_anchor(&repo_s, &config);
    let head = crate::head_sha(&repo_s);
    let dirty = !argot_core::git_walk::uncommitted_source_paths(&repo_s).is_empty();
    match anchor {
        Some(anchor) if dirty || Some(anchor.as_str()) != head.as_deref() => {
            fit_in_worktree(repo, &anchor)
        }
        // Anchor == HEAD with a clean tree — or history the anchor can't be
        // resolved on (then a worktree wouldn't know where to stand either).
        _ => crate::fit_repo(repo, &[]).map(|_| ()),
    }
}

fn fit_in_worktree(repo: &Path, sha: &str) -> Result<(), ()> {
    let worktree =
        crate::worktree::TempWorktree::create(repo, sha, "argot-refit").map_err(|_| ())?;
    worktree.adopt_current_config(repo);
    crate::fit_repo(&worktree.path, &[])?;
    publish_artifacts(&worktree.path, repo).map_err(|_| ())
}

/// Copy the worktree fit's `.argot/` artifacts over the repo's. The corpus
/// listing is rewritten to repo paths (it records absolute paths and is the
/// one artifact humans read back); everything else is location-independent.
fn publish_artifacts(worktree_root: &Path, repo: &Path) -> std::io::Result<()> {
    let from = worktree_root.join(".argot");
    let to = repo.join(".argot");
    std::fs::create_dir_all(&to)?;
    let canon = |p: &Path| std::fs::canonicalize(p).unwrap_or_else(|_| p.to_path_buf());
    let wt_prefix = canon(worktree_root).to_string_lossy().into_owned();
    let repo_prefix = canon(repo).to_string_lossy().into_owned();
    for entry in std::fs::read_dir(&from)? {
        let entry = entry?;
        if !entry.file_type()?.is_file() {
            continue;
        }
        let name = entry.file_name();
        let dst = to.join(&name);
        if name == "repo-corpus.txt" {
            // The fit canonicalizes paths, but cover the raw prefix too so a
            // platform that skips symlink resolution still rewrites cleanly.
            let text = std::fs::read_to_string(entry.path())?;
            let text = text
                .replace(&wt_prefix, &repo_prefix)
                .replace(&worktree_root.to_string_lossy().into_owned(), &repo_prefix);
            std::fs::write(&dst, text)?;
        } else {
            std::fs::copy(entry.path(), &dst)?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attempt_state_roundtrips() {
        let dir = std::env::temp_dir().join(format!("argot_refit_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        assert_eq!(last_attempt(&dir), 0, "no state → epoch 0");
        record_attempt(&dir);
        assert!(last_attempt(&dir) > 0);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn publish_rewrites_corpus_paths_and_keeps_local_state() {
        let base = std::env::temp_dir().join(format!("argot_publish_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let wt = base.join("worktree");
        let repo = base.join("repo");
        std::fs::create_dir_all(wt.join(".argot")).unwrap();
        std::fs::create_dir_all(repo.join(".argot")).unwrap();
        std::fs::write(
            wt.join(".argot/repo-corpus.txt"),
            format!("{}/src/a.py\n{}/src/b.py", wt.display(), wt.display()),
        )
        .unwrap();
        std::fs::write(wt.join(".argot/scorer-config.json"), "{}").unwrap();
        // Pre-existing local state that the worktree fit doesn't produce
        // must survive the publish untouched.
        std::fs::write(repo.join(".argot/last-check.json"), "[]").unwrap();

        publish_artifacts(&wt, &repo).unwrap();

        let corpus = std::fs::read_to_string(repo.join(".argot/repo-corpus.txt")).unwrap();
        assert!(
            corpus.contains(&format!("{}/src/a.py", repo.display())),
            "worktree paths rewritten to repo paths: {corpus}"
        );
        assert!(!corpus.contains("worktree"), "{corpus}");
        assert!(repo.join(".argot/scorer-config.json").is_file());
        assert_eq!(
            std::fs::read_to_string(repo.join(".argot/last-check.json")).unwrap(),
            "[]"
        );
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn fit_identity_reads_sha_and_timestamp() {
        let dir = std::env::temp_dir().join(format!("argot_refit_id_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("scorer-config.json"),
            r#"{"languages":{"python":{"calibration":{"repo_sha":"abc123","timestamp_utc":"2026-07-01T00:00:00+00:00"}}}}"#,
        )
        .unwrap();
        let (sha, ts) = fit_identity(&dir).expect("identity parsed");
        assert_eq!(sha, "abc123");
        assert!(ts.starts_with("2026-07-01"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
