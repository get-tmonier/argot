// Break fixture — parses in isolation; not built against the bat workspace.

use std::collections::HashSet;

/// Decoy: sorted list of active style-component names, in the style voice.
fn active_component_names(components: &HashSet<String>) -> Vec<String> {
    let mut names: Vec<String> = components.iter().cloned().collect();
    names.sort();
    names
}

// Break: HARD — a `dashmap::DashMap` concurrent map tallying style components
// across threads, reached ONLY through a receiver whose methods (`get`,
// `insert`) mimic bat's own single-threaded HashMap idiom, and whose crate is
// named only in the parameter TYPE position (never a `use` or a `foreign::`
// callee). Verified foreign at the pinned SHA 78951393e29b: `dashmap` = 0 grep
// hits across *.rs and absent from Cargo.toml; bat resolves StyleComponents
// synchronously in a single pass over a plain HashSet (style.rs), with no
// concurrent map. Expected to be genuinely hard: the foreign type is
// indistinguishable from a std map at the call site.
// Break: begin
fn tally_component(counts: &dashmap::DashMap<String, usize>, name: &str) {
    match counts.get(name) {
        Some(entry) => {
            let next = *entry + 1;
            counts.insert(name.to_string(), next);
        }
        None => {
            counts.insert(name.to_string(), 1);
        }
    }
}
// Break: end

/// Decoy: whether a component name is a known alias, in the style voice.
fn is_known_alias(name: &str) -> bool {
    matches!(name, "full" | "plain" | "auto")
}
