//! Wire format for the dataset JSONL — a faithful port of
//! `engine/argot/dataset.py`. Every hunk emitted conforms to this schema.
//!
//! Field order is significant: the Python engine writes records with
//! `json.dumps(asdict(record))`, which serialises dataclass fields in
//! declaration order. serde serialises struct fields in declaration order
//! too, so keeping the field order identical here is what makes the JSONL
//! byte-compatible (given a Python-style separator formatter — see
//! [`crate::json`]).

use serde::{Deserialize, Serialize};

/// Source language of a hunk. Serialises to the exact lowercase strings the
/// wire format uses: `"typescript" | "javascript" | "python" | "cpp"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Typescript,
    Javascript,
    Python,
    Cpp,
}

/// A single leaf token from the tree-sitter parse (`dataset.Token`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Token {
    pub text: String,
    /// tree-sitter node kind, e.g. `"function_declaration"`.
    pub node_type: String,
    pub start_line: usize,
    pub end_line: usize,
}

/// One emitted hunk (`dataset.HunkRecord`). Field order mirrors the Python
/// dataclass exactly for JSONL byte parity.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HunkRecord {
    pub commit_sha: String,
    pub file_path: String,
    pub language: Language,
    pub hunk_start_line: usize,
    pub hunk_end_line: usize,
    /// up to 50 lines before, tokenized
    pub context_before: Vec<Token>,
    pub hunk_tokens: Vec<Token>,
    /// up to 50 lines after, tokenized
    pub context_after: Vec<Token>,
    pub parent_sha: Option<String>,
    pub author_date_iso: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_serialises_to_lowercase_literals() {
        assert_eq!(
            serde_json::to_string(&Language::Typescript).unwrap(),
            "\"typescript\""
        );
        assert_eq!(
            serde_json::to_string(&Language::Python).unwrap(),
            "\"python\""
        );
    }

    #[test]
    fn none_parent_sha_serialises_as_null() {
        let rec = HunkRecord {
            commit_sha: "abc".into(),
            file_path: "a.py".into(),
            language: Language::Python,
            hunk_start_line: 0,
            hunk_end_line: 1,
            context_before: vec![],
            hunk_tokens: vec![],
            context_after: vec![],
            parent_sha: None,
            author_date_iso: "0".into(),
        };
        let s = serde_json::to_string(&rec).unwrap();
        assert!(s.contains("\"parent_sha\":null"), "got: {s}");
    }
}
