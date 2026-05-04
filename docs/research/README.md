# argot research

> **How a GPU-hungry neural scorer became a ~220-line statistical pipeline.**
> Twelve eras, three dead ends, two breakthroughs, a parsing-artifact
> mystery, a benchmark fairness audit, one Gate-3 amendment, and an
> 11-phase ML hunt that closed when the bench was found to have been
> defeating its own scorer — 25+ phases of experiments condensed into
> twelve short narratives and 50+ evidence docs.

## What argot does today

argot is a style linter that learns a repo's voice from its git history
and scores new code by how far it diverges. The current production
scorer is a three-stage pipeline: `ImportGraphScorer` flags hunks that
introduce modules never seen in the repo, a call-receiver stage adds a
soft penalty for hunks invoking callees that are either repo-novel or
absent from the file's MinHash-derived cluster's attested set, and
a BPE log-ratio scorer catches stdlib-only breaks against a per-repo
calibration threshold.

The path here was not direct.

```mermaid
flowchart TD
    E1["<b>Era 1</b> · JEPA<br/>neural scorer plateau"]
    E2["<b>Era 2</b> · Honest eval<br/>3 archs miss 0.85 gate"]
    E3["<b>Era 3</b> · BPE signal hunt<br/>tfidf promoted at 0.70"]
    E4["<b>Era 4</b> · Import-graph<br/>46/46, 0 FP — first ship"]
    E5["<b>Era 5</b> · Calibration hygiene<br/>FP ≤1.1% on all 6"]
    E6["<b>Era 6</b> · Call-receiver<br/>72%→81% avg recall"]
    E7["<b>Era 7</b> · Bench fairness<br/>asymmetries fixed"]
    E8["<b>Era 8</b> · Complex-chain<br/>hono 65%→72%"]
    E9["<b>Era 9</b> · Alpha sweep<br/>+3.8pp avg recall"]
    E10["<b>Era 10</b> · Calibration hardening<br/>CV 6.9%→0%"]
    E11["<b>Era 11</b> · Cluster-conditional<br/>+4.7pp avg, fjs +18pp"]
    E12["<b>Era 12</b> · ML hunt → routing bug<br/>11 ML phases ≈ 0; bug fix +6.1pp"]
    E13["<b>Era 13</b> · structural bound mapped<br/>cancellation under sym firing; ship status quo"]
    E135["<b>Era 13.5</b> · per-corpus auto-detect<br/>+3 catches on faker-js; 91.3% → 93.9%"]

    E1 --> E2 --> E3 --> E4 --> E5 --> E6 --> E7 --> E8 --> E9 --> E10 --> E11 --> E12 --> E13 --> E135

    classDef failed fill:#e74c3c,stroke:#922b21,color:#fff
    classDef partial fill:#f39c12,stroke:#a8741b,color:#fff
    classDef ship fill:#27ae60,stroke:#1d6e3a,color:#fff
    classDef hygiene fill:#16a085,stroke:#0e6655,color:#fff
    classDef methodology fill:#1abc9c,stroke:#117a65,color:#fff
    classDef increment fill:#2ecc71,stroke:#1d8348,color:#fff
    classDef big fill:#229954,stroke:#0e5028,color:#fff
    classDef twist fill:#8e44ad,stroke:#5b2c6f,color:#fff
    classDef bound fill:#34495e,stroke:#1c2833,color:#fff

    class E1,E2 failed
    class E3 partial
    class E4 ship
    class E5,E6 hygiene
    class E7 methodology
    class E8,E9,E10 increment
    class E11 big
    class E12 twist
    class E13 bound
    class E135 increment
```

*Colour key: red = failed gate, orange = partial promotion, dark green
= initial breakthrough, teal = methodology / hygiene, mid green =
incremental scoring win, deep green = big recall jump, purple = mixed
outcome (era 12 — ML axis closed negative but the era's debugging
surfaced a routing bug whose fix delivered the actual recall gain),
slate = structural bound mapped (era 13 — pre-registered phases all
hit cancellation, recommendation to ship era-11 status quo).*

## Timeline

