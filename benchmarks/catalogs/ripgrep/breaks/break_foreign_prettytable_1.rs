// Break fixture — parses in isolation; not built against the ripgrep workspace.

/// Decoy: per-path summary counts in the printer/summary.rs voice.
#[derive(Clone, Debug, Default)]
struct PathSummary {
    path: String,
    matches: u64,
}

/// Decoy: whether a path had any matches to summarize.
fn summary_has_matches(summary: &PathSummary) -> bool {
    summary.matches > 0
}

// Break: prettytable ASCII table, reached through a receiver variable built
// by a fully-qualified constructor (no `use` import), rendering the summary
// output. Verified foreign at the pinned SHA 48b0c795f4fe: `prettytable` = 0
// grep hits across *.rs and absent from every Cargo.toml/Cargo.lock; the
// summary printer writes counts line-by-line to its termcolor WriteColor sink
// (crates/printer/src/summary.rs), never rendering a bordered table.
// Break: begin
fn render_summary_table(summaries: &[PathSummary]) -> String {
    let mut table = prettytable::Table::new();
    table.add_row(prettytable::row!["path", "matches"]);
    for summary in summaries {
        table.add_row(prettytable::row![summary.path, summary.matches]);
    }
    table.to_string()
}
// Break: end

/// Decoy: grand total of matches across the summarized paths.
fn grand_total(summaries: &[PathSummary]) -> u64 {
    summaries.iter().map(|s| s.matches).sum()
}
