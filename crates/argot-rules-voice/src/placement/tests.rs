use super::*;

fn sig(locs: &[&str], feats: &[&str]) -> FileSig {
    FileSig {
        locations: locs.iter().map(|s| s.to_string()).collect(),
        features: feats.iter().map(|s| s.to_string()).collect(),
    }
}

#[test]
fn location_labels_extract_dir_role_and_ext() {
    // Every directory segment is a label (universal ones self-filter later via
    // lift); the role is the last stem segment; plus the extension.
    let l = location_labels("frontend/domain/user.service.ts", ".ts");
    assert!(l.contains(&"dir:domain".to_string()), "{l:?}");
    assert!(l.contains(&"dir:frontend".to_string()), "{l:?}");
    assert!(l.contains(&"role:service".to_string()), "{l:?}");
    assert!(l.contains(&"ext:.ts".to_string()), "{l:?}");

    // A single-part stem is still a role (the whole name) — `capsule.ts` →
    // `capsule`, learned from recurrence, no hardcoded role list.
    let c = location_labels("app/widgets/capsule.ts", ".ts");
    assert!(c.contains(&"dir:widgets".to_string()), "{c:?}");
    assert!(c.contains(&"role:capsule".to_string()), "{c:?}");
    // Repeated segments don't double-count.
    assert_eq!(
        c.iter().filter(|x| x.starts_with("dir:")).count(),
        2,
        "{c:?}"
    );
}

#[test]
fn features_keep_real_calls_drop_self_refs() {
    let noise = argot_lang::adapters::adapter_for("python")
        .unwrap()
        .identifier_noise()
        .clone();
    let f = features(
        "db.execute('q')\nprint('x')\nself.run()\n",
        Language::Python,
        &noise,
    );
    assert!(f.contains("db"), "{f:?}");
    assert!(f.contains("db.execute"), "{f:?}");
    assert!(f.contains("print"), "{f:?}");
    // `self` is a universal self-reference — dropped (structural, not hardcoded
    // vocabulary).
    assert!(!f.iter().any(|x| x.starts_with("self")), "{f:?}");
}

#[test]
fn candidate_surfaces_concentrated_feature_only() {
    // 8 api files use `validate`; 32 ui files use `render`; both use a `shared`
    // helper. Only `validate` is a minority-location, high-concentration
    // convention; `render` is the majority (low lift) and `shared` is spread.
    let mut sigs = Vec::new();
    for _ in 0..8 {
        sigs.push(sig(&["dir:api"], &["validate", "shared"]));
    }
    for _ in 0..32 {
        sigs.push(sig(&["dir:ui"], &["render", "shared"]));
    }
    let cands = candidates_from(&sigs);

    let v = cands.iter().find(|c| c.feature == "validate");
    assert!(
        v.is_some(),
        "{:?}",
        cands
            .iter()
            .map(|c| (&c.feature, &c.location))
            .collect::<Vec<_>>()
    );
    let v = v.unwrap();
    assert_eq!(v.location, "dir:api");
    assert_eq!(v.home_files, 8);
    assert_eq!(v.out_files, 0);
    assert!((v.concentration - 1.0).abs() < 1e-9);
    assert!(v.lift >= MIN_LIFT);

    // A spread helper is never a placement convention.
    assert!(
        !cands.iter().any(|c| c.feature == "shared"),
        "spread helper leaked"
    );
    // A majority-location feature has too-low lift.
    assert!(
        !cands.iter().any(|c| c.feature == "render"),
        "majority feature leaked"
    );
}

#[test]
fn low_support_and_small_location_are_dropped() {
    // `rare` appears in only 3 files (< MIN_SUPPORT); the location has < MIN_GROUP.
    let mut sigs = vec![
        sig(&["dir:tiny"], &["rare"]),
        sig(&["dir:tiny"], &["rare"]),
        sig(&["dir:tiny"], &["rare"]),
    ];
    for _ in 0..40 {
        sigs.push(sig(&["dir:big"], &["common"]));
    }
    let cands = candidates_from(&sigs);
    assert!(
        !cands.iter().any(|c| c.feature == "rare"),
        "under-supported candidate leaked"
    );
}

