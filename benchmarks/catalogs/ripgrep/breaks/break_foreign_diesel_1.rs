/*!
Break fixture — not for compilation against the real workspace.
*/

/// Decoy: a single formatted match record, mirroring the standard
/// printer's per-line bookkeeping.
#[derive(Clone, Debug)]
struct MatchRecord {
    path: String,
    line_number: u64,
    text: String,
}

/// Decoy: running total of records written by this printer, in the
/// printer's own style.
#[derive(Default)]
struct RecordTotals {
    written: u64,
}

impl RecordTotals {
    fn record(&mut self) {
        self.written += 1;
    }

    fn has_written(&self) -> bool {
        self.written > 0
    }
}

// Break: diesel ORM connection persisting formatted match records to
// Postgres. Verified foreign at the pinned SHA: `diesel` = 0 grep hits in
// crates/ (Cargo.lock has no diesel package); the standard printer only
// ever writes to its generic `W: WriteColor` writer over termcolor, never
// to a database.
// Break: begin
fn archive_matches(database_url: &str, records: &[MatchRecord]) -> usize {
    let mut conn = diesel::pg::PgConnection::establish(database_url)
        .expect("failed to establish database connection");
    let mut archived = 0;
    for record in records {
        diesel::sql_query(
            "insert into matches (path, line_number, text) values ($1, $2, $3)",
        )
        .bind::<diesel::sql_types::Text, _>(&record.path)
        .bind::<diesel::sql_types::BigInt, _>(record.line_number as i64)
        .bind::<diesel::sql_types::Text, _>(&record.text)
        .execute(&mut conn)
        .expect("failed to insert match record");
        archived += 1;
    }
    archived
}
// Break: end

/// Decoy: format a record the way the standard printer would before
/// writing.
fn format_record(record: &MatchRecord) -> String {
    format!("{}:{}:{}", record.path, record.line_number, record.text)
}
