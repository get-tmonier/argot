//! `argot replay` — what would argot have caught in YOUR history?
//!
//! The honest demo problem: argot fits on the repo as it stands, so the code
//! it learned from is in-voice by definition — a fresh install has nothing to
//! show. Replay solves it the same way the benchmarks do, as a temporal
//! holdout turned user-facing:
//!
//! 1. take the commit `N` back on the first-parent line (default 50),
//! 2. fit the voice **as of that commit** in a temporary `git worktree`
//!    (the user's tree and `.argot/` are never touched; the current
//!    `argot.toml` rides along so today's excludes judge the past, and the
//!    current semantic index seeds the fit so unchanged functions reuse
//!    their embeddings instead of re-embedding),
//! 3. score `base..HEAD` against that voice and render a compact report:
//!    counts per rule, the strongest examples, and the honest framing —
//!    merged code is accepted code, so each hit is "would have prompted
//!    review before merge", not a bug list.
//!
//! Informational by design: always exits 0 on success, 2 when it can't run.

use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;

/// Default window: enough history to hold real findings, small enough that
/// the base voice is still "the same repo".
pub const DEFAULT_COMMITS: usize = 50;

/// Walk first-parents back `n` commits from HEAD. Returns (base, head) full
/// SHAs; base is clamped to the root commit on short histories.
fn resolve_range(repo: &Path, n: usize) -> Result<(String, String, usize), String> {
    let git_repo = git2::Repository::discover(repo).map_err(|e| format!("not a git repo: {e}"))?;
    let head = git_repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|e| format!("cannot resolve HEAD: {e}"))?;
    let head_sha = head.id().to_string();
    let mut base = head.clone();
    let mut walked = 0usize;
    for _ in 0..n {
        match base.parent(0) {
            Ok(p) => {
                base = p;
                walked += 1;
            }
            Err(_) => break, // root commit
        }
    }
    if walked == 0 {
        return Err("HEAD has no parent — nothing to replay".to_string());
    }
    Ok((base.id().to_string(), head_sha, walked))
}

/// `git -C <repo> <args>` — replay shells out for worktree management only
/// (the one git feature libgit2 makes harder than the porcelain).
fn git(repo: &Path, args: &[&str]) -> Result<(), String> {
    let status = Command::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .map_err(|e| format!("git not found on PATH ({e}) — replay needs the git CLI"))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!("git {} failed", args.join(" ")))
    }
}

/// A temp worktree that cleans up after itself (best-effort).
struct TempWorktree {
    repo: PathBuf,
    path: PathBuf,
}

