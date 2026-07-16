//! Reconstruct whole-identifier names from BPE-piece-level surprise spans.
//!
//! Real identifiers are usually split across multiple BPE pieces (`mongoose`
//! → `mongo` + `ose`). Reconstruction expands each surprising piece's span left
//! and right over the identifier character class in the source itself, then
//! keeps the substrings matching the identifier rule.
//!
//! Coordinate note: the Rust tokenizer reports **byte** offsets. Because the
//! identifier alphabet is ASCII-only, every non-ASCII byte is a boundary in
//! both byte- and char-space, so byte-space reconstruction yields identifiers
//! identical to a char-space reconstruction.

use argot_lang::bpe::BpeTokenizer;
use std::collections::HashSet;

/// Identifier alphabet: ASCII letters, digits, underscore (`_IDENT_CHARS`).
fn is_ident_byte(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

/// `^[A-Za-z_][A-Za-z0-9_]*$` over ASCII bytes (`_IDENTIFIER_RE`).
fn matches_identifier(bytes: &[u8]) -> bool {
    match bytes.first() {
        Some(&first) if first.is_ascii_alphabetic() || first == b'_' => {
            bytes[1..].iter().all(|&b| is_ident_byte(b))
        }
        _ => false,
    }
}

/// Expand each `(start, end)` byte span to its enclosing identifier. Walk left
/// from `start` and right from `end` while bytes belong to the identifier
/// class; keep substrings matching the identifier rule. Output is deduped in
/// occurrence order.
pub fn reconstruct_identifiers(source: &str, spans: &[(usize, usize)]) -> Vec<String> {
    let bytes = source.as_bytes();
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = Vec::new();
    for &(start, end) in spans {
        let mut s = start;
        while s > 0 && is_ident_byte(bytes[s - 1]) {
            s -= 1;
        }
        let mut e = end;
        while e < bytes.len() && is_ident_byte(bytes[e]) {
            e += 1;
        }
        let candidate = &bytes[s..e];
        if candidate.is_empty() || !matches_identifier(candidate) {
            continue;
        }
        // Guaranteed ASCII by `matches_identifier`, so UTF-8 decode is total.
        let cand = String::from_utf8_lossy(candidate).into_owned();
        if seen.insert(cand.clone()) {
            out.push(cand);
        }
    }
    out
}

/// Return the byte spans of the top-`top_k` surprising BPE pieces, in source
/// order. Empty spans (`(0, 0)` markers) are skipped, as are tokens rejected by
/// `is_meaningful`.
pub fn top_k_surprising_spans(
    hunk_source: &str,
    tokenizer: &BpeTokenizer,
    score_fn: &dyn Fn(u32) -> f64,
    top_k: usize,
    is_meaningful: Option<&dyn Fn(u32) -> bool>,
) -> Vec<(usize, usize)> {
    if top_k == 0 || hunk_source.is_empty() {
        return Vec::new();
    }
    let (ids, offsets) = tokenizer.encode_with_offsets(hunk_source);
    if ids.is_empty() {
        return Vec::new();
    }
    // (start, end, score) for each non-empty, meaningful piece — in source order.
    let scored: Vec<(usize, usize, f64)> = ids
        .iter()
        .zip(offsets.iter())
        .filter(|(tok, (s, e))| s != e && is_meaningful.is_none_or(|f| f(**tok)))
        .map(|(tok, (s, e))| (*s, *e, score_fn(*tok)))
        .collect();
    if scored.is_empty() {
        return Vec::new();
    }
    let threshold_idx = top_k.min(scored.len());
    // Stable sort by score descending — ties keep source order (matches
    // Python's `sorted(..., key=lambda t: -t[2])`).
    let mut by_score = scored.clone();
    by_score.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));
    let top_spans: HashSet<(usize, usize)> = by_score[..threshold_idx]
        .iter()
        .map(|(s, e, _)| (*s, *e))
        .collect();
    scored
        .iter()
        .filter(|(s, e, _)| top_spans.contains(&(*s, *e)))
        .map(|(s, e, _)| (*s, *e))
        .collect()
}

/// High-level helper: top-K surprising pieces → reconstructed identifiers,
/// capped at `max_identifiers`.
pub fn surprising_identifiers(
    hunk_source: &str,
    tokenizer: &BpeTokenizer,
    score_fn: &dyn Fn(u32) -> f64,
    top_k: usize,
    max_identifiers: usize,
    is_meaningful: Option<&dyn Fn(u32) -> bool>,
) -> Vec<String> {
    let spans = top_k_surprising_spans(hunk_source, tokenizer, score_fn, top_k, is_meaningful);
    let mut out = reconstruct_identifiers(hunk_source, &spans);
    out.truncate(max_identifiers);
    out
}

#[cfg(test)]
mod tests;
