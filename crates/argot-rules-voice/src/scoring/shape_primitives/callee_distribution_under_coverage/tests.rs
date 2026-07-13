use super::*;

fn files(sources: &[&str]) -> Vec<(PathBuf, String)> {
    sources
        .iter()
        .enumerate()
        .map(|(i, s)| (PathBuf::from(format!("f{i}.py")), s.to_string()))
        .collect()
}

#[test]
fn abstains_below_min_calls() {
    let prim = CalleeDistributionUnderCoverage::default();
    let cluster = files(&["foo()\nbar()\nbaz()\n"; 5]);
    let baseline = prim
        .fit_cluster_baseline(&cluster, Language::Python)
        .unwrap();
    assert_eq!(prim.score("qux()\n", Some(&baseline), 10), 0.0);
}

#[test]
fn abstains_below_cluster_size_floor() {
    let prim = CalleeDistributionUnderCoverage::default();
    let cluster = files(&["foo()\nbar()\nbaz()\n"; 5]);
    let baseline = prim
        .fit_cluster_baseline(&cluster, Language::Python)
        .unwrap();
    assert_eq!(prim.score("qux()\nquux()\n", Some(&baseline), 9), 0.0);
}

#[test]
fn cluster_matching_hunk_contributes_zero() {
    let prim = CalleeDistributionUnderCoverage::default();
    let cluster = files(&["foo()\nbar()\nbaz()\n"; 6]);
    let baseline = prim
        .fit_cluster_baseline(&cluster, Language::Python)
        .unwrap();
    assert_eq!(
        prim.score("foo()\nbar()\nbaz()\n", Some(&baseline), 10),
        0.0
    );
}

#[test]
fn language_agnostic_runs_on_typescript() {
    let prim = CalleeDistributionUnderCoverage::default();
    let cluster: Vec<(PathBuf, String)> = (0..5)
        .map(|i| {
            (
                PathBuf::from(format!("f{i}.ts")),
                "foo();\nbar();\nbaz();\n".to_string(),
            )
        })
        .collect();
    let baseline = prim
        .fit_cluster_baseline(&cluster, Language::Typescript)
        .unwrap();
    assert_eq!(
        prim.score("foo();\nbar();\nbaz();\n", Some(&baseline), 10),
        0.0
    );
}
