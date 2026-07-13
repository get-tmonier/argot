use super::*;

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argot_embed_cache_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    dir
}

/// A canonical (f16-representable) vector so equality is exact.
fn vec_for(seed: f32) -> Vec<f32> {
    (0..EMBED_DIM)
        .map(|i| f16::from_f32(seed + i as f32 / 1024.0).to_f32())
        .collect()
}

#[test]
fn persist_then_reopen_roundtrips_bit_identical() {
    let dir = scratch("roundtrip");
    let cache = EmbedCache::open_at(dir.clone());
    assert!(cache.is_empty());
    let hash = crate::index::embed_text_hash("def f():\n    pass\n");
    cache.persist(&[(hash.clone(), vec_for(0.25))]);

    let reopened = EmbedCache::open_at(dir.clone());
    assert_eq!(reopened.len(), 1);
    assert_eq!(reopened.get(&hash), Some(&vec_for(0.25)), "bit-identical");
    assert_eq!(reopened.get("0000000000000000"), None);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn persist_skips_already_cached_and_malformed_entries() {
    let dir = scratch("skip");
    let cache = EmbedCache::open_at(dir.clone());
    cache.persist(&[("aabbccddeeff0011".into(), vec_for(0.5))]);

    let second = EmbedCache::open_at(dir.clone());
    // Already cached → no new segment; wrong-dim and non-hex → dropped.
    second.persist(&[
        ("aabbccddeeff0011".into(), vec_for(0.5)),
        ("not-hex-not-hex!".into(), vec_for(0.1)),
        ("1122334455667788".into(), vec![1.0; 3]),
    ]);
    assert_eq!(segment_files(&dir).len(), 1, "no redundant segment");
    assert_eq!(EmbedCache::open_at(dir.clone()).len(), 1);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn trailing_partial_record_is_ignored() {
    let dir = scratch("partial");
    let cache = EmbedCache::open_at(dir.clone());
    cache.persist(&[("0102030405060708".into(), vec_for(0.75))]);
    // Simulate a crashed writer: append half a record to the segment.
    let seg = segment_files(&dir).pop().unwrap();
    let mut bytes = std::fs::read(&seg).unwrap();
    bytes.extend_from_slice(&[0u8; RECORD_BYTES / 2]);
    std::fs::write(&seg, bytes).unwrap();

    let reopened = EmbedCache::open_at(dir.clone());
    assert_eq!(reopened.len(), 1, "whole records survive, tail ignored");
    assert!(reopened.get("0102030405060708").is_some());
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn gc_evicts_oldest_segments_first() {
    let dir = scratch("gc");
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("seg-000000000001-1.bin"), vec![0u8; 100]).unwrap();
    std::fs::write(dir.join("seg-000000000002-1.bin"), vec![0u8; 100]).unwrap();
    std::fs::write(dir.join("seg-000000000003-1.bin"), vec![0u8; 100]).unwrap();
    gc_to_cap(&dir, 250);
    let left: Vec<String> = segment_files(&dir)
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        left,
        vec!["seg-000000000002-1.bin", "seg-000000000003-1.bin"],
        "oldest evicted"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

/// End-to-end with a real model when one is on disk (same skip convention
/// as the embedder tests): a cache hit must be bit-identical to a fresh
/// embed, and a second index build in a "different checkout" (no prior
/// artifact, warm cache) must reuse everything.
#[test]
fn cache_hit_is_bit_identical_to_fresh_embed() {
    let Some(emb) = crate::embedder::Embedder::ready().ok().flatten() else {
        eprintln!("skipping: no local model");
        return;
    };
    let dir = scratch("e2e");
    let texts = [
        "def slug(s):\n    s = s.lower()\n    return s.replace(' ', '-')\n",
        "def add(a, b):\n    total = a + b\n    return total\n",
    ];
    let cold = EmbedCache::open_at(dir.clone());
    let fresh = embed_with_cache(&emb, &texts, Some(&cold)).unwrap();

    let warm = EmbedCache::open_at(dir.clone());
    assert_eq!(warm.len(), 2, "both misses persisted");
    let hits = embed_with_cache(&emb, &texts, Some(&warm)).unwrap();
    assert_eq!(fresh, hits, "cache hit is bit-identical");

    // The same texts through an index build in a prior-less checkout: all
    // vectors come from the cache.
    let func = |symbol: &str, text: &str| crate::index::FunctionRef {
        symbol: symbol.into(),
        path: "src/m.py".into(),
        line: 1,
        end_line: 3,
        text: text.into(),
        embed_text: text.into(),
        callees: Vec::new(),
        subtokens: Vec::new(),
    };
    let (idx, stats) = crate::index::SemanticIndex::build_with_reuse(
        &emb,
        &[func("slug", texts[0]), func("add", texts[1])],
        None,
        Some(&warm),
    )
    .unwrap();
    assert_eq!(stats.from_cache, 2);
    assert_eq!(stats.from_prior, 0);
    assert_eq!(idx.entries[0].vec, fresh[0]);
    assert_eq!(idx.entries[1].vec, fresh[1]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn key_decode_matches_embed_text_hash_format() {
    // embed_text_hash yields 16 lowercase hex chars = exactly one key.
    let h = crate::index::embed_text_hash("x");
    assert!(decode_key(&h).is_some());
    assert_eq!(decode_key("short"), None);
    assert_eq!(decode_key("zzzzzzzzzzzzzzzz"), None);
}
