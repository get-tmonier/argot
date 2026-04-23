# Research Narrative Consolidation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Consolidate 14 phases of `argot` research into 5 narrative docs under `docs/research/` on `main`, telling the arc from JEPA origin through the import-graph breakthrough.

**Architecture:** Docs-only change. Five flat files (README + 4 era docs). Source material accessed via `git show research/phase-14-pre-cleanup:<path>` for committed markdown and `docs/research/scoring/signal/phase*/*.json` for in-tree result JSONs. Phase 13–14 experiment JSONs live on branch `research/phase-14-import-graph` under `engine/argot/research/signal/phase1{3,4}/experiments/`. Single-agent sequential write (era order), README last.

**Tech Stack:** Markdown only. `git show`, `git ls-tree`, `wc`, `rg` for exploration / validation.

**Working branch:** `docs/research-narrative-spec` (already checked out; spec commits already present). All docs land on this branch and ship as one PR.

**Source-availability note:** The spec's source table referenced `.argot/phaseN_results/` as primary result data, but `.argot/` was gitignored everywhere and is absent from the tag. Replacements: (a) per-phase markdown docs embed results tables and experiment summaries — these become the primary source for phases 1–9; (b) phases 10–12 have committed JSONs under `docs/research/scoring/signal/phase11/` and `signal/phase12/` on the tag; (c) phases 13–14 have committed JSONs under `engine/argot/research/signal/phase1{3,4}/experiments/` on the `research/phase-14-import-graph` branch. The spec's spot-check protocol handles this: figures that cannot be traced to a committed JSON or a markdown results table are either dropped or footnoted as "figure as reported in `<source doc>`".

---

## File Structure

```
docs/research/
  README.md                          # ~200–400 words. Written last.
  01-jepa-era.md                     # phases 1–6.   500–1000 words.
  02-pivot-to-honest-eval.md         # phases 7–9.   500–1000 words.
  03-bpe-signal-hunt.md              # phases 10–12. 500–1000 words.
  04-import-graph-breakthrough.md    # phases 13–14. 500–1000 words.
```

The existing empty placeholder `docs/research/scoring/` directory stays as-is (gitignored dir entry becomes meaningful again later; out of scope here).

---

## Task 0: Pre-flight checks

**Files:**
- None yet (verification only)

- [ ] **Step 1: Confirm working branch and clean tree**

Run:
```bash
git rev-parse --abbrev-ref HEAD
git status --short
```
Expected: branch `docs/research-narrative-spec`, no uncommitted changes.

- [ ] **Step 2: Verify the tag is reachable and has source docs**

Run:
```bash
git rev-parse research/phase-14-pre-cleanup >/dev/null && echo "tag OK"
git ls-tree research/phase-14-pre-cleanup -r --name-only | grep -c 'docs/research/scoring/'
```
Expected: `tag OK` and a count ≥ 40.

- [ ] **Step 3: Verify phase 13–14 experiment JSONs on the research branch**

Run:
```bash
git ls-tree research/phase-14-import-graph -r --name-only | \
  grep 'engine/argot/research/signal/phase1[34]/experiments/.*\.json$' | wc -l
```
Expected: count ≥ 4. If 0, STOP and report — era 4 cannot be spot-checked without these.

- [ ] **Step 4: Scaffold the output directory**

Run:
```bash
mkdir -p docs/research
ls docs/research/
```
Expected: directory exists, empty except for the pre-existing `scoring/` placeholder.

No commit for Task 0. Proceed to Task 1.

---

## Task 1: Write `01-jepa-era.md` (phases 1–6)

**Files:**
- Create: `docs/research/01-jepa-era.md`

**Primary sources (all on the tag unless noted):**
- `docs/research/scoring/DESIGN-phases-1-6.md` — era design and running hypothesis
- `docs/research/scoring/phases-1-6/synthesis.md` — consolidated era findings
- `docs/research/scoring/phases-1-6/03-context-after.md` through `14-token-embed-combined.md` — per-sweep results
- `docs/research/scoring/phases-1-6/sizing-study.md` — scaling behaviour
- `docs/research/scoring/phases-1-6/corpus.md` — corpus definition