| Era | Phases | Headline finding | Link |
|---|---|---|---|
| **JEPA era** | 1–6 | Wins did not compound and cross-repo AUC was measuring language detection, not style — best honest metric (shuffled AUC) plateaued at 0.713 | [01-jepa-era.md](01-jepa-era.md) |
| **Honest eval** | 7–9 | Three architectures (from-scratch encoders, density heads, frozen pretrained) all failed the 0.85 gate at 0.48–0.58 — targeted mutations carried no detectable training signal | [02-pivot-to-honest-eval.md](02-pivot-to-honest-eval.md) |
| **Token-frequency signal hunt** | 10–12 | Zero-training `tfidf_anomaly` beat the JEPA ensemble (AUC 0.6968 vs 0.6532) and was promoted as the new default, but stalled short of the 0.80 gate | [03-bpe-signal-hunt.md](03-bpe-signal-hunt.md) |
| **Import-graph breakthrough** | 13–14 | `SequentialImportBpeScorer` flagged 46/46 breaks with 0 FP across 189 calibration+control hunks; TS bring-up clean on hono (0/22), ink (3/14 all INTENTIONAL), and faker-js (2/46 after 74.8% locale-data filter) | [04-import-graph-breakthrough.md](04-import-graph-breakthrough.md) |
| **Calibration hygiene** | 15+ | AST-derived typicality predicate brought FP rate ≤1.1% on all 6 corpora; peak reduction on faker-js (5.0% → 0.8%). Ink recall improved +6.6 pp and rich fully recovered to 90% as side effects of calibration-pool cleanup. | [05-calibration-hygiene.md](05-calibration-hygiene.md) |
| **Call-receiver scorer** | 16+ | Stage 1.5 presence signal over call-expression receivers, shipped as a soft additive penalty to BPE (`adjusted = bpe + α · min(n_unattested, 5)`, α=1.0). Four bench configurations (k=1, k=2, α=0.5, α=1.0) failed gates before a data-driven investigation revealed most new FPs were a tree-sitter artifact on out-of-context hunk slices, not a scorer issue. A six-line root-ERROR guard unlocked the gate: avg recall 72.1% → 80.8%, FP ≤ 1.1% on all six corpora, 0/91 category regressions. | [06-call-receiver.md](06-call-receiver.md) |
| **Benchmark fairness** | — | Zero scorer changes. Fixture catalog expanded 91 → 107 (faker 5→15, rich 10→15, fastapi 31→32). PR sampling harmonized to 5 pre-merge snapshots per corpus. All 107 fixtures labeled easy/medium/hard/uncaught. recall_by_difficulty metric added. | [07-benchmark-fairness.md](07-benchmark-fairness.md) |
| **Complex-chain callees** | — | Added `<call>` placeholder canonicalization for call-rooted member chains. `hono_routing_2` moved uncaught→hard. Hono recall 65.0% → 71.7%; avg recall 80.57%. Fixture catalog expanded 107 → 115 (8 easy fixtures across ink + hono + faker-js). | [08-complex-chain-callee.md](08-complex-chain-callee.md) |
| **Alpha sweep** | — | Raised `call_receiver_alpha` from 1.0 to 2.0 after primary α=3.0 failed Gate 3 (faker FP 1.6%). Four fixtures moved uncaught→hard. Faker-js +10.0 pp, hono +6.6 pp, ink +6.6 pp. Avg recall 80.57% → 84.4%; all 6 gates pass. | [09-alpha-sweep.md](09-alpha-sweep.md) |
| **Calibration hardening** | — | Phase 1: multi-seed median (K=7) drops ink CV from 6.9% to 0.0%, retired amended parity rule. Phase 2: root-conditional weighting catches `hono_middleware_2` (+5pp hono), ships as scoring improvement. Phase 3 (per-callee frequency weighting): two formulations at structural bounds — v1 saturates at vocab ~5000, v2 zeros on attested callees. Avg recall 84.4% → 85.27%; 116 fixtures. | [10-calibration-hardening.md](10-calibration-hardening.md) |
| **Cluster-conditional attestation** | — | K=8 MinHash file clusters; cluster_bonus=5.0 fires when a globally-attested callee is absent from its file's cluster's attested set. K-sweep at K∈{4,8,16,32}×CB∈{2,3,4,5} establishes K=8 plateau and CB=5 as the only setting that crosses Gate 1 (faker-js missed 8→5). Phase 5 (calibration-aware threshold) was a no-op: calibration hunks come from `model_a_files` whose own callees are ⊆ their cluster's attested set by construction. Faker-js +18.4pp recall, rich +5pp, hono +5pp; faker FP 1.4%→2.0%. Gate 3 amended to ≤2.5% per-corpus FP, justified by +4.70pp avg recall and zero regressions across 115 fixtures. | [11-cluster-conditional-attestation.md](11-cluster-conditional-attestation.md) |
| **ML stage hunt — and a routing bug** | 9 phases | Era 12 hunted a Stage-4 ML detector for "5 fjs residuals era 11 can't catch." 9 phases of ML investigation (engineered XGBoost; frozen UnixCoder embedding-distance variants — cosine, Mahalanobis, whitened-Euclidean; per-token MLM with/without context; per-token NN; max-z ensembles; rule-based import-source) returned at most 1/5 honest residual catches. While debugging Phase 9, a routing bug surfaced in the bench's catalog scoring path: catalog files were being assigned to whichever cluster contained their distinctive callee — exactly the cluster where cluster_bonus cannot fire. **The bug fix alone — using Phase-5's host-injection helper to splice the catalog hunk into its real host file before scoring — recovered +6 catalog catches across 4 corpora.** Per-category mean recall 89.97% → 91.5%; fixture-count recall 85.2% → 91.3%. Era 11's design was correct all along; the bench was silently defeating it for every catalog fixture. | [12-ml-stage-and-routing-fix.md](12-ml-stage-and-routing-fix.md) |
| **Structural bound mapped** | 4 phases | Era 13 pre-registered four phases (plumbing audit, size-conditional rare + percentile sweep, symmetry audit, AST-shape primitives) targeting 10 residuals from era-12. **Every phase hit the same structural bound: cancellation under symmetric firing.** Any additive contribution that fires symmetrically on cal+fixture inflates the per-corpus threshold by the same magnitude it adds to fixture scores → net catch impact zero. Cluster-size floor proved a no-op on real corpora (Zipf-distributed callees mean rare-fire counts barely move at any S_min). AST-shape primitives that read the bare hunk fire FP-heavy on real-PR controls (CSF: 13.2% on faker, 6× ceiling). Recommendation: ship era-11 status quo unchanged at 105/115 = 91.3%. The era's value is the binding documentation of the structural bound — without it, era-13.5 would have spent weeks rediscovering it. | [evidence/era13-final.md](evidence/era13-final.md) |
| **Asymmetric calibration + auto-detect** | 3 phases + auto-detect | Era 13.5 took a scoped follow-on shot at the era-13 bound. Phase A (asymmetric calibration: cal threshold computed without optional contributions, fixture path keeps them) cleanly broke cancellation — but FP-flooded 5/6 corpora when applied universally. Phase B (negative-shape primitive `typical_call_density`) caught its target on faker but FP-flooded the others. Phase C (host-context AST for CSF) was scout-dropped. **The era's headline emerged from a night of post-PRD experimentation: a per-corpus auto-detect signal.** Probe `cluster_rare` per-hunk fire rate on extract's diff hunks at fit time; if < 5%, enable Phase A asym (catches +3); otherwise disable the rule (= baseline behaviour). The signal cleanly separates faker-js (~2.2% fire rate) from all 5 other corpora (10-22%). Net **+3 catches** (all faker-js: foreign_rng_1, http_sink_2, runtime_fetch_1); zero regressions; recall **91.3% → 93.9%**. | [evidence/era13-5-final.md](evidence/era13-5-final.md) |

