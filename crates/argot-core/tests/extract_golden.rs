//! Byte-exact golden test for the extract pipeline.
//!
//! Builds the deterministic fixture repo (fixed authors/dates → reproducible
//! SHAs) and asserts the extractor reproduces the committed golden
//! `dataset.jsonl` byte-for-byte.
//!
//! Requires `git` and `bash` on PATH (build step); the golden bytes are
//! committed, so the test is self-contained.

use std::path::PathBuf;
use std::process::Command;

fn fixture_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/extract")
}

fn build_fixture_repo() -> PathBuf {
    let out = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("extract_fixture_repo");
    let script = fixture_dir().join("build_fixture.sh");
    let status = Command::new(
        std::env::var_os("ARGOT_TEST_BASH")
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "bash".into()),
    )
    .arg(&script)
    .arg(&out)
    .status()
    .expect("run build_fixture.sh");
    assert!(status.success(), "fixture build failed");
    out
}

#[test]
fn extract_matches_python_golden() {
    let repo = build_fixture_repo();
    let mut buf: Vec<u8> = Vec::new();
    let stats = argot_core::extract::write_dataset(repo.to_str().unwrap(), None, None, &mut buf)
        .expect("extract");

    assert_eq!(stats.count, 2, "expected 2 records (root commit skipped)");

    let golden = std::fs::read(fixture_dir().join("golden.jsonl")).expect("read golden");
    let actual = String::from_utf8(buf).unwrap();
    let expected = String::from_utf8(golden).unwrap();

    // Compare line-by-line for a readable first-divergence diff.
    let a: Vec<&str> = actual.lines().collect();
    let e: Vec<&str> = expected.lines().collect();
    assert_eq!(a.len(), e.len(), "record count differs");
    for (i, (got, want)) in a.iter().zip(e.iter()).enumerate() {
        assert_eq!(got, want, "record {i} diverges from golden");
    }
    // Full byte equality (catches trailing-newline differences).
    assert_eq!(
        actual, expected,
        "extract output not byte-identical to golden"
    );
}
