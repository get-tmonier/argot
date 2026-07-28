use super::*;

fn mix(dir: &str, corpus_share: f64, churn_share: f64) -> DirectoryMix {
    DirectoryMix {
        dir: dir.to_string(),
        corpus_files: (corpus_share * 100.0) as usize,
        corpus_share,
        changed_files: (churn_share * 100.0) as usize,
        churn_share,
    }
}

#[test]
fn top_level_buckets_by_first_segment() {
    assert_eq!(top_level("src/uos.pas"), "src");
    assert_eq!(top_level("examples/demo/player.pas"), "examples");
    assert_eq!(top_level("README.md"), ".");
    assert_eq!(top_level(""), ".");
    assert_eq!(top_level("/leading"), ".");
}

#[test]
fn a_tree_that_teaches_but_is_never_edited_is_reported() {
    // uos: examples/ is 64% of the Pascal it learned from, and the work — and
    // every one of its 145 replayed false alarms but four — happens in src/.
    let uos = vec![mix("examples", 0.64, 0.05), mix("src", 0.30, 0.90)];
    let found = mismatched(&uos);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].dir, "examples");
    assert!(describe(found[0]).contains("64% of the voice"));
    assert!(describe(found[0]).contains("5% of recent changes"));
}

#[test]
fn a_stable_core_is_not_a_mismatch() {
    // The signal must not fire on a large, deliberately stable tree: plenty of
    // healthy repos have a core that is rarely touched, and reporting it would
    // train people to ignore the note. Three-to-one is the bar, and 2:1 here
    // stays under it.
    let steady = vec![mix("core", 0.60, 0.30), mix("cli", 0.40, 0.70)];
    assert!(mismatched(&steady).is_empty());
}

#[test]
fn a_small_tree_is_never_reported_however_lopsided() {
    // Below MIN_CORPUS_SHARE a directory cannot move the model much either
    // way, and every repo has a sleepy corner. Reporting those is noise.
    let sleepy = vec![mix("scripts", 0.05, 0.0), mix("src", 0.95, 1.0)];
    assert!(mismatched(&sleepy).is_empty());
}

#[test]
fn directory_mix_is_empty_without_history_to_compare() {
    // A repo with no walkable history is young, not mis-configured — the
    // caller must get nothing to report rather than a division by zero.
    let tmp = std::env::temp_dir().join("argot-train-serve-empty");
    let _ = std::fs::create_dir_all(&tmp);
    let corpus = vec!["src/main.rs".to_string()];
    assert!(directory_mix(&tmp, &corpus).is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}

#[test]
fn directory_mix_is_empty_for_an_empty_corpus() {
    let tmp = std::env::temp_dir().join("argot-train-serve-nocorpus");
    let _ = std::fs::create_dir_all(&tmp);
    assert!(directory_mix(&tmp, &[]).is_empty());
    let _ = std::fs::remove_dir_all(&tmp);
}
