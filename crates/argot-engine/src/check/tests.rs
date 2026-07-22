use super::collect::net_range_patches;
use super::render::{confidence, insert_ignore_comments};
use super::*;
use crate::rules;
use crate::suppress::parse_inline;

fn finding(
    reason: &str,
    suppressed_by: Option<crate::finding::SuppressedBy>,
) -> crate::finding::Finding {
    crate::finding::Finding {
        score: 1.0,
        file_path: "src/app.py".to_string(),
        line: 1,
        line_end: 1,
        source: "workdir".to_string(),
        reason: reason.to_string(),
        flagged: true,
        threshold: 1.0,
        hunk_content: "x\n".to_string(),
        evidence: None,
        hash: "a1b2c3d4e5f6".to_string(),
        suppressed_by,
    }
}

#[test]
fn hidden_findings_keep_their_gate_and_are_counted() {
    let error = finding("import", None);
    let warn = finding("test_weakened", None);
    let suppressed = finding("import", Some(crate::finding::SuppressedBy::Mute));
    let settings = rules::RuleSettings::resolve(&[]);
    let summary = super::orchestrate::result_summary(
        &[&error, &warn],
        &[&warn],
        usize::from(suppressed.suppressed_by.is_some()),
        &settings,
        false,
    );

    assert_eq!(summary.exit_code, 1, "a hidden error still fails the run");
    assert_eq!(summary.unsuppressed_hits, 2);
    assert_eq!(summary.visible_hits, 1);
    assert_eq!(summary.hidden_hits, 1);
    assert_eq!(summary.suppressed_hits, 1);
    assert_eq!(summary.error_hits, 1);
    assert_eq!(summary.warn_hits, 1);
    assert_eq!(summary.gating_hits, 1);
}

#[test]
fn status_uses_all_unsuppressed_findings_not_the_displayed_tier() {
    let error = finding("import", None);
    let warn = finding("test_weakened", None);
    let suppressed = finding("import", Some(crate::finding::SuppressedBy::Mute));
    let settings = rules::RuleSettings::resolve(&[]);

    // The three confidence tiers select presentation only. No displayed hit,
    // the warn hit only, and every hit must retain the same error status.
    for visible in [&[][..], &[&warn][..], &[&error, &warn][..]] {
        let summary =
            super::orchestrate::result_summary(&[&error, &warn], visible, 0, &settings, false);
        assert_eq!(summary.exit_code, 1);
    }

    let warn_default = super::orchestrate::result_summary(&[&warn], &[], 0, &settings, false);
    assert_eq!(
        warn_default.exit_code, 0,
        "warn-only is advisory by default"
    );
    let warn_strict = super::orchestrate::result_summary(&[&warn], &[], 0, &settings, true);
    assert_eq!(
        warn_strict.exit_code, 1,
        "strict warnings still gate when hidden"
    );
    let suppressed_only = super::orchestrate::result_summary(
        &[],
        &[],
        usize::from(suppressed.suppressed_by.is_some()),
        &settings,
        true,
    );
    assert_eq!(suppressed_only.exit_code, 0);
    assert_eq!(suppressed_only.suppressed_hits, 1);
}

#[test]
fn human_brief_leads_with_severity_then_rule_span_and_action() {
    let mut error = finding("import", None);
    error.file_path = "z.py".to_string();
    error.line = 9;
    error.hash = "errorhash001".to_string();
    let mut warn = finding("test_weakened", None);
    warn.file_path = "a.py".to_string();
    warn.line = 2;
    warn.hash = "warnhash0002".to_string();
    let settings = rules::RuleSettings::resolve(&[]);

    let mut out = String::new();
    assert!(!super::render::render_results(
        &[&warn, &error],
        None,
        false,
        &settings,
        &mut out
    ));
    assert!(out.starts_with("2 findings need a look (1 error, 1 warning)\n"));
    assert!(!out.contains("style linter"));
    assert!(
        out.find("error · foreign-import · z.py:L9").unwrap()
            < out.find("warn · test-weakened · a.py:L2").unwrap()
    );
    assert!(out.contains("argot mute errorhash001 --reason"));
}

