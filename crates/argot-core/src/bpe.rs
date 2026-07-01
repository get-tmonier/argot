//! BPE tokenisation — the production scoring tokenizer.
//!
//! The Python engine loads `microsoft/unixcoder-base` via
//! `transformers.AutoTokenizer` and calls
//! `encode(source, add_special_tokens=False)`. That fast tokenizer is backed
//! by HuggingFace's Rust `tokenizers` library; we embed the exact serialised
//! `tokenizer.json` and load it with the same library, so `encode` is
//! bit-identical.

use std::collections::HashMap;
use tokenizers::Tokenizer;

/// The serialised `microsoft/unixcoder-base` fast tokenizer, exported from
/// the Python `transformers` backend. Embedding keeps the binary a single
/// self-contained artifact (no HF Hub round-trip, no torch).
const TOKENIZER_JSON: &[u8] = include_bytes!("../data/unixcoder_tokenizer.json");

/// Byte-level BPE tokenizer matching the Python production path.
pub struct BpeTokenizer {
    inner: Tokenizer,
}

impl BpeTokenizer {
    /// Load the embedded unixcoder tokenizer.
    pub fn load() -> Self {
        let inner = Tokenizer::from_bytes(TOKENIZER_JSON)
            .expect("embedded unixcoder tokenizer.json is valid");
        Self { inner }
    }

    /// Encode `text` to token ids, `add_special_tokens=False`.
    pub fn encode(&self, text: &str) -> Vec<u32> {
        // `false` = do not add the RoBERTa <s>/</s> special tokens, matching
        // the Python `add_special_tokens=False`.
        let encoding = self
            .inner
            .encode(text, false)
            .expect("tokenizer encode never fails on valid UTF-8");
        encoding.get_ids().to_vec()
    }

    /// Encode `text`, returning token ids paired with their `(start, end)`
    /// **byte** offsets into `text` — the Rust equivalent of the Python
    /// `tokenizer(text, return_offsets_mapping=True, add_special_tokens=False)`.
    ///
    /// The HuggingFace `tokenizers` library reports offsets as byte indices,
    /// whereas Python's `offset_mapping` reports character indices. The
    /// evidence BPE reconstruction only walks ASCII identifier bytes (see
    /// `evidence::bpe_reconstruction`), and every multi-byte UTF-8 char is a
    /// boundary in both coordinate systems, so byte-space reconstruction
    /// yields identifiers identical to Python's char-space code.
    pub fn encode_with_offsets(&self, text: &str) -> (Vec<u32>, Vec<(usize, usize)>) {
        let encoding = self
            .inner
            .encode(text, false)
            .expect("tokenizer encode never fails on valid UTF-8");
        (encoding.get_ids().to_vec(), encoding.get_offsets().to_vec())
    }

    /// Reverse vocab lookup — Python `self._id_to_token.get(id, "")`.
    pub fn id_to_token(&self, id: u32) -> Option<String> {
        self.inner.id_to_token(id)
    }

    /// Full vocab (`get_vocab()` → token→id), excluding added tokens to match
    /// the Python `get_vocab()` shape.
    pub fn vocab(&self) -> HashMap<String, u32> {
        self.inner.get_vocab(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_and_reports_vocab_size() {
        let tok = BpeTokenizer::load();
        // Python reported vocab_size 51416 for microsoft/unixcoder-base.
        assert_eq!(tok.vocab().len(), 51416);
    }

    #[test]
    fn empty_string_encodes_empty() {
        let tok = BpeTokenizer::load();
        assert!(tok.encode("").is_empty());
    }

    #[test]
    fn matches_python_sample() {
        // Captured from Python: tok.encode("def foo(x):\n    return x + 1\n").
        let tok = BpeTokenizer::load();
        let ids = tok.encode("def foo(x):\n    return x + 1\n");
        assert_eq!(
            ids,
            vec![729, 5089, 126, 206, 953, 317, 377, 483, 868, 513, 524, 317]
        );
    }
}
