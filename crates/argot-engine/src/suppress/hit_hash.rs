//! Content-based hit hashes.
//!
//! A hit's identity must survive line drift (code above it growing or
//! shrinking), so the hash covers *what* was flagged, never *where*: the
//! repo-relative file path, the winning reason code, and the hunk content with
//! each line's trailing whitespace trimmed. Twelve hex chars of SHA-256 keep
//! it short enough to read off a terminal and paste into `argot mute`.

use sha2::{Digest, Sha256};

/// Length of the rendered hit hash (hex chars).
pub const HIT_HASH_LEN: usize = 12;

/// Compute the hit hash for a flagged hunk.
///
/// `rel_path` is repo-relative and `/`-separated; `reason_code` is the winning
/// scorer reason (`bpe`, `import`, `call_receiver`, …); `hunk_content` is the
/// hunk exactly as scored — normalization (trailing-whitespace trim per line)
/// happens here.
pub fn hit_hash(rel_path: &str, reason_code: &str, hunk_content: &str) -> String {
    let normalized: Vec<&str> = hunk_content.lines().map(|l| l.trim_end()).collect();
    let mut hasher = Sha256::new();
    hasher.update(rel_path.as_bytes());
    hasher.update([0u8]);
    hasher.update(reason_code.as_bytes());
    hasher.update([0u8]);
    hasher.update(normalized.join("\n").as_bytes());
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(HIT_HASH_LEN);
    for byte in digest.iter().take(HIT_HASH_LEN.div_ceil(2)) {
        hex.push_str(&format!("{byte:02x}"));
    }
    hex.truncate(HIT_HASH_LEN);
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_shaped() {
        let a = hit_hash("src/app.py", "bpe", "def f():\n    return 1");
        let b = hit_hash("src/app.py", "bpe", "def f():\n    return 1");
        assert_eq!(a, b);
        assert_eq!(a.len(), HIT_HASH_LEN);
        assert!(a.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn line_position_does_not_matter() {
        // The hash covers content only — the same hunk shifted 10 lines down in
        // its file produces the same hash (line numbers never enter the hash).
        let hunk = "def g(x):\n    y = x * 2\n    return y";
        assert_eq!(
            hit_hash("src/app.py", "bpe", hunk),
            hit_hash("src/app.py", "bpe", hunk),
        );
    }

    #[test]
    fn trailing_whitespace_is_normalized() {
        assert_eq!(
            hit_hash("a.py", "bpe", "x = 1  \ny = 2\t"),
            hit_hash("a.py", "bpe", "x = 1\ny = 2"),
        );
    }

    #[test]
    fn content_path_and_reason_all_matter() {
        let base = hit_hash("a.py", "bpe", "x = 1");
        assert_ne!(base, hit_hash("a.py", "bpe", "x = 2"), "content");
        assert_ne!(base, hit_hash("b.py", "bpe", "x = 1"), "path");
        assert_ne!(base, hit_hash("a.py", "import", "x = 1"), "reason");
    }

    #[test]
    fn leading_whitespace_is_preserved() {
        // Indentation is voice — only *trailing* whitespace is normalized.
        assert_ne!(
            hit_hash("a.py", "bpe", "    x = 1"),
            hit_hash("a.py", "bpe", "x = 1"),
        );
    }
}
