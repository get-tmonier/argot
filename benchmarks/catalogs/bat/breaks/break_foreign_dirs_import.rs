// Break fixture — parses in isolation; not built against the bat workspace.

use std::path::PathBuf;

/// Decoy: append the bat subfolder to a base dir, in the directories voice.
fn bat_subdir(base: PathBuf) -> PathBuf {
    base.join("bat")
}

// Break: `dirs` config-directory lookup reached through an ALIASED import
// (`use dirs as user_dirs`), the alias masking the crate name. Verified foreign
// at the pinned SHA 78951393e29b: `dirs` = 0 grep hits across *.rs and absent
// from Cargo.toml; bat resolves cache/config dirs through `etcetera`
// (BaseStrategy in bin/bat/directories.rs) plus the BAT_CACHE_PATH /
// BAT_CONFIG_DIR env vars, never the `dirs` crate.
// Break: begin
use dirs as user_dirs;

fn resolve_bat_config_dir() -> Option<PathBuf> {
    user_dirs::config_dir().map(|base| base.join("bat"))
}
// Break: end

/// Decoy: whether a cache-dir override is set, in the directories voice.
fn has_cache_override(value: Option<&str>) -> bool {
    value.map(|v| !v.is_empty()).unwrap_or(false)
}
