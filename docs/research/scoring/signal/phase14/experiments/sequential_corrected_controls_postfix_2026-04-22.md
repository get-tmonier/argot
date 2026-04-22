# Phase 14 — Sequential Corrected Controls: Post-Fix Delta Note

**Date**: 2026-04-22
**Relates to**: Exp #6 Step 2

## Change applied

Stage 1 regex fallback removed. In `_imports_from_ast`, `SyntaxError` now returns
`set()` instead of delegating to `_imports_from_regex`. `_imports_from_regex` is
retained as dead code pending removal in a follow-up.

## Synthetic recall and FP: unchanged

Re-ran `sequential_corrected_controls_2026_04_22.py` unmodified after the fix.

| domain   | seeds | recall | FP (mean per seed) |
|----------|-------|--------|--------------------|
| FastAPI  | 5     | 100%   | ≤1 per seed (≤5%)  |
| rich     | 5     | 100%   | ≤1 per seed (≤5%)  |
| faker    | 1     | 100%   | 0%                 |

All 46 synthetic breaks detected. FP rate at or below the ≤5% per-seed bound from
the original exp #2c run. No regression introduced.

## Conclusion

Stage 1 regex fallback removal is safe. Synthetic signal is fully preserved.
Proceeding to Step 3: real-PR re-measurement with the fix applied.