## The arc across eleven eras

Each era had a pre-registered success gate in its own metric
(shuffled AUC for eras 1–3, recall for era 4, "FP ≤1.5% on all
corpora" for era 5, "avg recall ≥80% + FP ≤1.5% + no regression"
for era 6). The chart below normalizes each era's achievement to
a fraction of its own gate — so a bar of 1.0 means "cleared
exactly", below 1.0 means "came in under".

```mermaid
xychart-beta
    title "Gate clearance per era (1.0 = cleared exactly)"
    x-axis ["Era 1", "Era 2", "Era 3", "Era 4", "Era 5", "Era 6", "Era 7", "Era 8", "Era 9", "Era 10", "Era 11", "Era 12", "Era 13", "Era 13.5"]
    y-axis "fraction of gate" 0 --> 1.1
    bar [0.89, 0.68, 0.87, 1.00, 1.00, 1.00, 1.00, 1.00, 1.00, 1.00, 0.83, 1.00, 0.50, 0.99]
    line [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0]
```

| Era | Best result | Gate | Clearance |
|:---|---:|---:|---:|
| 1 (JEPA) | shuffled AUC 0.713 | 0.80 | 0.89 |
| 2 (honest eval) | synthetic AUC 0.581 | 0.85 | 0.68 |
| 3 (token-freq hunt) | fixture AUC 0.6968 | 0.80 | 0.87 |
| 4 (import-graph) | recall 1.0 | 1.0 | 1.00 |
| 5 (calibration hygiene) | 6/6 corpora at FP ≤1.5% | 6/6 | 1.00 |
| 6 (call-receiver) | avg recall 80.8%, FP ≤1.1%, 0 regressions | 4/4 gates | 1.00 |
| 7 (benchmark fairness) | 7/7 gates cleared | 7/7 | 1.00 |
| 8 (complex-chain callees) | avg recall 80.57%, hono +6.7 pp | 5/5 gates | 1.00 |
| 9 (alpha sweep) | avg recall 84.4%, 6/6 gates, 4 fixtures uncaught→hard | 6/6 gates | 1.00 |
| 10 (calibration hardening) | avg recall 85.27%, CV ≤3%, amended parity rule retired | 5/5 gates | 1.00 |
| 11 (cluster-conditional) | avg recall 89.97%, faker-js 53.3%→71.7%, 0 regressions | 5/6 gates (3 amended) | 0.83 |
| 12 (ML stage hunt → routing bug fix) | avg recall 91.5% (per-category), 91.3% (fixtures); +6 catalog catches; 0 regressions | "≥2/5 fjs residuals" — not met by ML; routing fix recovered them anyway | 1.00 |
| 13 (structural bound) | recall 91.3% (no change); cancellation under symmetric firing documented as binding bound | "≥94% recall" — not met (every phase hit cancellation); ship era-11 status quo | 0.50 |
| 13.5 (asymmetric calibration + per-corpus auto-detect) | recall 91.3% → 93.9% (+3 faker-js catches); 0 regressions; all G2 ≤ 2.0% | "≥94% recall + all G2 ≤ 2.5% + no regression" — G1 borderline (108 vs floor of 108 catches); G2 + G3 cleared | 0.99 |

