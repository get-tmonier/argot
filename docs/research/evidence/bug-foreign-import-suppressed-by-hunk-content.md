# Bug (FIXED): a foreign import hit suppressed by an import-gated call-receiver

Found while rebuilding the demo GIF (2026-07-06); fixed the same day.

## Symptom
A new file with a foreign `django` import read **clean** when it also contained
attested call content:

```python
from django.views import View
from django.http import JsonResponse, HttpResponseNotFound

class ReceiptView(View):
    def get(self, request, user_id):
        receipt = self.repo.find(user_id)      # ← this line suppressed the hit
        if receipt is None:
            return HttpResponseNotFound()
        return JsonResponse(receipt.to_dict())
```

Simpler variants (imports only, empty class, a method returning `JsonResponse({})`)
all fired; adding `self.repo.find(...)` / `.to_dict()` made it clean.

## Root cause
Debug trace (`sequential.rs::score_hunk`):

- `import_score=1` (django foreign) → `import_fired=true` → an Import candidate is
  always pushed.
- `self.repo.find` / `.to_dict()` produce a spurious call-receiver contribution
  (5.0, via the cluster branch on the repo's own attested callees) → `adjusted_bpe
  10.28 > threshold 7.79`.
- Because a foreign import is present, `cr_fired` became true via the
  `import_score >= IMPORT_THRESHOLD` gate — so a **CallReceiver candidate** was
  pushed alongside Import.
- Candidates: `[(Import, ratio 1.0), (CallReceiver, ratio 1.32)]`. **CallReceiver
  won the ratio tiebreak.** Its callee evidence is empty (those callees are not
  foreign), so `check.rs` dropped the whole hit → "clean", losing the valid
  foreign-import flag.

## Fix
`crates/argot-core/src/scoring/sequential.rs`: the call-receiver *reason* now
carries a hunk only on a **genuine foreign-callee** signal (hunk reach / foreign
binding / explicit namespace). A foreign *import* still opens the surprisal gate,
but no longer makes call-receiver a competing candidate — the Import reason names
that dependency. Removed `import_score >= IMPORT_THRESHOLD` from the `cr_fired`
push condition. Regression test:
`foreign_import_wins_over_import_gated_call_receiver`.

## Bench (A/B: production recall all corpora + holdout FP rich/hono/ripgrep)
- Recall: baseline 640/881, fix 640/881 — **0 caught→uncaught, 0 changes** (gated
  85.1% identical). No bench fixture exercised the exact pattern; the fix is
  real-world only.
- Over-fire: baseline == fix, byte-identical (rich 0.88%, hono 0.00%, ripgrep
  0.61%), all ≤0.98%.
