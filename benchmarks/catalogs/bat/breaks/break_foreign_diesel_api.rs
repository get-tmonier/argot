// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: whether all lines are visible under a config, in the config voice.
fn shows_all_lines(range_count: usize) -> bool {
    range_count == 0
}

// Break: diesel Postgres connection loading user configuration rows from a
// database, referenced by fully-qualified path (no `use` import). Verified
// foreign at the pinned SHA 78951393e29b: `diesel` = 0 grep hits across *.rs
// and absent from Cargo.toml; bat builds its Config from CLI args and a local
// config file (config.rs / bin/bat/config.rs), never from a SQL database.
// Break: begin
fn load_config_rows(database_url: &str) -> Vec<String> {
    let mut conn = diesel::pg::PgConnection::establish(database_url)
        .expect("failed to connect to config db");
    diesel::sql_query("SELECT value FROM config")
        .load::<String>(&mut conn)
        .expect("failed to load config rows")
}
// Break: end

/// Decoy: default pager name, in the config voice.
fn default_pager() -> &'static str {
    "less"
}
