/*!
Break fixture — not for compilation against the real workspace.
*/

use std::path::Path;
use std::time::Duration;

/// Decoy: per-search tallies in grep-printer's Stats voice.
#[derive(Clone, Debug, Default)]
struct Tally {
    matched_lines: u64,
    matches: u64,
    elapsed: Duration,
}

impl Tally {
    fn add(&mut self, other: &Tally) {
        self.matched_lines += other.matched_lines;
        self.matches += other.matches;
        self.elapsed += other.elapsed;
    }
}

// Break: println!-driven match and summary output. At the pinned SHA there
// is zero println! in crates/core src; all match output flows through the
// repo's own grep-printer (Standard/Summary) over termcolor::WriteColor
// buffers, with colors and separators handled by the printer config.
// Break: begin
fn print_file_results(path: &Path, lines: &[(u64, String)], tally: &Tally) {
    println!("{}", path.display());
    for (line_number, line) in lines {
        println!("{}:{}", line_number, line);
    }
    println!();
    println!(
        "{} matched lines, {} matches in {:?}",
        tally.matched_lines, tally.matches, tally.elapsed
    );
}
// Break: end

/// Decoy: elapsed formatting helper in the summary printer's voice.
fn fractional_seconds(elapsed: Duration) -> f64 {
    elapsed.as_secs_f64()
}
