//! End-to-end audit fixture for source that is transient within the window.
//!
//! The audit must score the net base-to-HEAD diff, not every intermediate
//! commit. This fixture deliberately adds then removes supported Python source
//! before running the real CLI against that history.

use std::path::Path;
use std::process::Command;

fn commit(repo: &git2::Repository, message: &str) {
    let mut index = repo.index().expect("open index");
    index
        .add_all(["*"].iter(), git2::IndexAddOption::DEFAULT, None)
        .expect("stage fixture files");
    index.write().expect("write index");
    let tree = repo
        .find_tree(index.write_tree().expect("write fixture tree"))
        .expect("find fixture tree");
    let signature =
        git2::Signature::now("fixture", "fixture@example.com").expect("fixture signature");
    let parents = repo
        .head()
        .ok()
        .and_then(|head| head.peel_to_commit().ok())
        .into_iter()
        .collect::<Vec<_>>();
    let parent_refs = parents.iter().collect::<Vec<_>>();
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        message,
        &tree,
        &parent_refs,
    )
    .expect("commit fixture history");
}

fn write(path: &Path, source: &str) {
    std::fs::write(path, source).expect("write fixture source");
}

#[test]
fn audit_ignores_supported_source_added_then_removed_within_its_window() {
    let repo_path = Path::new(env!("CARGO_TARGET_TMPDIR"))
        .join(format!("audit_transient_history_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&repo_path);
    std::fs::create_dir_all(&repo_path).expect("create fixture repository");
    let repo = git2::Repository::init(&repo_path).expect("initialize fixture repository");

    let app = repo_path.join("app.py");
    let baseline = "\
def increment(value):
    return value + 1


def double(value):
    return value * 2


def render(value):
    return str(value)
";
    write(&app, baseline);
    commit(&repo, "baseline source");

    write(
        &app,
        &format!("{baseline}\n\ndef transient_supported_source(value):\n    return value - 1\n"),
    );
    commit(&repo, "add transient source");

    write(&app, baseline);
    commit(&repo, "remove transient source");

    let output = Command::new(env!("CARGO_BIN_EXE_argot"))
        .args(["audit", "--repo"])
        .arg(&repo_path)
        .args(["--commits", "2", "--format", "terminal"])
        .output()
        .expect("run argot audit");
    let _ = std::fs::remove_dir_all(&repo_path);

    assert!(
        output.status.success(),
        "audit failed:\n{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let card = String::from_utf8(output.stdout).expect("audit terminal output is UTF-8");
    assert!(
        card.contains("no supported source changed"),
        "a net-empty base-to-HEAD source diff must scan no hunks:\n{card}"
    );
    assert!(
        !card.contains("Worst offender"),
        "a transient change must not create an offender:\n{card}"
    );
    assert!(
        !card.contains("Share this"),
        "a net-empty audit must not offer a share caption:\n{card}"
    );
}
