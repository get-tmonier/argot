# The import-graph breakthrough (phases 13–14)

## The hypothesis we were testing

Era 3 closed with `tfidf_anomaly` promoted over JEPA on the strength of
+0.0436 AUC — a clean win, but one that stalled at overall AUC 0.6968
against the 0.80 gate, with `background_tasks` still inverted and the
simplex blend refusing to mix `tfidf_anomaly` with any other scorer
[`docs/research/03-bpe-signal-hunt.md` §What broke the era]. The
implication was mechanical: token frequency had taken the scorer as far
as it could, and whatever came next had to draw signal from a
structurally different place than the hunk tokens themselves.

Phase 13 picked up that thread with an AST-structural answer — a
contrastive treelet scorer that compared the hunk's subtree frequencies
against a repo-fit model and the generic Python stdlib. If BPE captured
"what words are rare", AST treelets would capture "what *shapes* are
rare", and the two axes would complement each other.

## What we tried

- **Phase 13 — `ContrastiveAstTreeletScorer`.** Depth-3 AST treelets,
  `max` aggregation, log-ratio of hunk treelet frequency against a
  repo-specific `model_A` over a stdlib `model_B`. Validated with a
  full three-tier gauntlet on FastAPI (smoke, LOO stability, wrong-contrast
  Django substitution) before promotion.
- **Phase 13 — Cross-domain validation on rich and click.** Same scorer,
  swapped corpora; both with size-matched rerun controls to rule out a
  methodology artefact.
- **Phase 13 — Ensemble with BPE-tfidf on faker.** `max(bpe, ast)` over
  5 faker paradigm-breaks, expressly to see whether the AST axis rescued
  the `mimesis_alt` break that BPE had missed (score 4.20, below calibration p90).
- **Phase 14 — `ImportGraphScorer`.** A single-line idea: count
  top-level module imports in the hunk that were never seen in `model_A`.
  Zero training; one pass over the source tree.
- **Phase 14 — `SequentialImportBpeScorer`.** Two stages in series —
  Stage 1 imports flags any hunk with ≥1 foreign module, Stage 2 BPE
  catches the rest against a per-repo calibration threshold.
- **Phase 14 — Language adapters + TS bring-up.** Refactor to a
  `LanguageAdapter` seam, then validate on TypeScript repos (hono, ink,
  faker-ts) through the same calibration protocol.

## What the numbers said

| experiment | key result | citation |
|:-----------|:-----------|:---------|
| 13 AST on FastAPI (in-domain) | `ast_contrastive_max` AUC 0.9742, beats tfidf by +0.27; LOO mean 0.9619 | [`docs/research/scoring/signal/phase13/b_mlm_and_existing_2026-04-21.md` §Summary] |
| 13 AST on rich | overall AUC 0.2900 — `termcolor` at 0.0000, controls outscore breaks | [`docs/research/scoring/signal/phase13/experiments/ast_contrastive_rich_2026-04-21.md` §1] |
| 13 AST on click (matched rerun) | AUC 0.2500, both FAIL conditions met, abandonment upheld | [`docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md` §Verdict] |
| 14 ImportGraphScorer on phase-13 fixtures | 67% combined break recall, 0% FP, 100% on faker, `faker_hunk_0047` correctly ignored | [`docs/research/scoring/signal/phase14/experiments/import_graph_phase13_validation_2026-04-22.md` §5] |
| 14 Sequential Import→BPE | 100% recall on 46 breaks, 0 FP across 189 controls, all three domains STRONG | [`docs/research/scoring/signal/phase14/experiments/sequential_import_bpe_phase13_validation_2026-04-22.md` §7] |
| 14 Corrected-controls protocol (5 seeds) | threshold CV 3.5% FastAPI / 3.8% rich, recall 100%, FP 1% | [`docs/research/scoring/signal/phase14/experiments/sequential_corrected_controls_postfix_v2_2026-04-22.md` §8] |

Three findings changed the next move.

**AST contrastive was FastAPI-tuned, not general.** The in-domain result
(AUC 0.9742) was so strong it passed all three Phase 13 Stage-2 gates —
smoke, leave-one-out over 20 control files (min 0.9315, mean 0.9619),
and Django wrong-contrast (0.5323, Δ −0.4419) — and we promoted it to
cross-domain validation. It collapsed immediately: AUC 0.2900 on rich
[`docs/research/scoring/signal/phase13/experiments/ast_contrastive_rich_2026-04-21.md`
§1] and 0.1187 on click, then 0.2500 on click after a methodology-fixed
rerun with a larger `model_A` and size-matched controls
[`docs/research/scoring/signal/phase13/stage3_tier3_click_matched_2026-04-21.md`
§Comparison to v1]. The rerun hardened the verdict: the scorer is
fragile to corpus size, and on small or lexically-generic repos the
control hunks score as anomalously as the breaks.

**The import-graph insight.** The FN patterns on AST and BPE pointed at
the same class of breaks — ones that introduced foreign libraries
(`import flask`, `import colorama`, `import mimesis`). A scorer that
counted "modules in this hunk we've never seen in this repo" needed no
training, no calibration, and no model at all. It scored 100% recall on
faker's 5 breaks, 60–65% on FastAPI and rich, 0% FP across every
validation set, and — crucially — returned 0 on `faker_hunk_0047`, the
error-handling hunk whose BPE score had caused faker's FULL OVERLAP
verdict in era 3
[`docs/research/scoring/signal/phase14/experiments/import_graph_phase13_validation_2026-04-22.md`
§3].

**Two axes, combined in series, cleared the gate.** The
`SequentialImportBpeScorer` runs the import check first — a fast, high-precision
pre-filter — and falls through to BPE for stdlib-only breaks. On the
46-break phase-13 fixture set, it flagged all 46 with 0 false positives
across 189 calibration/control hunks, with `faker_hunk_0047` correctly
suppressed at `bpe_score = threshold`
[`docs/research/scoring/signal/phase14/experiments/sequential_import_bpe_phase13_validation_2026-04-22.md`
§4]. Robustness under 5-seed random calibration held at 100% recall on
all three domains with threshold CV 3.5% (FastAPI) and 3.8% (rich) once
the fixture-vs-source distribution mismatch was corrected
[`docs/research/scoring/signal/phase14/experiments/sequential_corrected_controls_postfix_v2_2026-04-22.md`
§8].

## What broke the era

Nothing broke — the era ended with a promotion, not a pivot. The two
constraints that had defined the previous three eras (no GPU, no
production-time training) now lined up with the winning architecture
for free: Stage 1 is a single AST pass, Stage 2 is BPE surprise against
a calibration sample of the same repo. The `LanguageAdapter` refactor
generalised both stages across Python and TypeScript without changing
the scorer core, and the TS validation on hono returned 0 source flags
across 22 hunks from 5 PRs
[`docs/research/scoring/signal/phase14/experiments/ts_validation_hono_2026-04-22.md`
§0].

## → today

`SequentialImportBpeScorer` is the current production scorer, living at
`engine/argot/scoring/scorers/sequential_import_bpe.py`; its language
seam is in `engine/argot/scoring/adapters/`. For where the scorer is
going next, see the research README's "What's next" section.
