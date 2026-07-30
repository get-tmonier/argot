# The two Pascal grammar rules, benchmarked

**Date:** 2026-07-30 · **Status:** measured — headline unchanged, one cost
recorded. Closes the debt in
[`pascal-parse-loss-two-more-rules.md`](pascal-parse-loss-two-more-rules.md).

PR #332 went out as a hotfix and deferred the recall/false-alarm harness with the
open question stated plainly: making 36,000 previously-invisible lines feed
calibration moved the Pascal threshold, and *"whether that improves or degrades
catch-rate on the Pascal corpora is unmeasured."* This is that measurement, plus
the architecture and integrity fixture guards.

## Method

`39708097` (v0.2.118, the released grammar) against `b54a6846` (v0.2.119, the two
rules + the iterative token walk). The trees differ in exactly four production
files — `vendor/tree-sitter-pascal/{grammar.js,src/grammar.json,src/parser.c}`
and `crates/argot-lang/src/tokenize.rs` — plus tests. Catalogs, targets and the
harness itself are byte-identical between them, so the A/B has one variable.

```sh
git worktree add --detach /tmp/argot-v0.2.118 39708097
(cd /tmp/argot-v0.2.118 && cargo build --release -p argot-bench)

# same clones, same catalogs, same pinned SHAs, run sequentially
ARGOT_BENCH_COMMIT=39708097 /tmp/argot-v0.2.118/target/release/argot-bench \
  --targets      benchmarks/targets.yaml \
  --catalogs-dir benchmarks/catalogs \
  --data-dir     benchmarks/data \
  --results-dir  benchmarks/results/ab-before
ARGOT_BENCH_COMMIT=b54a6846 ./target/release/argot-bench \
  … --results-dir benchmarks/results/ab-after
```

Full `honest` mode — production-path recall on the curated catalogs + leak-free
temporal-holdout FP — over all 36 corpora / 12 languages. 44 min and 38 min on
11 cores.

## The control: identical code gives identical numbers

`benchmarks/results/latest` (the last full run, 2026-07-29) was deliberately
**not** reused as the baseline, and the reason is itself measurable: 32 of its 36
rows come out bit-identical to the fresh v0.2.118 run and 4 do not — several
merges landed in between, #318, #319 and #320 among them. So the harness is
deterministic at the row level, and everything that moves below moved because of
the grammar.

## Headline: unchanged

| | v0.2.118 | v0.2.119 |
|---|--:|--:|
| gated novel-pattern catch | 645/756 = **85.32%** | 645/756 = **85.32%** |
| all-fixture recall | 718/977 = 73.49% | 718/977 = 73.49% |
| worst over-fire (existing files) | 1.75% | 1.75% |
| worst over-fire (new files) | 0.00% | 0.00% |
| rows that moved | — | **2 of 36** |

No corpus lost a fixture. On all three catalogued Pascal corpora every fixture's
score is identical to the last digit.

## The 34 rows that did not move

**31 non-Pascal corpora — the other 11 languages plus the polyglot dagster
monorepo — are bit-identical**: recall, hunk counts, hit counts, calibrated
thresholds. That is the measurement the iterative
rewrite of `collect_tokens` needed: same tokens, same order, on the production
path, not only in the goldens.

Three of the five Pascal corpora are bit-identical too, down to the calibrated
threshold (castle-engine 6.1704, mormot2 5.6308, uos 10.8568 — before and after).
Neither construct costs those repositories a parse; the two rules land squarely
on the MSEgui family.

## The 2 rows that moved

| corpus | gated recall | over-fire (existing) | novel-pattern detection | calibrated pascal threshold |
|---|---|---|---|---|
| castle-engine | 11/11 → 11/11 | 0.15% (1/657) → 0.15% | 0.61% (4/657) → 0.61% | 6.1704 → 6.1704 |
| mormot2 | 11/11 → 11/11 | 0.00% (0/666) → 0.00% | 0.60% (4/666) → 0.60% | 5.6308 → 5.6308 |
| **mseide-msegui** | 8/10 → **8/10** | 0.00% (0/2285) → **0.31%** (7/2285) | 0.74% (17) → **0.79%** (18) | 7.1730 → **5.4949** |
| uos | — | 0.00% (0/3492) → 0.00% | 0.63% (22/3492) → 0.63% | 10.8568 → 10.8568 |
| **ideu** | — | 0.03% (1/3604) → **0.50%** (18/3604) | 0.19% (7) → **0.14%** (5) | 5.2249 → **5.1614** |

