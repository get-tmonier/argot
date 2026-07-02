//! Temporary era-15 debug probe (deleted after use).
//!
//! Mode 1 (two args: break-file host-rel): score a planted hunk through the
//! exact from_model path check uses, with the parse-error host fallback on,
//! printing stage scores for the staged-diff-shaped fragment (leading `}` +
//! blank, like git hunks produce).
//!
//! Mode 2 (one arg: `caldist`): print the calibration score distribution
//! (500-hunk sample, seed 0) for moneta — quantiles the threshold mechanism
//! work needs.

use argot_core::scoring::adapters::typescript::TypeScriptAdapter;
use argot_core::scoring::calibration::{
    collect_candidates_with, multi_seed_thresholds, sample_indices, ThresholdRunConfig,
};
use argot_core::scoring::call_receiver::{CallReceiverScorer, RarityWeighting};
use argot_core::scoring::model::LanguageModel;
use argot_core::scoring::sequential::{SequentialConfig, SequentialImportBpeScorer};
use argot_core::scoring::typicality::TypicalityModel;
use argot_core::suppress::PathSuppressions;
use argot_core::text::read_text_lossy;
use std::path::{Path, PathBuf};

const MONETA: &str = "/Users/damienmeur/firstassethr/moneta";

fn scorer_from_artifact() -> SequentialImportBpeScorer {
    let moneta = Path::new(MONETA);
    let cfg_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(moneta.join(".argot/scorer-config.json")).unwrap(),
    )
    .unwrap();
    let lc = &cfg_json["languages"]["typescript"];
    let model: LanguageModel = serde_json::from_value(lc["model"].clone()).unwrap();
    let baseline = std::fs::read(moneta.join(".argot/generic-baseline.json")).unwrap();
    let cfg = SequentialConfig {
        bpe_threshold: lc["threshold"].as_f64().unwrap(),
        enable_typicality: true,
        exclude_data_dominant: true,
        call_receiver_alpha: 2.0,
        call_receiver_cap: 5,
        call_receiver_root_bonus: 2.0,
        call_receiver_n_clusters: 8,
        call_receiver_cluster_seed: 0,
        call_receiver_cluster_bonus: 5.0,
        call_receiver_cluster_rare_threshold: 0,
        call_receiver_cluster_size_min: 0,
        call_receiver_rarity_weighting: RarityWeighting::Off,
        call_receiver_shape_primitive_names: Vec::new(),
        call_receiver_parse_error_host_fallback: true,
        conventions: None,
        convention_bonus: 0.0,
        import_modules: lc["import_modules"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect(),
        import_module_prefixes: Vec::new(),
        evidence_corpus: None,
    };
    SequentialImportBpeScorer::from_model(
        &model,
        &baseline,
        Box::new(TypeScriptAdapter::new()),
        cfg,
    )
    .unwrap()
}

