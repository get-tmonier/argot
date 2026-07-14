//! AI attribution from concrete commit markers — never from style.
//!
//! A commit is `ai-assisted` iff it carries a marker an agent actually
//! writes: a `Co-authored-by:` trailer with a known agent identity, an
//! agent/bot author or committer, or an agent marker line in the body.
//! Everything else that resolves is `human` (rendered as "no AI markers" —
//! an unmarked agent commit is indistinguishable from a human one, so the
//! AI share is a floor, not a census). `unknown` is reserved for findings
//! whose introducing commit cannot be resolved (blame failure, shallow-clone
//! horizon) — first-class, never folded into `human`.
//!
//! Matching is allowlist-only: there is no generic "[bot] ⇒ AI" rule, so
//! automation bots (dependabot, renovate, github-actions…) stay `human`-
//! bucketed rather than inflating the AI share. The marker table lives in
//! `.scratch/audit-command/ATTRIBUTION-MARKERS.md` with sources.

use std::collections::HashMap;
use std::path::Path;

/// Who a commit (or a finding's introducing commit) is attributed to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Attribution {
    AiAssisted,
    Human,
    Unknown,
}

impl Attribution {
    pub fn as_str(self) -> &'static str {
        match self {
            Attribution::AiAssisted => "ai-assisted",
            Attribution::Human => "human",
            Attribution::Unknown => "unknown",
        }
    }
}

/// Agent identity emails, matched exactly (lowercased) against trailer,
/// author, and committer emails. Email beats name: model-named trailer
/// variants ("Claude Fable 5 <noreply@anthropic.com>") keep the email stable.
/// Verified defaults only — see `.scratch/audit-command/ATTRIBUTION-MARKERS.md`
/// for the sourced table.
const AGENT_EMAILS: &[&str] = &[
    "noreply@anthropic.com",   // Claude Code co-author trailer
    "cursoragent@cursor.com",  // Cursor
    "copilot@github.com",      // VS Code Copilot chat/agent (opt-in)
    "noreply@openai.com",      // Codex CLI
    "noreply@aider.chat",      // aider co-author trailer
    "amp@ampcode.com",         // Amp (Sourcegraph)
    "noreply@opencode.ai",     // opencode
    "openhands@all-hands.dev", // OpenHands
];

/// GitHub bot slugs of AI coding agents. GitHub App bot emails are
/// `<numeric-id>+<slug>@users.noreply.github.com` and the numeric ID varies
/// per bot account (Copilot alone has three), so we match the slug after
/// stripping the optional `<digits>+` prefix — never a hardcoded ID.
/// Allowlist only: automation bots (dependabot, renovate, github-actions…)
/// are absent by construction.
const AGENT_BOT_SLUGS: &[&str] = &[
    "copilot",                      // Copilot coding agent trailer/author
    "copilot-swe-agent[bot]",       // Copilot coding agent (former login)
    "chatgpt-codex-connector[bot]", // OpenAI Codex cloud
    "devin-ai-integration[bot]",    // Devin (Cognition)
    "google-labs-jules[bot]",       // Google Jules
    "cursoragent[bot]",             // Cursor cloud agent
    "claude[bot]",                  // Claude GitHub App
    "zencoder-ai[bot]",             // Zencoder
];

/// Marker lines agents write into commit message bodies, matched as a
/// lowercased substring of a message line.
const AGENT_BODY_MARKERS: &[&str] = &[
    "generated with [claude code]", // Claude Code footer (older, linked)
    "generated with claude code",   // Claude Code footer (newer, plain)
    "claude-session:",              // Claude Code session trailer
    "agent-logs-url:",              // Copilot coding agent session log
    "generated with opencode",      // opencode footer
];

/// One classified commit from the audited range.
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub short: String,
    /// First line of the message (untruncated; renderers clip).
    pub subject: String,
    pub attribution: Attribution,
    /// The concrete markers that matched, for the card/JSON (empty ⇒ human).
    pub markers: Vec<String>,
}

