# Issue #92 — call_receiver amplification fix (file-level → binding-scoped)

## Problem (hit-by-hit diagnosis)

Reconstructed every existing-file holdout FP on the three corpora above the 2%
line by pulling the flagged span from the git blob at the hit commit:

- **ink 8.7%**: 29/33 hits `call_receiver`, firing on `performance.now()`,
  `clearTimeout()`, `useCallback`, `this.exitPromise.catch(noop)` — built-ins
  and attested methods, on hunks up to 93 lines. Not "genuine new deps."
- **bat 7.4%**: ~21 `call_receiver` on `.canonicalize()`, `.unwrap()`,
  `use crate::…`, hits landing on blank/`///` lines. Plus genuine new-dep
  imports (`gix`, `itertools`, `encoding_rs`) and one `#[test]` bpe fire.
- **rocksdb**: 81 `call_receiver`, same shape (C++ std/attested methods).

Root cause: the call-receiver fire gate had a **file-level** term —
`file_has_foreign_import(file_source)` — so *one* foreign `#include`/import
anywhere in a file opened the gate for **every** hunk in it. A benign refactor
whose callees are all the repo's own attested code was flagged because its file
imported something foreign elsewhere (ink pulling `terminal-size`/`wrap-ansi`).

## Fix

Two moves in `sequential.rs` cr_fired gate:

1. **Remove** the file-level `file_has_foreign_import` amplifier.
2. **Restore** the load-bearing catches with a binding-scoped term:
   `hunk_uses_foreign_import_binding` — a file-level foreign import opens the
   gate only for a hunk that actually *uses a name that import bound*
   (colorama's `Fore`/`Style`, numpy's `np`/`default_rng`, `httpx`). New adapter
   method `import_bindings(source) -> [(binding, module)]` (Python impl;
   default-empty elsewhere, so a language without it falls back to hunk-local
   reach). Foreignness comes from the import scorer.

The binding term is **strictly less permissive** than the removed file-level
term (`file-has-foreign-import ∧ hunk-uses-its-binding ⊂ file-has-foreign-import`),
so no corpus can exceed its prior FP; the over-fire corpora (TS/Rust/C++, where
`import_bindings` is empty) get the full drop.

## Why the first cut (remove-only) was wrong

Removing the file-level term alone regressed **4 broad-foreign catches** — all
Python, all where the foreign import sits in a diff hunk separate from the
usage: faker `numpy_random_1`/`requests_source_2`, rich `colorama_1`, dagster
`luigi framework_swap`. Their usage hunks reference foreign-import bindings
(`default_rng`, `httpx`, `Fore`) but contain no foreign *callee* the hunk-local
reach could see. The binding term restores exactly these.

## Results

Catch (production path, all catalogued corpora):

| Metric | Baseline (16018c1f) | Remove-only | Binding-scoped (this fix) |
|---|---|---|---|
| Gated (RUBRIC foreign_import/api/concurrency) | 48/49 | 48/49 | **48/49** ✓ |
| Broad foreign (incl. legacy foreign_* classes) | 104/122 | 100/122 (−4) | **105/122** (+1) |

Remove-only regressed 4 (faker `numpy_random_1`/`requests_source_2`, rich
`colorama_1`, dagster luigi). The binding-scoped restore recovers all 4 **and**
gains faker +1 (aliased `import numpy as np` now surfaces `np` as a foreign
binding, which the old extract_imports missed). Zero regressions vs baseline.

Existing-file FP (temporal holdout), by winning reason:

| Corpus | Baseline existing | call_receiver | This fix existing | call_receiver | Note |
|---|---|---|---|---|---|
| **fastapi** | 2.21% | 18 | **1.28%** ✓ | **2** | amplifier was the dominant cause — fixed |
| ink | 8.73% | 29 | 6.08% | 19 | 10 dropped; **19 remain via `hunk_foreign_reach`** |
| bat | 7.40% | 20 | 5.62% | 14 | 6 dropped; 14 remain via `hunk_foreign_reach` |
| rocksdb | 1.52% | 22 | 1.48% | 21 | already ≤2%; residual is namespace-reach |

No FP regression on faker/rich/dagster (binding-term-active Python corpora), as
expected (binding term ⊂ old file-level term).

### Corrected diagnosis

The file-level amplifier was the **dominant** existing-FP driver only on
**fastapi** (internal helpers `get_authorization_scheme_param`/`jsonable_encoder`
lit up because the file imported the new `annotated_doc` dep elsewhere). On
ink/bat/rocksdb the residual call_receiver FPs fire via `hunk_foreign_reach`
itself: `is_namespace_foreign` treats a **single-dot receiver form** whose
receiver the repo never attested as a namespace (`performance.now`,
`options.stdout.off`, C++ `x.method` on a fresh receiver) as foreign. That is a
**separate** over-fire, tracked next — not fixed by the amplifier removal.

fastapi's remaining 22 existing FP = **14 import** (genuinely-new deps
`annotated_doc`/`pwdlib`/`typing_inspection`, counted once per file) + 6 bpe +
2 call_receiver. The 14 are correct novel-pattern detections the temporal-holdout
mislabels as false alarms (see the metric-framing note).

## Tests

- `sequential::tests::file_level_foreign_import_does_not_amplify_benign_hunk` —
  a benign `math.newhelper` hunk in a file importing foreign `requests` stays
  quiet.
- `python::tests::import_bindings_pairs_bound_names_with_modules` — aliased,
  dotted, `from … import a, b`, and relative-skip cases.
