//! Passive update notice — the gh-CLI / npm-update-notifier pattern:
//! *notify, don't install*, with zero perceived latency and ≤1 network call
//! per 24 h per machine.
//!
//! Flow, at the end of a successful human-facing command:
//! 1. Read the cached state (`~/.cache/argot/update-check.json`) and print a
//!    one-line dim notice if it says a newer version (or newer skills) exists
//!    — the notice always comes from the *previous* run's refresh, so no run
//!    ever waits on the network.
//! 2. If the state is stale (>24 h), spawn a detached `argot
//!    refresh-version-cache` child that GETs the published `version.json`
//!    (ETag-conditional, 5 s timeouts) and rewrites the state for next time.
//!
//! Silenced by: non-tty stderr, `CI`, `--quiet`, machine formats,
//! `ARGOT_OFFLINE`, `ARGOT_UPDATE_CHECK=0`, or `[update] check = false` in
//! argot.toml. A given version is announced at most once per 24 h.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The published freshness document (built by the landing from the repo).
const VERSION_URL: &str = "https://argot.tmonier.com/version.json";
/// Env opt-out (`0` disables).
const UPDATE_CHECK_ENV: &str = "ARGOT_UPDATE_CHECK";
/// One refresh — and at most one notice — per this window.
const CHECK_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60);

/// This build's generation of the `skills/` bundle (the repo file is the
/// single source: the landing publishes the same number in version.json).
fn skills_version() -> u32 {
    include_str!("../../../skills/VERSION")
        .trim()
        .parse()
        .unwrap_or(0)
}

/// On-disk state, written atomically by the refresh child.
#[derive(Debug, Default, Serialize, Deserialize)]
struct State {
    /// Unix seconds of the last completed refresh attempt.
    #[serde(default)]
    last_checked: u64,
    /// ETag of the last-seen version.json (304s keep refreshes ~free).
    #[serde(default)]
    etag: Option<String>,
    /// Latest published release version.
    #[serde(default)]
    latest: Option<String>,
    /// Model release tag the latest binary pins.
    #[serde(default)]
    model_tag: Option<String>,
    /// Latest published skills bundle generation.
    #[serde(default)]
    skills_version: Option<u32>,
    /// Unix seconds of the last printed version notice (throttle).
    #[serde(default)]
    last_notified: u64,
    /// Skills generation the user was last nagged about (notify once each).
    #[serde(default)]
    notified_skills_version: Option<u32>,
}

fn state_path() -> Option<PathBuf> {
    argot_core::cache::cache_dir()
        .ok()
        .map(|d| d.join("update-check.json"))
}

fn read_state() -> State {
    state_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

fn write_state(state: &State) {
    let Some(path) = state_path() else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let tmp = path.with_extension(format!("json.tmp.{}", std::process::id()));
    if let Ok(body) = serde_json::to_string_pretty(state) {
        if std::fs::write(&tmp, body).is_ok() {
            let _ = std::fs::rename(&tmp, &path);
        }
    }
}

fn now_secs() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0)
}

/// Parse `major.minor.patch` (extra pre-release/build suffixes ignored).
fn parse_version(v: &str) -> Option<(u64, u64, u64)> {
    let mut it = v.trim().trim_start_matches('v').splitn(3, '.');
    let major = it.next()?.parse().ok()?;
    let minor = it.next()?.parse().ok()?;
    let patch: u64 = it
        .next()?
        .chars()
        .take_while(|c| c.is_ascii_digit())
        .collect::<String>()
        .parse()
        .ok()?;
    Some((major, minor, patch))
}

fn is_newer(candidate: &str, current: &str) -> bool {
    match (parse_version(candidate), parse_version(current)) {
        (Some(c), Some(cur)) => c > cur,
        _ => false,
    }
}

/// Every silencer in one place.
fn enabled() -> bool {
    use std::io::IsTerminal;
    if !std::io::stderr().is_terminal() {
        return false;
    }
    if crate::auto_refit::is_ci() {
        return false;
    }
    if std::env::var(UPDATE_CHECK_ENV).is_ok_and(|v| v == "0") {
        return false;
    }
    if std::env::var("ARGOT_OFFLINE").is_ok_and(|v| !v.is_empty() && v != "0") {
        return false;
    }
    // Repo-level opt-out ([update] check = false), read from the CWD's config.
    if !argot_core::config::ArgotConfig::load(std::path::Path::new(".")).update_check {
        return false;
    }
    true
}

/// Was this binary installed via npm (different update command)?
fn is_npm_install() -> bool {
    std::env::current_exe()
        .map(|p| p.components().any(|c| c.as_os_str() == "node_modules"))
        .unwrap_or(false)
}

fn dim(s: &str) -> String {
    format!("\x1b[2m{s}\x1b[0m")
}