/// Attribution counts over every commit in the range.
#[derive(Debug, Clone, Copy, Default)]
pub struct RangeCounts {
    pub total: usize,
    pub ai_assisted: usize,
    pub human: usize,
}

fn agent_email(email: &str) -> bool {
    let email = email.trim().to_lowercase();
    if AGENT_EMAILS.contains(&email.as_str()) {
        return true;
    }
    // GitHub bot identity: `<id>+<slug>@users.noreply.github.com` (the id is
    // sometimes absent in hand-written trailers). Compare the slug exactly —
    // a substring match would let `notclaude[bot]` ride in.
    if let Some(local) = email.strip_suffix("@users.noreply.github.com") {
        let slug = local
            .split_once('+')
            .filter(|(id, _)| !id.is_empty() && id.chars().all(|c| c.is_ascii_digit()))
            .map(|(_, s)| s)
            .unwrap_or(local);
        return AGENT_BOT_SLUGS.contains(&slug);
    }
    false
}

/// aider's default commit attribution is an author-name suffix.
fn aider_name(name: &str) -> bool {
    name.trim_end().to_lowercase().ends_with("(aider)")
}

/// Classify one commit from its metadata. Returns the bucket plus the
/// concrete markers that matched (for evidence on the card).
pub fn classify(
    author_name: &str,
    author_email: &str,
    committer_name: &str,
    committer_email: &str,
    message: &str,
) -> (Attribution, Vec<String>) {
    let mut markers = Vec::new();

    for (who, name, email) in [
        ("author", author_name, author_email),
        ("committer", committer_name, committer_email),
    ] {
        if agent_email(email) {
            markers.push(format!("{who}: {name} <{email}>"));
        } else if aider_name(name) {
            markers.push(format!("{who}: {name}"));
        }
    }

    for line in message.lines() {
        let trimmed = line.trim();
        let lower = trimmed.to_lowercase();
        if let Some(rest) = lower.strip_prefix("co-authored-by:") {
            // `Name <email>` — match the email; fall back to aider's suffix.
            let email = rest
                .rfind('<')
                .and_then(|i| rest[i + 1..].strip_suffix('>'))
                .unwrap_or("")
                .trim();
            let name = rest[..rest.rfind('<').unwrap_or(rest.len())].trim();
            if agent_email(email) || aider_name(name) {
                markers.push(trimmed.to_string());
                continue;
            }
        }
        if AGENT_BODY_MARKERS.iter().any(|m| lower.contains(m)) {
            markers.push(trimmed.to_string());
        }
    }

    markers.dedup();
    if markers.is_empty() {
        (Attribution::Human, markers)
    } else {
        (Attribution::AiAssisted, markers)
    }
}

fn classify_commit(commit: &git2::Commit) -> CommitInfo {
    let sig = |s: &git2::Signature| {
        (
            s.name().unwrap_or("").to_string(),
            s.email().unwrap_or("").to_string(),
        )
    };
    let (an, ae) = sig(&commit.author());
    let (cn, ce) = sig(&commit.committer());
    let message = commit.message().unwrap_or("");
    let (attribution, markers) = classify(&an, &ae, &cn, &ce, message);
    CommitInfo {
        short: commit.id().to_string().chars().take(7).collect(),
        subject: message.lines().next().unwrap_or("").to_string(),
        attribution,
        markers,
    }
}

