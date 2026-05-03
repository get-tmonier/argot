# Era 13 — Day 1 Audit Memo

Combined output of Phase 1 (Phase 10 plumbing audit) and Phase 3 (bench
symmetry audit). Both ran in parallel as cmux executors on the `era13`
team. Synthesis below; verbatim per-executor reports follow.

## Synthesis

### Headline

- **Phase 1 — plumbing-bug-found and fixed.** The era-12 discrepancy
  ("standalone probe shows threshold +5.0, full bench shows +0.0") is
  fully explained by a missing argument in the bench's subprocess
  fan-out, not by `K=7` median masking and not by cal hunks lacking
  rare-attested callees. With the fix, the Phase 2 sweep grid actually
  exercises the configurations it claims to.
- **Phase 3 — audit-clean.** Four path comparisons walked. Zero
  undocumented asymmetries. Zero MANDATORY-FIX items. One latent
  defensive note (silent `OSError` fallback to catalog-phantom path)
  filed for follow-up but not blocking.
- **Phase 2 substrate is sound.** No orchestrator-applied fixes
  required before Phase 2 starts. Phase 1's fix is the only behavior
  change, and it only matters when `cluster_rare_threshold > 0` is
  passed (i.e. inside the Phase 2 sweep) — the era-11 production
  shipping config (`cluster_rare_threshold=0`) is bit-identical
  before/after.

### Phase 1 root cause

`benchmarks/src/argot_bench/cli.py:_run` builds `base_cmd` for spawning
`run-one` workers with one `if arg != default: base_cmd.extend(...)`
guard per propagated parameter. The guard for
`call_receiver_cluster_rare_threshold` was missing entirely. Compounding
the gap, the `run-one` subparser didn't define
`--call-receiver-cluster-rare-threshold` either, so even direct
`argot-bench run-one ...` invocations couldn't override the default.

Result: every corpus worker booted with `cluster_rare_threshold=0`
regardless of what the parent invocation specified. The era-12 standalone
probe didn't go through this fan-out (it instantiated the scorer
directly), so it saw the rule fire correctly. The era-12 full bench did
go through the fan-out, dropped the parameter, and saw zero change.

The trace from `RunConfig` down through
`build_scorer → calibrate_multi_seed → SequentialImportBpeScorer →
CallReceiverScorer.weighted_contribution_for_file` is otherwise fully
correct (see Phase 1 trace table below). The bug is exactly one
boundary: `cli.py` argv-builder for spawned subprocesses.

### G3.c implications

- **Production scoring unchanged.** Era-11 ships with
  `cluster_rare_threshold=0`. The fix is a no-op for the rule's `> 0`
  guard at `call_receiver.py:467`, so the 105 currently-caught fixtures
  re-score identically. No G3.c re-score required from this fix alone.
- **Phase 2 sweep substrate corrected.** Sweep configurations with
  `cluster_rare_threshold ∈ {1, 2}` now actually take effect. Prior
  Phase 10 evidence (era12-phase10-cluster-rare-threshold.md) measuring
  "+0.0 threshold delta in full bench" was characterizing a no-op — that
  characterization is now obsolete; Phase 2 must re-measure on the
  fixed harness.
- **Era-12 evidence-doc cancellation theory is now untestable from
  prior data.** The era-12 memo theorized symmetric firing on
  cal+fixture caused threshold inflation to cancel net catch impact.
  That theory was based on the standalone probe behavior, which we
  trust. Whether it holds at K=7 multi-seed median in the post-fix
  harness is the question Phase 2's sweep answers directly — no
  separate disambiguation run needed.

### Phase 1 disambiguation bench (Run A/B/C) — recommendation: skip

The executor staged three diagnostic bench runs (Run A baseline, Run B
K=7 + rare=2, Run C K=1 + rare=2) to characterize whether median masking
or cal-cancellation explains residual zero-delta. **Recommendation:
fold these into Phase 2's sweep, do not run separately.**

