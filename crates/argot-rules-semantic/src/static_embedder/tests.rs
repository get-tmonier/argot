use super::*;

/// argot ships its own weights, so a model is always available: no file, no
/// cache directory, no network. This is the property that makes the tool
/// offline, and it is worth failing loudly if it ever regresses.
#[test]
fn the_embedded_model_always_loads() {
    let emb = StaticEmbedder::embedded().expect("the shipped weights must load");
    assert!(emb.dim() > 0);
    assert!(!emb.fingerprint().is_empty());
}

/// Without an override, `ready` hands back the embedded model rather than
/// falling through to the transformer.
#[test]
fn ready_uses_the_embedded_model_by_default() {
    if std::env::var(STATIC_MODEL_ENV).is_ok_and(|v| !v.is_empty()) {
        return; // another model is configured in this environment
    }
    let emb = StaticEmbedder::ready().expect("load").expect("a model");
    assert_eq!(emb.name(), EMBEDDED_NAME);
}

/// A directory missing either file must fail loudly at load rather than produce
/// an embedder that returns nonsense.
#[test]
fn load_rejects_a_directory_without_the_model_files() {
    let dir = std::env::temp_dir().join("argot-static-embedder-empty");
    std::fs::create_dir_all(&dir).expect("temp dir");
    assert!(StaticEmbedder::load(&dir).is_err());
}

#[test]
fn from_bytes_rejects_weights_that_are_not_safetensors() {
    let err = match StaticEmbedder::from_bytes(b"not a tensor file", b"{}", "x".into()) {
        Ok(_) => panic!("garbage weights must not load"),
        Err(e) => format!("{e:#}"),
    };
    assert!(
        err.contains("safetensors") || err.contains("tokenizer"),
        "unhelpful error: {err}"
    );
}

/// The two names in the wild: `model2vec` writes `embeddings`,
/// sentence-transformers writes `embedding.weight`. Both must be found, or a
/// perfectly good model silently fails to load.
#[test]
fn both_known_tensor_names_are_accepted() {
    for name in TENSOR_NAMES {
        let blob = tiny_weights(name, 4, 3);
        let st = safetensors::SafeTensors::deserialize(&blob).expect("round-trip");
        assert!(st.tensor(name).is_ok(), "{name} not found");
    }
}

#[test]
fn decode_rows_widens_f32_and_f16_alike() {
    let f32_blob = tiny_weights("embeddings", 2, 2);
    let st = safetensors::SafeTensors::deserialize(&f32_blob).unwrap();
    let rows = decode_rows(&st.tensor("embeddings").unwrap(), None).unwrap();
    assert_eq!(rows.len(), 4);
    assert!((rows[0] - 0.0).abs() < 1e-6);
    assert!((rows[3] - 3.0).abs() < 1e-6);
}

#[test]
fn l2_normalize_makes_a_unit_vector() {
    let mut v = vec![3.0f32, 4.0];
    l2_normalize(&mut v);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-6, "norm was {norm}");
}

#[test]
fn l2_normalize_leaves_a_zero_vector_alone() {
    let mut v = vec![0.0f32; 4];
    l2_normalize(&mut v);
    assert!(v.iter().all(|x| *x == 0.0));
}

/// Canonicalisation is what lets a cache hit and a fresh embed produce the same
/// finding: rounding must be idempotent, or the two paths could disagree.
#[test]
fn canonicalize_f16_is_idempotent() {
    let mut a = vec![0.123_456_79f32, -0.987_654_3, 1e-8, 0.5];
    canonicalize_f16(&mut a);
    let once = a.clone();
    canonicalize_f16(&mut a);
    assert_eq!(a, once);
}

/// Two f32 values that differ below f16 precision must land on the same bits —
/// the property the whole cache-identity argument rests on.
#[test]
fn canonicalize_f16_collapses_sub_precision_jitter() {
    let mut a = vec![0.3f32];
    let mut b = vec![0.3f32 + 1e-7];
    canonicalize_f16(&mut a);
    canonicalize_f16(&mut b);
    assert_eq!(a, b);
}

/// Embedding must be reproducible call to call — the invariant a committed
/// index depends on.
#[test]
fn embedding_is_reproducible() {
    let emb = StaticEmbedder::ready().expect("load").expect("a model");
    let texts = ["fn a() { b(); }", "def a():\n    return b()"];
    let first = emb.embed(&texts).expect("embed");
    let second = emb.embed(&texts).expect("embed");
    assert_eq!(first, second);
    assert!(first.iter().all(|v| v.len() == emb.dim()));
    for v in &first {
        let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-2, "norm was {n}");
    }
}

/// A tiny `vocab × dim` f32 tensor, serialised the way a real model file is.
fn tiny_weights(name: &str, vocab: usize, dim: usize) -> Vec<u8> {
    let data: Vec<f32> = (0..vocab * dim).map(|i| i as f32).collect();
    let bytes: Vec<u8> = data.iter().flat_map(|x| x.to_le_bytes()).collect();
    let view =
        safetensors::tensor::TensorView::new(safetensors::Dtype::F32, vec![vocab, dim], &bytes)
            .expect("view");
    safetensors::serialize([(name.to_string(), view)], None).expect("serialize")
}
