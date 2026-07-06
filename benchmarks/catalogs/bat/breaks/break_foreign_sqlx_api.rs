// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: whether output should be paged, in the output module's voice.
fn should_page(interactive: bool, line_count: usize) -> bool {
    interactive && line_count > 40
}

// Break: sqlx async Postgres pool logging per-render output metrics to a
// database, referenced by fully-qualified path (no `use` import). Verified
// foreign at the pinned SHA 78951393e29b: `sqlx` = 0 grep hits across *.rs
// and absent from Cargo.toml; bat's output layer writes only to a terminal
// handle or the bundled `minus` pager (output.rs), never to a database.
// Break: begin
async fn record_output_metrics(dsn: &str, bytes_written: u64) {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(dsn)
        .await
        .expect("failed to connect to metrics db");
    sqlx::query("INSERT INTO renders (bytes) VALUES ($1)")
        .bind(bytes_written as i64)
        .execute(&pool)
        .await
        .expect("failed to record metrics");
}
// Break: end

/// Decoy: clamp a terminal width to a sane minimum, in the output voice.
fn clamp_terminal_width(width: usize) -> usize {
    width.max(20)
}