Rationale: the Phase 2 grid (`cluster_size_min ∈ {0, 20}` ×
`cluster_rare_threshold ∈ {1, 2}` × `threshold_percentile ∈ {p95, p99,
max}`) already includes Run B's `(K=7 default, rare=2, percentile=max)`
at the `(S_min=0, R=2, max)` cell. Phase 2's sweep is what answers the
proceed/no-proceed decision (does any config land catches?), and
running a separate K=1 disambiguation only adds value if it would
change whether to run Phase 2 — it wouldn't (the era-13 plan §Phase 2
explicitly says Phase 2 runs regardless of Phase 1 verdict). Run A is
just the era-11 baseline (`rare=0`) which the production config gives
us for free as the sweep's baseline corner.

If Phase 2's `(rare=2, K=7)` cells all show zero delta from the `(rare=0)`
baseline despite the Phase 1 fix, *then* a K=1 follow-up is informative
to localize median vs cal-cancellation. Save the cost until that
question becomes load-bearing.

### Phase 3 audit summary

Four comparisons walked; all four return symmetric or not-applicable:

| # | Comparison | Verdict | Touches residuals? |
|--:|:-----------|:--------|:-------------------|
| 1 | `_score_fixtures` (catalog) vs `_score_real_hunks` (real PRs) | symmetric (3 documented intentional asymmetries on `file_source`, prose-blanking, file-typicality — all stem from the post-era-12 `file_source=None` for fixtures) | yes (intentional) |
| 2 | Calibration path vs scoring path | symmetric (5 documented design choices on prose scope, `_strip_break_meta`, alpha/root_bonus, `enable_typicality_filter`, calibration CLI) | yes (intentional) |
| 3 | Fixtures without `host_file` metadata | not-applicable (115/115 fixtures have `host_file`; catalog-phantom path is dead in normal operation) | no |
| 4 | `is_atypical_file` short-circuit asymmetric per-corpus firing | not-applicable (uniformly does not fire on fixtures across all 6 corpora; era-12 fix is symmetric) | no |

Non-mandatory defensive item filed: `_score_fixtures` `OSError` paths
at run.py:173–177 and 179–183 silently revert to the catalog-phantom
path on read failure. Currently dead code (all host/catalog files exist
on disk), but it would silently mask future host-file regressions. Log
a warning rather than silently degrade. **Filed for a separate small
PR; not gating Phase 2.**

### Phase 2 readiness check

| Readiness criterion | Status |
|:--|:--|
| Phase 1 plumbing trace clean | ✓ (with fix applied) |
| Phase 3 audit clean of mandatory fixes | ✓ |
| Phase 2 sweep substrate exercises real config | ✓ (post Phase 1 fix) |
| `just verify` passes on staged Phase 1 fix | ✓ (237/237 tests, all lints) |
| Production scoring unchanged on 105 caught | ✓ (rare=0 default → no-op fix path) |
| Bench symmetry across 6 corpora trustworthy | ✓ |

### Recommendation

**Proceed to Phase 2.** The plumbing fix is staged but uncommitted;
Phase 3 audit clears the substrate. The Phase 2 sweep (twelve configs,
orchestrator-owned bench in background) is the next dispatch. No
disambiguation bench needed first — Phase 2 subsumes it.

Open question for the user: should the staged Phase 1 fix be committed
on this branch (`feat/era-13`) before Phase 2 starts, or kept staged
through Phase 2 to ship as a single coherent era-13 commit at the end?
The Phase 1 fix alone is small (one missing argv block + one missing
subparser arg + counter instrumentation) and would make a clean
"chore: bench plumbing fix" commit if you'd prefer to land it
independently.

---

## Phase 1 — Plumbing Audit (verbatim, p1-plumbing@era13)

### Verdict: **plumbing-bug-found** (and fixed)

#### 1. Trace Table

