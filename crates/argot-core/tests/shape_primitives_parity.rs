//! Golden parity for the five shape primitives — checks both
//! `fit_cluster_baseline` (against `expected_baseline`) and `score` (against
//! `expected_score`) for every captured case. Goldens are authoritative; the
//! Rust port is fixed to match them, never the reverse.

use argot_core::scoring::adapters::Language;
use argot_core::scoring::shape_primitive::{Baseline, ShapePrimitive, ShapePrimitiveRegistry};
use serde::Deserialize;
use serde_json::Value;
use std::path::PathBuf;

#[derive(Deserialize)]
struct Golden {
    name: String,
    min_cluster_size: usize,
    cluster_bonus_clip: f64,
    baseline_kind: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    id: String,
    cluster_files: Option<Vec<(String, String)>>,
    fit_language: Option<String>,
    fit_returns_none: Option<bool>,
    expected_baseline: Option<Value>,
    score_baseline: Option<Value>,
    #[serde(default)]
    score_language: Option<String>,
    hunk: Option<String>,
    cluster_size: Option<i64>,
    expected_score: Option<f64>,
}

fn lang(s: &str) -> Language {
    match s {
        "python" => Language::Python,
        "typescript" => Language::Typescript,
        other => panic!("unknown language {other:?}"),
    }
}

fn build(name: &str) -> Box<dyn ShapePrimitive> {
    ShapePrimitiveRegistry::with_builtins()
        .build(&[name.to_string()])
        .unwrap()
        .pop()
        .unwrap()
}

fn parse_baseline(kind: &str, v: &Value) -> Baseline {
    match kind {
        "namespace" => Baseline::Namespace {
            language: lang(v["language"].as_str().unwrap()),
            alphabet: v["alphabet"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect(),
            distribution: v["distribution"]
                .as_object()
                .unwrap()
                .iter()
                .map(|(k, val)| (k.clone(), val.as_f64().unwrap()))
                .collect(),
        },
        "mean_std" => Baseline::MeanStd {
            mean: v["mean"].as_f64().unwrap(),
            std: v["std"].as_f64().unwrap(),
        },
        "top10_mean_std" => Baseline::Top10MeanStd {
            top10_set: v["top10_set"]
                .as_array()
                .unwrap()
                .iter()
                .map(|x| x.as_str().unwrap().to_string())
                .collect(),
            mean: v["mean"].as_f64().unwrap(),
            std: v["std"].as_f64().unwrap(),
        },
        other => panic!("unknown baseline_kind {other:?}"),
    }
}

const EPS: f64 = 1e-9;

fn baselines_close(a: &Baseline, b: &Baseline) -> bool {
    match (a, b) {
        (
            Baseline::Namespace {
                language: l1,
                alphabet: a1,
                distribution: d1,
            },
            Baseline::Namespace {
                language: l2,
                alphabet: a2,
                distribution: d2,
            },
        ) => {
            l1 == l2
                && a1 == a2
                && d1.len() == d2.len()
                && d1
                    .iter()
                    .all(|(k, v)| d2.get(k).map(|v2| (v - v2).abs() < EPS).unwrap_or(false))
        }
        (Baseline::MeanStd { mean: m1, std: s1 }, Baseline::MeanStd { mean: m2, std: s2 }) => {
            (m1 - m2).abs() < EPS && (s1 - s2).abs() < EPS
        }
        (
            Baseline::Top10MeanStd {
                top10_set: t1,
                mean: m1,
                std: s1,
            },
            Baseline::Top10MeanStd {
                top10_set: t2,
                mean: m2,
                std: s2,
            },
        ) => t1 == t2 && (m1 - m2).abs() < EPS && (s1 - s2).abs() < EPS,
        _ => false,
    }
}

fn load(name: &str) -> Golden {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/shape_primitives")
        .join(format!("{name}_golden.json"));
    serde_json::from_str(&std::fs::read_to_string(&path).unwrap())
        .unwrap_or_else(|e| panic!("parse {path:?}: {e}"))
}

fn run_golden(name: &str) {
    let golden = load(name);
    assert_eq!(golden.name, name);

    for case in &golden.cases {
        let prim = build(&golden.name);
        assert_eq!(prim.min_cluster_size(), golden.min_cluster_size);
        assert_eq!(prim.cluster_bonus_clip(), golden.cluster_bonus_clip);

        // --- Fit ---
        if let Some(cfiles) = &case.cluster_files {
            let files: Vec<(PathBuf, String)> = cfiles
                .iter()
                .map(|(p, s)| (PathBuf::from(p), s.clone()))
                .collect();
            let flang = lang(case.fit_language.as_ref().expect("fit_language"));
            let got = prim.fit_cluster_baseline(&files, flang);
            if case.fit_returns_none == Some(true) {
                assert!(
                    got.is_none(),
                    "[{name}/{}] expected fit None, got {got:?}",
                    case.id
                );
            } else {
                let got = got.unwrap_or_else(|| {
                    panic!("[{name}/{}] expected a baseline, got None", case.id)
                });
                let exp = parse_baseline(
                    &golden.baseline_kind,
                    case.expected_baseline.as_ref().expect("expected_baseline"),
                );
                assert!(
                    baselines_close(&got, &exp),
                    "[{name}/{}] baseline mismatch: got {got:?}, exp {exp:?}",
                    case.id
                );
            }
        }

        // --- Score ---
        if let Some(hunk) = &case.hunk {
            if let Some(sl) = &case.score_language {
                prim.set_language(lang(sl));
            }
            let sb: Option<Baseline> = case
                .score_baseline
                .as_ref()
                .map(|v| parse_baseline(&golden.baseline_kind, v));
            let cluster_size = case.cluster_size.expect("cluster_size") as usize;
            let got = prim.score(hunk, sb.as_ref(), cluster_size);
            let exp = case.expected_score.expect("expected_score");
            assert!(
                (got - exp).abs() < EPS,
                "[{name}/{}] score mismatch: got {got}, exp {exp}",
                case.id
            );
        }
    }
}

#[test]
fn namespace_jsd_golden() {
    run_golden("namespace_jsd");
}

#[test]
fn call_scope_fraction_golden() {
    run_golden("call_scope_fraction");
}

#[test]
fn typical_call_density_golden() {
    run_golden("typical_call_density");
}

#[test]
fn except_return_raise_ratio_golden() {
    run_golden("except_return_raise_ratio");
}

#[test]
fn fall_through_guards_golden() {
    run_golden("fall_through_guards");
}