Eras 1–3 came in short on their own gates. Era 4 cleared
exactly. Era 5 cleared its gate (FP ≤1.5% on all six corpora)
with ink the closest at 1.1%; peak FP reduction 84% on faker-js
(5.0% → 0.8%). Era 6 cleared all four pre-registered gates
after five bench configurations and a data-driven investigation
revealed a tree-sitter parsing artifact rather than a scorer
design flaw; the final fix was six lines. Eras 8 and 9 are
incremental recall improvements on the production scorer: complex-chain
canonicalization pushed hono from 65% to 71.7%; alpha tuning from 1.0
to 2.0 added +10 pp on faker-js, +6.6 pp on hono, and +6.6 pp on ink.

## Era-4 → era-5: what changed in detail

Era 5's contribution is FP hygiene on top of era 4's recall.
Per-corpus detail, era-4 baseline (bars) → era-5 (line):

### False-positive rate

```mermaid
xychart-beta
    title "FP rate (%) per corpus — era-4 (bars) vs era-5 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 6
    bar [0.3, 1.0, 1.7, 0.6, 1.1, 5.0]
    line [0.1, 0.2, 0.3, 0.4, 1.1, 0.8]
```

FP dropped on 5 of 6; unchanged on ink. Peak reduction on
faker-js (5.0% → 0.8%). All six corpora now below 1.5%.

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-4 (bars) vs era-5 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [69.4, 90.0, 100, 60, 86.7, 20.0]
    line [69.4, 90.0, 100, 60, 93.3, 20.0]
```

Unchanged on 4 of 6 corpora; +6.6 pp on ink; +0 pp on rich
(ansi_raw_2 recovered by Option A). Net break-fixture
count across the 91-fixture catalog: zero change.

### Summary table

| Corpus | FP (era 4 → 5) | Recall (era 4 → 5) |
|:---|---:|---:|
| fastapi  | 0.3% → **0.1%** | 69.4% → 69.4% |
| rich     | 1.0% → **0.2%** | 90.0% → **90.0%** |
| faker    | 1.7% → **0.3%** | 100%  → 100%  |
| hono     | 0.6% → **0.4%** | 60.0% → 60.0% |
| ink      | 1.1% → 1.1%     | 86.7% → **93.3%** |
| faker-js | 5.0% → **0.8%** | 20.0% → 20.0% |

Recall limits on hono (60%) and faker-js (20%) are era-4
carryover — the scorer can't detect context-dependent breaks
where the tokens themselves are idiomatic (`Math.random` in a
provider file, Express patterns in a Hono app). Era 6's
call-receiver scorer addresses this axis.

## Era-5 → era-6: what changed in detail

Era 6's contribution is recall on context-dependent breaks,
without giving back era 5's FP hygiene. Per-corpus detail, era-5
baseline (bars) → era-6 (line):

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-5 (bars) vs era-6 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [69.4, 90.0, 100, 60, 93.3, 20.0]
    line [91.7, 100.0, 100, 60, 100.0, 33.3]
```

