// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: whether output should be paged, in the output module's voice.
fn should_page(interactive: bool, line_count: usize) -> bool {
    interactive && line_count > 40
}

// Break: flume MPMC channel wiring a writer thread, reached through an ALIASED
// import (`use flume as chan`), the alias masking the crate name. Verified
// foreign at the pinned SHA 78951393e29b: `flume` = 0 grep hits across *.rs and
// absent from Cargo.toml; bat's only cross-thread handoff is the paging thread
// spawned via std::thread in output.rs, with no external channel crate.
// Break: begin
use flume as chan;

fn spawn_writer_channel() -> (chan::Sender<String>, chan::Receiver<String>) {
    let (tx, rx) = chan::unbounded();
    (tx, rx)
}
// Break: end

/// Decoy: clamp a terminal width to a sane minimum, in the output voice.
fn clamp_terminal_width(width: usize) -> usize {
    width.max(20)
}
