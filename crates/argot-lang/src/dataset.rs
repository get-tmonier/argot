//! Wire format for the dataset JSONL — a faithful port of
//! `engine/argot/dataset.py`. Every hunk emitted conforms to this schema.
//!
//! Field order is significant: the Python engine writes records with
//! `json.dumps(asdict(record))`, which serialises dataclass fields in
//! declaration order. serde serialises struct fields in declaration order
//! too, so keeping the field order identical here is what makes the JSONL
//! byte-compatible (given a Python-style separator formatter — see
//! argot-core's `json` module).

use serde::{Deserialize, Serialize};

/// Source language of a hunk. Serialises to lowercase strings:
/// `"typescript" | "javascript" | "python" | "go" | "rust"`.
/// Source language of a hunk. Serialises to the exact lowercase strings the
/// wire format uses: `"typescript" | "javascript" | "python" | "java"`.
/// Source language of a hunk. Serialises to the exact lowercase strings used in
/// the wire format: `"typescript" | "javascript" | "python" | "csharp"`.
/// Source language of a hunk. Serialises to the exact lowercase strings:
/// `"typescript" | "javascript" | "python" | "php"`.
/// wire format uses: `"typescript" | "javascript" | "python" | "cpp"`.
/// Python `Language` `Literal` uses (`"typescript" | "javascript" | "python"`),
/// extended with `"ruby"`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Language {
    Typescript,
    Javascript,
    Python,
    Go,
    Rust,
    C,
    Java,
    Csharp,
    Php,
    Cpp,
    Ruby,
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
mod tests;
