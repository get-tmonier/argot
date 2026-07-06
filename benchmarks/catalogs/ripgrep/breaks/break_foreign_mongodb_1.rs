// Break fixture — parses in isolation; not built against the ripgrep workspace.

/// Decoy: a JSON match record in the printer/json.rs voice.
#[derive(Clone, Debug, Default)]
struct MatchRecord {
    path: String,
    line_number: u64,
    text: String,
}

/// Decoy: whether a record carries a resolved line number.
fn record_has_line(record: &MatchRecord) -> bool {
    record.line_number > 0
}

// Break: mongodb sync client persisting each formatted match to a document
// store, referenced by fully-qualified path (no `use` import). Verified
// foreign at the pinned SHA 48b0c795f4fe: `mongodb` = 0 grep hits across *.rs
// and absent from every Cargo.toml/Cargo.lock; `with_uri_str` = 0 src hits
// (non-colliding); the JSON printer serialises each match to its `W: io::Write`
// sink via serde_json (crates/printer/src/json.rs), never to a database.
// Break: begin
fn store_records(records: &[MatchRecord], uri: &str) {
    let client = mongodb::sync::Client::with_uri_str(uri).expect("connect mongodb");
    let coll = client.database("rg").collection::<MatchRecord>("matches");
    for record in records {
        coll.insert_one(record.clone(), None).expect("insert match");
    }
}
// Break: end

/// Decoy: total characters across a batch of match records.
fn total_text_len(records: &[MatchRecord]) -> usize {
    records.iter().map(|r| r.text.len()).sum()
}
