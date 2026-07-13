use super::*;

fn rarity(total: i64, where_: &str) -> RarityStat {
    RarityStat {
        flagged_count: 0,
        attested_total: total,
        noun: "callees".into(),
        where_: where_.into(),
    }
}

#[test]
fn bpe_line_matches_golden_shape() {
    let ev = Evidence::Bpe(BpeEvidence {
        surprising_identifiers: vec![
            CommonEntry {
                name: "sessionmaker".into(),
                count: 0,
            },
            CommonEntry {
                name: "connect".into(),
                count: 0,
            },
            CommonEntry {
                name: "url".into(),
                count: 0,
            },
            CommonEntry {
                name: "engine".into(),
                count: 0,
            },
            CommonEntry {
                name: "bind".into(),
                count: 0,
            },
        ],
    });
    let lines = format_evidence(&ev, false, 1);
    assert_eq!(
        lines,
        vec!["     ↳ sessionmaker (0×), connect (0×), url (0×) (+2 more)".to_string()]
    );
}

#[test]
fn call_receiver_lines_with_denominator() {
    // Exercises the "0 of N" rarity branch and the common-here line that
    // neither committed golden hits.
    let ev = Evidence::CallReceiver(CallReceiverEvidence {
        unfamiliar_callees: vec!["mongoose".into(), "connect".into()],
        rarity: rarity(1_247, "this cluster"),
        common_here: vec![
            CommonEntry {
                name: "select".into(),
                count: 40,
            },
            CommonEntry {
                name: "insert".into(),
                count: 12,
            },
        ],
    });
    let lines = format_evidence(&ev, false, 1);
    assert_eq!(
        lines,
        vec![
            "     ↳ mongoose, connect — 0 of 1,247 callees in this cluster".to_string(),
            "       common here: select (40×), insert (12×)".to_string(),
        ]
    );
}

#[test]
fn import_annotates_line_and_colors_when_asked() {
    let ev = Evidence::Import(ImportEvidence {
        foreign_specifiers: vec!["msgspec".into()],
        rarity: RarityStat {
            flagged_count: 0,
            attested_total: 7,
            noun: "module specifiers".into(),
            where_: "repo".into(),
        },
        common_here: vec![CommonEntry {
            name: "react".into(),
            count: 5,
        }],
        foreign_specifier_spans: vec![(
            "msgspec".into(),
            SourceSpan {
                line: 3,
                col_start: 7,
                col_end: 14,
            },
        )],
    });
    // hunk_start_line=5 → file line 5 + 3 - 1 = 7.
    let plain = format_evidence(&ev, false, 5);
    assert_eq!(
        plain[0],
        "     ↳ msgspec (L7) — never seen in repo".to_string()
    );
    assert_eq!(plain[1], "       common here: react (5×)".to_string());
    // Colored path wraps each line in dim ANSI.
    let colored = format_evidence(&ev, true, 5);
    assert!(colored[0].starts_with("     \x1b[2m↳ msgspec"));
    assert!(colored[0].ends_with("\x1b[0m"));
}

#[test]
fn lines_of_interest_and_carets() {
    let ev = Evidence::Import(ImportEvidence {
        foreign_specifiers: vec!["msgspec".into()],
        rarity: RarityStat {
            flagged_count: 0,
            attested_total: 7,
            noun: "module specifiers".into(),
            where_: "repo".into(),
        },
        common_here: vec![],
        foreign_specifier_spans: vec![(
            "msgspec".into(),
            SourceSpan {
                line: 2,
                col_start: 7,
                col_end: 17,
            },
        )],
    });
    let loi = evidence_lines_of_interest(Some(&ev));
    assert!(loi.contains(&2) && loi.len() == 1);
    let carets = evidence_caret_spans(Some(&ev));
    assert_eq!(carets.get(&2).map(|v| v.len()), Some(1));
    // BPE / None carry no carets or lines-of-interest.
    assert!(evidence_caret_spans(None).is_empty());
    assert!(evidence_lines_of_interest(None).is_empty());
}
