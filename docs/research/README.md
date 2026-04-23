# argot research

## What argot does today

argot is a style linter that learns a repo's voice from its git history
and scores new code by how far it diverges. The current production
scorer is a two-stage pipeline: `ImportGraphScorer` flags hunks that
introduce modules never seen in the repo, then a BPE log-ratio scorer
catches stdlib-only breaks against a per-repo calibration threshold.
The path here was not direct. Four eras in sequence: a JEPA-based
neural scorer that plateaued on a misleading eval, a pivot to
honest evaluation that ruled out three architecturally distinct
approaches, a token-frequency signal hunt that replaced JEPA with
zero-training `tfidf_anomaly`, and finally the import-graph
breakthrough that cleared the production gate by combining two
structurally different signals in series.

## Timeline

| Era | Phases | Headline finding | Link |
|---|---|---|---|
| JEPA era | 1–6 | Wins did not compound and cross-repo AUC was measuring language detection, not style — best honest metric (shuffled AUC) plateaued at 0.713 | [01-jepa-era.md](01-jepa-era.md) |
| Honest eval | 7–9 | Three architectures (from-scratch encoders, density heads, frozen pretrained) all failed the 0.85 gate at 0.48–0.58 — no training signal anywhere targeted mutations | [02-pivot-to-honest-eval.md](02-pivot-to-honest-eval.md) |
| Token-frequency signal hunt | 10–12 | Zero-training `tfidf_anomaly` beat the JEPA ensemble (AUC 0.6968 vs 0.6532) and was promoted as the new default, but stalled short of the 0.80 gate | [03-bpe-signal-hunt.md](03-bpe-signal-hunt.md) |
| Import-graph breakthrough | 13–14 | `SequentialImportBpeScorer` flagged 46/46 breaks with 0 FP across 189 calibration+control hunks — era ended with promotion, not pivot | [04-import-graph-breakthrough.md](04-import-graph-breakthrough.md) |

## What lives on the tag

Detailed per-experiment docs (~90 markdown files and committed result
JSONs) are preserved on git tag `research/phase-14-pre-cleanup`. Access
without switching branches:

```bash
git show research/phase-14-pre-cleanup:docs/research/scoring/phase-7/16-rebaseline.md
git ls-tree research/phase-14-pre-cleanup -r --name-only | grep docs/research/scoring/
```

## What's next

Phase 14 research code is scheduled for a clean merge onto main in a
separate PR from branch `research/phase-14-import-graph`. The current
production scorer lives at `engine/argot/scoring/` on main.
