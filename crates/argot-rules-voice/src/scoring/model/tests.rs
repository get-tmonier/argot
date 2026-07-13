use super::*;

fn sample_model() -> LanguageModel {
    let mut token_counts = BTreeMap::new();
    token_counts.insert("17".to_string(), 42u64);
    token_counts.insert("3".to_string(), 7u64);
    let mut clusters = BTreeMap::new();
    clusters.insert(
        "0".to_string(),
        ClusterModel {
            files: vec!["src/a.py".to_string(), "src/b.py".to_string()],
            callee_counts: BTreeMap::from([("foo".to_string(), 2), ("bar".to_string(), 1)]),
        },
    );
    LanguageModel {
        bpe: BpeStats {
            token_counts,
            total_tokens: 49,
        },
        call_receiver: CallReceiverModel {
            attested: vec!["bar".to_string(), "foo".to_string()],
            n_corpus_files: 2,
            clusters,
            defined_symbols: vec!["helper".to_string()],
        },
        conventions: Some(ConventionModel {
            node_kinds: BTreeMap::from([("call_expression".to_string(), 12u64)]),
            total_nodes: 40,
            ident_shapes: BTreeMap::from([("camel".to_string(), 30u64)]),
            total_idents: 30,
            syntax_bar: 9.5,
            ident_bars: BTreeMap::from([("camel".to_string(), 1.5)]),
        }),
    }
}

#[test]
fn roundtrip_preserves_model() {
    let model = sample_model();
    let json = serde_json::to_string(&model).unwrap();
    let back: LanguageModel = serde_json::from_str(&json).unwrap();
    assert_eq!(model, back);
}

#[test]
fn hash_is_deterministic_and_content_sensitive() {
    let a = sample_model();
    let b = sample_model();
    assert_eq!(a.hash(), b.hash());
    let mut c = sample_model();
    c.call_receiver.attested.push("baz".to_string());
    assert_ne!(a.hash(), c.hash());
}
