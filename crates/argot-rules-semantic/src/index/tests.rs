use super::*;
use argot_lang::adapters::python::PythonAdapter;

fn entry(symbol: &str, path: &str, line: usize, vec: Vec<f32>) -> IndexEntry {
    IndexEntry {
        symbol: symbol.into(),
        path: path.into(),
        line,
        vec,
        callees: Vec::new(),
        subtokens: Vec::new(),
        text_hash: String::new(),
    }
}

fn unit(v: Vec<f32>) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    v.into_iter().map(|x| x / n).collect()
}

fn tiny_index() -> SemanticIndex {
    SemanticIndex {
        dim: 3,
        entries: vec![
            entry("a", "src/a.py", 1, unit(vec![1.0, 0.0, 0.0])),
            entry("b", "src/b.py", 1, unit(vec![0.9, 0.1, 0.0])),
            entry("c", "src/c.py", 1, unit(vec![0.0, 1.0, 0.0])),
        ],
    }
}

#[test]
fn nearest_ranks_by_cosine_and_respects_filter() {
    let idx = tiny_index();
    let q = unit(vec![1.0, 0.05, 0.0]);
    // All entries: a and b are closest, c far.
    let all = idx.nearest(&q, 3, |_| true);
    assert_eq!(all.len(), 3);
    assert_eq!(idx.entry(all[0].entry_index).symbol, "a");
    assert_eq!(idx.entry(all[1].entry_index).symbol, "b");
    // Exclude a's file → b wins.
    let cross = idx.nearest(&q, 3, |e| e.path != "src/a.py");
    assert_eq!(idx.entry(cross[0].entry_index).symbol, "b");
    // Margin = cos1 - cos2 is positive and small for near-duplicates a,b.
    let m = all[0].cosine - all[1].cosine;
    assert!(m > 0.0 && m < 0.2, "near-dup margin small: {m}");
}

#[test]
fn artifact_roundtrip_preserves_index_within_f16_tolerance() {
    let mut idx = tiny_index();
    idx.entries[0].callees = vec!["lower".into(), "strip".into()];
    idx.entries[0].subtokens = vec!["normalize".into(), "slug".into()];
    let mut art = SemanticArtifact::new("deadbeef".into());
    let plc = crate::placement::PlacementConfig {
        enabled: true,
        k: 10,
        z: 1,
        area_map: BTreeMap::from([("src".to_string(), "src".to_string())]),
        ..Default::default()
    };
    art.insert(
        "python",
        &idx,
        plc,
        crate::redundant::ReinventionConfig::default(),
    );
    let json = art.to_json_string().unwrap();
    let back = SemanticArtifact::from_json_str(&json).unwrap();
    assert_eq!(back.repo_sha, "deadbeef");
    let loaded = back.load("python").unwrap().unwrap();
    assert!(loaded.placement.enabled);
    assert_eq!(loaded.placement.k, 10);
    assert_eq!(loaded.placement.area_map["src"], "src");
    let idx2 = loaded.index;
    assert_eq!(idx2.len(), idx.len());
    // Callee + subtoken fingerprints survive the round-trip.
    assert_eq!(idx2.entries[0].callees, vec!["lower", "strip"]);
    assert_eq!(idx2.entries[0].subtokens, vec!["normalize", "slug"]);
    for (a, b) in idx.entries.iter().zip(&idx2.entries) {
        assert_eq!(a.symbol, b.symbol);
        assert_eq!(a.path, b.path);
        assert_eq!(a.line, b.line);
        // f16 round-trip: cosine of original vs restored ~1.
        let c = dot(&a.vec, &b.vec);
        assert!(c > 0.999, "f16 storage preserves direction: {c}");
    }
    assert!(back.load("typescript").unwrap().is_none());
}

#[test]
fn embed_text_hash_is_stable_and_content_keyed() {
    let a = embed_text_hash("def f():\n    return 1\n");
    let b = embed_text_hash("def f():\n    return 1\n");
    let c = embed_text_hash("def f():\n    return 2\n");
    assert_eq!(a, b);
    assert_ne!(a, c);
    assert_eq!(a.len(), 16, "16 hex chars");
}