/// Classify every commit in `base..head` (all parents, not just the first —
/// merged branch commits are where agent trailers live). Returns the counts
/// plus a full-sha → info map for finding lookups.
pub fn attribute_range(
    repo: &Path,
    base: &str,
    head: &str,
) -> Result<(RangeCounts, HashMap<String, CommitInfo>), String> {
    let git_repo = git2::Repository::discover(repo).map_err(|e| format!("not a git repo: {e}"))?;
    let mut walk = git_repo.revwalk().map_err(|e| e.to_string())?;
    walk.push(git2::Oid::from_str(head).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    walk.hide(git2::Oid::from_str(base).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())?;
    let mut counts = RangeCounts::default();
    let mut by_sha = HashMap::new();
    for oid in walk.flatten() {
        let Ok(commit) = git_repo.find_commit(oid) else {
            continue;
        };
        let info = classify_commit(&commit);
        counts.total += 1;
        match info.attribution {
            Attribution::AiAssisted => counts.ai_assisted += 1,
            _ => counts.human += 1,
        }
        by_sha.insert(oid.to_string(), info);
    }
    Ok((counts, by_sha))
}

/// The commit inside `base..head` that introduced most of the lines in
/// `line_start..=line_end` (head coordinates): blame bounded to the range,
/// most-lines-wins, ties broken deterministically (lexicographic sha).
/// `None` ⇒ the span predates the range or blame failed (shallow horizon,
/// rename) — attribution `unknown`.
pub fn introducing_commit(
    blame: &git2::Blame,
    base: &str,
    line_start: usize,
    line_end: usize,
) -> Option<String> {
    let mut lines_by_commit: HashMap<git2::Oid, usize> = HashMap::new();
    for hunk in blame.iter() {
        let start = hunk.final_start_line();
        let end = start + hunk.lines_in_hunk().saturating_sub(1);
        if line_start > end || line_end < start {
            continue;
        }
        if hunk.is_boundary() {
            continue; // blame stopped at the range edge — outside base..head
        }
        let id = hunk.final_commit_id();
        if id.to_string() == base || id.is_zero() {
            continue;
        }
        let overlap = line_end.min(end) - line_start.max(start) + 1;
        *lines_by_commit.entry(id).or_insert(0) += overlap;
    }
    lines_by_commit
        .into_iter()
        .max_by(|a, b| {
            a.1.cmp(&b.1)
                .then_with(|| a.0.to_string().cmp(&b.0.to_string()))
        })
        .map(|(id, _)| id.to_string())
}

/// Blame `path` at `head`, bounded below by `base` — one call per distinct
/// file with findings, then `introducing_commit` per span.
pub fn blame_file<'r>(
    git_repo: &'r git2::Repository,
    base: &str,
    head: &str,
    path: &str,
) -> Option<git2::Blame<'r>> {
    let mut opts = git2::BlameOptions::new();
    opts.newest_commit(git2::Oid::from_str(head).ok()?)
        .oldest_commit(git2::Oid::from_str(base).ok()?);
    git_repo.blame_file(Path::new(path), Some(&mut opts)).ok()
}

/// The commit in `base..head` that removed `symbol` from `path`: the newest
/// commit whose version of the file lacks the name while a parent's version
/// contains it (pickaxe semantics, `git log -S`). Deletion findings can't be
/// span-blamed — the deleted lines don't exist at `head`, and blaming the
/// *neighbouring* lines credits whoever last edited around the deletion.
pub fn symbol_removal_commit(
    git_repo: &git2::Repository,
    base: &str,
    head: &str,
    path: &str,
    symbol: &str,
) -> Option<String> {
    let blob_has = |tree: &git2::Tree| -> bool {
        tree.get_path(Path::new(path))
            .ok()
            .and_then(|e| git_repo.find_blob(e.id()).ok())
            .map(|b| String::from_utf8_lossy(b.content()).contains(symbol))
            .unwrap_or(false)
    };
    let mut walk = git_repo.revwalk().ok()?;
    walk.set_sorting(git2::Sort::TOPOLOGICAL).ok()?;
    walk.push(git2::Oid::from_str(head).ok()?).ok()?;
    walk.hide(git2::Oid::from_str(base).ok()?).ok()?;
    let entry_oid = |tree: &git2::Tree| tree.get_path(Path::new(path)).map(|e| e.id()).ok();
    for oid in walk.flatten() {
        let commit = git_repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        if blob_has(&tree) {
            continue;
        }
        // TREESAME rule: a merge whose file matches one of its parents just
        // carried that side's deletion — keep walking to the real deleter.
        if commit.parent_count() > 1 {
            let own = entry_oid(&tree);
            let carried = (0..commit.parent_count()).any(|i| {
                commit
                    .parent(i)
                    .and_then(|p| p.tree())
                    .map(|t| entry_oid(&t) == own)
                    .unwrap_or(false)
            });
            if carried {
                continue;
            }
        }
        let parent_had = (0..commit.parent_count()).any(|i| {
            commit
                .parent(i)
                .and_then(|p| p.tree())
                .map(|t| blob_has(&t))
                .unwrap_or(false)
        });
        if parent_had {
            return Some(oid.to_string());
        }
    }
    None
}