| Hop | File:line | Parameter name at hop | Status |
|:----|:----------|:----------------------|:-------|
| CLI flag definition (main parser) | `benchmarks/src/argot_bench/cli.py:98–108` | `--call-receiver-cluster-rare-threshold` → `args.call_receiver_cluster_rare_threshold` | passes-through |
| `_run` → `base_cmd` for run-one subprocess | `cli.py:337–354` (pre-fix) | **ABSENT** — no conditional for `cluster_rare_threshold` | **DROPPED** |
| `run-one` subparser (secondary gap) | `cli.py:155–230` (pre-fix) | `--call-receiver-cluster-rare-threshold` not defined on `run-one` subparser | **DROPPED** |
| `_cmd_run_one` → `RunConfig` | `cli.py:298` | `call_receiver_cluster_rare_threshold=args.call_receiver_cluster_rare_threshold` | passes-through (always gets 0 from argparse default) |
| `RunConfig` field | `run.py:97` | `call_receiver_cluster_rare_threshold: int = 0` | passes-through |
| `run_corpus` → `build_scorer` | `run.py:372` | `call_receiver_cluster_rare_threshold=cfg.call_receiver_cluster_rare_threshold` | passes-through |
| `build_scorer` → `calibrate_multi_seed` | `score.py:172` | `call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold` | passes-through |
| `calibrate_multi_seed` → `SequentialImportBpeScorer` | `calibration/__init__.py:93,117` | `call_receiver_cluster_rare_threshold=call_receiver_cluster_rare_threshold` | passes-through |
| `SequentialImportBpeScorer` → `CallReceiverScorer` | `sequential_import_bpe.py:231` | `cluster_rare_threshold=call_receiver_cluster_rare_threshold` | passes-through |
| `CallReceiverScorer.__init__` | `call_receiver.py:297` | `self.cluster_rare_threshold: int = cluster_rare_threshold` | passes-through |
| `weighted_contribution_for_file` rare branch | `call_receiver.py:460–467` | `self.cluster_rare_threshold > 0` check | fires correctly **when value reaches it** |

**Root cause:** `_run` builds `base_cmd` to spawn `run-one` subprocesses
and has a series of `if arg != default: base_cmd.extend(...)` guards
(lines 337–354) — one for every non-default-propagated parameter
**except** `cluster_rare_threshold`. The subprocess always starts with
`cluster_rare_threshold=0` (argparse default from main parser). The
parameter chain from `RunConfig` down through `build_scorer →
calibrate_multi_seed → CallReceiverScorer` is fully correct; the value
just never gets there.

**Secondary gap:** `run-one` subparser also lacked
`--call-receiver-cluster-rare-threshold` definition, meaning it
couldn't receive the flag even from a direct `argot-bench run-one`
invocation.

#### 2. Instrumentation

Counter added: `CallReceiverScorer.rare_branch_fire_count: int = 0`,
incremented in the rare branch of `weighted_contribution_for_file`.
Exposed up the chain:

- `SequentialImportBpeScorer.rare_branch_fire_count` (property) →
  `calibration/__init__.py`
- `BenchScorer.rare_branch_fire_count` (property) →
  `benchmarks/src/argot_bench/score.py`

Observable from bench output:
- **Calibration path**: `[rare-counter]` lines to stderr after each
  seed scorer in `calibrate_multi_seed` (only fires when
  `cluster_rare_threshold > 0`)
- **Fixture path**: `[rare-counter]` line to stderr after
  `_score_fixtures` in `run_corpus` (only fires when
  `cluster_rare_threshold > 0`)

Format of log lines:
```
[rare-counter] cal seed=0: rare_branch_fire_count=3 threshold=8.9400
[rare-counter] faker-js fixture path seed=0: rare_branch_fire_count=5
```

The counter survives the bench's subprocess multiprocessing because
each corpus runs in its own subprocess with its own interpreter — no
shared state needed.

#### 3. Bench Plan (needs orchestrator to run)

`just verify` passes (237/237 tests, all lints clean).

Three runs offered:

##### Run A — post-fix baseline (no rare threshold, current shipping config)
```
argot-bench --corpus faker-js
```
Expected: `threshold_mean ≈ 4.86` (same as era-12 baseline). Confirms
fix didn't break anything.

##### Run B — post-fix treated, K=7 (multi-seed median)
```
argot-bench --call-receiver-cluster-rare-threshold 2 --corpus faker-js
```
Grep stderr for `[rare-counter]` lines. Compare `threshold_mean` to
Run A.

##### Run C — post-fix treated, K=1 (single-seed, isolates median masking)
```
argot-bench --call-receiver-cluster-rare-threshold 2 --threshold-n-seeds 1 --corpus faker-js
```
Grep stderr for `[rare-counter]` lines. Compare `threshold_mean` to
Run A.

**Numerical comparisons:**

