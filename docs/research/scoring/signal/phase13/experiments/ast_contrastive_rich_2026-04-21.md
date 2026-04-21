# Phase 13 Experiment 15 — AST Contrastive: Rich Domain Scoring (2026-04-21)

**Scorer:** `ContrastiveAstTreeletScorer(epsilon=1e-7, aggregation='max')` (depth-1/2/3 fixed in extractor)
**model_A:** 72 rich source files (rich/sources/model_a/)
**model_B:** generic_treelets.json (CPython 3.13 stdlib, depth-3 treelets)
**Fixtures:** same 10 breaks + 10 controls used in BPE-tfidf rich (exp #14) — directly comparable

FastAPI sanity check: AUC = **0.9742** ✓ (expected: [0.90, 1.00])

---

## 1. Overall and Per-Category AUC

| metric | value |
|---|---|
| **Rich overall AUC** | **0.2900** |
| FastAPI sanity AUC | 0.9742 ✓ |

**Per-category AUC** *(each break category vs all 10 controls)*:

| category | n_breaks | AUC |
|---|---|---|
| ansi_raw | 2 | 0.3000 |
| colorama | 2 | 0.6000 |
| curses | 2 | 0.3000 |
| print_manual | 2 | 0.2500 |
| termcolor | 2 | 0.0000 |

Every category is below 0.65.  `termcolor` scores AUC 0.0000 — controls literally outscore breaks in every pair.

---

## 2. Per-Fixture Scores

| fixture | category | is_break | score |
|---|---|---|---|
| control_console_capture | control_console | False | 7.0513 |
| control_console_init | control_console | False | 7.0513 |
| control_table_add_column_body | control_table | False | 5.8741 |
| control_table_rich_console | control_table | False | 5.8741 |
| control_live_enter_exit | control_live | False | 6.7923 |
| control_live_renderable | control_live | False | 6.7923 |
| control_text_init | control_text | False | 5.9265 |
| control_text_styled | control_text | False | 5.9265 |
| control_panel_init_body | control_panel | False | 5.1160 |
| control_panel_rich_console | control_panel | False | 5.1160 |
| break_ansi_raw_1 | ansi_raw | True | 4.0664 |
| break_ansi_raw_2 | ansi_raw | True | 6.0028 |
| break_colorama_1 | colorama | True | 6.3081 |
| break_colorama_2 | colorama | True | 6.3081 |
| break_termcolor_1 | termcolor | True | 2.6823 |
| break_termcolor_2 | termcolor | True | 4.0664 |
| break_curses_1 | curses | True | 6.3081 |
| break_curses_2 | curses | True | 4.0664 |
| break_print_manual_1 | print_manual | True | 5.7194 |
| break_print_manual_2 | print_manual | True | 5.8741 |

The majority of controls score **above** most breaks.  `control_console_capture` and
`control_console_init` both score 7.0513 — above every single break in the fixture set.

---

## 3. Side-by-Side vs BPE-tfidf Rich

| scorer | corpus | AUC | verdict |
|---|---|---|---|
| BPE-contrastive-tfidf | rich | **0.9900** | PASS (≥ 0.85) |
| AST-contrastive | rich | **0.2900** | FAIL (< 0.65) |
| AST-contrastive | faker | 1.0000* | CLEAN (all 5 > max cal) |
| AST-contrastive | click | 0.2500 | FAIL |
| AST-contrastive | FastAPI (sanity) | 0.9742 | PASS |

\* Faker used separation-from-calibration metric, not AUC over paired fixtures; CLEAN = all breaks above calibration max.

The failure on rich (AUC 0.2900) is nearly identical to the failure on click (AUC 0.2500)
despite the rich corpus being 72 files vs click's 13 files.  **Corpus size cannot explain this.**

---

## 4. Diagnostic: Why AST Fails on Rich

The root cause is **structural homogeneity of the rich domain itself**.

Rich is a terminal rendering library.  Its source files are full of:
- `Segment`/`Style`/`Text`/`Color` class bodies with method chains
- `__rich_console__` protocol implementations
- `@dataclass` and `@property` patterns everywhere

These structural patterns produce high AST treelet scores against the generic CPython stdlib
(`model_B`), because they are idiomatic Python but *exotic* from a stdlib perspective
(the stdlib does not use rich's `__rich_*` protocols, `Segment` dataclasses, or its console-output pattern).

Consequence: **controls drawn from rich source score just as high as breaks**, because
controls also contain rich's unusual structural idioms.  The scorer cannot distinguish
a `print("\x1b[31m...")` break from a `Console.__init__` control when both contain
exotic-to-stdlib treelets.

The faker success (all 5 breaks above calibration max) came from a different mechanism:
faker is mostly **data tables** (locale string lists, provider class stubs with `def word()`),
which are structurally plain and score near 0 on the AST axis.  Only the breaks introduced
exotic structural patterns.  The contrast was real because the background was flat.

Rich has no flat background.  Its own source code is structurally unusual by stdlib
standards, so the AST treelet contrast axis produces uniformly high scores for both
breaks and controls.

---

## 5. Verdict

**AUC 0.2900 < 0.65 — AST-contrastive is GENUINELY FRAGILE cross-domain.**

The click "FastAPI-tuned" verdict was correct.  The faker result was the fluke —
a domain where the background corpus happened to be structurally flat (data tables),
giving the scorer an easy separation problem.

Corpus starvation does **not** explain the failures:
- Click: 13 files → AUC 0.2500
- Rich: 72 files → AUC 0.2900

More data does not help when the scorer's failure mode is domain-level structural
homogeneity, not sample size.  The AST contrast axis conflates "structurally unusual
for the stdlib" with "structurally unusual for the repo" — those are only the same
thing in special cases (e.g., FastAPI, faker data tables).

| condition | AUC threshold | result |
|---|---|---|
| AST generalises; click was corpus-starved | ≥ 0.85 | **NO** (0.2900) |
| Partial; corpus-starvation plausible | [0.65, 0.85) | **NO** |
| AST genuinely fragile cross-domain | < 0.65 | **YES** ← |

---

## 6. Phase 13 Updated Recommendation

**Do not promote AST-contrastive as a production scorer.**

The faker result (exp #13) demonstrated that the scorer *can* work on a flat-background
corpus.  This experiment and the click rerun confirm that the scorer fails on domains
where the model_A source itself is structurally unusual by stdlib standards.

The scorer fails silently — it produces high scores for both breaks and controls,
so no threshold separates them.  This is the worst failure mode for a production scorer.

**Production recommendation:** BPE-contrastive-tfidf is the current winner on the
two domains where it has been tested (FastAPI AUC 1.00, rich AUC 0.99).  The minimum
viable corpus size appears to be ~50–72 files; click (16 files) and the click-equivalent
small corpora should show a warning to users.

**Next step candidates:**
- Ensemble of BPE-tfidf (reliable) + AST (complementary on flat-background corpora):
  evaluate whether the AST axis adds signal *on top of* BPE-tfidf for the faker domain
  without degrading the rich/FastAPI results.
- CodeBERT MLM surprise: context-conditional perplexity as a zero-training baseline
  that does not depend on model_A structural homogeneity.