#[test]
fn build_with_reuse_keeps_unchanged_vectors_and_embeds_the_rest() {
    // Needs a local model (same skip convention as the embedder tests).
    let Some(emb) = crate::static_embedder::StaticEmbedder::ready()
        .ok()
        .flatten()
    else {
        eprintln!("skipping: no local model");
        return;
    };
    let func = |symbol: &str, text: &str| FunctionRef {
        symbol: symbol.into(),
        path: "src/m.py".into(),
        line: 1,
        end_line: 3,
        text: text.into(),
        embed_text: text.into(),
        nested: false,
        callees: Vec::new(),
        subtokens: Vec::new(),
    };
    let f1 = func("a", "def a(x):\n    y = x + 1\n    return y\n");
    let f2 = func("b", "def b(x):\n    y = x * 2\n    return y\n");
    let (first, reused0) =
        SemanticIndex::build_with_reuse(&emb, &[f1.clone(), f2.clone()], None, None).unwrap();
    assert_eq!(reused0.total(), 0);

    // Second fit: f1 unchanged, f2 replaced by a new function.
    let f3 = func("c", "def c(x):\n    y = x - 3\n    return y\n");
    let (second, reused) =
        SemanticIndex::build_with_reuse(&emb, &[f1.clone(), f3], Some(&first), None).unwrap();
    assert_eq!(reused.from_prior, 1, "unchanged f1 reused");
    assert_eq!(reused.from_cache, 0);
    assert_eq!(
        second.entries[0].vec, first.entries[0].vec,
        "bit-identical reuse"
    );
    assert_ne!(second.entries[1].vec, first.entries[1].vec);
    // Hashes round-trip through the artifact so the NEXT fit reuses too.
    let mut art = SemanticArtifact::new("sha".into());
    art.insert(
        "python",
        &second,
        crate::placement::PlacementConfig::default(),
        crate::redundant::ReinventionConfig::default(),
    );
    let back = SemanticArtifact::from_json_str(&art.to_json_string().unwrap()).unwrap();
    let loaded = back.load("python").unwrap().unwrap();
    assert_eq!(
        loaded.index.entries[0].text_hash,
        second.entries[0].text_hash
    );
    assert!(!loaded.index.entries[0].text_hash.is_empty());
}

#[test]
fn fresh_artifact_validates_and_carries_model_identity() {
    let art = SemanticArtifact::new("deadbeef".into());
    assert_eq!(art.version, ARTIFACT_VERSION);
    assert_eq!(art.model, Some(ModelIdentity::current()));
    assert!(art.validate_current().is_ok());
    // Survives the JSON round-trip.
    let back = SemanticArtifact::from_json_str(&art.to_json_string().unwrap()).unwrap();
    assert!(back.validate_current().is_ok());
}

#[test]
fn stale_artifacts_are_rejected_with_a_reason() {
    // Older format version.
    let mut art = SemanticArtifact::new("sha".into());
    art.version = ARTIFACT_VERSION - 1;
    let reason = art.validate_current().unwrap_err();
    assert!(reason.contains("another argot version"), "{reason}");

    // Pre-identity artifact (v3 field missing on disk → None).
    let mut art = SemanticArtifact::new("sha".into());
    art.model = None;
    assert!(art
        .validate_current()
        .unwrap_err()
        .contains("model-identity"));

    // Different embedding model.
    let mut art = SemanticArtifact::new("sha".into());
    art.model = Some(ModelIdentity {
        name: "some-other-model".into(),
        sha256: "0".repeat(64),
        dim: 384,
    });
    let reason = art.validate_current().unwrap_err();
    assert!(reason.contains("different embedding model"), "{reason}");
    assert!(reason.contains("some-other-model"), "{reason}");
}

#[test]
fn pre_v3_json_parses_but_fails_validation() {
    // A v2 artifact on disk: no `model` field at all.
    let json = r#"{"version":2,"repo_sha":"abc","languages":{}}"#;
    let art = SemanticArtifact::from_json_str(json).unwrap();
    assert!(art.validate_current().is_err());
}