#[test]
fn insert_ignore_comments_bottom_up_with_indentation() {
    let src = "def a():\n    x = 1\n    y = 2\n\ndef b():\n    z = 3\n";
    let out = insert_ignore_comments(
        src,
        &[
            (2, "# argot: ignore-next-line — r1".to_string()),
            (
                6,
                "# argot: ignore-next-line rule=redundant — r2".to_string(),
            ),
        ],
    );
    let lines: Vec<&str> = out.lines().collect();
    // Indentation copied from the target line; both landed above their
    // original targets despite the insertions shifting line numbers.
    assert_eq!(lines[1], "    # argot: ignore-next-line — r1");
    assert_eq!(lines[2], "    x = 1");
    assert_eq!(
        lines[6],
        "    # argot: ignore-next-line rule=redundant — r2"
    );
    assert_eq!(lines[7], "    z = 3");
    // The inserted comments parse as real suppressions.
    let sup = parse_inline(&out, "#", crate::rules::Registry::builtin());
    assert_eq!(sup.rules.len(), 2);
    assert!(sup.warnings.is_empty());
}

#[test]
fn integrity_reasons_have_labels_and_pinned_confidence() {
    assert_eq!(
        rules::label_for_reason("test_disabled"),
        "test disabled alongside code change"
    );
    assert_eq!(rules::code_for_reason("test_weakened"), "test-weakened");
    // Integrity findings are discrete evidenced events — mid tier.
    assert_eq!(confidence("test_deleted", 1.0, 0.5), "suspicious");
    assert_eq!(confidence("test_disabled", 1.0, 0.5), "suspicious");
    assert_eq!(confidence("test_weakened", 1.0, 0.5), "suspicious");
}

#[test]
fn semantic_reasons_have_labels_and_pinned_confidence() {
    assert_eq!(
        rules::label_for_reason("redundant"),
        "already implemented here"
    );
    assert_eq!(rules::label_for_reason("misplaced"), "unusual location");
    // Advisory findings are the mildest tier regardless of score.
    assert_eq!(confidence("redundant", 5.0, 0.1), "unusual");
    assert_eq!(confidence("misplaced", 5.0, 0.1), "unusual");
}

#[test]
fn foreign_import_tiers_as_foreign_regardless_of_margin() {
    // The import signal is categorical: score is a count of never-before-seen
    // modules against a threshold of 1.0, so a lone foreign import sits exactly
    // at the bar. It must still read as `foreign` — the strongest tier — not
    // fall through the BPE-margin logic into `unusual`.
    assert_eq!(confidence("import", 1.0, 1.0), "foreign");
    assert_eq!(confidence("import", 3.0, 1.0), "foreign");
}

#[test]
fn distributional_signals_grade_by_margin() {
    // BPE / convention / call_receiver keep the additive-margin tiering, which
    // is calibrated for their nat-scale scores.
    let t = 8.0;
    assert_eq!(confidence("bpe", t, t), "unusual");
    assert_eq!(confidence("bpe", t + 0.5, t), "suspicious");
    assert_eq!(confidence("bpe", t + 1.5, t), "foreign");
    assert_eq!(confidence("call_receiver", t + 0.4, t), "unusual");
    assert_eq!(confidence("convention", t + 1.6, t), "foreign");
}

