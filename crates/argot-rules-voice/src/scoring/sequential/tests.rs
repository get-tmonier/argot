use super::*;
use crate::scoring::adapters::python::PythonAdapter;
use crate::train::GENERIC_BASELINE_JSON;

fn config(threshold: f64) -> SequentialConfig {
    SequentialConfig {
        bpe_threshold: threshold,
        enable_typicality: true,
        exclude_data_dominant: true,
        call_receiver_alpha: 2.0,
        call_receiver_cap: 5,
        call_receiver_root_bonus: 2.0,
        call_receiver_n_clusters: 2,
        call_receiver_cluster_seed: 0,
        call_receiver_cluster_bonus: 5.0,
        call_receiver_cluster_rare_threshold: 0,
        call_receiver_cluster_size_min: 0,
        call_receiver_rarity_weighting: RarityWeighting::Off,
        call_receiver_shape_primitive_names: Vec::new(),
        call_receiver_parse_error_host_fallback: false,
        conventions: None,
        convention_bonus: 0.0,
        import_modules: vec!["math".to_string()],
        check_only_import_modules: Vec::new(),
        check_only_patterns: Vec::new(),
        import_module_prefixes: Vec::new(),
        evidence_corpus: None,
        detect: argot_engine::config::DetectConfig::default(),
    }
}

fn toy_scorer() -> SequentialImportBpeScorer {
    let files: Vec<(PathBuf, String)> = (0..4)
        .map(|i| {
            (
                PathBuf::from(format!("m{i}.py")),
                "import math\n\n\ndef mean(xs):\n    total = math.fsum(xs)\n    return total / len(xs)\n".to_string(),
            )
        })
        .collect();
    SequentialImportBpeScorer::from_config(
        &files,
        GENERIC_BASELINE_JSON,
        Box::new(PythonAdapter::new()),
        config(100.0),
    )
    .unwrap()
}

/// The row-granular data gate: a hunk sitting inside the host file's
/// data-literal span is skipped, while a *code* hunk in the very same
/// data-dominant file is still scored (the old file-level vetoes zeroed
/// both).
#[test]
fn data_rows_are_skipped_but_code_rows_in_data_files_are_scored() {
    let mut scorer = toy_scorer();
    // A data-dominant host: one long table plus one function.
    let mut file = String::from("NAMES = [\n");
    for i in 0..40 {
        file.push_str(&format!("    \"entry_{i}\",\n"));
    }
    file.push_str("]\n\n\ndef helper(xs):\n    acc = process_batch(xs)\n    return acc\n");
    let file_lines: Vec<&str> = file.lines().collect();

    // Two rows inside the table — small enough to evade the hunk-level
    // typicality gate, previously only caught by the file-level fallback.
    let data_hunk = file_lines[4..6].join("\n");
    let scored = scorer.score_hunk(&data_hunk, Some(&file), Some(5), Some(6), None);
    assert_eq!(scored.reason, Reason::AtypicalFile, "data rows skipped");
    assert_eq!(scored.score, 0.0);

    // The function at the bottom of the same file: judged, not vetoed.
    let n = file_lines.len();
    let code_hunk = file_lines[n - 3..n].join("\n");
    let scored = scorer.score_hunk(&code_hunk, Some(&file), Some(n - 2), Some(n), None);
    assert!(
        !matches!(scored.reason, Reason::Atypical | Reason::AtypicalFile),
        "code hunk in a data file must be judged, got {:?}",
        scored.reason
    );
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "unattested callee in the planted code fires: {:?}",
        scored.stages
    );
}

/// Hunks without file context (bare fragments) are unaffected by the gate.
#[test]
fn bare_hunks_skip_the_data_row_gate() {
    let mut scorer = toy_scorer();
    let scored = scorer.score_hunk("def f(x):\n    return x + 1", None, None, None, None);
    assert!(!matches!(
        scored.reason,
        Reason::Atypical | Reason::AtypicalFile
    ));
}

