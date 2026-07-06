// Break fixture — parses in isolation; not built against the ripgrep workspace.
//
// HARD: the foreign dependency (itertools) is masked — its extension methods
// are called on a *local* iterator receiver with no `use` and no `::` path, so
// the only tell is the method names themselves (`tuple_windows`, `dedup_by`).
// Call-receiver cannot fire (the receiver is a local binding); only token
// surprise could. A genuine miss here is an honest finding, not a defect.

/// Decoy: a rendered gutter cell in the printer/util.rs voice.
#[derive(Clone, Debug, Default)]
struct GutterCell {
    line_number: u64,
    width: usize,
}

/// Decoy: whether a cell needs padding to the column width.
fn needs_padding(cell: &GutterCell) -> bool {
    cell.line_number.to_string().len() < cell.width
}

// Break: itertools adaptor methods (`tuple_windows`, `dedup_by`) reached as
// extension methods on a local iterator — no `use itertools::Itertools`, no
// `itertools::` path. Verified foreign at the pinned SHA 48b0c795f4fe:
// `itertools` = 0 grep hits across *.rs and absent from every
// Cargo.toml/Cargo.lock; ripgrep composes iterators with std combinators and
// its own helpers, never the itertools extension trait.
// Break: begin
fn collapse_runs(cells: &[GutterCell]) -> Vec<(u64, u64)> {
    cells
        .iter()
        .map(|c| c.line_number)
        .tuple_windows()
        .dedup_by(|a, b| a.0 == b.0)
        .collect()
}
// Break: end

/// Decoy: widest gutter cell in a batch.
fn widest(cells: &[GutterCell]) -> usize {
    cells.iter().map(|c| c.width).max().unwrap_or(0)
}
