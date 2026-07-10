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

use std::path::Path;
use std::process::ExitCode;

use argot_core::check::{run_check, CheckArgs, DEFAULT_HUNK_LINES};
use argot_core::output::OutputFormat;

use crate::worktree::TempWorktree;

/// Default window: enough history to hold real findings, small enough that
/// the base voice is still "the same repo".
pub const DEFAULT_COMMITS: usize = 50;

/// Walk first-parents back `n` commits from HEAD. Returns the HEAD SHA plus
/// the ancestor chain (`chain[k-1]` = the commit `k` back); the chain is
/// clamped to the root commit on short histories.
fn resolve_chain(repo: &Path, n: usize) -> Result<(String, Vec<String>), String> {
    let git_repo = git2::Repository::discover(repo).map_err(|e| format!("not a git repo: {e}"))?;
    let head = git_repo
        .head()
        .and_then(|h| h.peel_to_commit())
        .map_err(|e| format!("cannot resolve HEAD: {e}"))?;
    let head_sha = head.id().to_string();
    let mut chain = Vec::new();
    let mut cursor = head;
    for _ in 0..n {
        match cursor.parent(0) {
            Ok(p) => {
                chain.push(p.id().to_string());
                cursor = p;
            }
            Err(_) => break, // root commit
        }
    }
    if chain.is_empty() {
        return Err("HEAD has no parent — nothing to replay".to_string());
    }
    Ok((head_sha, chain))
}

/// True when the tree at `sha` holds at least one file today's config counts
/// as corpus source — i.e. a fit at that commit can succeed.
fn tree_has_scoped_source(
    git_repo: &git2::Repository,
    sha: &str,
    suppressions: &argot_core::suppress::PathSuppressions,
) -> bool {
    let Ok(oid) = git2::Oid::from_str(sha) else {
        return false;
    };
    let Ok(tree) = git_repo.find_commit(oid).and_then(|c| c.tree()) else {
        return false;
    };
    let mut found = false;
    let _ = tree.walk(git2::TreeWalkMode::PreOrder, |root, entry| {
        if entry.kind() == Some(git2::ObjectType::Blob) {
            if let Some(name) = entry.name() {
                let rel = format!("{root}{name}");
                if argot_core::train::is_corpus_source(&rel, suppressions) {
                    found = true;
                    return git2::TreeWalkResult::Abort;
                }
            }
        }
        git2::TreeWalkResult::Ok
    });
    found
}

