//! `argot review <target>` — score a PR (or arbitrary diff) against the local
//! voice without checking it out.
//!
//! For a PR the head commit is fetched into the object store via
//! `refs/pull/<n>/head` (a fetch, not a checkout — the working tree is never
//! touched), then the range `base..head` runs through the same `run_check`
//! scoring + rendering as `argot check`, with a PR header prepended. Arbitrary
//! `base..head` ranges and single commits pass straight through to check.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;

/// What the user asked us to review.
#[derive(Debug, PartialEq, Eq)]
pub enum ReviewTarget {
    /// A pull request: `#123`, `123`, or a full github URL.
    Pr { number: u64, repo: Option<String> },
    /// A `base..head` diff range.
    Range(String),
    /// A single commit sha or ref.
    Commit(String),
}

/// Parse a review target. Order matters: URLs and `#n`/`n` are PRs; anything
/// with `..` is a range; the rest is a single commit/ref.
pub fn parse_target(s: &str) -> ReviewTarget {
    let s = s.trim();
    // https://github.com/<owner>/<repo>/pull/<n>[/files...]
    if let Some(rest) = s
        .strip_prefix("https://github.com/")
        .or_else(|| s.strip_prefix("http://github.com/"))
    {
        let parts: Vec<&str> = rest.split('/').collect();
        if parts.len() >= 4 && parts[2] == "pull" {
            if let Ok(n) = parts[3]
                .trim_end_matches(|c: char| !c.is_ascii_digit())
                .parse::<u64>()
            {
                return ReviewTarget::Pr {
                    number: n,
                    repo: Some(format!("{}/{}", parts[0], parts[1])),
                };
            }
        }
    }
    let bare = s.strip_prefix('#').unwrap_or(s);
    if !bare.is_empty() && bare.bytes().all(|b| b.is_ascii_digit()) {
        if let Ok(n) = bare.parse::<u64>() {
            return ReviewTarget::Pr {
                number: n,
                repo: None,
            };
        }
    }
    if s.contains("..") {
        return ReviewTarget::Range(s.to_string());
    }
    ReviewTarget::Commit(s.to_string())
}

/// PR metadata for the header block (from `gh pr view --json`).
struct PrInfo {
    number: u64,
    title: String,
    author: String,
    base_ref: String,
    head_ref: String,
    base_oid: String,
    head_oid: String,
    url: String,
}

/// Render the header shown above the scored hits.
fn pr_header(info: &PrInfo) -> String {
    format!(
        "argot review · PR #{} — {}\n  {} → {} · by {}\n  {}\n\n",
        info.number, info.title, info.head_ref, info.base_ref, info.author, info.url
    )
}

pub fn run_review(target: &str, repo: PathBuf, format: &str, use_color: bool) -> ExitCode {
    let parsed = parse_target(target);
    match parsed {
        ReviewTarget::Range(range) => run_check_ref(&repo, &range, format, use_color, None),
        ReviewTarget::Commit(sha) => run_check_ref(&repo, &sha, format, use_color, None),
        ReviewTarget::Pr {
            number,
            repo: pr_repo,
        } => run_pr_review(number, pr_repo, repo, format, use_color),
    }
}

fn run_pr_review(
    number: u64,
    pr_repo: Option<String>,
    repo: PathBuf,
    format: &str,
    use_color: bool,
) -> ExitCode {
    let info = match fetch_pr_info(number, pr_repo.as_deref(), &repo) {
        Ok(i) => i,
        Err(msg) => {
            eprintln!("{msg}");
            return ExitCode::from(2);
        }
    };
    // Fetch the PR head into the object store (no checkout, no branch).
    if let Err(msg) = fetch_pr_head(number, &info.head_oid, &repo) {
        eprintln!("{msg}");
        return ExitCode::from(2);
    }
    let range = format!("{}..{}", info.base_oid, info.head_oid);
    // A PR-level voice-diff headline above the per-hunk hits (sibling #62).
    let mut header = pr_header(&info);
    if !OutputFormat::parse(format).unwrap_or_default().is_machine() {
        if let Some(summary) =
            crate::voice_diff::summary_for_ref(&repo, &range, crate::voice_diff::DEFAULT_TOP)
        {
            header.push_str(&crate::voice_diff::one_liner(&summary));
            header.push_str("\n\n");
        }
    }
    run_check_ref(&repo, &range, format, use_color, Some(header))
}