#[test]
fn dot_guards_against_dimension_mismatch() {
    let a = vec![1.0f32, 0.0, 0.0];
    let b = vec![1.0f32, 0.0];
    // Debug builds assert; release returns NEG_INFINITY (never nearest).
    let result = std::panic::catch_unwind(|| dot(&a, &b));
    if let Ok(v) = result {
        assert_eq!(v, f32::NEG_INFINITY);
    }
}

#[test]
fn functions_in_file_extracts_and_filters_trivial() {
    let src = "\
def big(a, b):
    total = a + b
    return total

def tiny():
    return 1

class C:
    def method(self, x):
        y = x * 2
        return y
";
    let adapter = PythonAdapter::new();
    let funcs = functions_in_file(&adapter, "src/m.py", src);
    let names: Vec<&str> = funcs.iter().map(|f| f.symbol.as_str()).collect();
    // `big` (3 lines) and `method` (3 lines) kept; `tiny` (2 lines) dropped.
    assert!(names.contains(&"big"), "got {names:?}");
    assert!(names.contains(&"method"), "methods indexed: {names:?}");
    assert!(!names.contains(&"tiny"), "trivial body dropped: {names:?}");
    // Path + line provenance recorded.
    let big = funcs.iter().find(|f| f.symbol == "big").unwrap();
    assert_eq!(big.path, "src/m.py");
    assert_eq!(big.line, 1);
    assert!(big.text.contains("return total"));
    // `text` is the real source (own name intact — shown verbatim in a
    // finding); `embed_text` is the name-normalised copy fed to the embedder.
    assert!(
        big.text.contains("def big("),
        "real name kept for display: {}",
        big.text
    );
    assert!(
        big.embed_text.contains("def f(") && !big.embed_text.contains("def big("),
        "own name normalised for embedding only: {}",
        big.embed_text
    );
    // Subtokens extracted from the body identifiers (≥3 chars).
    assert!(
        big.subtokens.contains(&"total".to_string()),
        "{:?}",
        big.subtokens
    );
}

#[test]
fn callee_set_parses_bare_php_method_body() {
    use argot_lang::adapters::Language;
    // A PHP method sliced out of its class has no `<?php` tag; without the
    // re-added tag tree-sitter reads it as inert HTML and finds no calls.
    let body = "public static function slug($t) {\n    $t = preg_replace('/x/', '', $t);\n    return str_replace('a', 'b', $t);\n}";
    let callees = callee_set(body, Language::Php);
    // Without the re-added `<?php` tag this is empty (tree-sitter reads it as
    // HTML). Plain function calls are captured (scoped `static::` calls are not,
    // which is fine — the plain callees carry the fingerprint).
    assert!(
        callees.iter().any(|c| c == "preg_replace"),
        "bare PHP method yields plain callees: {callees:?}"
    );
    assert!(callees.iter().any(|c| c == "str_replace"), "{callees:?}");
}

#[test]
fn normalize_own_name_replaces_whole_identifier_only() {
    // Own name + recursive self-call both normalised; substring matches left alone.
    let got = normalize_own_name("def slugify(s): return slugify(s) + slugifyish", "slugify");
    assert_eq!(got, "def f(s): return f(s) + slugifyish");
    // A different function's body is untouched.
    assert_eq!(
        normalize_own_name("return address(x)", "add"),
        "return address(x)"
    );
    // Two functions differing only in name normalise to the same embed text.
    let a = normalize_own_name(
        "func DisplayURL(u string) string { return u }",
        "DisplayURL",
    );
    let b = normalize_own_name("func ShrinkURL(u string) string { return u }", "ShrinkURL");
    assert_eq!(a, b);
}

#[test]
fn subtoken_set_splits_camel_snake_and_acronyms() {
    let set = subtoken_set("def parseHTTPResponse(url): return read_json_body(url)");
    // camelCase + acronym: parseHTTPResponse → parse, http, response
    for w in ["parse", "http", "response", "url", "read", "json", "body"] {
        assert!(set.contains(&w.to_string()), "missing {w} in {set:?}");
    }
    // sorted + deduped, all lowercase, all ≥3 chars.
    assert!(
        set.windows(2).all(|w| w[0] < w[1]),
        "sorted+deduped: {set:?}"
    );
    assert!(set
        .iter()
        .all(|s| s.len() >= 3 && s == &s.to_ascii_lowercase()));
}

