# Issue #92 — held-out generalization check (scrapy)

**Question:** are the scorer fixes and the ~99%-visible / ~0-over-fire numbers
overfit to the 27 corpora they were developed against?

**Method:** add a corpus **never touched** while tuning the scorer or authoring
the other catalogs — `scrapy` (Python, web-scraping / Twisted; 446 py files,
11,194 commits, pinned `dd10cb8e`). Author 24 difficulty-graded foreign fixtures
with the same discipline (0-usage-verified at the SHA, easy/medium/hard), then
run the exact production catch + leak-free temporal holdout used everywhere else.

## Result — generalizes cleanly, not overfit

| Metric | Training set (27 corpora) | Held-out scrapy |
|---|---|---|
| Visible-foreign catch (easy+medium) | 522/527 (99%) | **18/18 (100%)** |
| Hard-tier (masked foreign) | 24/106 (22.6%) | 0/6 |
| Over-fire — existing (true FP) | 0.23% agg / 0.98% worst | **0.00% (0/1628)** |
| Over-fire — new-file | 0.00% | **0.00% (0/55)** |
| Detection — existing (argot working) | — | 0.68% (11/1628): call_receiver 7 + import 4 |

- **Catch generalizes.** Visible-foreign is 100% on an unseen repo/domain with
  unseen dependencies — the detection capability is not memorised. The hard tier
  misses uniformly (0/6) via the two corpus-independent masking techniques
  (importlib-hidden imports; aliased import + attested-leaf collision), exactly
  the documented statistical limit, not a scrapy-specific failure.
- **Over-fire generalizes.** 0.00% on 1,628 unseen existing-file hunks and 55
  new-file hunks — the amplification removal, the binding-scoped call-receiver
  gate, the PHP file-context callee recovery, and the `__future__` fix all hold
  on data they were never tuned against. Had any been fit to the 27 corpora,
  scrapy would show elevated over-fire; it is zero.
- The 11 existing-file flags scrapy does produce are **detections** — argot
  correctly flagging genuinely-new dependencies/APIs in real scrapy commits
  (via call_receiver / import), the co-headline behaviour, not false alarms.

## Verdict

The false-alarm result is the sharper overfit signal (the FP fixes were the part
most at risk of being tuned to these repos), and it comes back at **0.00%** on a
held-out corpus. Combined with 100% visible-foreign catch, this is direct
evidence the model is **not overfit** to the training set.