/// Deletion fallback: blame sees only lines that still exist at `head`, so a
/// finding about *removed* code (a deleted test) resolves to nothing. When
/// exactly ONE commit in `base..head` touched the file, it is provably the
/// introducer; with several candidates we stay `unknown` rather than guess.
pub fn sole_touching_commit(
    git_repo: &git2::Repository,
    base: &str,
    head: &str,
    path: &str,
) -> Option<String> {
    let mut walk = git_repo.revwalk().ok()?;
    walk.push(git2::Oid::from_str(head).ok()?).ok()?;
    walk.hide(git2::Oid::from_str(base).ok()?).ok()?;
    let target = Path::new(path);
    let mut found: Option<String> = None;
    for oid in walk.flatten() {
        let commit = git_repo.find_commit(oid).ok()?;
        let tree = commit.tree().ok()?;
        let touched = if commit.parent_count() == 0 {
            tree.get_path(target).is_ok()
        } else {
            // The commit authored this file's content iff its entry differs
            // from EVERY parent — a merge that merely carries one side's
            // version is not the introducer (git's TREESAME rule).
            let entry = tree.get_path(target).map(|e| e.id()).ok();
            (0..commit.parent_count()).all(|i| {
                let parent_entry = commit
                    .parent(i)
                    .and_then(|p| p.tree())
                    .ok()
                    .and_then(|t| t.get_path(target).map(|e| e.id()).ok());
                parent_entry != entry
            })
        };
        if touched {
            if found.is_some() {
                return None; // ambiguous — several commits touched the file
            }
            found = Some(oid.to_string());
        }
    }
    found
}

#[cfg(test)]
mod tests {
    use super::*;

    fn human(msg: &str) -> (Attribution, Vec<String>) {
        classify(
            "Jane Dev",
            "jane@example.com",
            "Jane Dev",
            "jane@example.com",
            msg,
        )
    }

    #[test]
    fn claude_trailer_marks_ai_whatever_the_model_name() {
        for trailer in [
            "Co-Authored-By: Claude <noreply@anthropic.com>",
            "Co-authored-by: Claude Fable 5 <noreply@anthropic.com>",
            "co-authored-by: claude opus 4 <NOREPLY@ANTHROPIC.COM>",
        ] {
            let (a, markers) = human(&format!("add feature\n\n{trailer}"));
            assert_eq!(a, Attribution::AiAssisted, "{trailer}");
            assert_eq!(markers.len(), 1);
        }
    }

    #[test]
    fn body_markers_and_session_trailers_mark_ai() {
        let msg = "fix\n\n🤖 Generated with [Claude Code](https://claude.com/claude-code)";
        assert_eq!(human(msg).0, Attribution::AiAssisted);
        let msg = "fix\n\nClaude-Session: https://claude.ai/code/session_x";
        assert_eq!(human(msg).0, Attribution::AiAssisted);
    }

    #[test]
    fn agent_bot_author_marks_ai() {
        let (a, markers) = classify(
            "devin-ai-integration[bot]",
            "devin-ai-integration[bot]@users.noreply.github.com",
            "GitHub",
            "noreply@github.com",
            "implement endpoint",
        );
        assert_eq!(a, Attribution::AiAssisted);
        assert!(markers[0].starts_with("author:"), "{markers:?}");
    }

    #[test]
    fn aider_name_suffix_marks_ai() {
        let (a, _) = classify(
            "Jane Dev (aider)",
            "jane@example.com",
            "Jane Dev",
            "jane@example.com",
            "refactor",
        );
        assert_eq!(a, Attribution::AiAssisted);
        let msg = "tidy\n\nCo-authored-by: aider (gpt-5) <noreply@aider.chat>";
        assert_eq!(human(msg).0, Attribution::AiAssisted);
    }