#[test]
fn a_function_declared_inside_another_is_marked_nested() {
    // Object Pascal declares a local procedure in its parent's `var` section,
    // and `callable_bodies` returns a flat list — so `readbyte`, which lives
    // inside `dbtrystringtoguid` and assigns its result variable, arrived
    // looking exactly like a top-level function and was judged for placement.
    let adapter = argot_lang::adapters::adapter_for("pascal").unwrap();
    let src = "unit u;\ninterface\nimplementation\n\
        function dbtrystringtoguid(const value: string; out guid: tguid): boolean;\n\
        var\n po1: pchar;\n\n\
        \x20function readbyte: byte;\n\
        \x20begin\n\
        \x20 result:= hexchars[po1^];\n\
        \x20 inc(po1);\n\
        \x20 if shortint(result) < 0 then dbtrystringtoguid:= false;\n\
        \x20end;\n\n\
        begin\n\
         result:= true;\n\
         readbyte;\n\
         readbyte;\n\
         inc(po1);\n\
        end;\n\
        end.\n";
    let funcs = functions_in_file(adapter.as_ref(), "lib/common/db/msedb.pas", src);
    let by = |n: &str| funcs.iter().find(|f| f.symbol == n).cloned();
    let inner = by("readbyte").expect("the local procedure is extracted");
    let outer = by("dbtrystringtoguid").expect("the enclosing function is extracted");
    assert!(inner.nested, "{:?}", (inner.line, inner.end_line));
    assert!(!outer.nested, "an enclosing function is not nested");
}

/// The artifact stores int8, not f16: the blob must be one byte per component,
/// not two. Halving it is what makes a committed index affordable, so a silent
/// regression to a wider encoding is worth pinning.
#[test]
fn the_vector_blob_is_one_byte_per_component() {
    use base64::Engine as _;
    let mut art = SemanticArtifact::new("sha".into());
    let idx = tiny_index();
    art.insert("python", &idx, Default::default(), Default::default());
    let json: serde_json::Value = serde_json::from_str(&art.to_json_string().unwrap()).unwrap();
    let b64 = json["languages"]["python"]["vectors_b64"].as_str().unwrap();
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(b64)
        .unwrap();
    assert_eq!(bytes.len(), idx.entries.len() * idx.dim);
}

/// Quantisation is lossy by construction; what must survive is the *ranking*,
/// because every rule reads the index through `nearest`, never through raw
/// components.
#[test]
fn int8_round_trip_preserves_neighbour_order() {
    let mut art = SemanticArtifact::new("sha".into());
    let idx = tiny_index();
    art.insert("python", &idx, Default::default(), Default::default());
    let back = SemanticArtifact::from_json_str(&art.to_json_string().unwrap()).unwrap();
    let loaded = back.load("python").unwrap().unwrap().index;

    let q = unit(vec![1.0, 0.05, 0.0]);
    let before: Vec<&str> = idx
        .nearest(&q, 3, |_| true)
        .iter()
        .map(|n| idx.entry(n.entry_index).symbol.as_str())
        .collect();
    let after: Vec<&str> = loaded
        .nearest(&q, 3, |_| true)
        .iter()
        .map(|n| loaded.entry(n.entry_index).symbol.as_str())
        .collect();
    assert_eq!(before, after);
}

/// Vectors come back unit-norm, so a dot product is still a cosine — the
/// invariant every threshold in both rules is calibrated against.
#[test]
fn int8_round_trip_returns_unit_vectors() {
    let mut art = SemanticArtifact::new("sha".into());
    art.insert(
        "python",
        &tiny_index(),
        Default::default(),
        Default::default(),
    );
    let back = SemanticArtifact::from_json_str(&art.to_json_string().unwrap()).unwrap();
    let loaded = back.load("python").unwrap().unwrap().index;
    for e in &loaded.entries {
        let n: f32 = e.vec.iter().map(|x| x * x).sum::<f32>().sqrt();
        assert!((n - 1.0).abs() < 1e-3, "norm was {n}");
    }
}
