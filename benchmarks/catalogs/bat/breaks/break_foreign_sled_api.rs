// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: build the on-disk cache key for an asset, in the assets voice.
fn asset_cache_key(name: &str) -> String {
    format!("asset/{name}")
}

// Break: HARD — a `sled::Db` embedded key-value store read ONLY through a
// receiver whose methods (`get`, `ok`, `map`, `to_vec`) are all attested bat
// idioms, and whose crate is named only in the parameter TYPE position (never a
// `use` or a `foreign::` callee). Verified foreign at the pinned SHA
// 78951393e29b: `sled` = 0 grep hits across *.rs and absent from Cargo.toml;
// bat's asset cache is a directory of bincode blobs (asset_from_cache in
// assets.rs), never an embedded database. Expected to be genuinely hard: the
// only foreign token sits in a type annotation neither stage inspects.
// Break: begin
fn cached_asset(db: &sled::Db, key: &str) -> Option<Vec<u8>> {
    let found = db.get(key);
    let value = found.ok()?;
    value.map(|raw| raw.to_vec())
}
// Break: end

/// Decoy: whether an asset blob is fresh enough to reuse, in the assets voice.
fn is_fresh(age_secs: u64) -> bool {
    age_secs < 86_400
}