/// The widest replay window whose base commit can actually be fitted: the
/// largest `k ≤ chain.len()` such that the tree `k` commits back still holds
/// in-scope source under today's config. Repos whose in-scope code is younger
/// than the requested window (e.g. a rewrite, or early history that today's
/// excludes mute entirely) shrink instead of failing. Returns 0 when no
/// ancestor qualifies.
fn max_replayable_window(
    repo: &Path,
    chain: &[String],
    suppressions: &argot_core::suppress::PathSuppressions,
) -> usize {
    let Ok(git_repo) = git2::Repository::discover(repo) else {
        return 0;
    };
    (1..=chain.len())
        .rev()
        .find(|&k| tree_has_scoped_source(&git_repo, &chain[k - 1], suppressions))
        .unwrap_or(0)
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
    widen_hint: bool,
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
            if widen_hint {
                out.push_str(&dim("  Try a wider window: argot replay --commits 200\n"));
            }
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
    let (head, chain) = match resolve_chain(repo, commits) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    // Shrink the window to the oldest ancestor a fit can succeed on: today's
    // config rides into the historical worktree, so a base commit whose whole
    // tree is excluded or unsupported (in-scope code younger than the window)
    // would otherwise die on "no source files found".
    let suppressions = argot_core::config::ArgotConfig::load(repo).path_suppressions();
    let walked = max_replayable_window(repo, &chain, &suppressions);
    if walked == 0 {
        eprintln!(
            "error: nothing to replay — none of the last {} commit(s) contain source files in \
             argot's scope; if this repo has source code, check the [exclude] patterns in argot.toml",
            chain.len()
        );
        return ExitCode::from(2);
    }
    if walked < chain.len() {
        eprintln!(
            "argot: the repo {} commit(s) ago had no code in argot's scope yet — shrinking the \
             window to {walked} commit(s)",
            chain.len()
        );
    }
    let base = chain[walked - 1].clone();
    let base_short = &base[..12.min(base.len())];

    eprintln!(
        "argot: replaying your last {walked} commit(s) against the voice as of {base_short}…"
    );
    let worktree = match TempWorktree::create(repo, &base, "argot-replay") {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::from(2);
        }
    };
    // Today's config judges the past, and the current semantic index seeds
    // the historical fit (embeddings reuse: seconds, not a full re-embed).
    worktree.adopt_current_config(repo);

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
    print!(
        "{}",
        render_report(
            &hits,
            walked,
            base_short,
            hunks,
            walked == chain.len(),
            color
        )
    );
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
        let out = render_report(&hits, 50, "abc123def456", 900, true, false);
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
        let out = render_report(&[], 50, "abc123def456", 1200, true, false);
        assert!(out.contains("all in voice"));
        assert!(out.contains("1200 hunks"));
    }

    #[test]
    fn empty_window_suggests_widening_instead_of_claiming_in_voice() {
        let out = render_report(&[], 50, "abc123def456", 0, true, false);
        assert!(out.contains("no supported source files"));
        assert!(out.contains("--commits 200"));
        assert!(!out.contains("all in voice"));
    }

    #[test]
    fn shrunk_window_does_not_suggest_widening() {
        let out = render_report(&[], 3, "abc123def456", 0, false, false);
        assert!(out.contains("no supported source files"));
        assert!(!out.contains("--commits 200"));
    }

    /// Stage `files` (path → contents) on top of the current index and commit.
    fn commit(repo: &git2::Repository, files: &[(&str, &str)], msg: &str) {
        let root = repo.workdir().unwrap();
        let mut index = repo.index().unwrap();
        for (rel, contents) in files {
            let abs = root.join(rel);
            std::fs::create_dir_all(abs.parent().unwrap()).unwrap();
            std::fs::write(&abs, contents).unwrap();
            index.add_path(Path::new(rel)).unwrap();
        }
        index.write().unwrap();
        let tree = repo.find_tree(index.write_tree().unwrap()).unwrap();
        let sig = git2::Signature::now("t", "t@example.com").unwrap();
        let parents: Vec<git2::Commit> = repo
            .head()
            .ok()
            .and_then(|h| h.peel_to_commit().ok())
            .into_iter()
            .collect();
        let parent_refs: Vec<&git2::Commit> = parents.iter().collect();
        repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parent_refs)
            .unwrap();
    }

    #[test]
    fn window_shrinks_to_the_oldest_fittable_base() {
        let tmp = std::env::temp_dir().join(format!("argot_replay_shrink_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = git2::Repository::init(&tmp).unwrap();
        // History: c1 = excluded/unsupported only, c2 introduces in-scope
        // source, c3 = HEAD. chain = [c2, c1].
        commit(
            &repo,
            &[("site/page.astro", "<html/>"), ("site/x.ts", "let x=1")],
            "c1",
        );
        commit(&repo, &[("src/a.ts", "export const a = 1")], "c2");
        commit(&repo, &[("src/a.ts", "export const a = 2")], "c3");

        let (_head, chain) = resolve_chain(&tmp, 10).unwrap();
        assert_eq!(chain.len(), 2);

        // Without user excludes, c1's site/x.ts is in scope: full window.
        let none = argot_core::suppress::PathSuppressions::recommended();
        assert_eq!(max_replayable_window(&tmp, &chain, &none), 2);

        // With today's config muting site/, c1 has nothing in scope: shrink
        // to the window whose base is c2.
        std::fs::write(tmp.join("argot.toml"), "[exclude]\npaths = [\"site/\"]\n").unwrap();
        let sup = argot_core::config::ArgotConfig::load(&tmp).path_suppressions();
        assert_eq!(max_replayable_window(&tmp, &chain, &sup), 1);

        // A config that excludes everything leaves nothing to replay.
        std::fs::write(tmp.join("argot.toml"), "[exclude]\npaths = [\"*/\"]\n").unwrap();
        let all = argot_core::config::ArgotConfig::load(&tmp).path_suppressions();
        assert_eq!(max_replayable_window(&tmp, &chain, &all), 0);

        let _ = std::fs::remove_dir_all(&tmp);
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