    #[test]
    fn github_bot_emails_match_by_slug_whatever_the_numeric_id() {
        // Real-world form: numeric-id prefix (varies per bot account).
        for email in [
            "158243242+devin-ai-integration[bot]@users.noreply.github.com",
            "devin-ai-integration[bot]@users.noreply.github.com",
            "198982749+Copilot@users.noreply.github.com",
            "175728472+Copilot@users.noreply.github.com",
            "161369871+google-labs-jules[bot]@users.noreply.github.com",
            "199175422+chatgpt-codex-connector[bot]@users.noreply.github.com",
        ] {
            let msg = format!("x\n\nCo-authored-by: Bot <{email}>");
            assert_eq!(human(&msg).0, Attribution::AiAssisted, "{email}");
        }
        // A slug that merely contains an agent slug must not match.
        let msg = "x\n\nCo-authored-by: X <1+notclaude[bot]@users.noreply.github.com>";
        assert_eq!(human(msg).0, Attribution::Human);
    }

    #[test]
    fn other_agent_trailers_and_footers_match() {
        for marker in [
            "Co-authored-by: Codex <noreply@openai.com>",
            "Co-authored-by: Cursor <cursoragent@cursor.com>",
            "Co-authored-by: Copilot <copilot@github.com>",
            "Co-authored-by: Amp <amp@ampcode.com>",
            "Co-Authored-By: opencode <noreply@opencode.ai>",
            "🤖 Generated with opencode",
            "Agent-Logs-Url: https://github.com/x/y/runs/1",
        ] {
            let (a, m) = human(&format!("subject\n\n{marker}"));
            assert_eq!(a, Attribution::AiAssisted, "{marker}");
            assert!(!m.is_empty(), "{marker}");
        }
    }

    #[test]
    fn automation_bots_and_plain_humans_are_not_ai() {
        // Allowlist only: automation is not AI authorship.
        for (name, email) in [
            (
                "dependabot[bot]",
                "49699333+dependabot[bot]@users.noreply.github.com",
            ),
            (
                "renovate[bot]",
                "29139614+renovate[bot]@users.noreply.github.com",
            ),
            (
                "github-actions[bot]",
                "41898282+github-actions[bot]@users.noreply.github.com",
            ),
        ] {
            let (a, m) = classify(name, email, name, email, "chore: bump deps");
            assert_eq!(a, Attribution::Human, "{name}");
            assert!(m.is_empty());
        }
        // A human co-author trailer stays human.
        let msg = "pair work\n\nCo-authored-by: Sam Peer <sam@example.com>";
        assert_eq!(human(msg).0, Attribution::Human);
        // Mentioning Claude in prose is not a marker.
        assert_eq!(
            human("docs: describe claude code setup").0,
            Attribution::Human
        );
    }

    #[test]
    fn markers_are_reported_verbatim() {
        let (_, markers) = human("x\n\nCo-Authored-By: Claude <noreply@anthropic.com>");
        assert_eq!(
            markers,
            vec!["Co-Authored-By: Claude <noreply@anthropic.com>"]
        );
    }

