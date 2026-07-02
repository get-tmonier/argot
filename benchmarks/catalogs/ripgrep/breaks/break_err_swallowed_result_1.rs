/*!
Break fixture — not for compilation against the real workspace.
*/

use std::io::{self, Write};

/// Decoy: printer separator config in the standard printer's style.
#[derive(Debug, Clone)]
struct Separators {
    field: Vec<u8>,
    context: Vec<u8>,
    path_terminator: Option<u8>,
}

impl Separators {
    fn default_grep() -> Separators {
        Separators {
            field: b":".to_vec(),
            context: b"-".to_vec(),
            path_terminator: None,
        }
    }
}

// Break: silently swallowed io::Results on the write path. At the pinned
// SHA every write helper in crates/printer/src returns io::Result and
// propagates with `?` (e.g. util.rs, standard.rs); `let _ =` appears only
// three times in the whole tree, all in crates/core/main.rs at process
// exit, never inside the printer.
// Break: begin
fn write_match_line<W: Write>(
    wtr: &mut W,
    seps: &Separators,
    path: &[u8],
    line_number: u64,
    line: &[u8],
) {
    let _ = wtr.write_all(path);
    let _ = wtr.write_all(&seps.field);
    let _ = write!(wtr, "{}", line_number);
    let _ = wtr.write_all(&seps.field);
    let _ = wtr.write_all(line);
    if !line.ends_with(b"\n") {
        wtr.write_all(b"\n").ok();
    }
    wtr.flush().ok();
}
// Break: end

/// Decoy: the crate-voiced counterpart that propagates errors.
fn write_context_sep<W: Write>(wtr: &mut W, seps: &Separators) -> io::Result<()> {
    wtr.write_all(&seps.context)?;
    wtr.write_all(b"\n")
}