fn caldist() {
    let moneta = Path::new(MONETA);
    let adapter = TypeScriptAdapter::new();
    let suppressions = PathSuppressions::load(moneta);
    let candidates = collect_candidates_with(moneta, &adapter, &suppressions);
    eprintln!("{} candidates", candidates.len());

    // Rebuild the cal-side scorers exactly like run_calibrate.
    let corpus_txt = read_text_lossy(&moneta.join(".argot/repo-corpus.txt")).unwrap();
    let files: Vec<PathBuf> = corpus_txt
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(PathBuf::from)
        .filter(|p| {
            p.extension()
                .map(|e| e == "ts" || e == "tsx")
                .unwrap_or(false)
        })
        .collect();
    let repo_files: Vec<(PathBuf, String)> = files
        .iter()
        .filter_map(|p| read_text_lossy(p).ok().map(|s| (p.clone(), s)))
        .collect();
    let filtered: Vec<(PathBuf, String)> = repo_files
        .iter()
        .filter(|(_, s)| !adapter.is_data_dominant(s))
        .cloned()
        .collect();
    let sources: Vec<String> = filtered.iter().map(|(_, s)| s.clone()).collect();
    let bpe = argot_core::scoring::bpe_scorer::BpeScorer::new(
        argot_core::bpe::BpeTokenizer::load(),
        argot_core::train::GENERIC_BASELINE_JSON,
        &sources,
    )
    .unwrap();
    let mut cal_cr = CallReceiverScorer::new(
        &filtered,
        argot_core::scoring::adapters::Language::Typescript,
        2.0,
        5,
        &adapter,
        8,
        0,
        0,
        0,
    )
    .unwrap();
    let typicality = TypicalityModel::new(argot_core::scoring::adapters::Language::Typescript);

    // One seed, full per-hunk scores (mirror of multi_seed_thresholds body).
    let idx = sample_indices(candidates.len(), 500.min(candidates.len()), 0);
    let mut scores: Vec<f64> = Vec::new();
    for &i in &idx {
        let c = &candidates[i];
        if typicality.is_atypical(&c.hunk).0 {
            continue;
        }
        let prose = adapter.prose_line_ranges(&c.hunk);
        let mut blanked = String::new();
        {
            // blank_prose_lines is private; approximate by skipping — the
            // distribution shape is what matters here, use raw hunk.
            let _ = &prose;
            blanked.push_str(&c.hunk);
        }
        let raw_bpe = bpe.bpe_score(&blanked);
        let contrib = cal_cr.weighted_contribution_for_file(
            &c.hunk,
            Some(&c.file_path),
            0.0,
            0.0,
            5.0,
            5.0,
            Some(&c.file_source),
            Some((&c.file_source, c.hunk_start_line, c.hunk_end_line)),
            &Default::default(),
        );
        scores.push(raw_bpe + contrib);
    }
    scores.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |p: f64| scores[(p * (scores.len() - 1) as f64) as usize];
    println!("n={} min={:.2}", scores.len(), scores[0]);
    for p in [0.5, 0.75, 0.9, 0.95, 0.98, 0.99, 1.0] {
        println!("P{:>4.1} = {:.3}", p * 100.0, q(p));
    }
    // multi-seed thresholds for reference
    let seeds = multi_seed_thresholds(
        &candidates,
        &bpe,
        &mut cal_cr,
        &adapter,
        &typicality,
        &ThresholdRunConfig {
            n_cal: 500.min(candidates.len()),
            base_seed: 0,
            n_seeds: 7,
            cluster_bonus: 5.0,
            cap: 5.0,
        },
    );
    println!("seed maxes: {seeds:?}");
}

/// Sustained-surprise scout: for the calibration population and each gauntlet
/// break, print max surprise, mean of the top-k surprises, and the fraction
/// of meaningful tokens above a mild-surprise bar. Question: does *sustained*
/// mild alienness separate where the max saturates?
fn sustained(break_dir: &str) {
    let moneta = Path::new(MONETA);
    let adapter = TypeScriptAdapter::new();
    let suppressions = PathSuppressions::load(moneta);
    let candidates = collect_candidates_with(moneta, &adapter, &suppressions);
    let cfg_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(moneta.join(".argot/scorer-config.json")).unwrap(),
    )
    .unwrap();
    let model: LanguageModel =
        serde_json::from_value(cfg_json["languages"]["typescript"]["model"].clone()).unwrap();
    let baseline = std::fs::read(moneta.join(".argot/generic-baseline.json")).unwrap();
    let bpe = argot_core::scoring::bpe_scorer::BpeScorer::from_stats(
        argot_core::bpe::BpeTokenizer::load(),
        &baseline,
        &model.bpe,
    )
    .unwrap();
    let typicality = TypicalityModel::new(argot_core::scoring::adapters::Language::Typescript);

    let features = |hunk: &str| -> (f64, f64, f64, usize) {
        let ids = bpe.tokenizer().encode(hunk);
        let mut surprises: Vec<f64> = ids
            .iter()
            .filter(|&&i| bpe.is_meaningful_token_id(i))
            .map(|&i| bpe.token_surprise(i))
            .collect();
        surprises.sort_by(|a, b| b.partial_cmp(a).unwrap());
        let n = surprises.len();
        if n == 0 {
            return (0.0, 0.0, 0.0, 0);
        }
        let k = 10.min(n);
        let topk_mean = surprises[..k].iter().sum::<f64>() / k as f64;
        let frac_mild = surprises.iter().filter(|&&s| s > 1.5).count() as f64 / n as f64;
        (surprises[0], topk_mean, frac_mild, n)
    };

    let idx = sample_indices(candidates.len(), 500.min(candidates.len()), 0);
    let mut maxes = Vec::new();
    let mut topks = Vec::new();
    let mut fracs = Vec::new();
    for &i in &idx {
        let c = &candidates[i];
        if typicality.is_atypical(&c.hunk).0 {
            continue;
        }
        let (mx, tk, fr, _) = features(&c.hunk);
        maxes.push(mx);
        topks.push(tk);
        fracs.push(fr);
    }
    for v in [&mut maxes, &mut topks, &mut fracs] {
        v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    }
    let q = |v: &Vec<f64>, p: f64| v[(p * (v.len() - 1) as f64) as usize];
    println!("cal population (n={}):", maxes.len());
    println!("  {:>8} {:>8} {:>8}", "max", "top10", "frac>1.5");
    for p in [0.5, 0.9, 0.95, 0.99, 1.0] {
        println!(
            "  P{:<4.0} {:>7.3} {:>8.3} {:>8.3}",
            p * 100.0,
            q(&maxes, p),
            q(&topks, p),
            q(&fracs, p)
        );
    }
    println!("\nbreaks:");
    let mut entries: Vec<_> = std::fs::read_dir(break_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        let hunk = std::fs::read_to_string(&path).unwrap();
        let (mx, tk, fr, n) = features(&hunk);
        println!(
            "  {:32} max={:>6.3} top10={:>6.3} frac>1.5={:>5.3} n={}",
            path.file_name().unwrap().to_string_lossy(),
            mx,
            tk,
            fr,
            n
        );
    }
}