**Era beat** (from spec): Six tuning phases of JEPA: context, embed dim, n-grams, path embed, token embeddings, transformer encoder. The approach was wrung out over six sweeps.

- [ ] **Step 1: List all phase-1–6 sources**

Run:
```bash
git ls-tree research/phase-14-pre-cleanup -r --name-only | \
  grep -E 'docs/research/scoring/(DESIGN-phases-1-6|phases-1-6)/'
```
Record the full list.

- [ ] **Step 2: Read `synthesis.md` first (era-level summary)**

Run:
```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/phases-1-6/synthesis.md
```
Then read `DESIGN-phases-1-6.md` and skim the per-sweep docs (`03-*` through `14-*`) for headline numbers.

- [ ] **Step 3: Draft candidate headline figures (aim for 3–5)**

Write them down first — do not draft prose yet. Each must name the metric (AUC, break_mean, etc.), the corpus (fastapi / rich / httpx), and the source doc.

Examples of the shape (actual numbers from your reading):
- "JEPA peaked at AUC X.XX on fastapi at phase N" → source: `phases-1-6/synthesis.md` §Results
- "Adding char-ngrams moved AUC by +X.XX" → source: `phases-1-6/06-char-ngrams.md` §Results
- "Token embeddings regressed by X.XX vs baseline" → source: `phases-1-6/11-token-embeddings.md` §Results

- [ ] **Step 4: Spot-check each headline figure**

For each figure:
1. Open the source doc with `git show research/phase-14-pre-cleanup:<path>`.
2. Locate the results table or stated number.
3. If the number is present and unambiguous: keep, cite using the full-path form `[`docs/research/scoring/<path>` §<section>]`.
4. If the number is ambiguous (multiple values quoted, rounded with no raw source): drop the figure. Rewrite the sentence without it, or drop the claim.
5. Never fabricate, never say "approximately X" for a number pulled from memory.

- [ ] **Step 5: Draft the era doc per template**

Structure (from spec):
```
# The JEPA era (phases 1–6)

## The hypothesis we were testing
<1–2 paragraphs: what we believed going in and why. Unpack JEPA on first use.>

## What we tried
<2–5 bullets, each one sentence + key numbers. Hyperparameter sweeps within the era get one line total in this section — never their own section.>

## What the numbers said
<~3–5 row table: sweep / corpus / headline metric / delta vs prior best / citation>

## What broke the era
<1–2 paragraphs: the finding that forced the pivot to honest eval. Why was JEPA's plateau not cured by more tuning?>

## → next era
<1 line transition pointing to `02-pivot-to-honest-eval.md`>
```

Unpack JEPA on first mention:
> JEPA — a joint embedding predictive architecture, i.e., a model that embeds context and target into the same vector space and learns to predict the target from the context.

- [ ] **Step 6: Apply noise-removal rules**

Drop:
1. Experiments whose outcome didn't change the next move (e.g., a sweep variant that lost to an already-planned alternative).
2. Repeated framings of the same finding — pick the cleanest-numbered one.
3. Plumbing docs (rerun plans, execution logs, fixture audits) — not findings.
4. Hyperparameter sweeps with no frontier shift — one line in "What we tried", never a section.

- [ ] **Step 7: Word count**

Run:
```bash
wc -w docs/research/01-jepa-era.md
```
Target: 500–1000. If >1200, trim — noise-removal wasn't applied hard enough. Do not move on until word count is in range.

- [ ] **Step 8: Self-check acceptance**

Template shape:
```bash
rg '^## ' docs/research/01-jepa-era.md
```
Expected output (exactly these 5 lines):
```
## The hypothesis we were testing
## What we tried
## What the numbers said
## What broke the era
## → next era
```

