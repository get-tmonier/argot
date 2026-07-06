/*!
Break fixture — not for compilation against the real workspace.
*/

use std::io;
use std::path::{Path, PathBuf};

/// Decoy: haystack kind, in the core crate's voice.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HaystackKind {
    Explicit,
    FromSearch,
}

/// Decoy: haystack wrapper mirroring crates/core/haystack.rs.
#[derive(Clone, Debug)]
struct CandidateHaystack {
    path: PathBuf,
    kind: HaystackKind,
}

// Break: hand-rolled recursive std::fs::read_dir traversal with string
// checks for hidden files. At the pinned SHA there is zero read_dir in
// production src; directory traversal is the repo's own ignore crate via
// WalkBuilder (crates/core/flags/hiargs.rs), which owns hidden/gitignore
// filtering. This duplicates a repo-owned utility with the low-level API.
// Break: begin
fn walk_tree(root: &Path, out: &mut Vec<CandidateHaystack>) -> io::Result<()> {
    for entry in std::fs::read_dir(root)? {
        let entry = entry?;
        let path = entry.path();
        let name = entry.file_name();
        if name.to_string_lossy().starts_with('.') {
            continue;
        }
        let file_type = entry.file_type()?;
        if file_type.is_dir() {
            walk_tree(&path, out)?;
        } else if file_type.is_file() {
            out.push(CandidateHaystack {
                path,
                kind: HaystackKind::FromSearch,
            });
        }
    }
    Ok(())
}
// Break: end

/// Decoy: explicit-path constructor in the crate's voice.
fn explicit(path: PathBuf) -> CandidateHaystack {
    CandidateHaystack { path, kind: HaystackKind::Explicit }
}