/// Convention-rarity scout: corpus-derived AST node-kind frequencies and
/// identifier-shape frequencies; per hunk, the max surprisal of a present
/// convention. Question: do syntax conventions (var, class lifecycle) and
/// naming morphology (snake_case in a camelCase corpus) separate where token
/// surprise cannot?
fn conventions(break_dir: &str) {
    use std::collections::HashMap;
    let moneta = Path::new(MONETA);
    let adapter = TypeScriptAdapter::new();
    let suppressions = PathSuppressions::load(moneta);
    let candidates = collect_candidates_with(moneta, &adapter, &suppressions);
    let typicality = TypicalityModel::new(argot_core::scoring::adapters::Language::Typescript);

    // Corpus sources (non-data-dominant), like the scorers use.
    let corpus_txt = read_text_lossy(&moneta.join(".argot/repo-corpus.txt")).unwrap();
    let sources: Vec<String> = corpus_txt
        .lines()
        .filter(|l| l.ends_with(".ts") || l.ends_with(".tsx"))
        .filter_map(|p| read_text_lossy(Path::new(p)).ok())
        .filter(|s| !adapter.is_data_dominant(s))
        .collect();
    eprintln!("{} corpus sources", sources.len());

    fn node_kind_counts(src: &str, counts: &mut HashMap<String, usize>) -> usize {
        let mut parser = tree_sitter::Parser::new();
        parser
            .set_language(&tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into())
            .unwrap();
        let Some(tree) = parser.parse(src.as_bytes(), None) else {
            return 0;
        };
        let mut stack = vec![tree.root_node()];
        let mut total = 0;
        while let Some(node) = stack.pop() {
            if node.is_named() {
                *counts.entry(node.kind().to_string()).or_insert(0) += 1;
                total += 1;
            }
            for i in (0..node.child_count()).rev() {
                if let Some(c) = node.child(i) {
                    stack.push(c);
                }
            }
        }
        total
    }

    fn ident_shape(ident: &str) -> &'static str {
        let has_under = ident.contains('_');
        let has_upper = ident.chars().any(|c| c.is_ascii_uppercase());
        let has_lower = ident.chars().any(|c| c.is_ascii_lowercase());
        let starts_upper = ident
            .chars()
            .next()
            .map(|c| c.is_ascii_uppercase())
            .unwrap_or(false);
        match (has_under, has_upper, has_lower, starts_upper) {
            (true, _, true, _) => "snake",
            (true, true, false, _) => "scream",
            (false, true, true, true) => "pascal",
            (false, true, true, false) => "camel",
            (false, false, true, _) => "flat",
            _ => "other",
        }
    }

    fn ident_shape_counts(src: &str, counts: &mut HashMap<&'static str, usize>) -> usize {
        let mut total = 0;
        let bytes = src.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            let c = bytes[i];
            if c == b'_' || c.is_ascii_alphabetic() {
                let start = i;
                i += 1;
                while i < bytes.len() && (bytes[i] == b'_' || bytes[i].is_ascii_alphanumeric()) {
                    i += 1;
                }
                let ident = &src[start..i];
                if ident.len() >= 3 {
                    *counts.entry(ident_shape(ident)).or_insert(0) += 1;
                    total += 1;
                }
            } else {
                i += 1;
            }
        }
        total
    }

    let mut corpus_kinds: HashMap<String, usize> = HashMap::new();
    let mut corpus_nodes = 0usize;
    let mut corpus_shapes: HashMap<&'static str, usize> = HashMap::new();
    let mut corpus_idents = 0usize;
    for s in &sources {
        corpus_nodes += node_kind_counts(s, &mut corpus_kinds);
        corpus_idents += ident_shape_counts(s, &mut corpus_shapes);
    }
    eprintln!("corpus: {corpus_nodes} named nodes, {corpus_idents} identifiers");
    eprintln!("shape mix: {corpus_shapes:?}");

    let kind_surprisal = |k: &str, count: usize| -> f64 {
        let c = corpus_kinds.get(k).copied().unwrap_or(0) as f64;
        let rate = (c + 1.0) / (corpus_nodes as f64 + 1.0);
        -(rate.ln()) * (count.min(3) as f64 / 3.0)
    };
    let scores = |hunk: &str| -> (f64, f64) {
        let mut kinds: HashMap<String, usize> = HashMap::new();
        node_kind_counts(hunk, &mut kinds);
        let syntax = kinds
            .iter()
            .map(|(k, &n)| kind_surprisal(k, n))
            .fold(0.0f64, f64::max);
        let mut shapes: HashMap<&'static str, usize> = HashMap::new();
        let total = ident_shape_counts(hunk, &mut shapes);
        let mut ident = 0.0f64;
        if total >= 5 {
            for (class, &n) in &shapes {
                let frac = n as f64 / total as f64;
                if n >= 3 && frac >= 0.3 {
                    let cf = (corpus_shapes.get(class).copied().unwrap_or(0) as f64 + 1.0)
                        / (corpus_idents as f64 + 1.0);
                    ident = ident.max(-(cf.ln()) * frac);
                }
            }
        }
        (syntax, ident)
    };

    let idx = sample_indices(candidates.len(), 500.min(candidates.len()), 0);
    let mut syn: Vec<f64> = Vec::new();
    let mut idn: Vec<f64> = Vec::new();
    for &i in &idx {
        let c = &candidates[i];
        if typicality.is_atypical(&c.hunk).0 {
            continue;
        }
        let (s, d) = scores(&c.hunk);
        syn.push(s);
        idn.push(d);
    }
    syn.sort_by(|a, b| a.partial_cmp(b).unwrap());
    idn.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let q = |v: &Vec<f64>, p: f64| v[(p * (v.len() - 1) as f64) as usize];
    println!("cal population (n={}):", syn.len());
    println!("  {:>8} {:>8}", "syntax", "ident");
    for p in [0.5, 0.9, 0.95, 0.99, 1.0] {
        println!(
            "  P{:<4.0} {:>7.3} {:>8.3}",
            p * 100.0,
            q(&syn, p),
            q(&idn, p)
        );
    }
    println!("\nbreaks:");
    let mut entries: Vec<_> = std::fs::read_dir(break_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .collect();
    entries.sort();
    for path in entries {
        let hunk = std::fs::read_to_string(&path).unwrap();
        let (s, d) = scores(&hunk);
        println!(
            "  {:32} syntax={:>7.3} ident={:>7.3}",
            path.file_name().unwrap().to_string_lossy(),
            s,
            d
        );
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() == 2 && args[1] == "caldist" {
        caldist();
        return;
    }
    if args.len() == 3 && args[1] == "sustained" {
        sustained(&args[2]);
        return;
    }
    if args.len() == 3 && args[1] == "conventions" {
        conventions(&args[2]);
        return;
    }
    let brk = std::fs::read_to_string(&args[1]).unwrap();
    let host_rel = args[2].clone();
    let orig = std::fs::read_to_string(Path::new(MONETA).join(&host_rel)).unwrap();
    let mut planted = orig.clone();
    if !planted.ends_with('\n') {
        planted.push('\n');
    }
    planted.push('\n');
    planted.push_str(&brk);
    let total = planted.lines().count();
    let brk_lines = brk.trim_end().lines().count();
    let hs = total - brk_lines + 1;

    // The staged-diff-shaped fragment: git hunks pull in the neighbouring
    // construct tail, so simulate `}` + blank + break.
    let frag = format!("}}\n\n{brk}");
    let frag_hs = hs - 2;

    let mut scorer = scorer_from_artifact();
    for (label, hunk, s, e) in [
        ("clean fragment", brk.as_str(), hs, total),
        ("diff-shaped", frag.as_str(), frag_hs, total),
    ] {
        let scored = scorer.score_hunk(
            hunk,
            Some(planted.as_str()),
            Some(s),
            Some(e),
            Some(Path::new(host_rel.as_str())),
        );
        println!(
            "{label:15} flagged={} reason={} bpe={:.2} contrib={:.2}",
            scored.flagged,
            scored.reason.as_str(),
            scored.stages.bpe_score,
            scored.stages.call_receiver_contribution
        );
    }
}
