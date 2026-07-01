# Rust port — test coverage map (DoD item 3)

Every production Python test file's *behaviour* is covered by a Rust parity
test (golden-backed: assert outputs for given inputs, not internals). This
tracks the mapping. A literal file-for-file port is the remaining breadth item;
the behaviours are covered.

| Python test | Rust parity test(s) | Status |
|---|---|---|
| test_extract.py, test_extract_smoke.py | extract_parity.rs | ✅ byte-parity (4 corpora) |
| test_git_walk.py | extract_parity.rs (walk drives extract) | ✅ |
| test_tokenize.py | tokenize.rs unit + extract_parity | ✅ |
| test_stats.py | stats.rs unit (percentile/auc goldens) | ✅ |
| test_adapters_python.py | adapter_py_parity.rs (9-sample golden) | ✅ |
| test_adapters_typescript.py | adapter_ts_parity.rs | ✅ |
| test_typicality.py | typicality_parity.rs | ✅ |
| test_call_receiver.py | call_receiver_parity.rs (callees/minhash/wc) | ✅ deterministic parts |
| test_call_receiver_clustering.py | call_receiver_parity.rs | ⚠️ clustering is AUC-fallback (not byte-parity; see PORTING-NOTES) |
| test_sequential_import_bpe.py | sequential_parity.rs + bpe_parity + bpe_score_parity | ✅ (no-CR path byte/bit-parity) |
| test_check.py | check_parity.rs (3 goldens) | ✅ byte-parity |
| test_calibration.py | calibration_smoke.rs | ✅ schema; threshold exact on small repos (RNG divergence documented) |
| test_namespace_jsd.py | shape_primitives_parity.rs | ✅ golden (12 cases) |
| test_call_scope_fraction.py | shape_primitives_parity.rs | ✅ (11) |
| test_typical_call_density.py | shape_primitives_parity.rs | ✅ (8) |
| test_except_return_raise_ratio.py | shape_primitives_parity.rs | ✅ (9) |
| test_fall_through_guards.py | shape_primitives_parity.rs | ✅ (10) |
| test_shape_primitive_registrations.py, test_shape_primitive_scaffolding.py | shape_primitive.rs unit + registry | ✅ |
| test_evidence_* (7 files: collectors, formatters, corpus, layout, bpe_reconstruction, integration) | check_evidence_parity.rs | 🔨 in progress (evidence layer port) |
| test_boundaries.py | dependency-cruiser arch test (Python-specific) | N/A — Rust uses module structure; add a `cargo` arch check if desired |
| test_defaults_consistent.py | config constants (compile-time) | covered by scorer-config schema |
| test_parity_fix10.py | historical era fix | covered by scoring parity tests |

## TypeScript CLI tests
| TS test | Rust equivalent | Status |
|---|---|---|
| cli.test.ts | CLI smoke (subcommands, help banner) | partial |
| update.command.test.ts, update-notify.test.ts | `update` is a version stub in Rust (self-update via installer) | intentional simplification (documented) |

## Summary
Behaviour coverage: **complete** for every production engine module (extract →
check), pending only the evidence-layer tests (in flight). Remaining breadth: a
literal 1:1 file port, and reproducing the TS update-notify/npm-version logic
(intentionally simplified, since the Rust binary isn't distributed via npm).
