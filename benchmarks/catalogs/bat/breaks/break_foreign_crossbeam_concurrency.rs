// Break fixture — parses in isolation; not built against the bat workspace.

use std::collections::HashSet;

/// Decoy: sorted list of active style-component names, in the style voice.
fn active_component_names(components: &HashSet<String>) -> Vec<String> {
    let mut names: Vec<String> = components.iter().cloned().collect();
    names.sort();
    names
}

// Break: crossbeam scoped threads and channel fanning style-component
// resolution across worker threads. Verified foreign at the pinned SHA
// 78951393e29b: `crossbeam` = 0 grep hits across *.rs and absent from
// Cargo.toml; bat resolves its StyleComponents synchronously in a single pass
// (style.rs) and its only threading is the paging thread spawned via
// std::thread in output.rs.
// Break: begin
fn resolve_components_parallel(inputs: Vec<String>) -> Vec<String> {
    let (tx, rx) = crossbeam::channel::unbounded();
    crossbeam::thread::scope(|scope| {
        for input in inputs {
            let tx = tx.clone();
            scope.spawn(move |_| {
                tx.send(input.to_lowercase()).expect("send failed");
            });
        }
    })
    .expect("worker scope panicked");
    drop(tx);
    rx.iter().collect()
}
// Break: end

/// Decoy: whether a component name is a known alias, in the style voice.
fn is_known_alias(name: &str) -> bool {
    matches!(name, "full" | "plain" | "auto")
}
