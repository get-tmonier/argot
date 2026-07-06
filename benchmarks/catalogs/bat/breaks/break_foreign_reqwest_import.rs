// Break fixture — parses in isolation; not built against the bat workspace.

use std::path::{Path, PathBuf};

/// Decoy: locate a cached asset file, in bat's asset-directory voice.
fn cached_asset_path(cache_dir: &Path, name: &str) -> PathBuf {
    cache_dir.join("themes").join(format!("{name}.bin"))
}

// Break: reqwest blocking HTTP client pulling a theme set over the network
// before the serialized syntax set is loaded. Verified foreign at the pinned
// SHA 78951393e29b: `reqwest` = 0 grep hits across *.rs and absent from
// Cargo.toml; bat loads every theme/syntax asset from an embedded
// `include_bytes!` blob or the local cache dir (assets.rs), never over HTTP.
// Break: begin
use reqwest::blocking::Client;

fn fetch_remote_theme_set(url: &str) -> Vec<u8> {
    let client = reqwest::blocking::Client::builder()
        .build()
        .expect("failed to build http client");
    let response = client.get(url).send().expect("theme request failed");
    response
        .bytes()
        .expect("failed to read theme bytes")
        .to_vec()
}
// Break: end

/// Decoy: pick a fallback theme name, in bat's voice.
fn fallback_theme_name(is_dark: bool) -> &'static str {
    if is_dark {
        "Monokai Extended"
    } else {
        "Monokai Extended Light"
    }
}