#[test]
fn empty_corpus_yields_nothing() {
    assert!(candidates_from(&[]).is_empty());
}

#[test]
fn mine_placement_surfaces_a_convention_end_to_end() {
    // Lay out a repo where `queryInterface` lives only in `migrations/`, then
    // run the full pipeline (walk → features → aggregate) and assert it surfaces.
    let dir = std::env::temp_dir().join(format!("argot_placement_e2e_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join("migrations")).unwrap();
    std::fs::create_dir_all(dir.join("app")).unwrap();
    for i in 0..10 {
        std::fs::write(
            dir.join("migrations").join(format!("m{i}.ts")),
            "export const up = queryInterface.addColumn(\"t\", \"c\");\n",
        )
        .unwrap();
        std::fs::write(
            dir.join("app").join(format!("a{i}.ts")),
            "export const view = render();\n",
        )
        .unwrap();
    }

    let places = mine_placement(&dir);
    let _ = std::fs::remove_dir_all(&dir);

    let migrations = places.iter().find(|p| p.location == "dir:migrations");
    assert!(
        migrations.is_some(),
        "mined: {:?}",
        places.iter().map(|p| &p.location).collect::<Vec<_>>()
    );
    let m = migrations.unwrap();
    assert!(
        m.signature.iter().any(|f| f.feature == "queryInterface"),
        "signature: {:?}",
        m.signature.iter().map(|f| &f.feature).collect::<Vec<_>>()
    );
    // Rule-ready: the home glob scopes a rule to files outside migrations/.
    assert_eq!(m.location_globs, vec!["**/migrations/**"]);
}

#[test]
fn location_globs_scope_the_home() {
    assert_eq!(location_globs("dir:migrations"), vec!["**/migrations/**"]);
    assert_eq!(location_globs("ext:.tsx"), vec!["**/*.tsx"]);
    assert_eq!(
        location_globs("role:capsule"),
        vec!["**/capsule.*", "**/*.capsule.*"]
    );
    assert!(location_globs("weird").is_empty());
}

fn raw(feat: &str, loc: &str, home: usize) -> RawCandidate {
    RawCandidate {
        feature: feat.to_string(),
        location: loc.to_string(),
        loc_files: home,
        home_files: home,
        out_files: 0,
        lift: 5.0,
        concentration: 1.0,
    }
}

#[test]
fn aggregate_dedupes_receiver_and_groups_by_location() {
    // One "migrations = queryInterface" convention, expressed as the bare
    // receiver plus three of its methods, must collapse to a single place whose
    // signature drops the `queryInterface.*` variants.
    let flat = vec![
        raw("queryInterface", "dir:migrations", 10),
        raw("queryInterface.addColumn", "dir:migrations", 9),
        raw("queryInterface.removeColumn", "dir:migrations", 8),
        raw("queryInterface.addIndex", "dir:migrations", 7),
    ];
    let places = aggregate(flat);
    assert_eq!(places.len(), 1, "one place");
    let p = &places[0];
    assert_eq!(p.location, "dir:migrations");
    // Receiver-deduped: only the bare `queryInterface` survives.
    assert_eq!(p.signature.len(), 1, "{:?}", p.signature);
    assert_eq!(p.signature[0].feature, "queryInterface");
}

#[test]
fn aggregate_caps_places_and_signature_length() {
    // 40 distinct locations, each with 8 distinct features → capped.
    let mut flat = Vec::new();
    for l in 0..40 {
        for f in 0..8 {
            flat.push(raw(&format!("feat{l}_{f}.m"), &format!("dir:d{l}"), 10 - f));
        }
    }
    let places = aggregate(flat);
    assert!(places.len() <= MAX_CONVENTIONS, "places={}", places.len());
    assert!(places.iter().all(|p| p.signature.len() <= MAX_SIGNATURE));
}
