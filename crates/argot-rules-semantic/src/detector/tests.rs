use super::{format_semantic_evidence, SemanticHitEvidence};

#[test]
fn semantic_evidence_renders_nearest_code() {
    let redundant = SemanticHitEvidence::Redundant {
        nearest_symbol: "slugify".into(),
        nearest_path: "src/utils/text.py".into(),
        nearest_line: 1,
        similarity: 0.86,
    };
    let lines = format_semantic_evidence(&redundant, false);
    assert!(lines[0].contains("duplicates slugify (src/utils/text.py:1)"));
    assert!(lines[0].contains("0.86"));

    let misplaced = SemanticHitEvidence::Misplaced {
        neighbor_area: "src/db".into(),
        actual_area: "src/ui".into(),
        peers: vec![("load_row".into(), "src/db/models.py".into(), 12)],
    };
    let lines = format_semantic_evidence(&misplaced, false);
    assert!(lines[0].contains("looks like src/db code filed under src/ui"));
    assert!(lines[1].contains("load_row (src/db/models.py:12)"));
}
