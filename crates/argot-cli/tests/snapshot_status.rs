//! Snapshot status is an external CLI contract: a staged fit must not pretend
//! to be available to another checkout or the CI Action.

use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};

use serde_json::Value;

fn commit_all(repo: &git2::Repository, message: &str) -> String {
    let mut index = repo.index().expect("open index");
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .expect("stage fixture");
    index.write().expect("write index");
    let tree = repo
        .find_tree(index.write_tree().expect("write tree"))
        .unwrap();
    let sig = git2::Signature::now("test", "test@example.com").unwrap();
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect::<Vec<_>>();
    repo.commit(
        Some("HEAD"),
        &sig,
        &sig,
        message,
        &tree,
        &parents.iter().collect::<Vec<_>>(),
    )
    .expect("commit fixture")
    .to_string()
}

fn status(repo: &Path) -> Value {
    let output = Command::new(env!("CARGO_BIN_EXE_argot"))
        .args(["status", "--repo"])
        .arg(repo)
        .args(["--format", "json"])
        .output()
        .expect("run argot status");
    assert!(
        output.status.success(),
        "status failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).expect("status is JSON")
}

#[test]
fn snapshot_status_requires_a_commit_and_reports_optional_detector_abstention() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("snapshot_status_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(path.join(".argot")).expect("create snapshot");
    let repo = git2::Repository::init(&path).expect("init fixture repository");
    for name in [
        "generic-baseline.json",
        "scorer-config.json",
        "health.json",
        "manifest.json",
    ] {
        std::fs::write(path.join(".argot").join(name), "{}").expect("write artifact");
    }

    let staged = status(&path);
    assert_eq!(staged["snapshot"]["complete"], true);
    assert_eq!(staged["snapshot"]["committed"], false);
    assert_eq!(
        staged["snapshot"]["unavailable"],
        serde_json::json!(["semantic-index.json", "layering.json", "integrity.json"])
    );
    assert_eq!(
        staged["snapshot"]["uncommitted"].as_array().unwrap().len(),
        4
    );

    commit_all(&repo, "fit snapshot");
    let committed = status(&path);
    assert_eq!(committed["snapshot"]["committed"], true);
    assert!(committed["snapshot"]["uncommitted"]
        .as_array()
        .unwrap()
        .is_empty());

    std::fs::write(path.join(".argot/health.json"), "{\"changed\":true}").unwrap();
    let changed = status(&path);
    assert_eq!(changed["snapshot"]["committed"], false);
    assert_eq!(
        changed["snapshot"]["uncommitted"],
        serde_json::json!(["health.json"])
    );
    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn status_exposes_the_adaptive_refresh_verdict() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("adaptive_status_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(path.join("src")).unwrap();
    let repo = git2::Repository::init(&path).unwrap();
    let source = |changed: usize| {
        (0..100)
            .map(|i| {
                if i < changed {
                    format!("value_{i} = changed_{i}\n")
                } else {
                    format!("value_{i} = {i}\n")
                }
            })
            .collect::<String>()
    };
    std::fs::write(path.join("src/app.py"), source(0)).unwrap();
    let fit_sha = commit_all(&repo, "fit point");

    std::fs::create_dir_all(path.join(".argot")).unwrap();
    for name in [
        "generic-baseline.json",
        "scorer-config.json",
        "manifest.json",
    ] {
        std::fs::write(path.join(".argot").join(name), "{}").unwrap();
    }
    let config = argot_core::config::ArgotConfig::default();
    let health = serde_json::json!({
        "fit_sha": fit_sha,
        "config_fingerprint": argot_core::health::config_fingerprint(&config),
        "drift_candidates": [],
        "refresh_profile": {
            "schema": 1,
            "source": {
                "files": 1,
                "lines": 100,
                "languages": { "python": { "files": 1, "lines": 100 } },
                "areas": { "src": { "files": 1, "lines": 100 } }
            }
        }
    });
    std::fs::write(
        path.join(".argot/health.json"),
        serde_json::to_vec_pretty(&health).unwrap(),
    )
    .unwrap();
    commit_all(&repo, "commit fit snapshot");

    let fresh = status(&path);
    assert_eq!(fresh["refresh"]["compatibility"], "ready");
    assert_eq!(fresh["refresh"]["recommendation"], "fresh");
    assert_eq!(fresh["refresh"]["next_action"], "none");

    std::fs::write(path.join("src/app.py"), source(40)).unwrap();
    commit_all(&repo, "large accepted refactor");
    let changed = status(&path);
    assert_eq!(changed["refresh"]["recommendation"], "recommended");
    assert_eq!(changed["refresh"]["score"], 40);
    assert_eq!(changed["refresh"]["next_action"], "fit");
    assert!(changed["refresh"]["summary"]
        .as_str()
        .unwrap()
        .contains("40.0%"));
    let _ = std::fs::remove_dir_all(&path);
}

#[test]
fn mcp_fit_status_exposes_the_snapshot_contract() {
    let path = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("mcp_snapshot_status_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&path);
    std::fs::create_dir_all(&path).expect("create fixture repository");
    git2::Repository::init(&path).expect("init fixture repository");

    let mut child = Command::new(env!("CARGO_BIN_EXE_argot"))
        .args(["mcp", "--repo"])
        .arg(&path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("start argot mcp");
    child
        .stdin
        .take()
        .expect("mcp stdin")
        .write_all(
            br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"argot.fit_status","arguments":{}}}
"#,
        )
        .expect("request fit status");
    let output = child.wait_with_output().expect("finish argot mcp");
    assert!(output.status.success());
    let envelope: Value = serde_json::from_slice(&output.stdout).expect("MCP envelope");
    let text = envelope["result"]["content"][0]["text"]
        .as_str()
        .expect("MCP text content");
    let fit_status: Value = serde_json::from_str(text).expect("fit status JSON");
    assert_eq!(fit_status["snapshot"]["complete"], false);
    assert_eq!(fit_status["snapshot"]["committed"], false);
    assert!(fit_status["refresh"].is_null());
    let _ = std::fs::remove_dir_all(&path);
}
