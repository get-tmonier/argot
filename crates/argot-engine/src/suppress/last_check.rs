//! `.argot/last-check.json` — a small machine-readable cache of the most
//! recent `check` run's visible hits, written on every check run. `argot mute
//! <hash>` resolves the hash against this cache to learn which file the hit
//! belongs to.

use serde::{Deserialize, Serialize};
use std::path::Path;

/// File name of the cache inside the argot dir.
pub const LAST_CHECK_FILE: &str = "last-check.json";

/// One visible hit from the last check run.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LastCheckHit {
    /// Repo-relative, `/`-separated file path.
    pub path: String,
    /// Winning scorer reason code.
    pub reason: String,
    /// Content-based hit hash (see [`crate::suppress::hit_hash`]).
    pub hash: String,
    pub line_start: usize,
    pub line_end: usize,
}

#[derive(Serialize, Deserialize)]
struct LastCheckDoc {
    hits: Vec<LastCheckHit>,
}

/// Write the cache (best-effort callers may ignore the result).
pub fn write_last_check(argot_dir: &Path, hits: &[LastCheckHit]) -> std::io::Result<()> {
    std::fs::create_dir_all(argot_dir)?;
    let doc = LastCheckDoc {
        hits: hits.to_vec(),
    };
    let json = serde_json::to_string_pretty(&doc).expect("serializing hits cannot fail");
    std::fs::write(argot_dir.join(LAST_CHECK_FILE), json)
}

/// Read the cache. `None` when absent or unreadable.
pub fn read_last_check(argot_dir: &Path) -> Option<Vec<LastCheckHit>> {
    let content = std::fs::read_to_string(argot_dir.join(LAST_CHECK_FILE)).ok()?;
    let doc: LastCheckDoc = serde_json::from_str(&content).ok()?;
    Some(doc.hits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn tmp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("argot_lastcheck_{name}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn roundtrip() {
        let dir = tmp_dir("roundtrip");
        let hits = vec![LastCheckHit {
            path: "src/app.py".to_string(),
            reason: "bpe".to_string(),
            hash: "abc123def456".to_string(),
            line_start: 3,
            line_end: 12,
        }];
        write_last_check(&dir, &hits).unwrap();
        assert_eq!(read_last_check(&dir), Some(hits));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn absent_is_none() {
        assert_eq!(read_last_check(Path::new("/nonexistent/argotdir")), None);
    }
}
