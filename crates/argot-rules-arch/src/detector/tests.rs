use super::arch_evidence;

#[test]
fn arch_evidence_names_the_broken_direction() {
    use crate::graph::Violation;
    let edge = ("core".to_string(), "cli".to_string());
    assert_eq!(
        arch_evidence(&edge, Violation::Reversal),
        "cli → core is this repo's direction — this import reverses it"
    );
    assert!(arch_evidence(&edge, Violation::TransitiveReversal).contains("closes a cycle"));
    assert!(arch_evidence(&edge, Violation::SinkOut).contains("never imports out of"));
}