Both remain far under the RUBRIC's ≤2% over-fire bar, and both are the
explicitly non-gated extra corpora. New-file FP is unchanged everywhere
(mseide-msegui 5.13%, uos 9.09%, the rest 0%).

### mseide-msegui — 8 new fires, 0 lost

| | fires | where |
|---|--:|---|
| new `unfamiliar-callee` (detection) | 4 | `lib/common/kernel/linux/mseguiintf.pas` ×3, `lib/common/widgets/msegrids.pas` ×1 |
| new `rare-tokens` (over-fire) | 4 | `msegdbutils.pas`, `mselibc.pas`, `tools/POtools/POtoMO/POtoMO.pas` ×2 |
| reclassified detection → over-fire | 3 | `msebufdataset.pas` ×2, `msedbedit.pas` — same hunks, same fire, top reason changed |
| lost fires | 0 | — |

Three of the four new detections are inside `mseguiintf.pas` — the X11 backend
that carried **1,921 `ERROR` nodes** and of whose 102 routines only 21 were
recoverable. Those fires were not missed before; they were impossible.

The over-fire cost is the four `rare-tokens` fires, plus the three hunks whose
top reason flipped from `unfamiliar-callee` to `rare-tokens` (the split counts
`rare-tokens` as over-fire even on a genuinely novel pattern, by design — the
reported rate is an honest ceiling). Recall is unchanged at 8/10 gated, with the
same two fixtures uncaught as before.

### ideu — a 0.06 threshold move and one repetitive commit

The 17 new fires are the same shape 17 times: every one scores **5.1814** against
a threshold of **5.1614** — 0.02 of margin — and all sit in `src/po2arrays.pas`
and `src/potools.pas` across two commits of the same
`utf8String` → `msestring` conversion sweep (`14b42ba9`). The calibrated
threshold dropped 0.06 and that cluster fell through the gap.

The two lost fires are both whole-file hunks of `src/main.pas`
(`@@ -1,5722 +1,5735 @@` — every line replaced) on two line-ending rewrites.
The metric counted them as novel-pattern detection because `unfamiliar-callee`
fires only on 0-usage callees; nothing about a wholesale line-ending flip is a
novel dependency, so losing them is not a loss of real catch.

## Integrity layer: unchanged

