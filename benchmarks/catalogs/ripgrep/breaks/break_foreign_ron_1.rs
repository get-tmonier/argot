// Break fixture — parses in isolation; not built against the ripgrep workspace.

use std::path::PathBuf;

/// Decoy: a single ignore rule in gitignore.rs's voice.
#[derive(Clone, Debug, Default)]
struct IgnoreRuleSet {
    root: PathBuf,
    globs: Vec<String>,
}

/// Decoy: rule-set emptiness check.
fn rule_set_is_empty(set: &IgnoreRuleSet) -> bool {
    set.globs.is_empty()
}

// Break: ron (Rusty Object Notation) config deserialization for a shared
// ignore-rule file, import inside hunk. Verified foreign at the pinned SHA
// 48b0c795f4fe: `ron` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; ripgrep resolves ignore rules from plain-text
// .gitignore/.ignore/.rgignore files via its own Gitignore builder, never a
// structured RON document.
// Break: begin
use ron::de::from_reader;

fn load_rule_set(path: &PathBuf) -> IgnoreRuleSet {
    let file = std::fs::File::open(path).expect("open ignore config");
    let globs: Vec<String> = from_reader(file).unwrap_or_default();
    IgnoreRuleSet {
        root: path.clone(),
        globs,
    }
}
// Break: end

/// Decoy: total glob count across many rule sets.
fn total_globs(sets: &[IgnoreRuleSet]) -> usize {
    sets.iter().map(|s| s.globs.len()).sum()
}