/// B1: an `import` that lives inside a docstring is not real code, so it must
/// not fire `foreign-import` even when the hunk (a range/commit-path fragment)
/// slices the docstring interior. A real top-level foreign import still fires.
#[test]
fn import_inside_a_docstring_is_not_flagged_foreign() {
    let mut scorer = toy_scorer();
    let file = "import math\n\n\ndef run(xs):\n    \"\"\"Do the thing.\n\n    Example::\n\n        import gevent\n        gevent.spawn(run)\n    \"\"\"\n    return math.fsum(xs)\n";
    let lines: Vec<&str> = file.lines().collect();
    // A hunk covering the docstring body (lines 5..=11), including `import gevent`.
    let hunk = lines[4..11].join("\n");
    let scored = scorer.score_hunk(&hunk, Some(file), Some(5), Some(11), None);
    assert_ne!(
        scored.reason,
        Reason::Import,
        "a docstring-embedded import must not fire foreign-import: {scored:?}"
    );

    // Control: a genuinely introduced top-level foreign import still fires.
    let real = scorer.score_hunk("import gevent\ngevent.spawn(f)", None, None, None, None);
    assert_eq!(
        real.reason,
        Reason::Import,
        "a real foreign import must still fire: {real:?}"
    );
}

/// Local-binding attestation: callees the change itself defines (in the
/// host file or the changeset) or imports from repo-internal paths do not
/// count as unattested — only truly neighbourhood-less callees fire.
#[test]
fn locally_bound_callees_are_not_foreign() {
    let mut scorer = toy_scorer();

    // Self-defined: the hunk defines process_batch and calls it.
    let hunk =
        "def process_batch(xs):\n    return xs\n\n\ndef run(xs):\n    return process_batch(xs)";
    let scored = scorer.score_hunk(hunk, None, None, None, None);
    assert_eq!(
        scored.stages.call_receiver_contribution, 0.0,
        "self-defined callee attested: {:?}",
        scored.stages
    );

    // Changeset-defined: another file in the same change defines it.
    let hunk = "def run(xs):\n    return shared_new_helper(xs)";
    let scored = scorer.score_hunk(hunk, None, None, None, None);
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "unknown callee fires before the changeset binds it"
    );
    scorer.set_changeset_bindings(HashSet::from(["shared_new_helper".to_string()]));
    let scored = scorer.score_hunk(hunk, None, None, None, None);
    assert_eq!(
        scored.stages.call_receiver_contribution, 0.0,
        "changeset-defined callee attested"
    );
    scorer.set_changeset_bindings(HashSet::new());

    // Relative-imported: the host file imports it from a repo-internal
    // path (python relative import).
    let file = "from .helpers import shared_new_helper\n\n\ndef run(xs):\n    return shared_new_helper(xs)\n";
    let hunk = "def run(xs):\n    return shared_new_helper(xs)";
    let scored = scorer.score_hunk(hunk, Some(file), Some(4), Some(5), None);
    assert_eq!(
        scored.stages.call_receiver_contribution, 0.0,
        "relative-imported callee attested: {:?}",
        scored.stages
    );

    // A truly foreign callee keeps firing under all of the above.
    let hunk = "def run(xs):\n    return totally_alien_call(xs)";
    let scored = scorer.score_hunk(hunk, Some(file), Some(4), Some(5), None);
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "neighbourhood-less callee still fires"
    );
}

/// Foreign-context gate: the call-receiver reason may only *flag* a hunk
/// when the hunk reaches into a foreign module. A bare unattested callee
/// (the codebase's own new function) contributes but must not cry wolf; a
/// namespace-qualified callee into an unknown module (or a foreign import)
/// does flag.
#[test]
fn call_receiver_flags_only_with_foreign_context() {
    // Threshold above both hunks' BPE surprisal, contribution (uncapped
    // here) large enough to cross it on its own: isolates the firing
    // decision to the gate.
    let mut cfg = config(12.0);
    cfg.call_receiver_alpha = 25.0;
    cfg.call_receiver_cap = 25;
    cfg.call_receiver_root_bonus = 0.0;
    let files: Vec<(PathBuf, String)> = (0..4)
        .map(|i| {
            (
                PathBuf::from(format!("m{i}.py")),
                "import math\n\n\ndef mean(xs):\n    total = math.fsum(xs)\n    return total / len(xs)\n".to_string(),
            )
        })
        .collect();
    let mut scorer = SequentialImportBpeScorer::from_config(
        &files,
        GENERIC_BASELINE_JSON,
        Box::new(PythonAdapter::new()),
        cfg,
    )
    .unwrap();

    // New method on a KNOWN module (`math` is attested via `math.fsum`):
    // the repo's own new code reaching into a module it already uses —
    // contributes, but the file reaches no foreign module, so it does not
    // flag under the call-receiver reason.
    let known = "def run(xs):\n    return math.newhelper(xs)";
    let scored = scorer.score_hunk(known, None, None, None, None);
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "new method on a known module still contributes"
    );
    assert!(
        !scored.flagged,
        "new API on a known module (no foreign reach) must not flag: {scored:?}"
    );

    // Namespace-qualified callee into a module the repo never uses: foreign
    // voice — flags under the call-receiver reason.
    let foreign = "def run(xs):\n    return foreignlib.connect(xs)";
    let scored = scorer.score_hunk(foreign, None, None, None, None);
    assert_eq!(
        scored.reason,
        Reason::CallReceiver,
        "namespace-foreign callee flags: {scored:?}"
    );
}

