// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: the cache key for a named theme set, in the assets voice.
fn theme_cache_key(name: &str) -> String {
    format!("bat:theme:{name}")
}

// Break: redis client caching serialized theme sets — the foreign namespace is
// named once on the constructor (`redis::Client::open`), then the writes flow
// through a local `client` / `con` RECEIVER variable. Verified foreign at the
// pinned SHA 78951393e29b: `redis` = 0 grep hits across *.rs and absent from
// Cargo.toml; bat caches assets on the local filesystem (asset_from_cache in
// assets.rs), never in an external key-value store.
// Break: begin
fn cache_theme_set(dsn: &str, name: &str, bytes: &[u8]) {
    let client = redis::Client::open(dsn).expect("bad redis dsn");
    let mut con = client.get_connection().expect("no redis connection");
    con.set::<_, _, ()>(name, bytes).expect("cache write failed");
}
// Break: end

/// Decoy: whether a cached blob is worth keeping, in the assets voice.
fn is_cacheable(bytes: &[u8]) -> bool {
    !bytes.is_empty() && bytes.len() < 4 * 1024 * 1024
}
