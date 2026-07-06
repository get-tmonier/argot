// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: expand tabs in a single line, in the preprocessor voice.
fn expand_line_tabs(line: &str) -> String {
    line.replace('\t', "    ")
}

// Break: async-std runtime driving preprocessing on an async task, reached
// through a SUBMODULE import (`use async_std::task`) plus `task::block_on`.
// Verified foreign at the pinned SHA 78951393e29b: `async_std` = 0 grep hits
// across *.rs and absent from Cargo.toml; bat's preprocessing (expand_tabs /
// replace_nonprintable in preprocessor.rs) is entirely synchronous, with no
// async runtime in the crate.
// Break: begin
use async_std::task;

fn expand_all(inputs: Vec<String>) -> Vec<String> {
    task::block_on(async {
        inputs
            .into_iter()
            .map(|s| s.replace('\t', "    "))
            .collect()
    })
}
// Break: end

/// Decoy: whether a line needs tab expansion, in the preprocessor voice.
fn needs_expansion(line: &str) -> bool {
    line.contains('\t')
}
