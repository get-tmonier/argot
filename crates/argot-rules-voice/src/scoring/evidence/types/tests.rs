use super::*;

#[test]
fn scope_label_joins_noun_and_where() {
    let rarity = RarityStat {
        flagged_count: 0,
        attested_total: 5,
        noun: "callees".to_string(),
        where_: "this cluster".to_string(),
    };
    assert_eq!(rarity.scope_label(), "callees in this cluster");
}

#[test]
fn span_for_finds_a_present_name_and_none_for_an_absent_one() {
    let evidence = ImportEvidence {
        foreign_specifiers: vec!["left-pad".to_string()],
        rarity: RarityStat {
            flagged_count: 0,
            attested_total: 0,
            noun: "module specifiers".to_string(),
            where_: "repo".to_string(),
        },
        common_here: Vec::new(),
        foreign_specifier_spans: vec![(
            "left-pad".to_string(),
            SourceSpan {
                line: 3,
                col_start: 4,
                col_end: 12,
            },
        )],
    };
    assert_eq!(
        evidence.span_for("left-pad"),
        Some(&SourceSpan {
            line: 3,
            col_start: 4,
            col_end: 12
        })
    );
    assert_eq!(evidence.span_for("requests"), None);
}

fn valid_json() -> Value {
    serde_json::json!({
        "imports": [{"name": "os", "count": 10}],
        "identifiers": {"connect": 3},
        "callees_by_cluster": {
            "0": [{"name": "save", "count": 2}]
        },
        "totals": {
            "import_specifiers_attested": 42,
            "callees_attested_by_cluster": {"0": 7}
        }
    })
}

#[test]
fn from_json_parses_a_well_formed_corpus() {
    let corpus = EvidenceCorpus::from_json(&valid_json()).expect("well-formed input parses");
    assert_eq!(
        corpus.imports,
        vec![CommonEntry {
            name: "os".to_string(),
            count: 10
        }]
    );
    assert_eq!(corpus.identifiers.get("connect"), Some(&3));
    assert_eq!(
        corpus.callees_by_cluster.get(&0),
        Some(&vec![CommonEntry {
            name: "save".to_string(),
            count: 2
        }])
    );
    assert_eq!(corpus.totals.import_specifiers_attested, 42);
    assert_eq!(corpus.totals.callees_attested_by_cluster.get(&0), Some(&7));
}

#[test]
fn from_json_returns_none_when_a_required_block_is_missing() {
    let mut raw = valid_json();
    raw.as_object_mut().unwrap().remove("totals");
    assert!(EvidenceCorpus::from_json(&raw).is_none());
}

#[test]
fn from_json_returns_none_when_a_cluster_key_is_not_a_valid_usize() {
    let mut raw = valid_json();
    raw["callees_by_cluster"] = serde_json::json!({"not-a-number": []});
    assert!(EvidenceCorpus::from_json(&raw).is_none());
}

#[test]
fn from_json_returns_none_when_an_identifier_count_is_not_an_integer() {
    let mut raw = valid_json();
    raw["identifiers"] = serde_json::json!({"connect": "not-a-number"});
    assert!(EvidenceCorpus::from_json(&raw).is_none());
}

#[test]
fn from_json_returns_none_for_a_non_object_input() {
    assert!(EvidenceCorpus::from_json(&serde_json::json!([1, 2, 3])).is_none());
}
