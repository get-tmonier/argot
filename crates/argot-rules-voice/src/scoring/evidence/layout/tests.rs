use super::*;

#[test]
fn comma_int_matches_python() {
    assert_eq!(comma_int(0), "0");
    assert_eq!(comma_int(847), "847");
    assert_eq!(comma_int(1_800), "1,800");
    assert_eq!(comma_int(12_400), "12,400");
    assert_eq!(comma_int(1_000_000), "1,000,000");
}

#[test]
fn frequency_suffix() {
    assert_eq!(format_frequency(0), "0×");
    assert_eq!(format_frequency(3_200), "3,200×");
}

#[test]
fn rarity_floor_switches_wording() {
    let below = RarityStat {
        flagged_count: 0,
        attested_total: 7,
        noun: "module specifiers".into(),
        where_: "repo".into(),
    };
    assert_eq!(format_rarity(&below), "never seen in repo");
    let above = RarityStat {
        flagged_count: 0,
        attested_total: 12_400,
        noun: "identifiers".into(),
        where_: "repo".into(),
    };
    assert_eq!(format_rarity(&above), "0 of 12,400 identifiers in repo");
}

#[test]
fn common_here_floor() {
    let show = vec![CommonEntry {
        name: "x".into(),
        count: 3,
    }];
    assert!(should_show_common_here(&show));
    let hide = vec![CommonEntry {
        name: "x".into(),
        count: 2,
    }];
    assert!(!should_show_common_here(&hide));
    assert!(!should_show_common_here(&[]));
}