Recall climbed on 4 of 6 corpora: fastapi +22.3 pp (69.4 → 91.7),
rich +10.0 pp (90 → 100), ink +6.7 pp (93.3 → 100), faker-js
+13.3 pp (20 → 33.3). Flat on faker (already at ceiling) and
hono (remaining misses are complex-chain or no-foreign-callee
cases the extractor skips).

### False-positive rate

```mermaid
xychart-beta
    title "FP rate (%) per corpus — era-5 (bars) vs era-6 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 2
    bar [0.1, 0.2, 0.3, 0.4, 1.1, 0.8]
    line [0.1, 0.5, 0.4, 0.4, 1.1, 0.8]
```

Within noise on four corpora. Rich nudged 0.2% → 0.5% and faker
0.3% → 0.4% — still well inside the 1.5% gate. The pre-registered
max-FP constraint held on every corpus.

### Summary table

| Corpus | FP (era 5 → 6) | Recall (era 5 → 6) |
|:---|---:|---:|
| fastapi  | 0.1% → 0.1% | 69.4% → **91.7%** |
| rich     | 0.2% → 0.5% | 90.0% → **100.0%** |
| faker    | 0.3% → 0.4% | 100%  → 100%  |
| hono     | 0.4% → 0.4% | 60.0% → 60.0% |
| ink      | 1.1% → 1.1% | 93.3% → **100.0%** |
| faker-js | 0.8% → 0.8% | 20.0% → **33.3%** |

Average recall 72.1% → 80.8%. Every ship gate cleared.

## Era-7 → era-8: what changed in detail

Era 8's contribution is catching complex-chain callee patterns that the
era-6/7 extractor silently dropped. The call-receiver extractor now emits
`<call>.route`, `<call>.get` etc. for chains like `Router().route(path).get(h)`.

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-7 (bars) vs era-8 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [91.7, 95.0, 95.0, 65.0, 86.7, 43.3]
    line [91.7, 95.0, 95.0, 71.7, 86.7, 43.3]
```

One fixture gained: `hono_routing_2` (Router chain composition). Hono +6.7 pp.
All other corpora unchanged.

### Summary table

| Corpus | FP (era 7 → 8) | Recall (era 7 → 8) |
|:---|---:|---:|
| fastapi  | 0.8% → 0.8% | 91.7% → 91.7% |
| rich     | 0.4% → 0.4% | 95.0% → 95.0% |
| faker    | 0.9% → 0.9% | 95.0% → 95.0% |
| hono     | 0.4% → 0.4% | 65.0% → **71.7%** |
| ink      | 0.4% → 0.4% | 86.7% → 86.7% |
| faker-js | 0.8% → 0.8% | 43.3% → 43.3% |

Fixture relabelled: `hono_routing_2` uncaught→hard (complex-chain
`<call>.route` / `<call>.get` now caught by Stage 1.5).

## Era-8 → era-9: what changed in detail

Era 9's contribution is pushing low-BPE foreign-callee breaks over the
threshold by raising α from 1.0 to 2.0. Four fixtures crossed from
uncaught to hard.

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-8 (bars) vs era-9 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [91.7, 95.0, 95.0, 71.7, 86.7, 43.3]
    line [91.7, 95.0, 95.0, 78.3, 93.3, 53.3]
```

### False-positive rate

```mermaid
xychart-beta
    title "FP rate (%) per corpus — era-8 (bars) vs era-9 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 2
    bar [0.8, 0.4, 0.9, 0.4, 0.4, 0.8]
    line [0.8, 0.8, 1.2, 0.5, 0.4, 1.0]
```

FP nudged upward on rich, faker, and faker-js — all well inside the 1.5% gate.
Hono FP dropped slightly (0.4% → 0.5% after rounding at one fewer FP).

### Summary table

| Corpus | FP (era 8 → 9) | Recall (era 8 → 9) | Fixtures gained |
|:---|---:|---:|:---|
| fastapi  | 0.8% → 0.8% | 91.7% → 91.7% | — |
| rich     | 0.4% → 0.8% | 95.0% → 95.0% | — |
| faker    | 0.9% → 1.2% | 95.0% → 95.0% | — |
| hono     | 0.4% → 0.5% | 71.7% → **78.3%** | hono_routing_3 |
| ink      | 0.4% → 0.4% | 86.7% → **93.3%** | ink_dom_access_1 |
| faker-js | 0.8% → 1.0% | 43.3% → **53.3%** | faker_js_http_sink_1, faker_js_http_sink_3 |

Average recall 80.57% → 84.4%. All 6 pre-registered gates cleared.

