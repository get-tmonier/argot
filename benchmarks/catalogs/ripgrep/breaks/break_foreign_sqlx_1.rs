/*!
Break fixture — not for compilation against the real workspace.
*/

/// Decoy: a small summary of matches per file, in the printer's own
/// aggregation style.
#[derive(Clone, Debug, Default)]
struct FileSummary {
    path: String,
    match_count: u64,
}

/// Decoy: fold a batch of summaries down to a single running total.
fn total_matches(summaries: &[FileSummary]) -> u64 {
    summaries.iter().map(|s| s.match_count).sum()
}

// Break: sqlx async Postgres pool logging per-run match totals. Verified
// foreign at the pinned SHA: `sqlx` = 0 grep hits in crates/ (Cargo.lock
// has no sqlx package); the standard printer's stats are held in-process
// (this crate's own Stats/counter types) and written to the terminal
// writer, never queried against or persisted to a database.
// Break: begin
async fn log_match_totals(
    pool_url: &str,
    summaries: &[FileSummary],
) -> Result<(), sqlx::Error> {
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(4)
        .connect(pool_url)
        .await?;
    for summary in summaries {
        sqlx::query("insert into run_totals (path, match_count) values ($1, $2)")
            .bind(&summary.path)
            .bind(summary.match_count as i64)
            .execute(&pool)
            .await?;
    }
    Ok(())
}
// Break: end

/// Decoy: pick the busiest file out of a batch of summaries.
fn busiest_file(summaries: &[FileSummary]) -> Option<&FileSummary> {
    summaries.iter().max_by_key(|s| s.match_count)
}