Headline-figure citations — every numerical claim in the ## What the numbers said table must have an inline citation like `[\`docs/research/scoring/...\` §...]`. Visually scan the table; fix any row missing a citation.

Hedge words — run:
```bash
rg -i 'approximately|roughly|about [0-9]|around [0-9]' docs/research/01-jepa-era.md
```
Expected: no matches. Any hit means a number came from memory — trace it or drop it.

- [ ] **Step 9: Commit**

```bash
git add docs/research/01-jepa-era.md
git commit -m "$(cat <<'EOF'
docs: research narrative — era 1 (JEPA, phases 1–6)

Covers the six tuning phases of JEPA (context, embed dim, char/word
n-grams, path embed, token embeddings, transformer encoder) and the
plateau that forced the pivot to honest evaluation.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 2: Write `02-pivot-to-honest-eval.md` (phases 7–9)

**Files:**
- Create: `docs/research/02-pivot-to-honest-eval.md`

**Primary sources:**
- `docs/research/scoring/DESIGN-phase-7.md` — framing of the honest-eval pivot
- `docs/research/scoring/phase-7/plan-7-0.md` — phase-7 plan
- `docs/research/scoring/phase-7/corpus.md` — new corpus
- `docs/research/scoring/phase-7/16-rebaseline.md` — rebaseline results
- `docs/research/scoring/phase-7/17-density-heads.md` — density head results
- `docs/research/scoring/phase-7/18-pretrained-jepa.md` — pretrained JEPA results
- `docs/research/scoring/phase-8/spot-check.md` — cross-check
- Any `docs/research/scoring/phase-9/*.md` that exist on the tag (list below)

**Era beat:** Rebaseline + density heads + pretrained JEPA. Confronted whether JEPA added signal over trivial baselines.

- [ ] **Step 1: List all phase-7–9 sources**

Run:
```bash
git ls-tree research/phase-14-pre-cleanup -r --name-only | \
  grep -E 'docs/research/scoring/(DESIGN-phase-7|phase-[789])/'
```
Record the full list. If `phase-9/` has no docs, note it — the era still covers phases 7–8 substantively.

- [ ] **Step 2: Read DESIGN-phase-7.md and 16-rebaseline.md first**

Run:
```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/DESIGN-phase-7.md
git show research/phase-14-pre-cleanup:docs/research/scoring/phase-7/16-rebaseline.md
```
Then read 17-density-heads, 18-pretrained-jepa, and phase-8/spot-check.

- [ ] **Step 3: Draft candidate headline figures (aim for 3–5)**

Examples of the shape (actual numbers from your reading):
- "Rebaseline AUC on fastapi for JEPA vs BoW baseline"
- "Density heads delta vs rebaseline"
- "Pretrained JEPA delta vs from-scratch JEPA"

- [ ] **Step 4: Spot-check each headline figure**

Same protocol as Task 1 Step 4. Drop unverifiable numbers.

- [ ] **Step 5: Draft the era doc per template**

```
# The pivot to honest evaluation (phases 7–9)

## The hypothesis we were testing
## What we tried
## What the numbers said
## What broke the era
## → next era
```

Transition pointer: `03-bpe-signal-hunt.md`.

- [ ] **Step 6: Apply noise-removal rules**

Same as Task 1 Step 6.

- [ ] **Step 7: Word count**

```bash
wc -w docs/research/02-pivot-to-honest-eval.md
```
Target: 500–1000. >1200 → trim.

- [ ] **Step 8: Self-check acceptance**

```bash
rg '^## ' docs/research/02-pivot-to-honest-eval.md
rg -i 'approximately|roughly|about [0-9]|around [0-9]' docs/research/02-pivot-to-honest-eval.md
```
Expected template headers match; no hedge-word matches.

- [ ] **Step 9: Commit**

```bash
git add docs/research/02-pivot-to-honest-eval.md
git commit -m "$(cat <<'EOF'
docs: research narrative — era 2 (honest eval, phases 7–9)

Rebaseline, density heads, and pretrained JEPA — the phase where honest
comparison against trivial baselines confronted whether JEPA was adding
signal on top of BoW/n-gram controls.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 3: Write `03-bpe-signal-hunt.md` (phases 10–12)

**Files:**
- Create: `docs/research/03-bpe-signal-hunt.md`

**Primary sources (markdown on the tag):**
- `docs/research/scoring/signal/README.md` — signal subdir overview
- `docs/research/scoring/signal/jepa_detection_limits.md` — JEPA's failure modes
- `docs/research/scoring/signal/phase10_corpus_analysis_2026-04-21.md`
- `docs/research/scoring/signal/phase10_structural_context_2026-04-21.md`
- `docs/research/scoring/signal/phase10_execution_log.md` (plumbing — skim only)
- `docs/research/scoring/signal/phase11/phase11_context_variant_sweep_2026-04-21.md`
- `docs/research/scoring/signal/phase11/sweep_fastapi_stage5_baseline_2026-04-21.md`
- `docs/research/scoring/signal/phase11_context_variants_2026-04-21.md`
- `docs/research/scoring/signal/phase12/final_2026-04-21.md`
- `docs/research/scoring/signal/phase12/b_mlm_and_existing_2026-04-21.md`
- `docs/research/scoring/signal/phase12/e_blend_2026-04-21.md`

**Primary data (committed JSON on the tag):**
- `docs/research/scoring/signal/phase11/scores_fastapi_stage5_baseline_2026-04-21.json`
- `docs/research/scoring/signal/phase11/scores_fastapi_stage5_file_only_2026-04-21.json`
- `docs/research/scoring/signal/phase12/b_scores_2026-04-21.json`

**Era beat:** Corpus analysis → context variants → first BPE log-ratio experiments. The search for signal JEPA wasn't providing.

- [ ] **Step 1: List all phase-10–12 sources (incl. JSONs)**

Run:
```bash
git ls-tree research/phase-14-pre-cleanup -r --name-only | \
  grep -E 'docs/research/scoring/signal/(phase1[012]|jepa_detection_limits|README)'
```

- [ ] **Step 2: Read the summary docs first**

Run in order:
```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/signal/jepa_detection_limits.md
git show research/phase-14-pre-cleanup:docs/research/scoring/signal/phase12/final_2026-04-21.md
git show research/phase-14-pre-cleanup:docs/research/scoring/signal/phase11_context_variants_2026-04-21.md
```
Then read the per-sweep docs.

- [ ] **Step 3: Draft candidate headline figures (aim for 3–5)**

Examples of the shape (actual numbers from your reading):
- "BPE log-ratio alone hit break_mean X on fastapi"
- "Context variant X beat variant Y by +Z"
- "MLM-surprise baseline AUC on fastapi"

- [ ] **Step 4: Spot-check each headline figure against committed JSON where possible**

For figures referenced in `sweep_*.md`, open the matching `scores_*.json`:
```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/signal/phase11/scores_fastapi_stage5_baseline_2026-04-21.json | head -50
```
Confirm the numbers quoted in the markdown match the JSON. Cite the markdown doc (not the JSON) in prose — JSON is the audit trail. If the markdown number disagrees with the JSON, trust the JSON and update the claim.

For phase 12 blend-config numbers, read `blend_config_2026-04-21.json` similarly.

- [ ] **Step 5: Draft the era doc per template**

```
# The BPE signal hunt (phases 10–12)

## The hypothesis we were testing
## What we tried
## What the numbers said
## What broke the era
## → next era
```

Transition pointer: `04-import-graph-breakthrough.md`.

- [ ] **Step 6: Apply noise-removal rules**

Phase 10 has an execution log and a fixture audit doc. These are plumbing — one sentence total, not a section. Phase 11 has four variant-specific sweep docs (baseline / file-only / parent-only / siblings-only); pick the cleanest-numbered synthesis (likely `phase11_context_variants_2026-04-21.md`) and don't re-narrate each variant.

- [ ] **Step 7: Word count**

```bash
wc -w docs/research/03-bpe-signal-hunt.md
```
Target: 500–1000. >1200 → trim.

- [ ] **Step 8: Self-check acceptance**

```bash
rg '^## ' docs/research/03-bpe-signal-hunt.md
rg -i 'approximately|roughly|about [0-9]|around [0-9]' docs/research/03-bpe-signal-hunt.md
```

- [ ] **Step 9: Commit**

```bash
git add docs/research/03-bpe-signal-hunt.md
git commit -m "$(cat <<'EOF'
docs: research narrative — era 3 (BPE signal hunt, phases 10–12)

Corpus analysis, context variants, and the first BPE log-ratio
experiments — the search for the signal JEPA wasn't providing.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 4: Write `04-import-graph-breakthrough.md` (phases 13–14)

**Files:**
- Create: `docs/research/04-import-graph-breakthrough.md`

**Primary sources (markdown on the tag):**
- `docs/research/scoring/signal/phase13/b_mlm_and_existing_2026-04-21.md`
- Other `docs/research/scoring/signal/phase13/*.md` on the tag (list in Step 1)
- `docs/research/scoring/signal/phase14/experiments/*.md` on the tag (list in Step 1)

**Primary data (committed JSON on `research/phase-14-import-graph` branch):**
- `engine/argot/research/signal/phase13/experiments/ast_contrastive_faker_breaks_scores.json`
- `engine/argot/research/signal/phase13/experiments/ast_contrastive_rich_scores.json`
- `engine/argot/research/signal/phase13/experiments/ensemble_bpe_ast_faker_breaks_scores.json`
- `engine/argot/research/signal/phase14/experiments/import_graph_phase13_validation_scores.json`
- `engine/argot/research/signal/phase14/experiments/sequential_corrected_controls_2026_04_22_scores.json`
- `engine/argot/research/signal/phase14/experiments/sequential_corrected_controls_postfix_v2_2026_04_22_scores.json`
- `engine/argot/research/signal/phase14/experiments/sequential_import_bpe_phase13_validation_scores.json`
- `engine/argot/research/signal/phase14/experiments/sequential_import_bpe_robustness_2026_04_22_scores.json`

**On-main reference** (current scorer ground truth):
- `engine/argot/scoring/calibration/fix10_reference.json` on `main` (or wherever calibration fixtures live today)

**Era beat:** AST contrastive dead-end (13) → import-graph insight → two-stage `SequentialImportBpeScorer` + language adapters + production promotion (14). Ends at today.

- [ ] **Step 1: List all phase-13–14 sources (both paths)**

Run:
```bash
git ls-tree research/phase-14-pre-cleanup -r --name-only | \
  grep -E 'docs/research/scoring/signal/phase1[34]'

git ls-tree research/phase-14-import-graph -r --name-only | \
  grep -E 'engine/argot/research/signal/phase1[34]/experiments/'
```

- [ ] **Step 2: Read the dead-end doc first, then the breakthrough**

The era has two distinct halves — read in that order:
```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/signal/phase13/b_mlm_and_existing_2026-04-21.md
# then: whatever phase-13 synthesis exists
# then: phase-14 experiment docs from the tag
```

- [ ] **Step 3: Draft candidate headline figures (aim for 3–5)**

Examples of the shape (actual numbers from your reading):
- "AST contrastive dead-end: break_mean X on rich, no delta vs BPE-only"
- "Import-graph stage alone scores N% of foreign-module hunks"
- "Two-stage scorer break_mean X on faker / Y on fastapi vs single-stage baseline"
- "Production promotion: AUC / recall@k on calibration fixture"

At least one figure must be from the committed JSONs on `research/phase-14-import-graph`. Open them with:
```bash
git show research/phase-14-import-graph:engine/argot/research/signal/phase14/experiments/sequential_corrected_controls_2026_04_22_scores.json | head -80
```

- [ ] **Step 4: Spot-check each headline figure**

- For markdown claims backed by JSON: verify the two agree, cite the markdown.
- For JSON-only figures (no narrative doc): cite the JSON path directly.
- For figures about the *production* scorer (post-promotion), cross-check against `fix10_reference.json` on main via `cat engine/argot/scoring/calibration/fix10_reference.json` (or current equivalent). If the production figure and the research figure diverge, use the production figure for the "→ today" close and flag the research figure as the pre-promotion version.

- [ ] **Step 5: Draft the era doc per template**

```
# The import-graph breakthrough (phases 13–14)

## The hypothesis we were testing
## What we tried
## What the numbers said
## What broke the era
## → today
```

This era's "→ next" is not another era — it's the current production scorer. One line pointing the reader at the code path (`engine/argot/scoring/` on main) and at the README's "What's next" section.

- [ ] **Step 6: Apply noise-removal rules**

Spec-flagged case: phase 13 "has 25 docs; many re-describe the same dead-end from different angles. Pick the cleanest-numbered one, drop the rest." Apply this aggressively. The reader needs to know AST contrastive didn't work, not why it didn't work 25 times.

- [ ] **Step 7: Word count**

```bash
wc -w docs/research/04-import-graph-breakthrough.md
```
Target: 500–1000. >1200 → trim.

- [ ] **Step 8: Self-check acceptance**

```bash
rg '^## ' docs/research/04-import-graph-breakthrough.md
rg -i 'approximately|roughly|about [0-9]|around [0-9]' docs/research/04-import-graph-breakthrough.md
```

- [ ] **Step 9: Commit**

```bash
git add docs/research/04-import-graph-breakthrough.md
git commit -m "$(cat <<'EOF'
docs: research narrative — era 4 (import-graph breakthrough, phases 13–14)

AST contrastive dead-end, the import-graph insight, and the two-stage
SequentialImportBpeScorer that became the production scorer.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 5: Write `README.md` (index)

**Files:**
- Create: `docs/research/README.md`

Written last so it can accurately summarize and cross-link the 4 era docs.

- [ ] **Step 1: Re-read the 4 era docs you just wrote**

```bash
cat docs/research/01-jepa-era.md
cat docs/research/02-pivot-to-honest-eval.md
cat docs/research/03-bpe-signal-hunt.md
cat docs/research/04-import-graph-breakthrough.md
```

- [ ] **Step 2: Draft the README per spec structure**

Target 200–400 words, four parts:

```markdown
# argot research

## What argot does today
<One paragraph. argot is a style linter that learns a repo's voice from
its git history and scores new code by how far it diverges. The current
production scorer is the two-stage import-graph + BPE log-ratio
pipeline. One paragraph on the path that got there: JEPA → honest eval →
BPE signal hunt → import-graph breakthrough.>

## Timeline

| Era | Phases | Headline finding | Link |
|---|---|---|---|
| JEPA era | 1–6 | <one-line finding, taken from `01-jepa-era.md` §What the numbers said> | [01-jepa-era.md](01-jepa-era.md) |
| Honest eval | 7–9 | <one-line finding> | [02-pivot-to-honest-eval.md](02-pivot-to-honest-eval.md) |
| BPE signal hunt | 10–12 | <one-line finding> | [03-bpe-signal-hunt.md](03-bpe-signal-hunt.md) |
| Import-graph breakthrough | 13–14 | <one-line finding> | [04-import-graph-breakthrough.md](04-import-graph-breakthrough.md) |

## What lives on the tag
<Paragraph: detailed per-experiment docs (~90 markdown files and
committed result JSONs) are preserved on git tag
`research/phase-14-pre-cleanup`. Access without switching branches:>

```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/phase-7/16-rebaseline.md
git ls-tree research/phase-14-pre-cleanup -r --name-only | grep docs/research/scoring/
```

## What's next
<One line: the research Phase 14 code is scheduled for a clean merge
onto main in a separate PR. Point at the branch (`research/phase-14-import-graph`)
and the current production scorer (`engine/argot/scoring/` on main).>
```

- [ ] **Step 3: Word count**

```bash
wc -w docs/research/README.md
```
Target: 200–400. >500 → trim.

- [ ] **Step 4: Verify internal links resolve**

```bash
for f in 01-jepa-era.md 02-pivot-to-honest-eval.md 03-bpe-signal-hunt.md 04-import-graph-breakthrough.md; do
  test -f "docs/research/$f" && echo "OK: $f" || echo "MISSING: $f"
done
```
Expected: 4 × OK.

- [ ] **Step 5: Commit**

```bash
git add docs/research/README.md
git commit -m "$(cat <<'EOF'
docs: research narrative — index README

Four-part index: elevator pitch, timeline table cross-linking the era
docs, tag pointer for detailed per-experiment material, and a line on
the planned research-code merge onto main.

Co-Authored-By: Claude Sonnet 4.6 <noreply@anthropic.com>
EOF
)"
```

---

## Task 6: Final acceptance check + PR

**Files:**
- None modified; verification and PR creation only.

- [ ] **Step 1: File count and shape**

```bash
ls docs/research/
```
Expected output (exactly these five files, plus the pre-existing `scoring/` placeholder):
```
01-jepa-era.md
02-pivot-to-honest-eval.md
03-bpe-signal-hunt.md
04-import-graph-breakthrough.md
README.md
scoring/
```

- [ ] **Step 2: Word counts all in range**

```bash
wc -w docs/research/*.md
```
Expected:
- `README.md`: 200–400
- `01-jepa-era.md`: 500–1000 (hard ceiling 1200)
- `02-pivot-to-honest-eval.md`: 500–1000 (hard ceiling 1200)
- `03-bpe-signal-hunt.md`: 500–1000 (hard ceiling 1200)
- `04-import-graph-breakthrough.md`: 500–1000 (hard ceiling 1200)

- [ ] **Step 3: Success criterion #1 — stale JEPA references**

```bash
rg -i 'jepa|JEPA' docs/research/
```
Expected: matches appear only in `01-jepa-era.md` (the era explicitly about JEPA), in the README elevator pitch (one mention), and in the `02-pivot-to-honest-eval.md` hypothesis section (JEPA is the subject of the pivot). Any match in `03-*` or `04-*` should be the single JEPA-limits citation used to bridge eras, not stale/repeated mentions. If unexpected matches, trim.

- [ ] **Step 4: Success criterion #2 — every headline figure cited**

Per era doc, visually inspect the `## What the numbers said` table. Every data row must have an inline citation in `[\`path\` §section]` form. Fix any uncited row (add citation, or drop the row).

- [ ] **Step 5: Success criterion #3 — template shape**

```bash
for f in 01-jepa-era.md 02-pivot-to-honest-eval.md 03-bpe-signal-hunt.md 04-import-graph-breakthrough.md; do
  echo "=== $f ==="
  rg '^## ' "docs/research/$f"
done
```
Expected: each era doc has exactly 5 `## ` headers matching:
```
## The hypothesis we were testing
## What we tried
## What the numbers said
## What broke the era
## → next era      (or "## → today" for 04-import-graph-breakthrough.md)
```

- [ ] **Step 6: Success criterion #6 — no hedge words**

```bash
rg -i 'approximately|roughly|about [0-9]|around [0-9]' docs/research/
```
Expected: no matches. Any hit means a number came from memory. Trace or drop.

- [ ] **Step 7: Success criterion #5 — read top-to-bottom**

Read the 5 files in order (README, 01, 02, 03, 04). Check each "→ next era" transition holds: does the finding that broke era N match the hypothesis of era N+1? If not, fix the bridging paragraph — most often the problem is in "What broke the era" of the earlier doc, not "The hypothesis we were testing" of the later one.

- [ ] **Step 8: Success criterion #7 — verify repo checks**

```bash
just verify
```
Expected: green. Docs-only change; should be trivially true.

- [ ] **Step 9: Push branch and open PR**

```bash
git push -u origin docs/research-narrative-spec
gh pr create --title "docs: research narrative consolidation" --body "$(cat <<'EOF'
## Summary

Consolidates 14 phases of argot research into 5 narrative docs under `docs/research/`:

- `README.md` — index with elevator pitch + timeline + tag pointer
- `01-jepa-era.md` — phases 1–6
- `02-pivot-to-honest-eval.md` — phases 7–9
- `03-bpe-signal-hunt.md` — phases 10–12
- `04-import-graph-breakthrough.md` — phases 13–14 → production scorer

Each era doc follows the same four-beat template (hypothesis → tried → numbers → what broke → transition) and anchors headline figures on committed sources (markdown result tables on `research/phase-14-pre-cleanup`, JSONs on `research/phase-14-import-graph`). Detailed per-experiment docs (~90 files) stay reachable via the tag without a branch switch.

Docs-only change. No code, no test edits. Design spec at `docs/superpowers/specs/2026-04-23-research-narrative-design.md`.

## Test plan

- [x] Five files exist under `docs/research/`
- [x] Each era doc is 500–1000 words (hard ceiling 1200); README 200–400
- [x] Each era doc has the 5 template headers
- [x] Every headline figure in each `## What the numbers said` table has an inline citation
- [x] No hedge words (`approximately|roughly|about N|around N`) in any doc
- [x] JEPA mentions confined to eras 1–2 + README elevator pitch
- [x] `just verify` green

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Failure protocols (from spec — applied during execution)

- **Primary source for a headline figure is missing or ambiguous.** Drop the figure. Do not hedge. Rewrite the sentence if the narrative still works without the number; else drop the claim.
- **An era doc exceeds 1200 words after first draft.** Noise-removal rules weren't applied hard enough. Trim before moving on — do not start the next era with a bloated prior one unresolved.
- **An era's narrative requires a claim that crosses into another era to make sense.** Boundary is wrong for that specific claim; place the claim in the other era instead of awkwardly straddling.
- **The tag is unreachable or phase data is missing.** Stop and report — do not proceed with docs-only prose that can't be spot-checked.

## Self-review (completed inline during plan writing)

**Spec coverage:**
- Output structure (5 files, flat layout): Tasks 0–5 ✓
- Era boundaries table: Tasks 1–4, one per era ✓
- Per-era doc template (5 sections): Task N Step 5 + Task 6 Step 5 verification ✓
- Audience/style (JEPA unpack on first use): Task 1 Step 5 ✓
- Source material per era: Tasks 1–4 "Primary sources" sections (with corrections for the .argot/ gap) ✓
- Spot-check protocol: each era Step 4 + Task 6 Step 4 ✓
- Noise-removal rules: each era Step 6 ✓
- Top-level README (4 parts): Task 5 Step 2 ✓
- Execution model (single-agent sequential): implicit — tasks are sequential; no parallel fan-out ✓
- Success criteria (#1–#7): Task 6 Steps 3–8 ✓
- Failure protocol: documented at bottom ✓

**Placeholder scan:** No TBD/TODO/vague. `<actual numbers from your reading>` is a deliberate instruction to the writer (not a placeholder in the plan sense — the writer discovers the figures during Step 2/3 of each era task).

**Type consistency:** File names, tag name, branch name, and citation format `[\`path\` §section]` are consistent across all tasks.

**Scope check:** Single-plan scope — 5 related files, one PR. Not decomposable into separate plans; splitting would break the narrative coherence that the single-agent execution model is designed to preserve.
