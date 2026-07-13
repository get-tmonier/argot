//! The user-level cache directory (`~/.cache/argot`) — one resolution shared
//! by everything that persists regenerable state (the semantic model, the
//! update-check state). XDG on Linux/macOS, `%LOCALAPPDATA%` on Windows.

use anyhow::{Context, Result};
use std::path::PathBuf;

/// Resolve the argot cache root. Errors only when no home can be determined.
pub fn cache_dir() -> Result<PathBuf> {
    if let Ok(x) = std::env::var("XDG_CACHE_HOME") {
        if !x.is_empty() {
            return Ok(PathBuf::from(x).join("argot"));
        }
    }
    #[cfg(target_os = "windows")]
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if !local.is_empty() {
            return Ok(PathBuf::from(local).join("argot"));
        }
    }
    let home = std::env::var("HOME").context("neither XDG_CACHE_HOME nor HOME is set")?;
    Ok(PathBuf::from(home).join(".cache").join("argot"))
}
