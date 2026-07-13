//! Window resolution for `argot audit` — turn `--commits N` / `--since
//! <duration|date>` into a base commit on the first-parent line.
//!
//! Two guardrails, both loud (stderr + recorded on the card, never silent):
//! a hard cap on how far back an audit walks, and the fit-shrink inherited
//! from the replay engine (the base must be a commit today's config can fit).

use std::path::Path;

/// Default window: enough history to hold real findings, small enough that
/// the base voice is still "the same repo".
pub const DEFAULT_COMMITS: usize = 50;

/// Hard cap on the window, whatever the flags say. A `--since` on an old,
/// busy repo can resolve to tens of thousands of commits; past this the
/// "same repo" premise breaks down and the clamp is announced instead.
pub const MAX_WINDOW: usize = 1000;

/// What the user asked for, before resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WindowSpec {
    /// `--commits N` (also the default).
    Commits(usize),
    /// `--since <spec>`, kept verbatim for the card.
    Since(String),
}

/// Why the effective window differs from the requested one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Clamp {
    /// Hit `MAX_WINDOW`.
    Cap,
    /// History is shorter than the request (chain ends at the root).
    History,
    /// Shrunk so the base commit's tree still holds in-scope source.
    FitShrink,
}

impl Clamp {
    pub fn as_str(self) -> &'static str {
        match self {
            Clamp::Cap => "cap",
            Clamp::History => "history",
            Clamp::FitShrink => "fit-shrink",
        }
    }
}

/// One first-parent ancestor of HEAD.
pub struct ChainCommit {
    pub sha: String,
    /// Commit (not author) time, unix seconds.
    pub time: i64,
}

/// Parse a `--since` value against `now` (unix seconds): either a date
/// (`YYYY-MM-DD`, midnight UTC) or a duration — `<n>d`/`<n>w`/`<n>m`/`<n>y`
/// (months = 30 days, years = 365, calendar-naive by design; the card prints
/// the resolved cutoff date so there is no ambiguity). Returns the cutoff.
pub fn parse_since(spec: &str, now: i64) -> Result<i64, String> {
    let spec = spec.trim();
    if let Some(date) = parse_date(spec) {
        return Ok(date);
    }
    let (num, unit) = spec.split_at(spec.len().saturating_sub(1));
    let n: i64 = num
        .parse()
        .ok()
        .filter(|&n| n > 0)
        .ok_or_else(|| bad_since(spec))?;
    let days = match unit {
        "d" => n,
        "w" => n * 7,
        "m" => n * 30,
        "y" => n * 365,
        _ => return Err(bad_since(spec)),
    };
    Ok(now - days * 86_400)
}

fn bad_since(spec: &str) -> String {
    format!(
        "cannot parse --since '{spec}' — expected a date (YYYY-MM-DD) or a duration like 90d, 12w, 6m, 1y"
    )
}

/// `YYYY-MM-DD` → unix seconds at midnight UTC. Days-from-civil (Howard
/// Hinnant's algorithm) — no chrono dependency for one conversion.
fn parse_date(s: &str) -> Option<i64> {
    let parts: Vec<&str> = s.split('-').collect();
    if parts.len() != 3 || parts[0].len() != 4 {
        return None;
    }
    let y: i64 = parts[0].parse().ok()?;
    let m: u32 = parts[1].parse().ok()?;
    let d: u32 = parts[2].parse().ok()?;
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    let y = if m <= 2 { y - 1 } else { y };
    let era = if y >= 0 { y } else { y - 399 } / 400;
    let yoe = y - era * 400;
    let mp = (m as i64 + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    Some((era * 146_097 + doe - 719_468) * 86_400)
}

/// Unix seconds → `YYYY-MM-DD` (UTC). Inverse of `parse_date`, for cards.
pub fn format_date(unix: i64) -> String {
    let z = unix.div_euclid(86_400) + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = if m <= 2 { y + 1 } else { y };
    format!("{y:04}-{m:02}-{d:02}")
}

/// Walk first-parents back from HEAD, at most `MAX_WINDOW + 1` steps (one
/// past the cap so the cap clamp is distinguishable from exact fit). Returns
/// the HEAD sha + the ancestor chain (`chain[k-1]` = the commit `k` back),
/// clamped to the root on short histories.
pub fn resolve_chain(repo: &Path, max: usize) -> Result<(ChainCommit, Vec<ChainCommit>), String> {
    let git_repo = git2::Repository::discover(repo).map_err(|e| format!("not a git repo: {e}"))?;
    let head = git_repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|e| format!("cannot resolve HEAD: {e}"))?;
    let head_commit = ChainCommit {
        sha: head.id().to_string(),
        time: head.time().seconds(),
    };
    let mut chain = Vec::new();
    let mut cursor = head;
    for _ in 0..max {
        match cursor.parent(0) {
            Ok(p) => {
                chain.push(ChainCommit {
                    sha: p.id().to_string(),
                    time: p.time().seconds(),
                });
                cursor = p;
            }
            Err(_) => break, // root commit (or a shallow-clone horizon)
        }
    }
    if chain.is_empty() {
        return Err("HEAD has no parent — nothing to audit".to_string());
    }
    Ok((head_commit, chain))
}