/// Run `check` on a ref/range and print the outcome, optionally prefixing a
/// human-format header (machine formats own stdout, so the header is skipped).
fn run_check_ref(
    repo: &Path,
    reference: &str,
    format: &str,
    use_color: bool,
    header: Option<String>,
) -> ExitCode {
    let fmt = OutputFormat::parse(format).unwrap_or_default();
    let outcome = run_check(CheckArgs {
        repo_path: repo.to_string_lossy().into_owned(),
        reference: reference.to_string(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: repo.join(".argot"),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_severity: "unusual".to_string(),
        use_color,
        format: fmt,
        today: crate::today_utc(),
    });
    if let Some(h) = header {
        if !fmt.is_machine() {
            print!("{h}");
        }
    }
    print!("{}", outcome.stdout);
    eprint!("{}", outcome.stderr);
    ExitCode::from(outcome.exit_code as u8)
}

/// `gh pr view` for the PR's metadata + base/head SHAs.
fn fetch_pr_info(number: u64, repo: Option<&str>, cwd: &Path) -> Result<PrInfo, String> {
    let mut cmd = Command::new("gh");
    cmd.current_dir(cwd).args([
        "pr",
        "view",
        &number.to_string(),
        "--json",
        "number,title,author,baseRefName,headRefName,headRefOid,baseRefOid,url",
    ]);
    if let Some(r) = repo {
        cmd.args(["--repo", r]);
    }
    let out = cmd.output().map_err(|e| {
        if e.kind() == std::io::ErrorKind::NotFound {
            "error: `gh` CLI not found — install it and run `gh auth login`".to_string()
        } else {
            format!("error: could not run `gh`: {e}")
        }
    })?;
    if !out.status.success() {
        let err = String::from_utf8_lossy(&out.stderr);
        return Err(format!(
            "error: could not read PR #{number} via gh: {}",
            err.trim()
        ));
    }
    let v: serde_json::Value = serde_json::from_slice(&out.stdout)
        .map_err(|e| format!("error: unexpected gh output: {e}"))?;
    let s = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or("").to_string();
    Ok(PrInfo {
        number,
        title: s("title"),
        author: v
            .get("author")
            .and_then(|a| a.get("login"))
            .and_then(|l| l.as_str())
            .unwrap_or("unknown")
            .to_string(),
        base_ref: s("baseRefName"),
        head_ref: s("headRefName"),
        base_oid: s("baseRefOid"),
        head_oid: s("headRefOid"),
        url: s("url"),
    })
}

/// Fetch the PR head commit into the local object store without touching the
/// working tree. Tries the head SHA directly, then `refs/pull/<n>/head`.
fn fetch_pr_head(number: u64, head_oid: &str, cwd: &Path) -> Result<(), String> {
    // Already present (e.g. same-repo branch)? Then nothing to fetch.
    let have = Command::new("git")
        .current_dir(cwd)
        .args(["cat-file", "-e", head_oid])
        .status()
        .map(|s| s.success())
        .unwrap_or(false);
    if have {
        return Ok(());
    }
    let pull_ref = format!("refs/pull/{number}/head");
    let status = Command::new("git")
        .current_dir(cwd)
        .args(["fetch", "--quiet", "origin", &pull_ref])
        .status()
        .map_err(|_| "error: `git` not found".to_string())?;
    if !status.success() {
        return Err(format!(
            "error: could not fetch {pull_ref} from origin (is the PR on this repo's origin?)"
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_pr_urls_numbers_and_hashes() {
        assert_eq!(
            parse_target("https://github.com/get-tmonier/argot/pull/89"),
            ReviewTarget::Pr {
                number: 89,
                repo: Some("get-tmonier/argot".to_string())
            }
        );
        assert_eq!(
            parse_target("https://github.com/get-tmonier/argot/pull/89/files"),
            ReviewTarget::Pr {
                number: 89,
                repo: Some("get-tmonier/argot".to_string())
            }
        );
        assert_eq!(
            parse_target("#123"),
            ReviewTarget::Pr {
                number: 123,
                repo: None
            }
        );
        assert_eq!(
            parse_target("123"),
            ReviewTarget::Pr {
                number: 123,
                repo: None
            }
        );
    }

    #[test]
    fn distinguishes_ranges_from_single_commits() {
        assert_eq!(
            parse_target("main..HEAD"),
            ReviewTarget::Range("main..HEAD".to_string())
        );
        assert_eq!(
            parse_target("abc1234"),
            ReviewTarget::Commit("abc1234".to_string())
        );
    }

    #[test]
    fn pr_header_names_the_pr_and_the_direction() {
        let info = PrInfo {
            number: 7,
            title: "Add widget".to_string(),
            author: "alice".to_string(),
            base_ref: "main".to_string(),
            head_ref: "feat/widget".to_string(),
            base_oid: "aaa".to_string(),
            head_oid: "bbb".to_string(),
            url: "https://github.com/o/r/pull/7".to_string(),
        };
        let h = pr_header(&info);
        assert!(h.contains("PR #7 — Add widget"));
        assert!(h.contains("feat/widget → main"));
        assert!(h.contains("by alice"));
    }
}
