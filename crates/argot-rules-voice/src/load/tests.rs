use super::*;
use std::sync::atomic::{AtomicUsize, Ordering};

static TEMP_DIR_SEQUENCE: AtomicUsize = AtomicUsize::new(0);

fn temp_argot_dir(case: &str) -> std::path::PathBuf {
    let sequence = TEMP_DIR_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!(
        "argot_load_{case}_{}_{}",
        std::process::id(),
        sequence
    ));
    let _ = fs::remove_dir_all(&dir);
    fs::create_dir_all(&dir).unwrap();
    dir
}

fn load_error(argot_dir: &Path) -> (String, i32) {
    match load_scorers(argot_dir, &DetectConfig::default(), &[]) {
        Err(error) => error,
        Ok(_) => panic!("fixture must not load"),
    }
}

#[test]
fn missing_fit_artifacts_offer_audit_and_init_as_distinct_next_actions() {
    let dir = temp_argot_dir("missing");

    let (message, code) = load_error(&dir);

    assert_eq!(code, 2);
    assert_eq!(
        message,
        format!(
            "error: {} not found — run `argot audit` for a no-setup history check, or `argot init` to set up recurring checks\n",
            dir.join("generic-baseline.json").display()
        )
    );
    let _ = fs::remove_dir_all(dir);
}

#[test]
fn old_and_malformed_artifacts_keep_specific_machine_safe_errors() {
    let old_dir = temp_argot_dir("old");
    fs::write(old_dir.join("generic-baseline.json"), "{}").unwrap();
    fs::write(old_dir.join("scorer-config.json"), r#"{"version": 2}"#).unwrap();
    let (old_message, old_code) = load_error(&old_dir);
    assert_eq!(old_code, 2);
    assert_eq!(
        old_message,
        format!(
            "error: {} uses config version 2 — regenerate via `argot fit`.\n",
            old_dir.join("scorer-config.json").display()
        )
    );
    assert!(!old_message.contains("argot audit"));
    let _ = fs::remove_dir_all(old_dir);

    let malformed_dir = temp_argot_dir("malformed");
    fs::write(malformed_dir.join("generic-baseline.json"), "{}").unwrap();
    fs::write(malformed_dir.join("scorer-config.json"), "{").unwrap();
    let (malformed_message, malformed_code) = load_error(&malformed_dir);
    assert_eq!(malformed_code, 2);
    assert!(malformed_message.starts_with("error: "));
    assert!(malformed_message.ends_with('\n'));
    assert!(!malformed_message.contains("argot audit"));
    assert!(!malformed_message.contains('\u{1b}'));
    let _ = fs::remove_dir_all(malformed_dir);
}
