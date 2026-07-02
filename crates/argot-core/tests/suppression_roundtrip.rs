//! Round-trip integration tests for the suppression surfaces (issue #57):
//! fixture repo → fit → check finds hits → suppress via each of the three
//! surfaces (`.argotignore`, inline comments, `suppressions.yaml`) → check
//! again → hit gone, exit code clean, stderr summary counts it.
//!
//! Reuses the deterministic `check` fixture repo (fixed authors/dates →
//! reproducible SHAs and hit hashes). Requires `git` and `bash` on PATH.

use std::path::{Path, PathBuf};
use std::process::Command;

use argot_core::check::{run_check, run_review_mutes, CheckArgs};
use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::calibration::{
    collect_candidates, collect_candidates_with, run_calibrate, CalibrateOptions,
};
use argot_core::suppress::{
    load_suppressions_file, mute_hash, read_last_check, PathSuppressions, SUPPRESSIONS_FILE,
};

const TODAY: &str = "2026-07-02";

fn fixture_check_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/check")
}

fn build_fixture_repo(suffix: &str) -> PathBuf {
    let out =
        PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join(format!("suppress_fixture_repo_{suffix}"));
    let script = fixture_check_dir().join("build_check_repo.sh");
    let status = Command::new(
        std::env::var_os("ARGOT_TEST_BASH")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "bash".into()),
    )
    .arg(&script)
    .arg(&out)
    .status()
    .expect("run build_check_repo.sh");
    assert!(status.success(), "fixture build failed");
    out
}

/// Full fit (train → calibrate; deterministic v3 config with the model
/// snapshot) against the current checkout.
fn fit(repo: &Path) {
    let argot_dir = repo.join(".argot");
    std::fs::create_dir_all(&argot_dir).unwrap();
    argot_core::train::run_train(
        repo,
        &argot_dir.join("repo-corpus.txt"),
        &argot_dir.join("generic-baseline.json"),
    )
    .expect("train");
    let opts = CalibrateOptions {
        repo_sha: "fixture".to_string(),
        timestamp_utc: "1970-01-01T00:00:00+00:00".to_string(),
        ..Default::default()
    };
    run_calibrate(
        repo,
        &argot_dir.join("repo-corpus.txt"),
        argot_core::train::GENERIC_BASELINE_JSON,
        &argot_dir.join("scorer-config.json"),
        &opts,
    )
    .expect("calibrate");
}

/// Build repo + `.argot/` artifacts (full fit at HEAD).
fn prepare_repo(suffix: &str) -> PathBuf {
    let repo = build_fixture_repo(suffix);
    fit(&repo);
    repo
}

fn base_args(repo: &Path) -> CheckArgs {
    CheckArgs {
        repo_path: repo.to_str().unwrap().to_string(),
        reference: String::new(),
        staged: false,
        unstaged: false,
        commit: None,
        only: vec![],
        exclude: vec![],
        threshold: None,
        argot_dir: repo.join(".argot"),
        hunk_lines: 6,
        verbose: false,
        min_severity: "unusual".to_string(),
        use_color: false,
        format: argot_core::output::OutputFormat::Human,
        today: TODAY.to_string(),
    }
}

/// The range check that surfaces both fixture hits (threshold widened).
fn range_args(repo: &Path) -> CheckArgs {
    let mut args = base_args(repo);
    args.reference = "HEAD~2..HEAD".to_string();
    args.threshold = Some(-1000.0);
    args
}

#[test]
fn argotignore_suppresses_hits_and_reports_summary() {
    let repo = prepare_repo("argotignore");

    // Baseline: two hits; stderr names the model that scored the diff but
    // carries no suppression traffic.
    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 1);
    assert!(out.stdout.contains("integration.py"));
    assert!(out.stdout.contains("pipeline.py"));
    assert!(
        out.stderr.contains("[argot] model: "),
        "check names its model on stderr"
    );
    assert!(
        !out.stderr.contains("suppressed"),
        "no suppression traffic on the baseline run"
    );

    // Path-level mute via .argotignore.
    std::fs::write(repo.join(".argotignore"), "integration.py\npipeline.py\n").unwrap();
    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 0, "all hits suppressed → clean exit");
    assert!(!out.stdout.contains("integration.py"));
    assert!(out
        .stderr
        .contains("2 hits suppressed (2 by .argotignore, 0 inline, 0 by suppressions.yaml)"));
}