#[test]
fn net_range_scores_the_pr_result_not_each_commit() {
    // base → (add file with a foreign import) → (rewrite it clean). The net
    // diff base..head is the clean file, so the reverted import must not
    // appear in the scored range — a fix commit clears a prior flag.
    let dir = std::env::temp_dir().join(format!("argot_netrange_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let repo = git2::Repository::init(&dir).unwrap();
    std::fs::write(dir.join("keep.ts"), "export const x = 1\n").unwrap();
    let base = commit_all(&repo, "base");
    std::fs::write(
        dir.join("h.ts"),
        "import { Router } from 'express'\nexport const r = Router()\n",
    )
    .unwrap();
    commit_all(&repo, "add express handler");
    std::fs::write(
        dir.join("h.ts"),
        "import { Hono } from 'hono'\nexport const r = new Hono()\n",
    )
    .unwrap();
    let head = commit_all(&repo, "rewrite in hono style");

    let path = dir.to_str().unwrap();
    let patches = net_range_patches(path, &base.to_string(), &head.to_string()).unwrap();
    let h = patches
        .iter()
        .find(|p| p.file_path == "h.ts")
        .expect("h.ts in net diff");
    let content = String::from_utf8_lossy(&h.content);
    assert!(
        content.contains("Hono"),
        "net diff should carry the head content"
    );
    assert!(
        !content.contains("express"),
        "the reverted foreign import must not survive in the net range: {content}"
    );
}

fn commit_all(repo: &git2::Repository, msg: &str) -> git2::Oid {
    let mut index = repo.index().unwrap();
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .unwrap();
    index.write().unwrap();
    let tree_id = index.write_tree().unwrap();
    let tree = repo.find_tree(tree_id).unwrap();
    let sig = git2::Signature::now("t", "t@t").unwrap();
    let parent = repo.head().ok().and_then(|h| h.peel_to_commit().ok());
    let parents: Vec<&git2::Commit> = parent.iter().collect();
    repo.commit(Some("HEAD"), &sig, &sig, msg, &tree, &parents)
        .unwrap()
}

#[test]
fn commits_since_fit_counts_head_distance() {
    let dir = std::env::temp_dir().join(format!("argot_freshness_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let repo = git2::Repository::init(&dir).unwrap();
    std::fs::write(dir.join("a.py"), "x = 1\n").unwrap();
    let first = commit_all(&repo, "one");
    std::fs::write(dir.join("a.py"), "x = 2\n").unwrap();
    commit_all(&repo, "two");

    let path = dir.to_str().unwrap();
    assert_eq!(commits_since_fit(path, &first.to_string()), Some(1));
    let head = repo.head().unwrap().peel_to_commit().unwrap().id();
    assert_eq!(commits_since_fit(path, &head.to_string()), Some(0));
    // Unresolvable fit SHA must never break check.
    assert_eq!(commits_since_fit(path, "fixture"), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn freshness_ignores_feature_branch_and_docs_churn() {
    let dir = std::env::temp_dir().join(format!("argot_anchor_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let repo = git2::Repository::init(&dir).unwrap();
    std::fs::write(dir.join("src/a.py"), "x = 1\n").unwrap();
    let c1 = commit_all(&repo, "c1: source on default");
    // Pin the default branch name regardless of the machine's git config.
    let c1_commit = repo.find_commit(c1).unwrap();
    repo.branch("main", &c1_commit, true).unwrap();
    repo.set_head("refs/heads/main").unwrap();

    // A feature branch with one source commit and one docs commit.
    repo.branch("feat", &c1_commit, true).unwrap();
    repo.set_head("refs/heads/feat").unwrap();
    std::fs::write(dir.join("src/b.py"), "y = 2\n").unwrap();
    commit_all(&repo, "c2: feature source");
    std::fs::write(dir.join("README.md"), "docs\n").unwrap();
    let c3 = commit_all(&repo, "c3: docs only");

    let path = dir.to_str().unwrap();
    let config = crate::config::ArgotConfig::default();

    // The anchor is the merge-base with main — the feature commits are
    // not accepted history.
    assert_eq!(accepted_anchor(path, &config), Some(c1.to_string()));
    // A voice fitted at the anchor is fresh no matter how busy the branch.
    assert_eq!(
        accepted_source_commits_behind(path, &c1.to_string(), &config, 10),
        Some(0)
    );
    // Of the branch's own commits, only the source one is in scope.
    assert_eq!(
        in_scope_commits_between(
            path,
            &c1.to_string(),
            &c3.to_string(),
            &config.path_suppressions(),
            10
        ),
        Some(1)
    );
    // The manual-fit advisory sees the same single unmerged source commit…
    assert_eq!(
        unmerged_branch_source_commits(path, &config, 10),
        Some(("feat".to_string(), 1))
    );
    // …and stays quiet when the repo opted into current-branch refreshes.
    let opt_dir = std::env::temp_dir().join(format!("argot_anchor_cfg_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&opt_dir);
    std::fs::create_dir_all(&opt_dir).unwrap();
    std::fs::write(
        opt_dir.join("argot.toml"),
        "[fit]\nrefresh-from = \"current-branch\"\n",
    )
    .unwrap();
    let opted_out = crate::config::ArgotConfig::load(&opt_dir);
    let _ = std::fs::remove_dir_all(&opt_dir);
    assert_eq!(
        opted_out.fit_refresh_from,
        crate::config::FitRefreshFrom::CurrentBranch
    );
    assert_eq!(unmerged_branch_source_commits(path, &opted_out, 10), None);
    // Under the opt-out the anchor is plain HEAD.
    assert_eq!(freshness_anchor(path, &opted_out), Some(c3.to_string()));

    // Back on the default branch: HEAD is the anchor, no advisory.
    repo.set_head("refs/heads/main").unwrap();
    assert_eq!(accepted_anchor(path, &config), Some(c1.to_string()));
    assert_eq!(unmerged_branch_source_commits(path, &config, 10), None);
    let _ = std::fs::remove_dir_all(&dir);
}

/// A `[fit] refresh-from = "<branch>"` names the trunk explicitly for
/// repos whose accepted line isn't main/master.
#[test]
fn named_trunk_overrides_default_branch_detection() {
    let dir = std::env::temp_dir().join(format!("argot_trunk_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let repo = git2::Repository::init(&dir).unwrap();
    std::fs::write(dir.join("src/a.py"), "x = 1\n").unwrap();
    let c1 = commit_all(&repo, "c1: trunk");
    let c1_commit = repo.find_commit(c1).unwrap();
    // Trunk is `develop`; no main/master exists anywhere.
    repo.branch("develop", &c1_commit, true).unwrap();
    repo.set_head("refs/heads/develop").unwrap();
    for stray in ["main", "master"] {
        if let Ok(mut b) = repo.find_branch(stray, git2::BranchType::Local) {
            b.delete().unwrap();
        }
    }
    repo.branch("feat", &c1_commit, true).unwrap();
    repo.set_head("refs/heads/feat").unwrap();
    std::fs::write(dir.join("src/b.py"), "y = 2\n").unwrap();
    let c2 = commit_all(&repo, "c2: feature source");

    let path = dir.to_str().unwrap();
    std::fs::write(
        dir.join("argot.toml"),
        "[fit]\nrefresh-from = \"develop\"\n",
    )
    .unwrap();
    let named = crate::config::ArgotConfig::load(&dir);
    assert_eq!(
        named.fit_refresh_from,
        crate::config::FitRefreshFrom::Branch("develop".to_string())
    );
    // Named trunk: the anchor is the merge-base with develop, and the
    // advisory sees the unmerged feature commit.
    assert_eq!(accepted_anchor(path, &named), Some(c1.to_string()));
    assert_eq!(
        unmerged_branch_source_commits(path, &named, 10),
        Some(("feat".to_string(), 1))
    );
    // Without the override there is no main/master to detect — the
    // anchor degrades to HEAD (today's behaviour for unusual layouts).
    let auto = crate::config::ArgotConfig::default();
    assert_eq!(accepted_anchor(path, &auto), Some(c2.to_string()));
    // A named trunk missing from the clone degrades to detection, not to
    // a silent HEAD anchor pretending the config was honored.
    std::fs::write(dir.join("argot.toml"), "[fit]\nrefresh-from = \"gone\"\n").unwrap();
    let missing = crate::config::ArgotConfig::load(&dir);
    assert_eq!(accepted_anchor(path, &missing), Some(c2.to_string()));
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn in_scope_count_stops_at_threshold() {
    let dir = std::env::temp_dir().join(format!("argot_stopat_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("src")).unwrap();
    let repo = git2::Repository::init(&dir).unwrap();
    std::fs::write(dir.join("src/a.py"), "x = 0\n").unwrap();
    let base = commit_all(&repo, "base");
    for i in 1..=5 {
        std::fs::write(dir.join("src/a.py"), format!("x = {i}\n")).unwrap();
        commit_all(&repo, &format!("c{i}"));
    }
    let head = repo.head().unwrap().peel_to_commit().unwrap().id();
    let path = dir.to_str().unwrap();
    let sup = crate::suppress::PathSuppressions::recommended();
    assert_eq!(
        in_scope_commits_between(path, &base.to_string(), &head.to_string(), &sup, 3),
        Some(3),
        "count is capped at stop_at"
    );
    assert_eq!(
        in_scope_commits_between(path, &base.to_string(), &head.to_string(), &sup, 10),
        Some(5)
    );
    let _ = std::fs::remove_dir_all(&dir);
}
