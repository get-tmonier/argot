// Break fixture — parses in isolation; not built against the ripgrep workspace.
//
// HARD: the foreign dependency (redis) is named only as a `&mut
// redis::Connection` parameter type, and its API is reached through a local
// receiver whose leaf method names (`get`, `set`) collide with attested repo
// methods — so call-receiver's method-attested guard suppresses it and only
// token surprise on `redis`/`Connection` could fire. A miss is an honest
// finding, not a defect to paper over.

/// Decoy: a formatted match line in the standard printer's voice.
#[derive(Clone, Debug, Default)]
struct FormattedLine {
    path: String,
    line_number: u64,
    rendered: String,
}

/// Decoy: whether a line has been rendered yet.
fn is_rendered(line: &FormattedLine) -> bool {
    !line.rendered.is_empty()
}

// Break: redis client caching each rendered match line by key, named only via
// a `&mut redis::Connection` parameter type and reached through a local
// receiver with attested leaf methods (`.set`/`.get`). Verified foreign at the
// pinned SHA 48b0c795f4fe: `redis` = 0 grep hits across *.rs and absent from
// every Cargo.toml/Cargo.lock; the standard printer writes rendered lines
// straight to its `W: WriteColor` sink over termcolor, never to a cache.
// Break: begin
fn cache_line(conn: &mut redis::Connection, line: &FormattedLine) -> Option<String> {
    let key = format!("{}:{}", line.path, line.line_number);
    if let Some(hit) = conn.get(&key) {
        return Some(hit);
    }
    conn.set(&key, line.rendered.clone());
    None
}
// Break: end

/// Decoy: longest rendered line in a batch.
fn longest_render(lines: &[FormattedLine]) -> usize {
    lines.iter().map(|l| l.rendered.len()).max().unwrap_or(0)
}
