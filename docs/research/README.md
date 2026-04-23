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
    E4 -->|"FP tail on data/locale<br/>files needed a pre-filter"| E5

    E5["<b>Era 5 — Calibration hygiene</b><br/>typicality filter<br/>FP <1% on all 6 corpora;<br/>peak −84% on faker-js<br/>(phases 15+)"]

    style E4 fill:#d4edda,stroke:#28a745,stroke-width:2px
    style E5 fill:#d4edda,stroke:#28a745,stroke-width:2px
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
| **Calibration hygiene** | 15+ | AST-derived typicality predicate brought FP rate below 1% on all 6 corpora; peak reduction on faker-js (5.0% → 0.8%). Ink recall improved +6.6 pp as a side effect of calibration-pool cleanup; one fixture regression on rich (ansi_raw_2 at threshold boundary). | [05-calibration-hygiene.md](05-calibration-hygiene.md) |

## Era-4 → era-5: what changed

Era 5 added an AST-derived typicality filter on top of the era-4
scorer. The two metrics worth comparing across corpora are
false-positive rate and recall. Numbers below are from the
committed 5-seed full bench runs:
[`baseline/20260423T155121Z`](../../benchmarks/results/baseline/20260423T155121Z/report.md)
(era 4) and
[`baseline/20260423T223111Z`](../../benchmarks/results/baseline/20260423T223111Z/report.md)
(era 5).

### False-positive rate per corpus

```mermaid
xychart-beta
    title "FP rate (%) — era-4 baseline (bars) vs era-5 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 6
    bar [0.3, 1.0, 1.7, 0.6, 1.1, 5.0]
    line [0.1, 0.0, 0.1, 0.3, 1.1, 0.8]
```

FP rate dropped on 5 of 6 corpora. Five corpora now sit below
1%; the sixth (ink) is unchanged at 1.1%. Peak reduction on
faker-js (5.0% → 0.8%, −4.2 pp).

### Recall per corpus

```mermaid
xychart-beta
    title "Recall (%) — era-4 baseline (bars) vs era-5 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [69.4, 90.0, 100, 60, 86.7, 20.0]
    line [69.4, 80.0, 100, 60, 93.3, 20.0]
```

Recall is unchanged on 4 of 6 corpora, +6.6 pp on ink, and
−10 pp on rich (one fixture — `ansi_raw_2` — sitting at a
threshold boundary). Net break-fixture count across the
91-fixture catalog: 0 (ink gained one, rich lost one).

### Summary table

| Corpus | FP (era 4 → era 5) | Recall (era 4 → era 5) |
|:---|---:|---:|
| fastapi  | 0.3% → **0.1%** | 69.4% → 69.4% |
| rich     | 1.0% → **0.0%** | 90.0% → 80.0% |
| faker    | 1.7% → **0.1%** | 100%  → 100%  |
| hono     | 0.6% → **0.3%** | 60.0% → 60.0% |
| ink      | 1.1% → 1.1%     | 86.7% → **93.3%** |
| faker-js | 5.0% → **0.8%** | 20.0% → 20.0% |

Recall limitations on hono (60%) and faker-js (20%) are era-4
carryover — the scorer can't detect context-dependent breaks
where the tokens themselves are idiomatic (`Math.random` in a
provider file, Express patterns in a Hono app). Era 6's
planned call-receiver scorer addresses this axis; era 5 did
not.

## Evidence

Each era doc cites peer docs under `docs/research/evidence/`. Those are
freshly written, 200–400 word summaries of the experiments the narrative
load-bears on — 27 in total, covering every cited result. The era docs
are the story; the evidence docs are the receipts.

## What's next

The era-5 predicate lives at `engine/argot/scoring/filters/typicality.py`
and is applied by the production `SequentialImportBpeScorer` at both
calibration and inference. Remaining research items:

- **Calibration pool composition** — the file-level filter at
  `collect_candidates` changed from `is_data_dominant + is_auto_generated`
  to `is_atypical_file` during the port, which shifted rich's threshold up
  by 0.16 and cost the ansi_raw_2 fixture. A targeted fix (restore
  `is_data_dominant` as the primary file-level filter, make typicality
  opt-in at that scope) could recover rich's 90% recall but trades against
  FP elsewhere. See era-5 Interpretation for the tradeoff analysis.
- **Object-keyed structured data** (documented limit in era 5) — a 5th
  feature treating TS `property_identifier` nodes in `pair` position as
  literal-equivalent, or a Python class-boilerplate-stripped ratio.
- **Semantic breaks invisible to token novelty** (era-4 weakness #2)
  — rule-based or AST-pattern stage for categories like `foreign_rng`,
  `http_sink`, `runtime_fetch`.
- **Keyword-compatible reframings** (era-4 weakness #3) — Flask-style
  `@app.route` in a FastAPI corpus still scores below the threshold.
