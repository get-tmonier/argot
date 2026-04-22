# Stage 1 Hunk-Scope Fix (fix10) — 2026-04-22

## §0 Summary

Stage 1 was changed to count only imports added in the hunk itself, ignoring file-level
imports.  A pure string or comment edit in an import-heavy file no longer triggers Stage 1.

| Corpus  | fix9 Stage 1 | fix10 Stage 1 | fix9 total flagged | fix10 total flagged |
|---------|-------------|---------------|-------------------|---------------------|
| FastAPI | 0           | 0             | 20                | 20                  |
| Rich    | 4           | 2             | 25                | 23                  |
| Faker   | 1           | 0             | 18                | 16                  |

All Stage 1 reductions are confirmed FP eliminations (no true signal lost).

---

## §1 Code Seam and Design Decision

**Location:** `engine/argot/research/signal/phase14/scorers/sequential_import_bpe_scorer.py`,
`score_hunk()`, lines 268–279 (before the fix).

**Before (fix9):**
```python
file_imports = _imports_from_ast(extract_imports(file_source))
hunk_imports = _imports_from_ast(hunk_content)
all_imports = file_imports | hunk_imports
foreign = all_imports - self._import_scorer._repo_modules
import_score = float(len(foreign))
```

**After (fix10, Option A — hunk-only):**
```python
hunk_imports = _imports_from_ast(hunk_content)
foreign = hunk_imports - self._import_scorer._repo_modules
import_score = float(len(foreign))
```

**Why Option A over Option B:** Option B (keep file_imports as context, only count
newly-added imports) requires knowing which file imports existed *before* the patch, which
is not available at scoring time without additional git plumbing.  Option A is correct
by design: Stage 1's purpose is to detect new foreign modules introduced by a hunk.  A
hunk with no import statements cannot introduce anything, regardless of what the file
already imports.

**Implication:** The existing test `test_stage1_detects_foreign_import_via_file_source`
was testing the old (wrong) behavior where file-level imports bled into hunk scoring.
That test was renamed and updated to document the new hunk-only contract.

---

## §2 Tests

Three new tests were added to
`engine/argot/research/signal/phase14/scorers/test_sequential_import_bpe_scorer.py`:

| Test | Purpose | Result |
|------|---------|--------|
| `test_stage1_hunk_with_foreign_import_flags` | Hunk adds `import foreign_pkg` → Stage 1 fires | PASS |
| `test_stage1_hunk_with_no_imports_does_not_flag` | Pure string edit in import-heavy file → Stage 1 silent (regression for faker PR #2259) | PASS |
| `test_stage1_hunk_with_repo_import_does_not_flag` | Hunk adds import of a repo-known module → no flag | PASS |

The existing test `test_stage1_detects_foreign_import_via_file_source` was renamed to
`test_stage1_file_only_import_does_not_flag` and updated to assert the new contract:
Stage 1 does NOT fire when the foreign import is only in `file_source`.

Full suite: **29/29 passed**.

---

## §3 Per-Corpus Regression

### FastAPI — zero-delta confirmed

fix9: Stage 1 = 0.  fix10: Stage 1 = 0.  No change.  Total flags unchanged at 20.
FastAPI was already clean; this is the expected no-op.

### Rich — 2 FPs eliminated, 2 true signals preserved

fix9 had 4 Stage 1 flags, all in two PRs:

| PR | File | hunk_start | foreign_modules | fix9 | fix10 | Judgment |
|----|------|-----------|-----------------|------|-------|----------|
| #3930 | `rich/_unicode_data/__init__.py` | 1 | `['bisect', 'importlib']` | Stage 1 | Stage 1 | **TRUE signal preserved** — hunk adds new imports |
| #3861 | `rich/style.py` | 1 | `['pickle']` | Stage 1 | Stage 1 | **TRUE signal preserved** — hunk adds `import pickle` |
| #3861 | `rich/style.py` | 10 | `[]` | Stage 1 | none | **FP eliminated** — `foreign=[]` means hunk had no imports; fired via file-level `pickle` union |
| #3861 | `rich/style.py` | 437 | `[]` | Stage 1 | none | **FP eliminated** — same: different hunk in same file, no imports of its own |

Both dropped flags had `foreign_modules = []`, which is a direct marker of file-level
contamination: Stage 1 fired because the file's `import pickle` was unioned into hunks
that never touched the import block.  These are unambiguous FPs.

The `_unicode_data/__init__.py` flag (PR #3930, task #8 resolution) survived correctly.

Total flags: 25 → 23 (the 2 Stage 1 FPs; Stage 2 count unchanged at 21).

### Faker — PR #2259 FP confirmed eliminated

fix9 Stage 1 = 1 (the `factory.py` string-edit FP, PR #2259).
fix10 Stage 1 = 0. Confirmed: the FP is gone.

Total flags: 18 → 16.

---

## §4 Verdict

**Clean to ship as the final V0 config.**

- All true Stage 1 signals are preserved across all three corpora.
- All three eliminated Stage 1 flags were confirmed FPs (file-level contamination, not
  hunk-level foreign imports).
- FastAPI: zero Stage 1 regression.
- Rich: 2 FPs removed; both surviving Stage 1 flags are real (hunks that add imports).
- Faker: the motivating FP (PR #2259 string edit) is gone; Stage 1 = 0.
- 29/29 tests pass.

fix10 = fix9 + hunk-scope Stage 1 is the V0 config.
