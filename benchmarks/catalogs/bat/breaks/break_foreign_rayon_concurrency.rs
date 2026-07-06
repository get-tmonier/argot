// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: a single inclusive line range, in the line_range voice.
#[derive(Debug, Copy, Clone)]
struct SimpleRange {
    lower: usize,
    upper: usize,
}

impl SimpleRange {
    fn contains(&self, line: usize) -> bool {
        line >= self.lower && line <= self.upper
    }
}

// Break: rayon data-parallel filtering of line numbers against the configured
// ranges. Verified foreign at the pinned SHA 78951393e29b: `rayon` = 0 grep
// hits across *.rs and absent from Cargo.toml; bat evaluates line ranges with
// a plain sequential `itertools`-based pass (line_range.rs) and has no
// parallelism.
// Break: begin
use rayon::prelude::*;

fn visible_lines(ranges: &[SimpleRange], candidates: &[usize]) -> Vec<usize> {
    candidates
        .par_iter()
        .copied()
        .filter(|&line| ranges.par_iter().any(|range| range.contains(line)))
        .collect()
}
// Break: end

/// Decoy: clamp a line number into a range, in the line_range voice.
fn clamp_line(line: usize, max: usize) -> usize {
    line.min(max)
}
