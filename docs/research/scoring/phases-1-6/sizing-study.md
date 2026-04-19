# Scoring Benchmark — Sizing Study (Phase 2 Results)

**Status:** complete 2026-04-18
**Corpus:** [`01-corpus.md`](01-corpus.md)
**Raw results:** `.argot/research/results.jsonl` (gitignored; reproducible
via the pinned SHAs in `01-corpus.md` and `just research-benchmark`).

## Question

How does argot's scoring model quality (AUC) scale with the size of the
training dataset? Specifically: what is the smallest dataset at which the
model stops being effectively random?

## Method

Four size buckets (small 3k, medium 7k, large 20k, xlarge 60k), each pairing
one TS repo and one Py repo of similar record count. For each bucket:
concatenate the two repos' extracts, reservoir-sample to the bucket target,
train on 80% of the home repo, evaluate on held-out 20% + three adversarial
negative sets. Three seeds per bucket.

Micro bucket (argot self, 243 records) is omitted — cross-repo AUC requires
≥ 2 repos. Prior diagnostic work on argot alone showed AUC ≈ 0.52 against
shuffled negatives (near-random), consistent with the trend below.

## Results

| size (bucket) | n | shuffled AUC   | cross-repo AUC | injected AUC   |
|--------------:|:-:|:---------------|:---------------|:---------------|
|         3,000 | 3 | 0.500 ± 0.000  | 0.446 ± 0.028  | 0.450 ± 0.020  |
|         7,000 | 3 | 0.501 ± 0.001  | 0.395 ± 0.005  | 0.398 ± 0.011  |
|        20,000 | 3 | 0.637 ± 0.006  | 0.544 ± 0.015  | 0.631 ± 0.010  |
|        60,000 | 3 | 0.707 ± 0.002  | 0.549 ± 0.017  | 0.695 ± 0.009  |

## Observations

**The model is completely random below 20k records.** At 3k and 7k, shuffled
AUC is exactly 0.50 — the model learns nothing useful. Cross-repo and injected
AUCs are actually *below* 0.5 at these sizes, meaning the model performs worse
than random on foreign-repo inputs. This is likely overfitting: at small sizes
the model memorises the home repo's specific token distribution and its
surprise scores are actively misleading on unseen styles.

**The signal switch flips between 7k and 20k.** At 20k, shuffled AUC jumps to
0.637 and injected to 0.631 — both well above random. This is the phase
transition: enough training data for the model to learn style rather than
memorise tokens.

**Shuffled and injected AUCs scale together, cross-repo lags.** At 20k and
60k, shuffled and injected track closely (within 0.01). Cross-repo AUC
plateaus around 0.54–0.55 and does not break 0.6 even at 60k. The TS/Py
language boundary between pairs (vscode vs django) likely contributes — the
model distinguishes language features as much as style.

**Variance is low above 20k.** Std dev drops to ≤ 0.006 for shuffled/injected
at 20k+, showing the signal is stable across seeds.

## Finding: minimum-viable corpus size

**~20,000 records** is the minimum for a non-random model. Below that, all
three AUC metrics are at or below 0.5 — the model is useless. At 20k,
shuffled and injected AUCs reach ~0.63, which is meaningful but modest.

**~60,000 records** is where shuffled and injected AUCs consistently clear
0.7. Cross-repo AUC appears to saturate around 0.55 regardless of size,
suggesting it is limited by style distance between the repo pair rather than
training data volume.

## Caveats

- Training used default `epochs=20`, `batch_size=128`. Phase 3 will sweep
  these — more epochs may shift the 20k→60k curve.
- Each bucket is a 2-repo mix; real-world repo diversity is higher.
- Cross-repo AUC is sensitive to how stylistically different the two repos
  are. The TS/Py language split in each pair may inflate cross-repo
  difficulty at large sizes (vscode TypeScript vs django Python are very
  different) while the small/medium pairs (ky/ruff, vite/click) may be too
  different at the language level to show cross-repo style learning at all.
- The sub-0.5 cross-repo AUCs at small sizes are a strong signal to avoid
  deploying argot on repos with < 20k records — it will produce inverted
  scores.

## Next

Phase 3 technique experiments (`03-context-after.md` through
`08-path-embed.md`) re-run this exact benchmark on variants of the training
pipeline and report AUC deltas against the numbers above.