#[test]
fn inline_comment_suppresses_workdir_hit() {
    let repo = prepare_repo("inline");

    // Appended workdir change carrying its own suppression comment.
    let text_py = repo.join("text.py");
    let mut content = std::fs::read_to_string(&text_py).unwrap();
    content.push_str(
        "\n\n# argot: ignore-next-line — fixture noise, accepted\ndef extra_helper(items):\n    total = 0\n    for it in items:\n        total += len(it)\n    return total\n",
    );
    std::fs::write(&text_py, content).unwrap();

    let mut args = base_args(&repo);
    args.threshold = Some(-1000.0);
    let out = run_check(args);
    assert_eq!(out.exit_code, 0, "inline comment mutes the only hit");
    assert!(out
        .stderr
        .contains("1 hits suppressed (0 by .argotignore, 1 inline, 0 by suppressions.yaml)"));
}

#[test]
fn inline_comment_without_reason_warns_and_does_not_suppress() {
    let repo = prepare_repo("inline_noreason");

    let text_py = repo.join("text.py");
    let mut content = std::fs::read_to_string(&text_py).unwrap();
    content.push_str(
        "\n\n# argot: ignore-next-line\ndef extra_helper(items):\n    total = 0\n    for it in items:\n        total += len(it)\n    return total\n",
    );
    std::fs::write(&text_py, content).unwrap();

    let mut args = base_args(&repo);
    args.threshold = Some(-1000.0);
    let out = run_check(args);
    assert_eq!(out.exit_code, 1, "reason-less comment must not suppress");
    assert!(out.stderr.contains("missing reason"));
    assert!(!out.stderr.contains("hits suppressed"));
}

#[test]
fn mute_roundtrip_via_last_check_and_suppressions_yaml() {
    let repo = prepare_repo("mute");
    let argot_dir = repo.join(".argot");

    // First check: hashes are printed per hit and cached in last-check.json.
    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 1);
    let hits = read_last_check(&argot_dir).expect("last-check.json written");
    assert_eq!(hits.len(), 2);
    for h in &hits {
        assert_eq!(h.hash.len(), 12);
        assert!(
            out.stdout.contains(&format!("[{}]", h.hash)),
            "hash [{}] rendered on the hit header",
            h.hash
        );
    }

    // Mute the integration.py hit by hash.
    let integration = hits
        .iter()
        .find(|h| h.path == "integration.py")
        .expect("integration.py hit");
    let rule = mute_hash(&argot_dir, &integration.hash, None, None, TODAY).expect("mute");
    assert_eq!(rule.path, "integration.py");

    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 1, "pipeline.py hit still fires");
    assert!(!out.stdout.contains("integration.py"), "muted hit gone");
    assert!(out.stdout.contains("pipeline.py"));
    assert!(out
        .stderr
        .contains("1 hits suppressed (0 by .argotignore, 0 inline, 1 by suppressions.yaml)"));

    // Mute the remaining hit (its hash is in the fresh last-check cache).
    let hits = read_last_check(&argot_dir).expect("cache rewritten");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "pipeline.py");
    mute_hash(&argot_dir, &hits[0].hash, Some("known-good"), None, TODAY).expect("mute 2");

    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 0, "everything muted → clean exit");
    assert!(out
        .stderr
        .contains("2 hits suppressed (0 by .argotignore, 0 inline, 2 by suppressions.yaml)"));
}

#[test]
fn expired_yaml_entry_is_ignored_with_stderr_note() {
    let repo = prepare_repo("expired");
    std::fs::write(
        repo.join(".argot").join(SUPPRESSIONS_FILE),
        "- path: integration.py\n  expires: 2020-01-01\n  reason: long gone\n",
    )
    .unwrap();

    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 1, "expired rule must not suppress");
    assert!(out.stdout.contains("integration.py"));
    assert!(out.stderr.contains("expired 2020-01-01"));
    assert!(!out.stderr.contains("hits suppressed"));
}

#[test]
fn yaml_path_glob_with_scorer_scope() {
    let repo = prepare_repo("glob");
    // The fixture hits carry reason "none" (visible via --threshold); a rule
    // scoped to another scorer must not match, a path-wide rule must.
    std::fs::write(
        repo.join(".argot").join(SUPPRESSIONS_FILE),
        "- path: \"integration.*\"\n  scorer: bpe\n  reason: wrong scope\n",
    )
    .unwrap();
    let out = run_check(range_args(&repo));
    assert_eq!(out.exit_code, 1);
    assert!(
        out.stdout.contains("integration.py"),
        "scorer scope excludes"
    );

    std::fs::write(
        repo.join(".argot").join(SUPPRESSIONS_FILE),
        "- path: \"integration.*\"\n  reason: path-wide mute\n",
    )
    .unwrap();
    let out = run_check(range_args(&repo));
    assert!(!out.stdout.contains("integration.py"), "path glob matches");
    assert!(out
        .stderr
        .contains("1 hits suppressed (0 by .argotignore, 0 inline, 1 by suppressions.yaml)"));
}

