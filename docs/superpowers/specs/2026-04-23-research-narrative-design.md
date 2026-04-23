# Research Narrative Consolidation — Design

## Goal

Consolidate 14 phases of chaotic `argot` research into a clean, fact-based narrative on `main` under `docs/research/`. The arc runs from the original JEPA architecture (phases 1–6) through an honest-eval pivot (7–9), a BPE signal hunt (10–12), and ends at the import-graph breakthrough and production scorer (13–14).

Source material lives on the `research/phase-14-pre-cleanup` git tag (the pre-filter-repo snapshot) — ~90 existing phase docs under `docs/research/scoring/` plus raw result data. The current `main` has an empty `docs/research/` placeholder.

Downstream goal (explicitly out of this spec): enable a future PR that merges the cleaned research code directly onto `main`. This spec produces only the narrative docs.

## Non-goals

- Porting existing phase docs to `main`. They stay reachable via the tag.
- Regenerating corpora or re-running experiments. Spot-checks use existing artifacts only.
- Touching research code on `main`. Docs-only change.
- Writing a blog post. Blog is downstream of this work, not part of it.

## Output structure

Five files on `main`, flat layout:

```
docs/research/
  README.md                          # top-level narrative + index
  01-jepa-era.md                     # phases 1–6
  02-pivot-to-honest-eval.md         # phases 7–9
  03-bpe-signal-hunt.md              # phases 10–12
  04-import-graph-breakthrough.md    # phases 13–14 → today's scorer
```

No sub-directories. No appendix tree. Detailed per-experiment docs stay on the `research/phase-14-pre-cleanup` tag — the README points readers there when they want receipts.

## Era boundaries

| Era | Phases | What defines the boundary |
|---|---|---|
| 1. JEPA era | 1–6 | Original embeddings + predictor architecture; six phases of tuning (context, embed dim, n-grams, path embed, token embeddings, transformer encoder). Ends when the approach had been wrung out. |
| 2. Honest eval | 7–9 | Rebaseline + density heads + pretrained JEPA. The phase where honest comparison against trivial baselines confronted whether JEPA was adding signal. |
| 3. BPE signal hunt | 10–12 | Search for signal JEPA wasn't providing. Corpus analysis, context variants, first BPE log-ratio experiments. |
| 4. Import-graph breakthrough | 13–14 | AST contrastive dead-end (13), then the import-graph insight, then the two-stage scorer + language adapters + production promotion (14). Ends at today. |

These boundaries match the existing `docs/research/scoring/` tree on the tag — the research was already clustered this way.

## Per-era doc template

Each era file follows the same shape:

```
# <Era title>

## The hypothesis we were testing
<1–2 paragraphs: what we believed going in and why>

## What we tried
<2–5 experiments max, each one sentence + key numbers>

## What the numbers said
<Headline figures table — ~3–5 rows — with spot-check citations>

## What broke the era
<1–2 paragraphs: the finding that forced the next era>

## → next era
<1 line transition>
```

The four-beat structure (hypothesis → tried → numbers → what broke) forces a red line per era. It also forces fact selection: if an experiment doesn't fit one of the beats, it's probably noise and gets dropped.

Target length: **500–1000 words per era doc**. Past 1200, noise-removal wasn't applied hard enough.

## Audience and style

Technical but readable — written for yourself + future collaborators. Assumes the reader knows what style linting is. Unpacks ML jargon on first use:

> JEPA — a joint embedding predictive architecture, i.e., a model that embeds context and target into the same vector space and learns to predict the target from the context.

Rich on numbers. Plain English over jargon where either works.

## Source material per era

Accessed via `git show research/phase-14-pre-cleanup:<path>` — no branch switching.

| Era | Primary source docs | Primary result data |
|---|---|---|
| 1. JEPA era | `docs/research/scoring/DESIGN-phases-1-6.md`, `docs/research/scoring/phases-1-6/*.md` (03→14, synthesis, sizing-study, corpus) | `.argot/phase1_results/` → `phase6_results/` |
| 2. Honest eval | `docs/research/scoring/DESIGN-phase-7.md`, `docs/research/scoring/phase-7/*.md`, `docs/research/scoring/phase-8/spot-check.md` | `.argot/phase7_results/` → `phase9_results/` |
| 3. BPE signal hunt | `docs/research/scoring/signal/phase10_*.md`, `phase11/*.md`, `phase12/*.md`, `signal/jepa_detection_limits.md` | `.argot/phase10_results/` → `phase12_results/` |
| 4. Import-graph breakthrough | `docs/research/scoring/signal/phase13/*.md`, `signal/phase14/experiments/*.md`, `refactor-notes.md` on `pre-work/inventory` branch | `.argot/phase13_results/`, `phase14_results/`, `fix10_reference.json` on `main` |

