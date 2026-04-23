# argot research

> **How a GPU-hungry neural scorer became a 180-line statistical pipeline.**
> Four eras, three dead ends, one breakthrough — 14 phases of experiments
> condensed into four short narratives and 27 evidence docs.

## What argot does today

argot is a style linter that learns a repo's voice from its git history
and scores new code by how far it diverges. The current production
scorer is a two-stage pipeline: `ImportGraphScorer` flags hunks that
introduce modules never seen in the repo, then a BPE log-ratio scorer
catches stdlib-only breaks against a per-repo calibration threshold.

The path here was not direct.

```mermaid
flowchart LR
    E1["<b>Era 1 — JEPA</b><br/>neural scorer,<br/>6 sweeps<br/>(phases 1–6)"]
    E2["<b>Era 2 — Honest eval</b><br/>3 architectures<br/>all fail 0.85 gate<br/>(phases 7–9)"]
    E3["<b>Era 3 — BPE</b><br/>tfidf beats JEPA<br/>stalls at 0.6968<br/>(phases 10–12)"]
    E4["<b>Era 4 — Import-graph</b><br/>100% recall<br/>0 FP, PROMOTED<br/>(phases 13–14)"]

    E1 -->|"eval was measuring<br/>language detection"| E2
    E2 -->|"no training signal<br/>targets mutations"| E3
    E3 -->|"token frequency hit<br/>its ceiling"| E4

    style E4 fill:#d4edda,stroke:#28a745,stroke-width:2px
    style E1 fill:#f8d7da,stroke:#dc3545
    style E2 fill:#f8d7da,stroke:#dc3545
    style E3 fill:#fff3cd,stroke:#ffc107
```

## Timeline

| Era | Phases | Headline finding | Link |
|---|---|---|---|
| **JEPA era** | 1–6 | Wins did not compound and cross-repo AUC was measuring language detection, not style — best honest metric (shuffled AUC) plateaued at 0.713 | [01-jepa-era.md](01-jepa-era.md) |
| **Honest eval** | 7–9 | Three architectures (from-scratch encoders, density heads, frozen pretrained) all failed the 0.85 gate at 0.48–0.58 — targeted mutations carried no detectable training signal | [02-pivot-to-honest-eval.md](02-pivot-to-honest-eval.md) |
| **Token-frequency signal hunt** | 10–12 | Zero-training `tfidf_anomaly` beat the JEPA ensemble (AUC 0.6968 vs 0.6532) and was promoted as the new default, but stalled short of the 0.80 gate | [03-bpe-signal-hunt.md](03-bpe-signal-hunt.md) |
| **Import-graph breakthrough** | 13–14 | `SequentialImportBpeScorer` flagged 46/46 breaks with 0 FP across 189 calibration+control hunks; TS bring-up clean on hono (0/22), ink (3/14 all INTENTIONAL), and faker-js (2/46 after 74.8% locale-data filter) | [04-import-graph-breakthrough.md](04-import-graph-breakthrough.md) |

## The arc in one chart

Each era's best number on its own gate — the scorer got simpler, honest,
and eventually good enough:

```mermaid
xychart-beta
    title "Best scorer per era vs the era's pass/fail gate"
    x-axis ["Era 1 (shuffled AUC)", "Era 2 (synthetic AUC)", "Era 3 (fixture AUC)", "Era 4 (recall)"]
    y-axis "score (0–1)" 0 --> 1.05
    bar [0.713, 0.581, 0.6968, 1.0]
    line [0.80, 0.85, 0.80, 1.0]
```

Bars are the best result; the line marks the era's gate. Eras 1–3 came
in under; era 4 cleared it.

## Evidence

Each era doc cites peer docs under `docs/research/evidence/`. Those are
freshly written, 200–400 word summaries of the experiments the narrative
load-bears on — 27 in total, covering every cited result. The era docs
are the story; the evidence docs are the receipts.

## What's next

Phase 14 research code is scheduled for a clean merge onto main in a
separate PR from branch `research/phase-14-import-graph`. The current
production scorer lives at `engine/argot/scoring/` on main.
