# Signal Scorer Comparison — Phase A Decision

## Results

| entry   | jepa_pretrained | knn_cosine | tfidf_anomaly | lof_embedding | lm_perplexity |
|---------|----------------:|-----------:|--------------:|--------------:|--------------:|
| ky      | 0.2129 ✓        | 0.0601 ✗   | 0.0565 ✗      | 0.0907 ✗      | -0.3968 ✗     |
| httpx   | 0.1241 ✗        | 0.1873 ✗   | 0.2635 ✓      | 0.2795 ✓      | 0.0328 ✗      |
| fastapi | 0.1678 ✗        | 0.0722 ✗   | 0.0603 ✗      | 0.1047 ✗      | -0.2464 ✗     |
| **mean** | **0.1683**     | **0.1065** | **0.1268**    | **0.1583**    | **-0.2035**   |

Gate: delta ≥ 0.20 = ✓

## Decision: Ensemble

The data does not support a single scorer winning universally: jepa_pretrained clears the gate only on ky (style-heavy JS), while tfidf_anomaly and lof_embedding clear it only on httpx (explicit API paradigm shifts), and no scorer clears the gate on fastapi (subtle async/validation breaks). This fragmentation means the signal is corpus-specific — different anomaly geometries require different detectors — so the architecture must run all scorers and aggregate (e.g., max or learned combination) rather than rely on one champion. The next step is to design an ensemble layer that can be calibrated per-repo, and to expand the fastapi catalog with more distinct paradigm breaks to verify whether any scorer can separate those subtler cases.
