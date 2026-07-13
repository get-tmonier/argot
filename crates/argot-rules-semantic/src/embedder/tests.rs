use super::*;

/// Cosine of two equal-length vectors.
fn cosine(a: &[f32], b: &[f32]) -> f32 {
    a.iter().zip(b).map(|(x, y)| x * y).sum()
}

/// Load an embedder iff a model is already on disk (env override or a
/// present cache file). Never calls `resolve_model_path` — a unit test must
/// not hit the network, so on CI (no model, no download) it returns `None`
/// and the model-dependent tests skip.
fn local_embedder() -> Option<Embedder> {
    let path = std::env::var(MODEL_ENV)
        .ok()
        .map(PathBuf::from)
        .filter(|p| p.exists())
        .or_else(|| {
            let cached = cache_dir().ok()?.join("models").join(MODEL_FILENAME);
            cached.exists().then_some(cached)
        })?;
    Embedder::load(&path).ok()
}

fn scratch(name: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("argot_embedder_{name}_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

#[test]
fn gc_removes_stale_ggufs_and_partials_but_keeps_current() {
    let dir = scratch("gc");
    let keep = dir.join(MODEL_FILENAME);
    std::fs::write(&keep, b"current").unwrap();
    std::fs::write(dir.join("old-model-v0.gguf"), b"old").unwrap();
    std::fs::write(dir.join("x.gguf.partial.123"), b"orphan").unwrap();
    std::fs::write(dir.join("CACHEDIR.TAG"), b"tag").unwrap();
    gc_stale_cache_files(&dir, &keep);
    assert!(keep.exists(), "current model kept");
    assert!(!dir.join("old-model-v0.gguf").exists(), "old model gone");
    assert!(!dir.join("x.gguf.partial.123").exists(), "orphan gone");
    assert!(dir.join("CACHEDIR.TAG").exists(), "unrelated file kept");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn cachedir_tag_written_once_with_signature() {
    let dir = scratch("tag");
    write_cachedir_tag(&dir);
    let content = std::fs::read_to_string(dir.join("CACHEDIR.TAG")).unwrap();
    assert!(content.starts_with("Signature: 8a477f597d28d172789f06886806bc55"));
    // Idempotent: a hand-edited tag is left untouched.
    std::fs::write(dir.join("CACHEDIR.TAG"), "custom").unwrap();
    write_cachedir_tag(&dir);
    assert_eq!(
        std::fs::read_to_string(dir.join("CACHEDIR.TAG")).unwrap(),
        "custom"
    );
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn l2_normalize_makes_unit_vectors() {
    let mut v = vec![3.0f32, 4.0];
    l2_normalize(&mut v);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-6);
}

#[test]
fn embed_shape_and_semantics() {
    let Some(emb) = local_embedder() else {
        eprintln!("skipping: no local model (set {MODEL_ENV})");
        return;
    };
    let dup = "def add(a, b):\n    return a + b\n";
    let same = "def add(a, b):\n    return a + b\n";
    let diff = "class HttpRetryPolicy:\n    def backoff(self, n):\n        return 2 ** n\n";

    let vecs = emb.embed(&[dup, same, diff]).unwrap();
    assert_eq!(vecs.len(), 3);
    for v in &vecs {
        assert_eq!(v.len(), EMBED_DIM);
        let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((norm - 1.0).abs() < 1e-4, "vectors are L2-normalised");
    }
    // Identical inputs → cosine ~1; unrelated code → clearly lower.
    let self_cos = cosine(&vecs[0], &vecs[1]);
    let cross_cos = cosine(&vecs[0], &vecs[2]);
    assert!(
        self_cos > 0.999,
        "identical code near-identical: {self_cos}"
    );
    assert!(
        cross_cos < self_cos - 0.05,
        "unrelated code separates: self={self_cos} cross={cross_cos}"
    );
}

#[test]
fn embed_tolerates_a_nul_byte_in_the_source() {
    let Some(emb) = local_embedder() else {
        eprintln!("skipping: no local model (set {MODEL_ENV})");
        return;
    };
    // A real function can carry a raw NUL — e.g. a `\0` key separator in a
    // template literal (seen in the wild: moneta's change-coordinator). The
    // C-string tokenizer rejects NUL, so we strip it: embedding must succeed
    // and match the NUL-free text bit-for-bit (the byte carries no token).
    let with_nul = "const key = `${a}\u{0}${b}`\nreturn key\n";
    let without = "const key = `${a}${b}`\nreturn key\n";
    let a = emb
        .embed_one(with_nul)
        .expect("NUL must not fail the embed");
    let b = emb.embed_one(without).unwrap();
    assert_eq!(a, b, "stripping the NUL yields the NUL-free embedding");
}

#[test]
fn embed_output_is_f16_canonical_and_bit_stable_across_calls() {
    let Some(emb) = local_embedder() else {
        eprintln!("skipping: no local model (set {MODEL_ENV})");
        return;
    };
    let texts = [
        "def one(a):\n    b = a + 1\n    return b\n",
        "def two(a):\n    b = a * 2\n    return b\n",
        "def three(a):\n    b = a - 3\n    return b\n",
    ];
    let vecs = emb.embed(&texts).unwrap();
    for v in &vecs {
        // Canonical: every component is exactly f16-representable, so the
        // artifact/cache round-trip is bit-identical.
        assert!(v.iter().all(|&x| x == half::f16::from_f32(x).to_f32()));
    }
    // Embedding a text in a multi-text call, alone, and a second time must
    // all yield the *same bits* — the f16 canonicalisation absorbs the
    // encoder's low-bit run-to-run jitter, which is what makes a cache hit
    // interchangeable with a fresh embed.
    for (i, text) in texts.iter().enumerate() {
        let solo = emb.embed_one(text).unwrap();
        let again = emb.embed_one(text).unwrap();
        assert_eq!(vecs[i], solo, "slice vs solo bit-identical");
        assert_eq!(solo, again, "repeat embed bit-identical");
    }
}

#[test]
fn embeds_a_long_function_without_crashing() {
    // Regression: jina-code is an encoder; llama.cpp asserts
    // `n_ubatch >= n_tokens`, so a function longer than the default 512-token
    // ubatch used to crash. Build a body well over that.
    let Some(emb) = local_embedder() else {
        eprintln!("skipping: no local model (set {MODEL_ENV})");
        return;
    };
    let mut body = String::from("def huge():\n");
    for i in 0..1200 {
        body.push_str(&format!(
            "    value_{i} = compute_something({i}) + offset\n"
        ));
    }
    let v = emb.embed_one(&body).unwrap();
    assert_eq!(v.len(), EMBED_DIM);
    let norm: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    assert!((norm - 1.0).abs() < 1e-4);
}