/// The window size a spec asks for, given the chain: `--commits N` is N;
/// `--since` counts first-parent commits (HEAD included) at-or-after the
/// cutoff, so the base is the first commit older than it. Also returns the
/// clamp that applied, if any (cap and history; fit-shrink comes later).
pub fn requested_window(
    spec: &WindowSpec,
    head: &ChainCommit,
    chain: &[ChainCommit],
    now: i64,
) -> Result<(usize, Option<Clamp>), String> {
    let want = match spec {
        WindowSpec::Commits(n) => *n,
        WindowSpec::Since(raw) => {
            let cutoff = parse_since(raw, now)?;
            if head.time < cutoff {
                return Err(format!(
                    "nothing to audit — the newest commit ({}) is older than --since {raw} ({})",
                    format_date(head.time),
                    format_date(cutoff)
                ));
            }
            // Window k ⇒ base = chain[k-1] = the first commit older than the
            // cutoff. Everything at-or-after the cutoff is inside the window.
            // No commit older than the cutoff ⇒ the request reaches past the
            // walked chain — one-past-the-end so a clamp is reported below.
            chain
                .iter()
                .position(|c| c.time < cutoff)
                .map(|idx| idx + 1)
                .unwrap_or(chain.len() + 1)
        }
    };
    if want == 0 {
        return Err("window is empty — nothing to audit".to_string());
    }
    if want > MAX_WINDOW {
        return Ok((MAX_WINDOW.min(chain.len()), Some(Clamp::Cap)));
    }
    if want > chain.len() {
        return Ok((chain.len(), Some(Clamp::History)));
    }
    Ok((want, None))
}

#[cfg(test)]
mod tests {
    use super::*;

    const DAY: i64 = 86_400;

    fn chain(times: &[i64]) -> Vec<ChainCommit> {
        times
            .iter()
            .enumerate()
            .map(|(i, &t)| ChainCommit {
                sha: format!("{i:040x}"),
                time: t,
            })
            .collect()
    }

    #[test]
    fn since_parses_durations_and_dates() {
        let now = 1_750_000_000;
        assert_eq!(parse_since("90d", now).unwrap(), now - 90 * DAY);
        assert_eq!(parse_since("12w", now).unwrap(), now - 84 * DAY);
        assert_eq!(parse_since("6m", now).unwrap(), now - 180 * DAY);
        assert_eq!(parse_since("1y", now).unwrap(), now - 365 * DAY);
        // 2026-01-01 00:00 UTC.
        assert_eq!(parse_since("2026-01-01", now).unwrap(), 1_767_225_600);
        for bad in ["", "x", "10", "-3d", "0d", "3 d", "2026-13-01", "6mo"] {
            assert!(parse_since(bad, now).is_err(), "{bad} should not parse");
        }
    }

    #[test]
    fn date_roundtrips() {
        for (unix, s) in [
            (0, "1970-01-01"),
            (1_767_225_600, "2026-01-01"),
            (951_782_400, "2000-02-29"),
        ] {
            assert_eq!(parse_date(s), Some(unix));
            assert_eq!(format_date(unix), s);
        }
    }

    #[test]
    fn since_window_covers_commits_at_or_after_cutoff() {
        let now = 1000 * DAY;
        let head = ChainCommit {
            sha: "h".into(),
            time: now,
        };
        // Parents at 9, 8, 7, ... days ago.
        let c = chain(&[
            now - 9 * DAY,
            now - 18 * DAY,
            now - 27 * DAY,
            now - 36 * DAY,
        ]);
        // Cutoff 20 days ago: HEAD + parents 1-2 are inside; base = parent 3.
        let (k, clamp) =
            requested_window(&WindowSpec::Since("20d".into()), &head, &c, now).unwrap();
        assert_eq!(k, 3);
        assert_eq!(clamp, None);
        // Cutoff older than the whole chain: clamp to the root, loudly.
        let (k, clamp) =
            requested_window(&WindowSpec::Since("100d".into()), &head, &c, now).unwrap();
        assert_eq!(k, 4);
        assert_eq!(clamp, Some(Clamp::History));
    }

    #[test]
    fn since_newer_than_head_is_an_error() {
        let now = 1000 * DAY;
        let head = ChainCommit {
            sha: "h".into(),
            time: now - 30 * DAY,
        };
        let c = chain(&[now - 40 * DAY]);
        let err = requested_window(&WindowSpec::Since("7d".into()), &head, &c, now).unwrap_err();
        assert!(err.contains("older than --since"), "{err}");
    }

    #[test]
    fn commits_window_clamps_to_cap_and_history() {
        let now = 0;
        let head = ChainCommit {
            sha: "h".into(),
            time: now,
        };
        let c = chain(&[0; 10]);
        let (k, clamp) = requested_window(&WindowSpec::Commits(5), &head, &c, now).unwrap();
        assert_eq!((k, clamp), (5, None));
        let (k, clamp) = requested_window(&WindowSpec::Commits(50), &head, &c, now).unwrap();
        assert_eq!((k, clamp), (10, Some(Clamp::History)));
        let big = chain(&[0; MAX_WINDOW + 1]);
        let (k, clamp) =
            requested_window(&WindowSpec::Commits(MAX_WINDOW + 1), &head, &big, now).unwrap();
        assert_eq!((k, clamp), (MAX_WINDOW, Some(Clamp::Cap)));
    }
}