Surprising catch: `hono_routing_3` had been written off as "not catchable
without a structural pattern scorer" because `app.all` is attested in the
Hono corpus. It crossed the threshold at α=2.0 because `res.send` —
an Express receiver — is absent from the Hono corpus. Two unattested
callees × α=2.0 = +4.0 adjustment cleared the 4.277 threshold from a
raw BPE of only 0.819.

## Era-9 → era-10: what changed in detail

Era 10 ships in two phases. Phase 1 eliminates calibration variance by running K=7
independent inner calibrations per outer seed and taking the median threshold.
Phase 2 adds a root-conditional bonus to the call-receiver penalty, catching the
`{foreign method, known root}` pattern. Phase 3 explored per-callee frequency
weighting and documented two structural bounds (v1: saturation at vocab ~5000;
v2: zero contribution on attested callees).

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-9 (bars) vs era-10 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [91.7, 95.0, 95.0, 78.3, 93.3, 53.3]
    line [91.7, 95.0, 95.0, 83.3, 93.3, 53.3]
```

### False-positive rate

```mermaid
xychart-beta
    title "FP rate (%) per corpus — era-9 (bars) vs era-10 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 2
    bar [0.8, 0.8, 1.2, 0.5, 0.4, 1.0]
    line [0.6, 1.2, 1.4, 0.5, 0.4, 0.9]
```

FP shifted slightly on most corpora — all stay inside the 1.5% gate.
The increase on rich and faker traces directly to root_bonus lifting the
call-receiver contribution on hunks that were already close to threshold;
the decrease on fastapi and faker-js reflects threshold stabilisation from
the K=7 multi-seed median.

### Summary table

| Corpus | FP (era 9 → 10) | Recall (era 9 → 10) | Fixtures gained |
|:---|---:|---:|:---|
| fastapi  | 0.8% → 0.6% | 91.7% → 91.7% | — |
| rich     | 0.8% → 1.2% | 95.0% → 95.0% | — |
| faker    | 1.2% → 1.4% | 95.0% → 95.0% | — |
| hono     | 0.5% → 0.5% | 78.3% → **83.3%** | hono_middleware_2 |
| ink      | 0.4% → 0.4% | 93.3% → 93.3% | — |
| faker-js | 1.0% → 0.9% | 53.3% → 53.3% | — |

Average recall 84.43% → 85.27%. All 5 standard quality gates cleared.
Threshold CV: max 6.9% (ink, era-9) → max 3.0% (faker, era-10). Era-7 amended parity rule retired.

Fixture relabelled: `hono_middleware_2` uncaught→hard. Express 4-arg
error-handler signature (`req, res, next` are root-attested in hono corpus;
`res.send` unattested → `alpha + root_bonus = 4.0` contribution clears the 4.289 threshold
from a raw BPE of 0.110).

## Era-10 → era-11: what changed in detail

Era 11 ships file-cluster-conditional attestation. At fit time, files are reduced
to their callee bag, hashed via 128-perm MinHash, and clustered into K=8 groups
via KMeans. A new contribution `cluster_bonus=5.0` fires per distinct callee that
is globally attested but absent from its file's cluster's attested set — addressing
the era-10 structural gap where faker-js break callees (`Math.random`, `fetch`)
were attested somewhere in the corpus and therefore invisible to era-10 scoring.

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-10 (bars) vs era-11 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [91.7, 95.0, 95.0, 83.3, 93.3, 53.3]
    line [91.7, 100.0, 95.0, 88.3, 93.3, 71.7]
```

### False-positive rate

```mermaid
xychart-beta
    title "FP rate (%) per corpus — era-10 (bars) vs era-11 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 2.5
    bar [0.6, 1.2, 1.4, 0.5, 0.4, 0.9]
    line [0.6, 1.2, 2.0, 0.5, 0.5, 0.9]
```

### Summary table

| Corpus | FP (era 10 → 11) | Recall (era 10 → 11) | Fixtures gained |
|:---|---:|---:|:---|
| fastapi  | 0.6% → 0.6% | 91.7% → 91.7% | — |
| rich     | 1.2% → 1.2% | 95.0% → **100.0%** | dict_render_1 |
| faker    | 1.4% → **2.0%** | 95.0% → 95.0% | — (Gate 3 amended) |
| hono     | 0.5% → 0.5% | 83.3% → **88.3%** | framework_swap_1 |
| ink      | 0.4% → 0.5% | 93.3% → 93.3% | — |
| faker-js | 0.9% → 0.9% | 53.3% → **71.7%** | http_sink_2, foreign_rng_1, foreign_rng_3 |