    /// Stage `files` (path → contents) and commit; returns the new sha.
    fn commit(repo: &git2::Repository, files: &[(&str, &str)], msg: &str) -> String {
        let root = repo.workdir().unwrap();
        let mut index = repo.index().unwrap();
        for (rel, contents) in files {
            std::fs::write(root.join(rel), contents).unwrap();
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
            .unwrap()
            .to_string()
    }

    /// Canary for the libgit2 pin (root Cargo.toml): the 1.9.x blame rewrite
    /// inserts a NULL hunk when a commit in the blame graph has an empty
    /// author email (`name <>`) and segfaults in `git_blame_free`. Found on
    /// rubocop (`argot audit`, commit 3bff9d37 authored `5hun-s <>`). On the
    /// pinned 1.8.x this passes; on a buggy 1.9.x it crashes the test runner.
    #[test]
    fn blame_survives_empty_email_author() {
        let tmp = std::env::temp_dir().join(format!("argot_audit_noemail_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = git2::Repository::init(&tmp).unwrap();
        let base = commit(&repo, &[("a.py", "x = 1\n")], "base");

        // The trigger commit: author/committer with an empty email. libgit2
        // refuses to *create* such a signature — which is exactly why its
        // 1.9 blame chokes on one it merely *reads* (other tools write them
        // in the wild) — so forge the raw commit object through the ODB,
        // which does not validate content.
        std::fs::write(tmp.join("a.py"), "x = 1\ny = 2\n").unwrap();
        let mut index = repo.index().unwrap();
        index.add_path(Path::new("a.py")).unwrap();
        index.write().unwrap();
        let tree = index.write_tree().unwrap();
        let raw = format!(
            "tree {tree}\nparent {base}\n\
             author no-email <> 1700000000 +0000\n\
             committer no-email <> 1700000000 +0000\n\nempty-email edit\n"
        );
        let c1 = repo
            .odb()
            .unwrap()
            .write(git2::ObjectType::Commit, raw.as_bytes())
            .unwrap()
            .to_string();
        let head_ref = repo.head().unwrap().name().unwrap().to_string();
        repo.reference(
            &head_ref,
            git2::Oid::from_str(&c1).unwrap(),
            true,
            "forge empty-email commit",
        )
        .unwrap();

        let head = commit(&repo, &[("a.py", "x = 1\ny = 2\nz = 3\n")], "head");

        let blame = blame_file(&repo, &base, &head, "a.py").expect("blame succeeds");
        assert_eq!(
            introducing_commit(&blame, &base, 2, 2).as_deref(),
            Some(c1.as_str()),
            "the empty-email commit is still attributable"
        );
        drop(blame); // 1.9.x segfaults here (free_hunk on a NULL hunk)
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn symbol_removal_walk_finds_the_deleting_commit_not_a_neighbour() {
        let tmp = std::env::temp_dir().join(format!("argot_audit_symrm_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = git2::Repository::init(&tmp).unwrap();
        let base = commit(
            &repo,
            &[(
                "t.py",
                "def test_a():\n    assert 1\n\ndef test_b():\n    assert 2\n",
            )],
            "base",
        );
        // Deleter first, then a later commit edits nearby lines — the walk
        // must credit the deleter, where neighbour-blame would credit c2.
        let c1 = commit(
            &repo,
            &[("t.py", "def test_b():\n    assert 2\n")],
            "delete test_a",
        );
        let head = commit(
            &repo,
            &[("t.py", "def test_b():\n    assert 22\n")],
            "edit test_b",
        );

        assert_eq!(
            symbol_removal_commit(&repo, &base, &head, "t.py", "test_a").as_deref(),
            Some(c1.as_str())
        );
        // A symbol that never existed resolves to nothing.
        assert_eq!(
            symbol_removal_commit(&repo, &base, &head, "t.py", "test_zz"),
            None
        );
        let _ = std::fs::remove_dir_all(&tmp);
    }

    #[test]
    fn sole_touching_commit_attributes_deletions_only_when_provable() {
        let tmp = std::env::temp_dir().join(format!("argot_audit_sole_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&tmp);
        std::fs::create_dir_all(&tmp).unwrap();
        let repo = git2::Repository::init(&tmp).unwrap();
        let base = commit(
            &repo,
            &[("a.py", "def f():\n    pass\n"), ("b.py", "x = 1\n")],
            "base",
        );
        let c1 = commit(&repo, &[("a.py", "def f():\n    return 2\n")], "touch a");
        commit(&repo, &[("b.py", "x = 2\n")], "touch b once");
        let head = commit(&repo, &[("b.py", "x = 3\n")], "touch b twice");

        // a.py touched by exactly one commit in range → provable introducer.
        assert_eq!(
            sole_touching_commit(&repo, &base, &head, "a.py").as_deref(),
            Some(c1.as_str())
        );
        // b.py touched twice → ambiguous, stays unknown.
        assert_eq!(sole_touching_commit(&repo, &base, &head, "b.py"), None);
        let _ = std::fs::remove_dir_all(&tmp);
    }
}
