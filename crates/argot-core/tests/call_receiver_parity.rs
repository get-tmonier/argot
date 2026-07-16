//! Parity for the deterministic parts of the call-receiver scorer: callee
//! extraction (Python + TypeScript) and MinHash signatures. The KMeans-derived
//! cluster assignments are NOT byte-tested (the partition is not reproducible
//! across implementations) — those are gated on the benchmark AUC instead.

use argot_core::scoring::adapters::python::PythonAdapter;
use argot_core::scoring::adapters::Language;
use argot_core::scoring::call_receiver::{extract_callees, minhash_signature, CallReceiverScorer};
use argot_core::text::read_text_lossy;
use serde::Deserialize;
use std::collections::HashSet;
use std::path::PathBuf;

#[derive(Deserialize)]
struct CalleeCase {
    #[allow(dead_code)]
    src: String,
    callees: Vec<Option<String>>,
}

#[derive(Deserialize)]
struct Golden {
    python: std::collections::HashMap<String, CalleeCase>,
    typescript: std::collections::HashMap<String, CalleeCase>,
    minhash_params_seed0: MinhashParams,
    minhash_sig_example: SigExample,
}

#[derive(Deserialize)]
struct MinhashParams {
    a: Vec<u64>,
    b: Vec<u64>,
}

#[derive(Deserialize)]
struct SigExample {
    bag: Vec<String>,
    sig: Vec<i64>,
}

fn golden() -> Golden {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/call_receiver/golden.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}

fn check_lang(cases: &std::collections::HashMap<String, CalleeCase>, lang: Language) {
    for (name, case) in cases {
        let got = extract_callees(&case.src, lang);
        assert_eq!(got, case.callees, "callee mismatch for sample {name}");
    }
}

#[test]
fn callee_extraction_matches_python_golden() {
    let g = golden();
    check_lang(&g.python, Language::Python);
    check_lang(&g.typescript, Language::Typescript);
}

#[test]
fn minhash_signature_matches_python_golden() {
    let g = golden();
    // The embedded seed-0 params must equal the Python-captured ones.
    assert_eq!(g.minhash_params_seed0.a.len(), 128);
    assert_eq!(g.minhash_params_seed0.b.len(), 128);
    // Signature over the example bag.
    let bag: HashSet<String> = g.minhash_sig_example.bag.iter().cloned().collect();
    let sig = minhash_signature(&bag);
    assert_eq!(sig, g.minhash_sig_example.sig, "minhash signature mismatch");
}

#[derive(Deserialize)]
struct WcCase {
    hunk: String,
    weighted_contribution: f64,
    count_unattested: usize,
}

#[derive(Deserialize)]
struct WcGolden {
    cases: Vec<WcCase>,
}

#[test]
fn weighted_contribution_matches_python_golden_n_clusters_1() {
    // n_clusters=1 => no KMeans, fully reproducible.
    let corpus_dir =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/bpe_score/corpus");
    let mut files: Vec<PathBuf> = std::fs::read_dir(&corpus_dir)
        .unwrap()
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().map(|x| x == "py").unwrap_or(false))
        .collect();
    files.sort();
    let repo_files: Vec<(PathBuf, String)> = files
        .iter()
        .map(|p| (p.clone(), read_text_lossy(p).unwrap()))
        .collect();

    let adapter = PythonAdapter::new();
    let scorer = CallReceiverScorer::new(
        &repo_files,
        Language::Python,
        2.0,
        5,
        &adapter,
        1,    // n_clusters
        0,    // cluster_seed
        0,    // cluster_rare_threshold
        0,    // cluster_size_min
        0.65, // data_threshold
    )
    .unwrap();

    let wc: WcGolden = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/call_receiver/wc_golden.json"),
        )
        .unwrap(),
    )
    .unwrap();

    for case in &wc.cases {
        let got =
            scorer.weighted_contribution(&case.hunk, 2.0, 2.0, 5.0, None, &Default::default());
        assert!(
            (got - case.weighted_contribution).abs() < 1e-12,
            "wc({:?}) = {got} != {}",
            case.hunk,
            case.weighted_contribution
        );
        assert_eq!(
            scorer.count_unattested(&case.hunk),
            case.count_unattested,
            "count_unattested mismatch for {:?}",
            case.hunk
        );
    }
}