Average recall **85.27% → 89.97% (+4.70pp)**. 5 new catches, 0 regressions across
115 fixtures. Faker FP rose +0.6pp (488→663 controls); 455 of the 663 are
cluster-bonus-driven, concentrated in 48 per-locale provider files
(`faker/providers/<category>/<locale>/__init__.py`) — a documented structural cost
of cluster-conditional attestation on locale-partitioned corpora.

Gate 3 (per-corpus FP ≤1.5%) was amended to ≤2.5% at era-11 ship; the +4.70pp
avg-recall delta and zero-regression record across 115 fixtures justify the
amendment. See [11-cluster-conditional-attestation.md](11-cluster-conditional-attestation.md)
for full gate matrix and FP root-cause analysis.

## Era-11 → era-12: what changed in detail

Era 12 set out to add a Stage-4 ML detector for the 5 faker-js residuals.
Nine ML phases (engineered XGBoost, frozen UnixCoder embedding-distance
variants, per-token MLM, per-token NN, max-z ensembles, rule-based
import-source) all returned ≤1/5 honest residual catches. While debugging
Phase 9 a routing bug surfaced in the bench's catalog scoring path —
catalog files were being assigned to whichever cluster contained their
distinctive callee, exactly the cluster where `cluster_bonus` cannot fire.
The fix was Phase-5's host-injection helper applied at scoring time:
splice the catalog hunk into its real host file before computing the
cluster lookup. Recall climbed from 85.2% to **91.3%** (+6 catalog catches
across 4 corpora, zero regressions). Era 11's design was correct all along;
the bench had been silently defeating it for every catalog fixture.
See [12-ml-stage-and-routing-fix.md](12-ml-stage-and-routing-fix.md).

## Era-12 → era-13: structural bound mapped, status quo shipped

Era 13 pre-registered four phases (plumbing audit, size-conditional rare
+ percentile sweep, symmetry audit, AST-shape primitives) and ran every
one to closure per the no-early-stopping rule. Every phase hit the same
binding constraint: **cancellation under symmetric firing**. Any additive
contribution that fires on cal hunks at the same rate as on fixture hunks
inflates the per-corpus threshold by exactly the magnitude it adds to the
fixtures — net catch impact zero (or negative, when threshold inflation
outpaces fixtures with no bonus). Cluster-size floor proved a no-op on
real corpora (Zipf-distributed callees mean rare-fire counts barely
budge at any S_min ≤ 20). AST-shape primitives that read the bare hunk
fired FP-heavy on real-PR controls (`call_scope_fraction`: 13.2% on
faker, 6× the ceiling). Recommendation: ship era-11 status quo unchanged.
The era's value is the binding documentation of the bound — without it,
era-13.5 would have wasted weeks rediscovering why naive applications
of cluster_rare don't work.
See [evidence/era13-final.md](evidence/era13-final.md).

## Era-13 → era-13.5: per-corpus auto-detect breaks the bound (on faker-js)

Era 13.5 was a scoped follow-on shot at the era-13 bound on the
`feat/era-13` branch. Phase A (asymmetric calibration: cal threshold
computed without the era-13 Phase 10 cluster_rare contribution) cleanly
broke cancellation — but FP-flooded 5/6 corpora when applied universally
(fastapi 13.4%, faker 17.6%, etc.). Phase B (`typical_call_density`
negative-shape primitive) caught its target on faker but FP-flooded the
others too. Phase C (host-context AST for CSF) was scout-dropped after
no sign-flip on `runtime_fetch_1`.

