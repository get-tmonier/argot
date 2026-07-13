use super::*;

#[test]
fn known_is_alphabetical_and_complete() {
    let reg = ShapePrimitiveRegistry::with_builtins();
    assert_eq!(
        reg.known(),
        vec![
            "call_scope_fraction",
            "callee_distribution_under_coverage",
            "cluster_staple_deficit",
            "except_return_raise_ratio",
            "fall_through_guards",
            "namespace_jsd",
            "typical_call_density",
        ]
    );
}

#[test]
fn build_reports_unknown() {
    let reg = ShapePrimitiveRegistry::with_builtins();
    let err = match reg.build(&["nope".to_string()]) {
        Ok(_) => panic!("expected an error for an unknown primitive"),
        Err(e) => e,
    };
    assert!(err.contains("unknown shape primitive"));
    assert!(err.contains("namespace_jsd"));
}

#[test]
fn build_yields_named_instances() {
    let reg = ShapePrimitiveRegistry::with_builtins();
    let built = reg
        .build(&[
            "namespace_jsd".to_string(),
            "fall_through_guards".to_string(),
        ])
        .unwrap();
    assert_eq!(built[0].name(), "namespace_jsd");
    assert_eq!(built[1].name(), "fall_through_guards");
    assert_eq!(built[0].min_cluster_size(), 10);
    assert_eq!(built[0].cluster_bonus_clip(), 5.0);
}