| Comparison | Meaning |
|:-----------|:--------|
| Run B `threshold_mean` > Run A (e.g. ~9.86) | Plumbing fix unblocked the rare branch; K=7 median was NOT masking (the branch was just never firing before) |
| Run B `threshold_mean` ≈ Run A (4.86) AND Run C `threshold_mean` > Run A | K=7 median was masking (branch fires in some seeds, median absorbs it); rare branch is symmetric on cal+fixture → Phase 2 percentile fix is the lever |
| Run B `threshold_mean` ≈ Run A AND Run C `threshold_mean` ≈ Run A AND `[rare-counter] cal seed=*: rare_branch_fire_count=0` | Cal hunks don't have rare-attested callees → threshold inflation doesn't happen → fixture catches should work → check fixture `rare_branch_fire_count` |

(See orchestrator synthesis above for the recommendation to fold these
into the Phase 2 sweep instead of running standalone.)

#### 4. Staged Changes

| File | Change |
|:-----|:-------|
| `benchmarks/src/argot_bench/cli.py` | **Bug fix** — added `if args.call_receiver_cluster_rare_threshold != 0: base_cmd.extend(...)` in `_run`; added `--call-receiver-cluster-rare-threshold` to `run-one` subparser |
| `engine/argot/scoring/scorers/call_receiver.py` | Instrumentation — `rare_branch_fire_count: int = 0` counter, incremented in rare branch |
| `engine/argot/scoring/scorers/sequential_import_bpe.py` | Instrumentation — `rare_branch_fire_count` property exposing call_receiver's counter |
| `engine/argot/scoring/calibration/__init__.py` | Instrumentation — stderr log per seed when `cluster_rare_threshold > 0` |
| `benchmarks/src/argot_bench/score.py` | Instrumentation — `rare_branch_fire_count` property on `BenchScorer` |
| `benchmarks/src/argot_bench/run.py` | Instrumentation — stderr log after fixture scoring when `cluster_rare_threshold > 0` |

Nothing committed.

---

## Phase 3 — Bench Symmetry Audit (verbatim, p3-symmetry@era13)

**Verdict: audit-clean. Zero MANDATORY-FIX items. Zero undocumented
asymmetries.**

### Comparison 1 — `_score_fixtures` (catalog) vs `_score_real_hunks` (real PRs)

- **Path A:** `run.py:136–240`, `_score_fixtures` — host-injection path
  when `fx.host_file is not None`
- **Path B:** `run.py:243–307`, `_score_real_hunks`

**Diff (with quoted evidence):**

Three documented differences, all intentional:

1. **`file_source`**: fixtures pass `None`; real hunks pass
   `file_abs.read_text()`.
   - run.py:215: `scored_src = None`
   - run.py:284: `file_source = file_abs.read_text(encoding="utf-8")`
   - Intentional: comment run.py:197–214 states explicitly that
     passing synthesized content "produces garbage results when
     tree-sitter sees an out-of-place class mid-host-file (ERROR
     nodes)" and "(b) the typicality_filter on the synthesized file
     triggers `atypical_file` short-circuits".

2. **Prose-blanking**: fixtures get none (`bpe_input = hunk_content` at
   scorer line 419 because `file_source is None`); real hunks get
   file-context blanking.
   - sequential_import_bpe.py:420: `if file_source is not None and hunk_start_line is not None ...`
   - Intentional consequence of the `file_source=None` design choice.

3. **File-level typicality**: skipped for fixtures (`file_source is
   None` → line 391 guard not entered); fires for real hunks.
   - sequential_import_bpe.py:391: `if file_source is not None: is_atyp_file, _ = self._typicality_model.is_atypical_file(file_source)`
   - Same intentional consequence.

**Import scoring:** SYMMETRIC. Both paths ultimately call
`adapter.extract_imports(hunk_content)` + `is_foreign()`. The
`file_source is None` branch at sequential_import_bpe.py:416 calls
`self._import_scorer.score_hunk(hunk_content)` which is
`adapter.extract_imports` + `is_foreign` count — identical logic to
the `file_source is not None` branch at lines 412–414.

**Cluster routing:** SYMMETRIC. Both paths resolve to real repo paths
(`host_path = repo_dir / fx.host_file` for fixtures; `file_abs =
repo_dir / file_path_rel` for real hunks). Both land in
`file_to_cluster` via static lookup. No Jaccard fallback fires for
either in normal operation.