## Spot-check protocol ("pragmatic" rigor)

Per era, identify the **headline figures** — the ~3–5 numbers the narrative load-bears on. Examples:

- "JEPA plateaued at AUC X on fastapi"
- "N-grams gave +Y AUC vs BoW baseline"
- "Import-graph stage alone scores Z% of foreign hunks"

For each headline figure: trace to the primary source (result JSON, experiment script output, or a docstring / assertion in the `.py` file). Quote the source path inline:

> JEPA plateaued at AUC 0.57 on fastapi ([`phase-7/16-rebaseline.md` §Results]).

If a figure can't be verified because the primary source is ambiguous or missing:

- **Drop the number entirely.** Don't quote it.
- Rewrite the sentence without the number if the narrative still works; else drop the claim.
- Never fabricate, round up, or say "approximately X" when X came from memory.

Non-headline figures (tangential mentions, order-of-magnitude color) can be quoted from the existing docs without spot-check, but flagged with a `¹` footnote: *"figure as reported in `<source doc>`"*.

## Noise-removal rules

Drop from each era on the way to the final narrative:

1. **Experiments whose outcome didn't inform the next move.** If a sweep lost to a variant that was already planned regardless, the sweep is noise.
2. **Repeated framings of the same finding.** Phase 13 has 25 docs; many re-describe the same dead-end from different angles. Pick the cleanest-numbered one, drop the rest.
3. **Stages that were plumbing, not research.** Rerun plans, execution logs, fixture audits — these are engineering overhead, not findings.
4. **Hyperparameter sweeps within an era that produced no frontier shift.** One line in "what we tried", never a section.

Rule of thumb: if cutting the experiment leaves the hypothesis / finding / transition intact, it was noise.

## Top-level `docs/research/README.md`

Short (~200–400 words). Four parts:

1. **Elevator pitch** — one paragraph on what argot does today and the path that got there.
2. **Timeline** — 4-row table: era · phases · one-line finding · link.
3. **What lives on the tag** — one paragraph explaining detailed per-experiment docs stay on `research/phase-14-pre-cleanup` with a `git show` example.
4. **What's next** — one line pointing at the planned research-code merge onto `main`.

The README is written **after** all four era docs, so it can accurately summarize and cross-link.

## Execution model

**Single-agent sequential pass** — one agent writes all 5 docs in era order. Rationale:

- Narrative coherence matters. Parallel agents would produce disjoint voice; the "what broke this era → next era hypothesis" transitions would crack.
- Later eras reference earlier eras (the import-graph breakthrough lands harder framed against JEPA's plateau).
- The agent writes the index README last, after era drafts exist, so it can cross-link faithfully.

Estimated effort: ~2000 lines of existing docs consumed, ~3500 words produced across 5 files.

## Success criteria

1. `rg -i 'jepa|JEPA' docs/research/` returns only intentional historical mentions in the era docs.
2. Every headline figure has an inline source citation (tag path).
3. Each era doc fits the template exactly (hypothesis → tried → numbers → what broke → transition).
4. Each era doc is 500–1000 words.
5. Reading the 5 files top to bottom tells a coherent story; transitions hold up.
6. No unverified numbers. Every figure either has a citation or was dropped.
7. `just verify` green (docs-only change; should be trivially true).

## Failure protocol

- **Primary source for a headline figure is missing or ambiguous.** Drop the figure per spot-check protocol. Do not hedge.
- **An era doc exceeds 1200 words after first draft.** Noise-removal rules weren't applied hard enough. Trim before moving on to the next era.
- **An era's narrative requires a claim that crosses into another era to make sense.** Boundary is wrong for that specific claim; place the claim in the other era instead of awkwardly straddling.
- **The tag is unreachable or `.argot/phaseN_results/` paths don't exist on it.** Stop and report — the narrative can't be fact-based without the primary data. Don't proceed with docs-only prose that can't be spot-checked.

## Deliverable

One PR against `main` adding the 5 files under `docs/research/`. Docs-only change. No code, no test edits.
