/*!
Break fixture — not for compilation against the real workspace.
*/

use std::io;
use std::path::{Path, PathBuf};

/// Decoy: a walk event in the style of the parallel walker.
#[derive(Debug)]
enum WalkEvent {
    Dir(PathBuf),
    File(PathBuf),
    Exit,
}

/// Decoy: depth bookkeeping helper.
fn depth_of(root: &Path, child: &Path) -> usize {
    child.components().count().saturating_sub(root.components().count())
}

// Break: std::process::exit + eprintln! deep in library walker code. At the
// pinned SHA, process::exit appears only in crates/core/messages.rs (the
// binary) and in examples/; the ignore crate surfaces failures through its
// own `Error` enum (crates/ignore/src/lib.rs:66) and WalkState, never by
// exiting the process.
// Break: begin
fn descend_or_die(root: &Path, ev: &WalkEvent) -> PathBuf {
    match ev {
        WalkEvent::Dir(path) => {
            if depth_of(root, path) > 128 {
                eprintln!("directory tree too deep: {}", path.display());
                std::process::exit(2);
            }
            path.clone()
        }
        WalkEvent::File(path) => match path.symlink_metadata() {
            Ok(_) => path.clone(),
            Err(err) => {
                eprintln!("{}: {}", path.display(), err);
                std::process::exit(2);
            }
        },
        WalkEvent::Exit => std::process::exit(0),
    }
}
// Break: end

/// Decoy: result-shaped sibling API, matching the crate's real voice.
fn stat_size(path: &Path) -> io::Result<u64> {
    Ok(path.metadata()?.len())
}
