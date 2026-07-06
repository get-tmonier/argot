// Break fixture — parses in isolation; not built against the ripgrep workspace.

/// Decoy: a single formatted match record, mirroring the JSON printer's
/// per-line bookkeeping.
#[derive(Clone, Debug)]
struct MatchRecord {
    path: String,
    line_number: u64,
    text: String,
}

/// Decoy: whether a record carries a resolved line number, in the JSON
/// printer's voice.
fn has_line_number(record: &MatchRecord) -> bool {
    record.line_number > 0
}

// Break: rusqlite SQLite connection persisting formatted match records to a
// local database, referenced by fully-qualified path (no `use` import).
// Verified foreign at the pinned SHA 48b0c795f4fe: `rusqlite` = 0 grep hits
// across *.rs and absent from every Cargo.toml; the JSON printer serialises
// each match to its `W: io::Write` sink via serde_json (json.rs), never to a
// SQL database.
// Break: begin
fn persist_matches(db_path: &str, records: &[MatchRecord]) -> usize {
    let conn = rusqlite::Connection::open(db_path)
        .expect("failed to open match database");
    let mut written = 0;
    for record in records {
        conn.execute(
            "INSERT INTO matches (path, line_number, text) VALUES (?1, ?2, ?3)",
            rusqlite::params![record.path, record.line_number, record.text],
        )
        .expect("failed to insert match record");
        written += 1;
    }
    written
}
// Break: end

/// Decoy: format a record the way the JSON printer would before writing.
fn format_record(record: &MatchRecord) -> String {
    format!("{}:{}:{}", record.path, record.line_number, record.text)
}
