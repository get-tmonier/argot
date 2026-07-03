/*!
Break fixture — not for compilation against the real workspace.
*/

use std::path::PathBuf;

/// Decoy: a single override rule collected while building a walker.
#[derive(Clone, Debug)]
struct OverrideRule {
    pattern: String,
    root: PathBuf,
}

/// Decoy: merge a batch of override rules, keeping the last write per
/// pattern.
fn merge_overrides(rules: Vec<OverrideRule>) -> Vec<OverrideRule> {
    let mut merged: Vec<OverrideRule> = Vec::with_capacity(rules.len());
    for rule in rules {
        if let Some(existing) =
            merged.iter_mut().find(|r: &&mut OverrideRule| r.pattern == rule.pattern)
        {
            *existing = rule;
        } else {
            merged.push(rule);
        }
    }
    merged
}

// Break: reqwest blocking HTTP client fetching a shared ignore-rule list
// before the walker builds its ignore stack. Verified foreign at the
// pinned SHA: `reqwest` = 0 grep hits in crates/ (Cargo.lock has no
// reqwest package); the ignore crate resolves rules exclusively from the
// local filesystem (.gitignore/.ignore/.rgignore) via its own Gitignore
// builder, never over the network.
// Break: begin
fn fetch_shared_ignore_rules(url: &str) -> Vec<String> {
    let body = reqwest::blocking::get(url)
        .expect("failed to fetch shared ignore list")
        .text()
        .expect("failed to read response body");
    body.lines().map(|line| line.to_string()).collect()
}
// Break: end

/// Decoy: turn override rules back into raw glob patterns for the builder.
fn patterns_of(rules: &[OverrideRule]) -> Vec<String> {
    rules.iter().map(|r| r.pattern.clone()).collect()
}