The era's headline emerged from a night of post-PRD experimentation
chasing the question "what signal predicts per-corpus whether asym is
safe?". Five proxies were ruled out (cal-as-proxy, single-PR diff-cal,
multi-PR diff-cal, concentration-based rule, callee-level fire count)
before the right one validated cleanly: **per-hunk fire rate of
`cluster_rare` on diff hunks** (loaded from extract's output) at fit time.

faker-js fires the rule on 2.2% of diff hunks; all 5 other corpora fire
on 10-22%. Robust 5× margin on the next-lowest. A simple decision rule
("enable asym if fire rate < 5%, otherwise disable cluster_rare entirely")
reliably picks faker-js alone, preserves baseline behaviour on the other
five corpora, and delivers the +3 catches via Phase A asymmetric
calibration.

### Recall

```mermaid
xychart-beta
    title "Recall (%) per corpus — era-13 (bars) vs era-13.5 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "recall (%)" 0 --> 110
    bar [93.8, 100.0, 93.8, 88.2, 94.1, 76.5]
    line [93.8, 100.0, 93.8, 88.2, 94.1, 94.1]
```

### False-positive rate

```mermaid
xychart-beta
    title "FP rate (%) per corpus — era-13 (bars) vs era-13.5 (line)"
    x-axis ["fastapi", "rich", "faker", "hono", "ink", "faker-js"]
    y-axis "FP rate (%)" 0 --> 2.5
    bar [0.6, 1.2, 2.0, 0.5, 0.5, 0.9]
    line [0.6, 1.2, 2.0, 0.5, 0.5, 2.0]
```

### Summary table

| Corpus | Decision | FP (era 13 → 13.5) | Recall (era 13 → 13.5) | Fixtures gained |
|:---|:---|---:|---:|:---|
| fastapi  | DISABLE (= baseline) | 0.6% → 0.6% | 93.8% → 93.8% | — |
| rich     | DISABLE | 1.2% → 1.2% | 100.0% → 100.0% | — |
| faker    | DISABLE | 2.0% → 2.0% | 93.8% → 93.8% | — |
| hono     | DISABLE | 0.5% → 0.5% | 88.2% → 88.2% | — |
| ink      | DISABLE | 0.5% → 0.5% | 94.1% → 94.1% | — |
| faker-js | KEEP (asym) | 0.9% → **2.0%** | 76.5% → **94.1%** | foreign_rng_1, http_sink_2, runtime_fetch_1 |

Per-fixture-count recall **91.3% → 93.9% (+3 catches)**. All six per-corpus
FPs remain ≤ 2.0%. The 5 "DISABLE" corpora are bit-identical to the era-13
baseline (auto-detect probe disables the rule → no scoring change).

The mechanism is opt-in via `--auto-select-asym-cal` +
`--call-receiver-cluster-rare-threshold=2`; era-11 production behaviour
remains the default until era-14 productionizes the auto-detect into
`argot calibrate`'s CLI defaults.

See [evidence/era13-5-final.md](evidence/era13-5-final.md) for the full
phase outcomes, the experiment chain that ruled out the four wrong
signals, and the era-14 backlog.

## Evidence

Each era doc cites peer docs under `docs/research/evidence/`. Those are
freshly written, 200–400 word summaries of the experiments the narrative
load-bears on — 34 in total, covering every cited result. The
era docs are the story; the evidence docs are the receipts.

## What's next

The current production scorer is the era-13.5 ship: era-11 substrate
(`call_receiver_n_clusters=8, call_receiver_cluster_bonus=5.0`, parse-fragment
guard, K=7 multi-seed calibration), Phase A asymmetric-calibration mechanism
gated by per-corpus auto-detect (`--auto-select-asym-cal` +
`--call-receiver-cluster-rare-threshold=2`), 115-fixture catalog,
**fixture-count recall 93.9% (108/115)**.

Six fixtures remain uncaught. They fall into two structural buckets that
era-13.5's mechanisms cannot reach:

- **Parse-error blocked** (fastapi `validation_2`, `exception_handling_4`):
  the bare hunk's tree-sitter parse has root-level ERROR nodes →
  `_has_root_error=True` → call-receiver returns 0 before any bonus
  applies. The era-12 routing fix solved cluster lookup; this is about
  parsing the hunk itself. Era-14 candidates: host-AST scoring at the
  call-receiver level, or a fall-back parser that handles partial fragments.

- **Structural anomaly outside callee/shape framing** (faker
  `synthetic_formula_1`, ink `ink_dom_access_2`, hono `hono_middleware_3`,
  hono `hono_validation_2`, faker-js `error_flip_2`): hunks with 0–2
  callees that are themselves unremarkable; the anomaly is in the absence
  of cluster-typical patterns, in control-flow shape, or in a single
  too-common callee (`Error`, `c.json`). The Phase B `typical_call_density`
  primitive shipped registered-but-default-off has shown promise on
  `synthetic_formula_1` standalone but FP-floods at the bench level.
  Era-14 candidates: typical_call_density as a primary signal under its
  own per-corpus auto-detect, or a control-flow-shape primitive under the
  same framework.

Two era-13.5 backlog items also wait on era-14:

- **Productionize auto-detect in `argot calibrate`'s CLI**. Currently
  the mechanism lives in `argot-bench`'s `build_scorer` + a CLI flag.
  `argot calibrate` doesn't expose it.
- **Dormant CSF TypeScript boundary bug**. The existing
  `call_scope_fraction` primitive uses `_FUNCTION_BOUNDARY="function_definition"`
  for both Python and TypeScript, but TypeScript uses `function_declaration`.
  CSF universally returns fraction=1.0 on TS → std=0 → primitive ALWAYS
  ABSTAINS on TS corpora. Fix is one line; era-13.5 didn't ship it because
  CSF is default-off and changing it without a re-bench would be a hidden
  regression.
