// Break fixture — parses in isolation; not built against the bat workspace.

/// Decoy: normalize a config lookup key, in the config voice.
fn config_cache_key(path: &str) -> String {
    path.trim().to_lowercase()
}

// Break: HARD — an `lru::LruCache` reached ONLY through a receiver whose
// methods (`contains`, `get`, `cloned`) collide with bat's own attested map
// idioms, and whose crate is named only in the parameter TYPE position (never a
// `use` or a `foreign::` callee). Verified foreign at the pinned SHA
// 78951393e29b: `lru` = 0 grep hits across *.rs and absent from Cargo.toml; bat
// caches parsed configuration in plain owned structs (config.rs), not an LRU
// cache. Expected to be genuinely hard: the foreignness lives in a type
// annotation the import/call-receiver stages do not inspect.
// Break: begin
fn cached_config(store: &lru::LruCache<String, String>, key: &str) -> Option<String> {
    if store.contains(key) {
        store.get(key).cloned()
    } else {
        None
    }
}
// Break: end

/// Decoy: whether two config keys are equivalent, in the config voice.
fn keys_match(a: &str, b: &str) -> bool {
    a.eq_ignore_ascii_case(b)
}
