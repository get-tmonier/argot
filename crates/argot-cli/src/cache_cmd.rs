//! `argot cache` — inspect and clear the machine-wide cache
//! (`~/.cache/argot`). Two kinds of regenerable state live there:
//!
//! - `models/` — the downloaded embedding model (~100 MB, re-fetched on next
//!   use if removed),
//! - `embeddings/` — the content-addressed embedding cache that makes repeat
//!   fits/audits fast (re-computed on next fit if removed).
//!
//! Both are pure accelerators — deleting them only costs recompute/redownload,
//! never correctness. Plain filesystem work, so this is base functionality: it
//! needs no build feature and works whether or not the semantic layer is
//! compiled in.

use std::path::{Path, PathBuf};
use std::process::ExitCode;

/// Subdirectories of the cache root, by role. Stable paths, kept in sync with
/// `argot_core::cache` (models) and the embedding cache (`embeddings/`).
const MODELS: &str = "models";
const EMBEDDINGS: &str = "embeddings";

/// Human-readable byte size (binary units).
fn human(bytes: u64) -> String {
    const UNITS: [&str; 5] = ["B", "KiB", "MiB", "GiB", "TiB"];
    let mut v = bytes as f64;
    let mut u = 0;
    while v >= 1024.0 && u < UNITS.len() - 1 {
        v /= 1024.0;
        u += 1;
    }
    if u == 0 {
        format!("{bytes} B")
    } else {
        format!("{v:.1} {}", UNITS[u])
    }
}

/// Total size of a directory tree in bytes (0 if absent).
fn dir_size(path: &Path) -> u64 {
    let Ok(entries) = std::fs::read_dir(path) else {
        return 0;
    };
    entries
        .flatten()
        .map(|e| match e.file_type() {
            Ok(t) if t.is_dir() => dir_size(&e.path()),
            Ok(_) => e.metadata().map(|m| m.len()).unwrap_or(0),
            Err(_) => 0,
        })
        .sum()
}

/// `argot cache` (no subcommand) — show the location and per-role sizes.
fn run_status() -> ExitCode {
    let Ok(root) = argot_core::cache::cache_dir() else {
        eprintln!("cache: could not resolve a cache directory (no HOME / XDG_CACHE_HOME).");
        return ExitCode::FAILURE;
    };
    if !root.is_dir() {
        println!(
            "argot cache is empty ({} does not exist yet).",
            root.display()
        );
        return ExitCode::SUCCESS;
    }
    let models = dir_size(&root.join(MODELS));
    let embeddings = dir_size(&root.join(EMBEDDINGS));
    let total = dir_size(&root);
    println!("argot cache — {}", root.display());
    println!("  model         {}", human(models));
    println!("  embeddings    {}", human(embeddings));
    println!("  total         {}", human(total));
    println!();
    println!("Clear it with `argot cache clear` (embeddings) or `argot cache clear --all` (also the model).");
    ExitCode::SUCCESS
}

/// Remove `dir` if present; report what was reclaimed.
fn clear_one(dir: &PathBuf, label: &str) -> u64 {
    if !dir.is_dir() {
        return 0;
    }
    let size = dir_size(dir);
    match std::fs::remove_dir_all(dir) {
        Ok(()) => {
            println!("cleared {label} ({})", human(size));
            size
        }
        Err(e) => {
            eprintln!("could not clear {label}: {e}");
            0
        }
    }
}

/// `argot cache clear [--all]` — drop the embedding cache (and, with `--all`,
/// the downloaded model too). The update-check state is left alone.
fn run_clear(all: bool) -> ExitCode {
    let Ok(root) = argot_core::cache::cache_dir() else {
        eprintln!("cache: could not resolve a cache directory (no HOME / XDG_CACHE_HOME).");
        return ExitCode::FAILURE;
    };
    let mut freed = clear_one(&root.join(EMBEDDINGS), "embedding cache");
    if all {
        freed += clear_one(&root.join(MODELS), "downloaded model");
    }
    if freed == 0 {
        println!("Nothing to clear — the cache was already empty.");
    } else {
        println!("Reclaimed {}.", human(freed));
        if all {
            println!("The model re-downloads and functions re-embed on the next fit.");
        } else {
            println!("Functions re-embed on the next fit; the model is untouched.");
        }
    }
    ExitCode::SUCCESS
}

pub fn run(action: Option<CacheAction>) -> ExitCode {
    match action {
        None => run_status(),
        Some(CacheAction::Clear(args)) => run_clear(args.all),
    }
}

use clap::{Args, Subcommand};

#[derive(Args)]
pub struct CacheCmd {
    #[command(subcommand)]
    pub action: Option<CacheAction>,
}

#[derive(Subcommand)]
pub enum CacheAction {
    /// Delete the embedding cache (and, with --all, the downloaded model).
    Clear(CacheClearArgs),
}

#[derive(Args)]
pub struct CacheClearArgs {
    /// Also remove the downloaded embedding model (re-fetched on next use).
    #[arg(long)]
    pub all: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn human_sizes() {
        assert_eq!(human(0), "0 B");
        assert_eq!(human(512), "512 B");
        assert_eq!(human(1024), "1.0 KiB");
        assert_eq!(human(1536), "1.5 KiB");
        assert_eq!(human(104_857_600), "100.0 MiB");
    }

    #[test]
    fn dir_size_counts_nested_and_missing() {
        let root = std::env::temp_dir().join(format!("argot-cache-size-{}", std::process::id()));
        assert_eq!(dir_size(&root), 0, "missing dir is 0");
        let sub = root.join("a").join("b");
        std::fs::create_dir_all(&sub).unwrap();
        std::fs::write(root.join("a").join("x.bin"), vec![0u8; 100]).unwrap();
        std::fs::write(sub.join("y.bin"), vec![0u8; 200]).unwrap();
        assert_eq!(dir_size(&root), 300, "sums nested files");
        std::fs::remove_dir_all(&root).ok();
    }
}
