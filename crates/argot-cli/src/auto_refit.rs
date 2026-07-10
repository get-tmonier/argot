//! Background auto-refit — the voice model keeps itself fresh.
//!
//! A fit is a snapshot: as the repo merges new dependencies and modules, a
//! stale model reads its own accepted code as foreign (a month of drift once
//! measured ~14× the hit volume of a fresh fit). Nudging the user to re-run
//! `argot fit` makes freshness a chore; this module makes it automatic with
//! the same zero-latency shape as the update check:
//!
//! At the end of a `check`, when the fit is **≥ 10 commits behind HEAD** (or
//! **> 7 days old and at least 1 behind**), spawn a detached
//! `argot background-refit` child and say so in one dim line. The check that
//! noticed still used the old model — no added latency — and the next check
//! scores against the fresh one. The refit reads committed HEAD (never the
//! working tree), so dirty edits can't contaminate the voice; the semantic
//! index reuses embeddings for unchanged functions, so a routine refresh
//! costs seconds, not a full re-embed.
//!
//! Guard-rails: at most one attempt per 24 h (state file), a lock so two
//! checks can't race two fits, skipped in CI (the Action refits per base
//! advance already), and opt-out via `[fit] auto-refresh = false`.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// Commits-behind threshold — the same bar as check's drift warning.
const REFIT_BEHIND_COMMITS: usize = 10;
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

fn last_attempt(argot_dir: &Path) -> u64 {
    std::fs::read_to_string(state_path(argot_dir))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v.get("last_attempt")?.as_u64())
        .unwrap_or(0)
}

fn record_attempt(argot_dir: &Path) {
    let body = serde_json::json!({ "last_attempt": now_secs() }).to_string();
    let _ = std::fs::write(state_path(argot_dir), body);
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
    if !argot_core::config::ArgotConfig::load(repo).fit_auto_refresh {
        return;
    }
    let Some((fit_sha, timestamp)) = fit_identity(argot_dir) else {
        return; // no fit yet — nothing to refresh
    };
    let Some(behind) = argot_core::check::commits_since_fit(&repo.to_string_lossy(), &fit_sha)
    else {
        return; // unresolvable history (shallow clone, rewritten) — leave it be
    };
    let age_days = fit_age_days(&timestamp, today).unwrap_or(0);
    let stale = behind >= REFIT_BEHIND_COMMITS
        || (age_days >= (REFIT_AGE.as_secs() / 86_400) as i64 && behind >= 1);
    if !stale {
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
        eprintln!(
            "\x1b[2margot: voice model is {behind} commit(s) behind — refitting in the \
             background; your next check uses the fresh voice ([fit] auto-refresh = false \
             to disable)\x1b[0m"
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

    let _ = crate::fit_repo(repo, &[]);
    let _ = std::fs::remove_file(&lock);
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