`--mode integrity-verify` on castle-engine (the one Pascal corpus with gaming
fixtures; mORMot2's bespoke `TSynTestCase` framework is out of scope by design):
**11/11 caught, 0 missed, 0 invalid, 0/4 control-FP** at both revisions,
identical per tactic.

## Architecture layer: one fixture went 10/10 → 9/10, and it was right to

`--mode arch-verify` on the two Pascal arch corpora:

| | v0.2.118 | v0.2.119 |
|---|--:|--:|
| castle-engine | 10/10 · 0/4 controls | 10/10 · 0/4 controls |
| mormot2 | 10/10 · 0/4 controls | **9/10** · 0/4 controls |

The miss is the `misc → orm` `sink_out` fixture, reported by the harness as
`MISS (novel forward)` — the planted edge is no longer classified as a violation
at all. **The new classification is the correct one**, and the chain is
measurable end to end:

1. `--mode arch-candidates` shows **11 edges flipping `sink_out` → `forward`,
   every one of them sourced at `misc`**. No other layer moves; no target's
   in-mass changes.
2. Dumping the fitted graph at both revisions (`RepoLayering::fit` over the same
   525 files, `to_json`): `misc` goes from **in 1 / out 1** to **in 1 / out 2**.
   The near-sink rule is `out ≤ 0.5 × (in + out)`, so 1 ≤ 1.0 made it a sink and
   2 > 1.5 does not. Every other layer's sink status is identical.
3. The extra edge is `misc → core`, and it appears because the Pascal
   unit-name→layer index gained exactly two entries: **`mormot.core.base`** and
   `mormot.db.raw.oracle`. `src/misc/mormot.misc.iso.pas` imports precisely
   `mormot.core.base` and `mormot.core.os`; with neither resolvable, that file
   contributed no internal edge at all.
4. Parsing `src/core/mormot.core.base.pas` (14,130 lines) directly under each
   grammar: **before — 92 `ERROR`/`MISSING` nodes, widest span 14,131 lines, i.e.
   the entire unit inside one error**, so its `unit` declaration never registered;
   **after — 9 nodes, widest span 2 lines.**

So the fixture's stated premise — *"misc is a tiny leaf helper layer (in-mass 1)
and a net-importee"* — was an artefact of mORMot's foundational unit being
invisible. Post-fix no violation of any kind can be authored from `misc`: it
reverses nothing, closes no cycle, and is no longer a sink.

**The fixture was re-authored, not deleted or muted**: `misc → orm sink_out` is
replaced by `db → app sink_out` (`src/db/mormot.db.core.pas` + `uses
mormot.app.console;`), taken from the post-fix resolver-verified candidate menu —
`db` is a genuine net-importee (in 52 / out 32) and the DB core owning a
dependency on the console app framework is the same shape of tell. The retired
fixture's rationale is recorded in place, in the catalog. mormot2 is back to
**10/10 · 0/4 controls**, so the Pascal arch aggregate stands at 20/20 and the
capstone's 264/272 is unchanged.

## A parse gap that remains: `mormot.core.os`

The same probe says `src/core/mormot.core.os.pas` (12,534 lines) is **still one
whole-file `ERROR` under both grammars** — 23 nodes, widest span 12,535 lines —
which is why `mormot.core.os` is absent from the unit index before *and* after.
mORMot's OS-abstraction unit, the second-largest in the framework, is invisible
to every rule. That is a third construct these two rules do not cover, and it is
worth its own investigation; it is out of scope here and nothing in this branch
addresses it.

**Follow-up, 2026-07-31.** The third construct was an anonymous record type in a
variable declaration (`SystemEntropy: record … end;`). The grammar already
accepted that form for fields and array elements, but not variables. With the
same local corpus and Argot's directive masking path, the unit is no longer a
whole-file error: its 12,534-line span becomes 13 residual error rows, and its
unit declaration returns to the index. This document preserves the earlier
two-rule benchmark; the grammar follow-up carries the focused regression test.

## Semantic layer: no fixture exists to measure it

`benchmarks/semantic-fixtures/` covers 31 corpora and not one of them is Pascal,
so the semantic rules' Pascal behaviour cannot be benchmarked with what exists
today — stated as a gap rather than papered over. What is known is what the fix's
own PR recorded: on mseide-msegui's own fit the index grows 25,902 → 26,905
functions. Authoring Pascal semantic fixtures is separate work.

## Verdict

The 6.59% → 0.05% parse-loss win costs:

- **nothing on the gated headline** — 85.32% catch before and after, no fixture
  lost in any of 36 corpora, and 31 non-Pascal rows bit-identical;
- **+0.31 pp over-fire on mseide-msegui and +0.47 pp on ideu**, both non-gated
  extras, both ~4× under the ≤2% bar;
- **nothing on integrity** (11/11, 0/4 controls) and **nothing on architecture**
  once one mormot2 fixture whose premise was a parse artefact is re-authored
  (20/20, 0/8 controls, both before and after).

And it buys: 4 fires the released grammar could not physically produce, 3 of them
inside a file that was 100% invisible; `mormot.core.base` — mORMot's foundational
14,130-line unit — recovered from a whole-file parse error; and 11 mormot2 layer
edges that were being classified as violations on a distorted graph now reading
correctly as forward.

One thing this run says is **not** done: the semantic layer has no Pascal fixture
to be measured by. The `mormot.core.os` parse loss was fixed in the 2026-07-31
grammar follow-up noted above.

Raw results: `benchmarks/results/{ab-before,ab-after}` (voice),
`{ab-arch-before,ab-arch-after,ab-arch-after2}` (arch),
`{ab-int-before,ab-int-after}` (integrity),
`{ab-cand-before,ab-cand-after}` (candidate menus).