**Call-receiver `file_source` arg:** fixtures pass `file_source=None`
to `weighted_contribution_for_file`; real hunks pass actual content.
When `file_path` IS in `file_to_cluster` (which it is for both, since
both use real repo paths), the `file_source` argument is irrelevant —
the static lookup at call_receiver.py:439 fires before the Jaccard
fallback at line 440–441.

- **Verdict: symmetric**
- **Touches residuals?** All 10 residuals have `host_file` set, so all
  go through this path. Yes — but the asymmetries are intentional and
  documented.
- **MANDATORY-FIX? No.**

### Comparison 2 — Calibration path vs scoring path

- **Path A:** `calibration/__init__.py:35–123`, `calibrate_multi_seed`
  → `SequentialImportBpeScorer.__init__` lines 278–318
- **Path B:** `sequential_import_bpe.py:335–482`, `score_hunk`

**Diff (with quoted evidence):**

1. **Prose-blanking scope:**
   - Calibration (metadata path, lines 293–295): `raw_bpe =
     self._bpe_score(_blank_prose_lines(hunk,
     self._adapter.prose_line_ranges(hunk)))` — **hunk-scoped** prose
     blanking.
   - Fixture scoring (file_source=None path): `bpe_input = hunk_content`
     — **no** prose blanking (line 419, guard at 420 fails).
   - Real hunk scoring (file_source provided): file-context prose
     ranges intersected with hunk window — **file-scoped** blanking
     (lines 421–429).
   - These three modes are different, but intentional (same root cause:
     `file_source=None` decision documented at run.py:197–217).

2. **`_strip_break_meta` treatment:**
   - Only called at run.py:187: `cleaned, clean_hs, clean_he =
     _strip_break_meta(catalog_content, fx.hunk_start_line,
     fx.hunk_end_line)` — fixture scoring only.
   - Calibration does NOT call it (calibration hunks are drawn from
     the actual repo, which never contain `# Break: ...` comments).
   - **Symmetric by construction:** the treatment applies exactly
     where it's needed.

3. **`synthesize_hunk_in_host`:**
   - `synthesize_hunk_in_host` (ml/features.py:114) exists in the ML
     pipeline but is **not used** in the bench harness (`run.py`) or
     the production scorer (`sequential_import_bpe.py`). Neither
     calibration nor scoring calls it. **Symmetric: both ignore it.**

4. **Call-receiver alpha/root_bonus in calibration vs scoring:**
   - Calibration (metadata path, line 297–304): `alpha=0.0,
     root_bonus=0.0, cluster_bonus=call_receiver_cluster_bonus` — only
     cluster_bonus folded in.
   - Fixture/real hunk scoring (lines 444–458): full `alpha +
     root_bonus + cluster_bonus`.
   - Intentional by design: calibration hunks are drawn from the repo
     and have no unattested global callees → alpha/root_bonus naturally
     contribute 0 → not needed in threshold computation.

5. **`enable_typicality_filter` in calibration:**
   - `calibrate_multi_seed` at calibration/__init__.py:94:
     `enable_typicality_filter=enable_typicality_filter` — passed
     through.
   - `build_scorer` at score.py:173:
     `enable_typicality_filter=enable_typicality_filter` — passed
     through.
   - **Symmetric:** same parameter controls both paths.

6. **Calibration CLI `cluster_rare_threshold` omission (production-only
   concern):**
   - `calibration/__main__.py:221–237`: `calibrate_multi_seed` call
     does NOT pass `call_receiver_cluster_rare_threshold` (defaults to
     0).
   - Bench `build_scorer` DOES pass it (score.py:171). This is
     bench-internal and consistent (bench calibration + bench scoring
     both use the sweep value). Production CLI doesn't yet expose the
     parameter, but production currently ships
     `cluster_rare_threshold=0` anyway.
   - Not a bench asymmetry.

- **Verdict: symmetric**
- **Touches residuals?** Yes (all 10 use the calibration path to set
  their threshold). Differences are all intentional.
- **MANDATORY-FIX? No.**

### Comparison 3 — Fixtures without `host_file` metadata

- **Path A (expected):** `_score_fixtures` host-injection path
  (run.py:168–217)
