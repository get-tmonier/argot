use super::*;
use crate::index::IndexEntry;

fn unit(v: Vec<f32>) -> Vec<f32> {
    let n: f32 = v.iter().map(|x| x * x).sum::<f32>().sqrt();
    v.into_iter().map(|x| x / n).collect()
}

fn entry(symbol: &str, path: &str, vec: Vec<f32>) -> IndexEntry {
    IndexEntry {
        symbol: symbol.into(),
        path: path.into(),
        line: 1,
        vec: unit(vec),
        callees: Vec::new(),
        subtokens: Vec::new(),
        text_hash: String::new(),
    }
}

fn func(symbol: &str, path: &str) -> FunctionRef {
    FunctionRef {
        symbol: symbol.into(),
        path: path.into(),
        line: 10,
        end_line: 20,
        text: "def f():\n    a\n    b\n    c\n    d\n    e\n    g".into(),
        embed_text: "def f():\n    a\n    b\n    c\n    d\n    e\n    g".into(),
        callees: Vec::new(),
        subtokens: Vec::new(),
    }
}

/// Two clean semantic clusters in two packages under a `src/` container.
fn index() -> SemanticIndex {
    let mut entries = Vec::new();
    for i in 0..12 {
        entries.push(entry(
            &format!("db_{i}"),
            &format!("src/db/m{i}.py"),
            vec![1.0, 0.02 * i as f32, 0.0],
        ));
    }
    for i in 0..12 {
        entries.push(entry(
            &format!("ui_{i}"),
            &format!("src/ui/v{i}.py"),
            vec![0.0, 1.0, 0.02 * i as f32],
        ));
    }
    SemanticIndex { dim: 3, entries }
}

#[test]
fn adaptive_walk_descends_containers_and_stops_at_packages() {
    let idx = index();
    let walk = AreaWalk::new(idx.entries.iter().map(|e| e.path.as_str()));
    // src/ holds 100% (container) → the packages are the areas.
    assert_eq!(walk.area("src/db/m1.py"), "src/db");
    assert_eq!(walk.area("src/ui/v3.py"), "src/ui");
    // A new file in an unseen tiny dir merges up into the container.
    assert_eq!(walk.area("src/new_thing/x.py"), "src");
}

#[test]
fn calibration_enables_separable_repo_and_scorer_fires_on_transplant() {
    let idx = index();
    let cfg = calibrate_placement(&idx);
    assert!(cfg.enabled, "clean two-cluster repo is judgeable: {cfg:?}");
    assert!(cfg.sim_recall >= 0.85);
    let scorer = PlacementScorer::new(&idx, &cfg);
    // A db-flavoured function filed under src/ui → misplaced.
    let q = unit(vec![1.0, 0.03, 0.0]);
    let f = scorer
        .evaluate(&func("load_row", "src/ui/widgets.py"), &q)
        .expect("misplaced db-in-ui fires");
    assert_eq!(f.actual_area, "src/ui");
    assert_eq!(f.neighbor_area, "src/db");
    // The same function correctly filed → quiet.
    assert!(scorer
        .evaluate(&func("load_row", "src/db/new.py"), &q)
        .is_none());
}

#[test]
fn calibration_disables_unseparable_repo() {
    // One semantic blob spread over two dirs: every function's neighbours
    // straddle both areas → entangled → merged → single area → disabled.
    let mut entries = Vec::new();
    for i in 0..24 {
        let dir = if i % 2 == 0 { "src/a" } else { "src/b" };
        entries.push(entry(
            &format!("f{i}"),
            &format!("{dir}/m{i}.py"),
            vec![1.0, 0.001 * i as f32, 0.0],
        ));
    }
    let idx = SemanticIndex { dim: 3, entries };
    let cfg = calibrate_placement(&idx);
    assert!(!cfg.enabled, "entangled blob must abstain: {cfg:?}");
}

#[test]
fn disabled_config_abstains() {
    let idx = index();
    let cfg = PlacementConfig::default();
    let scorer = PlacementScorer::new(&idx, &cfg);
    let q = unit(vec![1.0, 0.03, 0.0]);
    assert!(scorer
        .evaluate(&func("load_row", "src/ui/widgets.py"), &q)
        .is_none());
}

#[test]
fn stub_and_new_dir_candidates_abstain() {
    let idx = index();
    let cfg = calibrate_placement(&idx);
    let scorer = PlacementScorer::new(&idx, &cfg);
    let q = unit(vec![1.0, 0.03, 0.0]);
    // New directory (no precedent) → abstain.
    assert!(scorer
        .evaluate(&func("load_row", "src/brand_new/x.py"), &q)
        .is_none());
    // Stub (< 6 body lines) → abstain even in a judged location.
    let mut stub = func("load_row", "src/ui/widgets.py");
    stub.text = "def f():\n    pass".into();
    assert!(scorer.evaluate(&stub, &q).is_none());
}

#[test]
fn a_merged_group_is_named_by_the_directories_it_holds() {
    // Three areas where two are entangled and merge under one label. The
    // finding must name the directories the reader can open — the file's own
    // and the peers' — never the merge group's label, which is a real
    // directory neither of them is in. On MSEide/MSEgui that rendered as
    // "looks like lib/common/kernel code filed under lib/common/dialogs" for a
    // file in lib/common/dialogx whose nearest peer was in lib/common/graphics:
    // both halves false, both naming real directories, so both believable.
    let mut entries = Vec::new();
    for i in 0..24 {
        // `graphics` and `image` are one semantic blob → they merge, and the
        // group is labelled after whichever is bigger. Both need enough
        // functions to be areas in their own right rather than merging up.
        let dir = if i % 2 == 0 {
            "src/image"
        } else {
            "src/graphics"
        };
        entries.push(entry(
            &format!("g{i}"),
            &format!("{dir}/m{i}.py"),
            vec![1.0, 0.01 * i as f32, 0.0],
        ));
    }
    for i in 0..12 {
        entries.push(entry(
            &format!("d{i}"),
            &format!("src/dialogs/v{i}.py"),
            vec![0.0, 1.0, 0.02 * i as f32],
        ));
    }
    let idx = SemanticIndex { dim: 3, entries };
    let cfg = calibrate_placement(&idx);
    assert!(cfg.enabled, "{cfg:?}");
    assert_eq!(
        cfg.area_map.get("src/image"),
        cfg.area_map.get("src/graphics"),
        "the two entangled areas must share one merge label: {:?}",
        cfg.area_map
    );
    let scorer = PlacementScorer::new(&idx, &cfg);
    let q = unit(vec![1.0, 0.02, 0.0]);
    let f = scorer
        .evaluate(&func("draw", "src/dialogs/form.py"), &q)
        .expect("graphics-flavoured function filed under dialogs fires");
    assert_eq!(f.actual_area, "src/dialogs", "the file's own directory");
    // The peers decide the name, so it is a directory that actually holds
    // them — not necessarily the label their merge group carries.
    let peer_dirs: Vec<&str> = f.peers.iter().map(|(_, p, _)| parent_dir(p)).collect();
    assert!(
        peer_dirs.contains(&f.neighbor_area.as_str()),
        "named {} but the peers live in {peer_dirs:?}",
        f.neighbor_area
    );
}

#[test]
fn parent_dir_extracts_directory() {
    assert_eq!(parent_dir("src/ui/widgets.py"), "src/ui");
    assert_eq!(parent_dir("main.py"), "");
}