/// The end-of-command hook: print any cached notice, then (maybe) spawn the
/// detached refresh. Never blocks on the network, never prints on error.
pub fn maybe_notify_and_refresh() {
    if !enabled() {
        return;
    }
    let mut state = read_state();
    let now = now_secs();
    let current = env!("CARGO_PKG_VERSION");

    // 1. Version notice (from the previous refresh), throttled to one per day.
    if let Some(latest) = state.latest.clone() {
        if is_newer(&latest, current)
            && now.saturating_sub(state.last_notified) >= CHECK_INTERVAL.as_secs()
        {
            let cmd = if is_npm_install() {
                "npm update -g @tmonier/argot"
            } else {
                "argot update"
            };
            eprintln!(
                "{}",
                dim(&format!(
                    "argot {latest} is available (you have {current}) — run '{cmd}'"
                ))
            );
            state.last_notified = now;
            write_state(&state);
        }
    }

    // 2. Skills notice: the published bundle moved past this binary's own
    //    generation AND the user has argot skills installed. Once per bump.
    if let Some(remote) = state.skills_version {
        if remote > skills_version()
            && state.notified_skills_version != Some(remote)
            && has_installed_skills()
        {
            eprintln!(
                "{}",
                dim(
                    "argot's agent skills were updated — re-run 'npx skills add get-tmonier/argot'"
                )
            );
            state.notified_skills_version = Some(remote);
            write_state(&state);
        }
    }

    // 3. Stale state → detached refresh for next time (zero latency here).
    if now.saturating_sub(state.last_checked) >= CHECK_INTERVAL.as_secs() {
        if let Ok(exe) = std::env::current_exe() {
            let _ = std::process::Command::new(exe)
                .arg("refresh-version-cache")
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::null())
                .stderr(std::process::Stdio::null())
                .spawn();
        }
    }
}

/// Does this machine have argot skills installed (user- or repo-level)?
fn has_installed_skills() -> bool {
    let mut roots: Vec<PathBuf> = vec![PathBuf::from(".claude/skills")];
    if let Ok(home) = std::env::var("HOME") {
        roots.push(PathBuf::from(home).join(".claude/skills"));
    }
    roots.iter().any(|root| {
        std::fs::read_dir(root)
            .map(|entries| {
                entries
                    .flatten()
                    .any(|e| e.file_name().to_string_lossy().starts_with("argot-"))
            })
            .unwrap_or(false)
    })
}

/// The hidden `argot refresh-version-cache` command body: one conditional GET,
/// bounded timeouts, atomic state rewrite. Always exits 0 — a failed refresh
/// just means the next run tries again.
pub fn run_refresh() {
    let mut state = read_state();
    state.last_checked = now_secs();

    let agent = ureq::AgentBuilder::new()
        .timeout_connect(Duration::from_secs(5))
        .timeout(Duration::from_secs(10))
        .build();
    let mut req = agent.get(VERSION_URL);
    if let Some(etag) = &state.etag {
        req = req.set("If-None-Match", etag);
    }
    match req.call() {
        Ok(resp) if resp.status() == 304 => write_state(&state),
        Ok(resp) => {
            state.etag = resp.header("ETag").map(String::from);
            if let Ok(doc) = resp
                .into_string()
                .map_err(anyhow::Error::from)
                .and_then(|body| {
                    serde_json::from_str::<serde_json::Value>(&body).map_err(Into::into)
                })
            {
                state.latest = doc["latest"].as_str().map(String::from);
                state.model_tag = doc["model_tag"].as_str().map(String::from);
                state.skills_version = doc["skills_version"].as_u64().map(|v| v as u32);
            }
            write_state(&state);
        }
        // Network failure: record the attempt so we don't retry for a day.
        Err(_) => write_state(&state),
    }
}

/// Post-`argot update` note: did the release we just installed change the
/// pinned embedding model? (Compares the published tag against the tag THIS
/// (old) binary pins — a difference means the next fit re-downloads.)
#[cfg(feature = "semantic")]
pub fn model_change_note() {
    let state = read_state();
    if let Some(remote_tag) = state.model_tag {
        if remote_tag != argot_core::scoring::semantic::embedder::model_tag() {
            eprintln!(
                "note: this release changed the embedding model — the next `argot fit` \
                 downloads ~100 MB and rebuilds the semantic index"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn version_parsing_and_comparison() {
        assert_eq!(parse_version("0.2.55"), Some((0, 2, 55)));
        assert_eq!(parse_version("v1.0.0"), Some((1, 0, 0)));
        assert_eq!(parse_version("0.3.1-rc1"), Some((0, 3, 1)));
        assert_eq!(parse_version("garbage"), None);
        assert!(is_newer("0.3.0", "0.2.55"));
        assert!(is_newer("0.2.56", "0.2.55"));
        assert!(!is_newer("0.2.55", "0.2.55"));
        assert!(!is_newer("0.2.54", "0.2.55"));
        // Unparseable never notifies.
        assert!(!is_newer("weird", "0.2.55"));
    }

    #[test]
    fn skills_version_is_a_positive_integer() {
        assert!(skills_version() >= 1, "skills/VERSION must parse");
    }

    #[test]
    fn state_roundtrips_through_json() {
        let s = State {
            last_checked: 42,
            etag: Some("abc".into()),
            latest: Some("0.3.0".into()),
            model_tag: Some("semantic-model-v1".into()),
            skills_version: Some(2),
            last_notified: 41,
            notified_skills_version: Some(2),
        };
        let back: State = serde_json::from_str(&serde_json::to_string(&s).unwrap()).unwrap();
        assert_eq!(back.latest.as_deref(), Some("0.3.0"));
        assert_eq!(back.skills_version, Some(2));
        // Old/empty state files parse via defaults.
        let empty: State = serde_json::from_str("{}").unwrap();
        assert_eq!(empty.last_checked, 0);
        assert!(empty.latest.is_none());
    }
}