/// Amplification guard: a benign refactor hunk whose callees are all the
/// repo's own attested code must NOT flag just because its *file* pulls a
/// foreign import elsewhere. Before the fix, one file-level foreign
/// dependency opened the call-receiver gate for every hunk in the file
/// (ink's `terminal-size` import lit up `performance.now()`/`clearTimeout`).
#[test]
fn file_level_foreign_import_does_not_amplify_benign_hunk() {
    let mut cfg = config(12.0);
    cfg.call_receiver_alpha = 25.0;
    cfg.call_receiver_cap = 25;
    cfg.call_receiver_root_bonus = 0.0;
    let files: Vec<(PathBuf, String)> = (0..4)
        .map(|i| {
            (
                PathBuf::from(format!("m{i}.py")),
                "import math\n\n\ndef mean(xs):\n    total = math.fsum(xs)\n    return total / len(xs)\n".to_string(),
            )
        })
        .collect();
    let mut scorer = SequentialImportBpeScorer::from_config(
        &files,
        GENERIC_BASELINE_JSON,
        Box::new(PythonAdapter::new()),
        cfg,
    )
    .unwrap();

    // The hunk's only reach is `math.newhelper` (a new method on an
    // attested module — the repo's own code). Its *file* adds a foreign
    // import (`requests`), but that must not open the gate for this hunk.
    let file_src =
        "import math\nimport requests\n\n\ndef run(xs):\n    return math.newhelper(xs)\n";
    let hunk = "def run(xs):\n    return math.newhelper(xs)";
    let scored = scorer.score_hunk(hunk, Some(file_src), None, None, None);
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "attested-module method still contributes"
    );
    assert!(
        !scored.flagged,
        "benign hunk must not flag on a file-level foreign import: {scored:?}"
    );
}

/// A hunk that both imports a foreign module AND calls the repo's own
/// attested methods must resolve to the **Import** reason — not an
/// import-gated call-receiver. Before the fix, the foreign import opened the
/// call-receiver gate, an import-gated call-receiver (riding on the attested
/// callee's contribution) won the ratio tiebreak, and — resolving to empty
/// callee evidence at check time — the whole hit was dropped, losing the
/// valid foreign-import flag (a Django view that imports `django` AND calls
/// `self.repo.find` read clean).
#[test]
fn foreign_import_wins_over_import_gated_call_receiver() {
    let mut cfg = config(12.0);
    cfg.call_receiver_alpha = 25.0;
    cfg.call_receiver_cap = 25;
    cfg.call_receiver_root_bonus = 0.0;
    let files: Vec<(PathBuf, String)> = (0..4)
        .map(|i| {
            (
                PathBuf::from(format!("m{i}.py")),
                "import math\n\n\ndef mean(xs):\n    total = math.fsum(xs)\n    return total / len(xs)\n".to_string(),
            )
        })
        .collect();
    let mut scorer = SequentialImportBpeScorer::from_config(
        &files,
        GENERIC_BASELINE_JSON,
        Box::new(PythonAdapter::new()),
        cfg,
    )
    .unwrap();

    // Foreign import (`foreignlib`, 0-usage) + a new method on an *attested*
    // module (`math.newhelper` → a call-receiver contribution that reaches no
    // foreign module). The import fires; the import-gated call-receiver must
    // not steal the reason.
    let hunk = "import foreignlib\n\n\ndef run(xs):\n    return math.newhelper(xs)";
    let scored = scorer.score_hunk(hunk, None, None, None, None);
    assert!(
        scored.stages.call_receiver_contribution > 0.0,
        "the attested-module method still contributes: {scored:?}"
    );
    assert!(scored.flagged, "a foreign import must flag: {scored:?}");
    assert_eq!(
        scored.reason,
        Reason::Import,
        "the foreign import — not an import-gated call-receiver — carries the hit: {scored:?}"
    );
}
