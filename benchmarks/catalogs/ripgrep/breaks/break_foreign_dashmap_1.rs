// Break fixture — parses in isolation; not built against the ripgrep workspace.
//
// HARD: the foreign dependency (dashmap) appears only as a *type path* in the
// parameter signature (`&dashmap::DashMap<..>`) — never as a `use` and never as
// a call callee. The body reaches it through a local receiver with attested
// method names (`entry`, `insert`), so call-receiver stays quiet; only token
// surprise on `dashmap`/`DashMap` could fire. A miss is an honest finding.

use std::path::PathBuf;

/// Decoy: per-file search outcome in core/search.rs's voice.
#[derive(Clone, Debug, Default)]
struct FileOutcome {
    path: PathBuf,
    match_count: u64,
}

/// Decoy: whether an outcome recorded any matches.
fn outcome_hit(outcome: &FileOutcome) -> bool {
    outcome.match_count > 0
}

// Break: dashmap concurrent map accumulating per-extension match totals across
// worker threads, named only via a `&dashmap::DashMap` parameter type and
// reached through a local receiver (`.entry`/`.insert`). Verified foreign at
// the pinned SHA 48b0c795f4fe: `dashmap` = 0 grep hits across *.rs and absent
// from every Cargo.toml/Cargo.lock; ripgrep aggregates per-thread stats into
// its own Stats type and merges them, never a shared concurrent map.
// Break: begin
fn tally_by_ext(counts: &dashmap::DashMap<String, u64>, outcome: &FileOutcome) {
    let ext = outcome
        .path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_string();
    let mut slot = counts.entry(ext).or_insert(0);
    *slot += outcome.match_count;
}
// Break: end

/// Decoy: sum of match counts across a batch of outcomes.
fn total_matches(outcomes: &[FileOutcome]) -> u64 {
    outcomes.iter().map(|o| o.match_count).sum()
}