impl TempWorktree {
    fn create(repo: &Path, sha: &str) -> Result<Self, String> {
        let path = std::env::temp_dir().join(format!("argot-replay-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&path);
        git(
            repo,
            &["worktree", "add", "--detach", &path.to_string_lossy(), sha],
        )?;
        Ok(Self {
            repo: repo.to_path_buf(),
            path,
        })
    }
}

impl Drop for TempWorktree {
    fn drop(&mut self) {
        let _ = git(
            &self.repo,
            &[
                "worktree",
                "remove",
                "--force",
                &self.path.to_string_lossy(),
            ],
        );
        let _ = std::fs::remove_dir_all(&self.path);
        let _ = git(&self.repo, &["worktree", "prune"]);
    }
}

/// One parsed hit from check's JSON document.
struct ReplayHit {
    rule: String,
    confidence: String,
    path: String,
    line_start: u64,
    line_end: u64,
    source: String,
    evidence: Option<String>,
}

fn parse_hits(json: &str) -> Vec<ReplayHit> {
    let Ok(doc) = serde_json::from_str::<serde_json::Value>(json) else {
        return Vec::new();
    };
    doc["hits"]
        .as_array()
        .map(|hits| {
            hits.iter()
                .filter_map(|h| {
                    Some(ReplayHit {
                        rule: h["rule"].as_str()?.to_string(),
                        confidence: h["confidence"].as_str().unwrap_or("unusual").to_string(),
                        path: h["path"].as_str()?.to_string(),
                        line_start: h["line_start"].as_u64().unwrap_or(0),
                        line_end: h["line_end"].as_u64().unwrap_or(0),
                        source: h["source"].as_str().unwrap_or("").to_string(),
                        evidence: h["evidence"]
                            .as_array()
                            .and_then(|e| e.first())
                            .and_then(|e| e.as_str())
                            .map(String::from),
                    })
                })
                .collect()
        })
        .unwrap_or_default()
}

fn paint(text: &str, code: &str, color: bool) -> String {
    if color {
        format!("\x1b[{code}m{text}\x1b[0m")
    } else {
        text.to_string()
    }
}

fn confidence_rank(c: &str) -> usize {
    match c {
        "foreign" => 2,
        "suspicious" => 1,
        _ => 0,
    }
}

/// Render the replay report: per-rule counts, the strongest examples, honest
/// framing. `commits` is the replayed window size.
fn render_report(
    hits: &[ReplayHit],
    commits: usize,
    base_short: &str,
    hunks_scanned: u64,
    color: bool,
) -> String {
    let mut out = String::new();
    let dim = |s: &str| paint(s, "2", color);
    let bold = |s: &str| paint(s, "1", color);

    out.push_str(&bold(&format!(
        "━━ argot replay · {commits} commits, judged by the voice as of {base_short} ━━"
    )));
    out.push('\n');
    out.push('\n');

    if hits.is_empty() {
        if hunks_scanned == 0 {
            out.push_str(&format!(
                "  These {commits} commits touched no supported source files (docs-only?).\n"
            ));
            out.push_str(&dim("  Try a wider window: argot replay --commits 200\n"));
        } else {
            out.push_str(&format!(
                "  Nothing argot would have raised — {hunks_scanned} hunks replayed, all in voice.\n"
            ));
            out.push_str(&dim(
                "  (A quiet replay is a good sign: your recent history speaks the repo's language.)\n",
            ));
        }
        return out;
    }

    out.push_str(&format!(
        "  {} finding(s) argot would have raised before merge, out of {hunks_scanned} hunks:\n\n",
        hits.len()
    ));

    // Per-rule counts, most frequent first, stable by name on ties.
    let mut counts: Vec<(String, usize)> = Vec::new();
    for h in hits {
        match counts.iter_mut().find(|(r, _)| *r == h.rule) {
            Some((_, n)) => *n += 1,
            None => counts.push((h.rule.clone(), 1)),
        }
    }
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    let width = counts.iter().map(|(r, _)| r.len()).max().unwrap_or(0);
    for (rule, n) in &counts {
        out.push_str(&format!("    {rule:<width$}  ×{n}\n"));
    }
    out.push('\n');

    // The strongest examples: highest confidence first, then diverse rules.
    let mut order: Vec<usize> = (0..hits.len()).collect();
    order.sort_by(|&a, &b| {
        confidence_rank(&hits[b].confidence).cmp(&confidence_rank(&hits[a].confidence))
    });
    let mut shown_rules: Vec<&str> = Vec::new();
    let mut examples: Vec<usize> = Vec::new();
    // First pass: one example per rule; second pass: fill up to 5.
    for &i in &order {
        if examples.len() >= 5 {
            break;
        }
        if !shown_rules.contains(&hits[i].rule.as_str()) {
            shown_rules.push(&hits[i].rule);
            examples.push(i);
        }
    }
    for &i in &order {
        if examples.len() >= 5 {
            break;
        }
        if !examples.contains(&i) {
            examples.push(i);
        }
    }

    out.push_str("  worth a look first:\n");
    for &i in &examples {
        let h = &hits[i];
        let glyph = match h.confidence.as_str() {
            "foreign" => paint("!", "31", color),
            "suspicious" => paint("?", "33", color),
            _ => paint(".", "34", color),
        };
        let lines = if h.line_start == h.line_end {
            format!("L{}", h.line_start)
        } else {
            format!("L{}-L{}", h.line_start, h.line_end)
        };
        out.push_str(&format!(
            "  {glyph} {}:{lines}  {}  · {}\n",
            h.path,
            bold(&h.rule),
            h.source
        ));
        if let Some(ev) = &h.evidence {
            out.push_str(&dim(&format!("      {ev}\n")));
        }
    }
    out.push('\n');
    out.push_str(&dim(
        "  Merged code is accepted code — read each as \"would have prompted review\",\n",
    ));
    out.push_str(&dim(
        "  not as a bug list. A fire on a dependency you adopted on purpose is a\n",
    ));
    out.push_str(&dim("  detection working as intended.\n"));
    out
}

/// Run the whole replay. Informational: exits 0 on success even with findings.
pub fn run_replay(repo: &Path, commits: usize) -> ExitCode {
    let (base, head, walked) = match resolve_range(repo, commits) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    let base_short = &base[..12.min(base.len())];

    eprintln!(
        "argot: replaying your last {walked} commit(s) against the voice as of {base_short}…"
    );
    let worktree = match TempWorktree::create(repo, &base) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };

    // Today's config judges the past: the user's current excludes and rule
    // severities ride into the worktree (the checkout has the old ones).
    for name in [
        argot_core::config::CONFIG_FILE,
        argot_core::config::LOCAL_CONFIG_FILE,
    ] {
        let src = repo.join(name);
        if src.is_file() {
            let _ = std::fs::copy(&src, worktree.path.join(name));
        }
    }
    // Seed the semantic index from the current fit (when present): the
    // incremental build reuses embeddings for every function that already
    // existed, so the historical fit costs seconds, not a full re-embed.
    let seed = repo.join(".argot").join("semantic-index.json");
    if seed.is_file() {
        let dst_dir = worktree.path.join(".argot");
        let _ = std::fs::create_dir_all(&dst_dir);
        let _ = std::fs::copy(&seed, dst_dir.join("semantic-index.json"));
    }

    eprintln!("argot: fitting the historical voice (one-off, in a temp worktree)…");
    if crate::fit_repo(&worktree.path, &[]).is_err() {
        eprintln!("error: fitting at {base_short} failed");
        return ExitCode::from(2);
    }

    let outcome = run_check(CheckArgs {
        repo_path: worktree.path.to_string_lossy().into_owned(),
        reference: format!("{base}..{head}"),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: worktree.path.join(".argot"),
        hunk_lines: DEFAULT_HUNK_LINES,
        verbose: false,
        min_confidence: "unusual".to_string(),
        rule_overrides: Vec::new(),
        error_on_warnings: false,
        add_ignores: false,
        use_color: false,
        format: OutputFormat::Json,
        today: crate::today_utc(),
    });
    if outcome.exit_code >= 2 {
        eprint!("{}", outcome.stderr);
        eprintln!("error: replay check failed");
        return ExitCode::from(2);
    }

    let hits = parse_hits(&outcome.stdout);
    let hunks = serde_json::from_str::<serde_json::Value>(&outcome.stdout)
        .ok()
        .and_then(|d| d["hunks_scanned"].as_u64())
        .unwrap_or(0);
    let color = std::env::var_os("NO_COLOR").is_none()
        && std::io::IsTerminal::is_terminal(&std::io::stdout());
    println!();
    print!("{}", render_report(&hits, walked, base_short, hunks, color));
    println!();
    println!("Full detail: argot check {base_short}..HEAD   (after refreshing the fit)");
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hit(rule: &str, confidence: &str, path: &str) -> ReplayHit {
        ReplayHit {
            rule: rule.to_string(),
            confidence: confidence.to_string(),
            path: path.to_string(),
            line_start: 1,
            line_end: 4,
            source: "a1b2c3d".to_string(),
            evidence: Some("↳ evidence line".to_string()),
        }
    }

    #[test]
    fn report_counts_rules_and_leads_with_strongest() {
        let hits = vec![
            hit("rare-tokens", "unusual", "a.py"),
            hit("foreign-import", "foreign", "b.py"),
            hit("foreign-import", "foreign", "c.py"),
            hit("redundant", "unusual", "d.py"),
        ];
        let out = render_report(&hits, 50, "abc123def456", 900, false);
        assert!(out.contains("4 finding(s)"));
        assert!(out.contains("foreign-import  ×2"));
        assert!(out.contains("redundant"));
        // The strongest (foreign confidence) example is listed first.
        let first_example = out
            .lines()
            .find(|l| l.trim_start().starts_with('!'))
            .unwrap();
        assert!(first_example.contains("foreign-import"), "{first_example}");
        assert!(out.contains("would have prompted review"));
    }

    #[test]
    fn quiet_replay_is_a_positive_message() {
        let out = render_report(&[], 50, "abc123def456", 1200, false);
        assert!(out.contains("all in voice"));
        assert!(out.contains("1200 hunks"));
    }

    #[test]
    fn empty_window_suggests_widening_instead_of_claiming_in_voice() {
        let out = render_report(&[], 50, "abc123def456", 0, false);
        assert!(out.contains("no supported source files"));
        assert!(out.contains("--commits 200"));
        assert!(!out.contains("all in voice"));
    }

    #[test]
    fn parse_hits_reads_check_json() {
        let json = r#"{"hunks_scanned": 3, "hits": [{"rule":"foreign-import","confidence":"foreign","path":"x.py","line_start":1,"line_end":2,"source":"deadbee","evidence":["↳ requests — 0 of 74"]}]}"#;
        let hits = parse_hits(json);
        assert_eq!(hits.len(), 1);
        assert_eq!(hits[0].rule, "foreign-import");
        assert_eq!(hits[0].evidence.as_deref(), Some("↳ requests — 0 of 74"));
    }
}