#[test]
fn last_check_cache_written_on_clean_run() {
    let repo = prepare_repo("clean_cache");
    let mut args = base_args(&repo);
    args.reference = "HEAD~1..HEAD".to_string();
    let out = run_check(args);
    assert_eq!(out.exit_code, 0);
    let hits = read_last_check(&repo.join(".argot")).expect("cache written on clean run");
    assert!(hits.is_empty());
}

#[test]
fn review_mutes_reports_rot_and_prunes() {
    // Fit at HEAD~1 (the evidence-parity dance) so integration.py genuinely
    // flags with the calibrated threshold — review needs a real flagged hit.
    let repo = build_fixture_repo("review");
    let git = |args: &[&str]| {
        let status = Command::new("git")
            .arg("-C")
            .arg(&repo)
            .args(args)
            .status()
            .expect("git");
        assert!(status.success());
    };
    let argot_dir = repo.join(".argot");
    git(&["checkout", "-q", "HEAD~1"]);
    fit(&repo);
    git(&["checkout", "-q", "main"]);

    // Check flags integration.py; mute it by hash.
    let mut args = base_args(&repo);
    args.reference = "HEAD~1..HEAD".to_string();
    let out = run_check(args);
    assert_eq!(out.exit_code, 1, "calibrated check flags the anomaly");
    let hits = read_last_check(&argot_dir).expect("cache");
    assert_eq!(hits.len(), 1);
    assert_eq!(hits[0].path, "integration.py");
    mute_hash(&argot_dir, &hits[0].hash, None, None, TODAY).expect("mute");

    let mut args = base_args(&repo);
    args.reference = "HEAD~1..HEAD".to_string();
    let out = run_check(args);
    assert_eq!(out.exit_code, 0, "muted → clean");

    // The muted content is unchanged → the suppression still fires.
    let repo_str = repo.to_str().unwrap();
    let review = run_review_mutes(repo_str, &argot_dir, TODAY, false);
    assert_eq!(review.exit_code, 0);
    assert!(review.stdout.contains("still fires"), "{}", review.stdout);
    assert!(!review.stdout.contains("no longer fires"));

    // Rewrite the file benignly → the mute is rot; --prune removes it.
    std::fs::write(
        repo.join("integration.py"),
        "from __future__ import annotations\nfrom stats import mean\n\n\ndef summarize_two(rows):\n    values = [len(r) for r in rows]\n    return {\"count\": len(rows), \"avg\": mean(values)}\n",
    )
    .unwrap();
    let review = run_review_mutes(repo_str, &argot_dir, TODAY, true);
    assert_eq!(review.exit_code, 0);
    assert!(
        review.stdout.contains("no longer fires"),
        "{}",
        review.stdout
    );
    assert!(review.stdout.contains("Pruned 1"), "{}", review.stdout);
    let rules = load_suppressions_file(&argot_dir.join(SUPPRESSIONS_FILE), TODAY);
    assert!(rules.active.is_empty(), "stale mute removed");
}

#[test]
fn calibration_scope_stays_in_lock_step_with_argotignore() {
    // collect_candidates (bench seam) keeps recommended-only semantics;
    // collect_candidates_with(load(...)) resolves `.argotignore` on top —
    // the same set check filters against.
    let dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("suppress_lockstep");
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let body = |name: &str| {
        format!(
            "def {name}(a, b):\n    x = a + b\n    y = x * 2\n    z = y - a\n    w = z + 1\n    return w\n"
        )
    };
    std::fs::write(dir.join("app.py"), body("app_fn")).unwrap();
    std::fs::write(dir.join("vendored.py"), body("vendored_fn")).unwrap();
    std::fs::write(dir.join(".argotignore"), "vendored.py\n").unwrap();

    let adapter = PythonAdapter::new();
    let legacy = collect_candidates(&dir, &adapter);
    assert_eq!(
        legacy.len(),
        2,
        "recommended-only view still sees both files"
    );

    let resolved = collect_candidates_with(&dir, &adapter, &PathSuppressions::load(&dir));
    assert_eq!(resolved.len(), 1, ".argotignore drops the vendored file");
    assert!(resolved[0].file_path.ends_with("app.py"));
    let _ = std::fs::remove_dir_all(&dir);
}
