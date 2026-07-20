//! Wire format for the dataset JSONL. Every hunk emitted conforms to this
//! schema.
//!
//! Field order is significant and fixed: records serialise their fields in
//! declaration order (the `json.dumps(asdict(record))` convention). serde
//! serialises struct fields in declaration order too, so keeping the field
//! order as written here is what makes the JSONL byte-compatible (given a
//! `json.dumps`-style separator formatter — see argot-core's `json` module).

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
    Pascal,
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

/// One emitted hunk. Field order is fixed (declaration order) for JSONL byte
/// parity.
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