- **Path B (feared):** catalog-phantom fallback — `file_path = repo_dir
  / fx.file` (a path not in the repo's `file_to_cluster`)

**Evidence — host_file coverage is 100%:**

```
faker-js: 17 fixtures, 17 host_file entries
faker:    16 fixtures, 16 host_file entries
fastapi:  32 fixtures, 32 host_file entries
hono:     17 fixtures, 17 host_file entries
ink:      17 fixtures, 17 host_file entries
rich:     16 fixtures, 16 host_file entries
```

All 115 fixtures across all 6 corpora have both `host_file` and
`host_inject_at_line` set. The catalog-phantom fallback path
(`file_path = repo_dir / fx.file`) at run.py:162 is never reached in
normal bench operation.

**Latent defensive note (non-blocking):** The `OSError` silent-fallback
path at run.py:173–217 — if `host_path.read_text()` or
`catalog_path.read_text()` raises, `file_path` silently reverts to
`repo_dir / fx.file` (the catalog-phantom path). No error is logged.
This is a latent robustness issue but does not currently affect any
fixture (all host files exist on disk).

All 10 residuals confirmed to have `host_file`:
- `validation_2` → `fastapi/params.py:755`
- `exception_handling_4` →
  `docs_src/security/tutorial003_an_py310.py:95`
- `synthetic_formula_1` →
  `faker/providers/phone_number/__init__.py:332`
- `hono_validation_2` → `src/middleware/jwt/jwt.ts:179`
- `hono_middleware_3` → `src/middleware/basic-auth/index.ts:154`
- `ink_dom_access_2` → `src/hooks/use-input.ts:272`
- `faker_js_error_flip_2` → `src/internal/locale-proxy.ts:159`
- `faker_js_foreign_rng_1` → `src/modules/string/index.ts:954`
- `faker_js_http_sink_2` → `src/modules/git/index.ts:221`
- `faker_js_runtime_fetch_1` → `src/internal/locale-proxy.ts:159`

- **Verdict: not-applicable** — no fixture lacks `host_file`;
  catalog-phantom path is dead code in practice.
- **Touches residuals? No** (all 10 correctly route via host_file).
- **MANDATORY-FIX? No.**

### Comparison 4 — `is_atypical_file` short-circuit asymmetry

- **Path A:** fixture scoring — `file_source=None` →
  sequential_import_bpe.py:391 guard (`if file_source is not None`)
  never entered → `is_atypical_file` **never fires** for any fixture.
- **Path B:** real hunk scoring — `file_source=file_abs.read_text()` →
  `is_atypical_file` fires on actual repo file content.

The era-12 bug was exactly this check firing on synthesized host
content (catalog injected into host file), producing
`reason="atypical_file"` on legitimate breaks. The fix — setting
`file_source=None` for all fixture scoring — applies **uniformly across
all 6 corpora** (confirmed: all 115 fixtures have `host_file`, all use
the `file_source=None` path).

No corpus uses synthesized content as `file_source`. The
`synthesize_hunk_in_host` function in `ml/features.py:114` is only used
by the ML CLI pipeline (`ml/cli.py:272`), not by the bench harness.

Is there asymmetric firing within any other corpus? No. The rule is
flat: fixtures → never fires; real hunks → fires based on actual file.
Consistent across all 6 corpora.

- **Verdict: not-applicable** — uniform non-firing for fixtures is by
  design; no corpus-specific divergence.
- **Touches residuals?** No impact — `is_atypical_file` would have
  suppressed them (false negative) if it fired; it doesn't, so it
  can't be the cause of non-detection.
- **MANDATORY-FIX? No.**

### Closing Summary

| | Verdict | MANDATORY-FIX? | Touches residuals? |
|---|---|---|---|
| 1. `_score_fixtures` vs `_score_real_hunks` | symmetric | No | Yes (intentional) |
| 2. Calibration path vs scoring path | symmetric | No | Yes (intentional) |
| 3. Fixtures without `host_file` | not-applicable | No | No |
| 4. `is_atypical_file` asymmetry | not-applicable | No | No |

- **Asymmetries found:** 0 undocumented
- **MANDATORY-FIX items:** none
- **Non-mandatory defensive items:** 1 — silent OSError fallback in
  `_score_fixtures` (run.py:173–177 and 179–183) reverts to
  catalog-phantom routing without logging; low risk since host/catalog
  files always exist, but should log a warning rather than silently
  degrade.
- **Overall verdict: audit-clean**

Phase 2 sweep substrate is sound. No fixes required before Phase 2
starts.

## End of Document
