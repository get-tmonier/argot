# Phase 7.0 — Honest Corpus

**Branch**: `research/phase-7-honest-eval`
**Status**: complete — Phase 7.0 done; Phase 7.1 benchmarks next

## Design

Same-language sub-corpora per bucket. Each bucket has a Python pair and a
TypeScript pair benchmarked **independently** (no TS+Py mixing).

Selection criteria:
- **Same domain** where possible (controls for domain-specific vocabulary).
- **Strong style contrast** on ≥ 3 axes from the Phase 7 style-axis reference
  (`DESIGN-phase-7.md` §Style-axis reference).

## Repos

| bucket | lang | repo A (URL, SHA)                            | repo B (URL, SHA)                            |
|:-------|:-----|:---------------------------------------------|:---------------------------------------------|
| small  | py   | https://github.com/encode/httpx @ `b5addb64` | https://github.com/psf/requests @ `514c1623` |
| small  | ts   | https://github.com/sindresorhus/ky @ `e9eeb357` | https://github.com/colinhacks/zod @ `c7805073` |
| medium | py   | https://github.com/tiangolo/fastapi @ `2fa00db8` | https://github.com/pallets/flask @ `2ac89889` |
| medium | ts   | https://github.com/vitejs/vite @ `e6e9fc9e`  | https://github.com/rollup/rollup @ `a6be82b8` |
| large  | py   | https://github.com/pydantic/pydantic @ `a6bf50b7` | https://github.com/django/django @ `a284a491` |
| large  | ts   | https://github.com/Effect-TS/effect @ `b56a6ec0` | https://github.com/angular/angular @ `a0d45639` |

**Pin SHAs to whatever `HEAD` is at clone time** (documented below per repo).
Do not hunt for a "nice" commit; reproducibility is what matters.

## Extracted record counts (post-extract)

| repo          | total records | lang records |
|:--------------|-------------:|-------------:|
| httpx         |        9,877 |   9,875 py   |
| requests      |        8,936 |   8,933 py   |
| ky            |        2,431 |   2,017 ts   |
| zod           |       14,715 |  14,575 ts   |
| fastapi       |       10,985 |  10,966 py   |
| flask         |        9,283 |   9,281 py   |
| vite          |       22,974 |  20,535 ts   |
| rollup        |       91,123 |  25,870 ts   |
| pydantic      |       36,267 |  36,196 py   |
| django        |       36,738 |  36,313 py   |
| effect        |       97,525 |  97,525 ts   |
| angular       |       44,467 |  38,712 ts   |

## Bucket composition (post-downsample)

Each bucket concatenates its two same-language repos and downsamples to the
target count. Minimum share per sub-corpus after downsample: ≥ 40%.

| bucket | lang | target | combined lang records | both ≥ 40% of target? |
|:-------|:-----|-------:|----------------------:|:----------------------|
| small  | py   |   3000 |  18,808               | ✅ (9,875 / 8,933)    |
| small  | ts   |   3000 |  16,592               | ✅ (2,017 / 14,575)   |
| medium | py   |   7000 |  20,247               | ✅ (10,966 / 9,281)   |
| medium | ts   |   7000 |  46,405               | ✅ (20,535 / 25,870)  |
| large  | py   |  20000 |  72,509               | ✅ (36,196 / 36,313)  |
| large  | ts   |  20000 | 136,237               | ✅ (97,525 / 38,712)  |

## Mutations

See `DESIGN-phase-7.md` §Feasible mutation set. Four mutations:
`case_swap`, `debug_inject`, `error_flip`, `quote_flip`.

Sample mutated records (captured in Task 13 sanity run).

### case_swap sample 0
- before: `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`
- after:  `assert response . statusCode == 200 assert response . reasonPhrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`

### case_swap sample 1
- before: `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " ,`
- after:  `" decoding_error " , " delete " , " digest_auth " , " function_auth " , " get " , " head " , " Headers " ,`

### debug_inject sample 0
- before: `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`
- after:  `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " print ( "DEBUG" ) ISO-8859-1 "`

### debug_inject sample 1
- before: `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " ,`
- after:  `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " print ( "DEBUG"`

### error_flip sample 0
- before: `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`
- after:  `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`

### error_flip sample 1
- before: `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " ,`
- after:  `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " ,`

### quote_flip sample 0
- before: `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`
- after:  `assert response . status_code == 200 assert response . reason_phrase == " OK " # The encoded byte string is consistent with either ISO-8859-1 or # WINDOWS-1252. Versions <6.0 of chardet claim the former, while chardet # 6.0 detects the latter. assert response . encoding in ( " ISO-8859-1 " , " WINDOWS-1252 "`

### quote_flip sample 1
- before: `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " ,`
- after:  `" DecodingError " , " delete " , " DigestAuth " , " FunctionAuth " , " get " , " head " , " Headers " ,`

## Harness usage

```bash
just research-honest-benchmark
# → runs the six (bucket × lang) combinations with the chosen encoder
#   writes to .argot/research/results-honest.jsonl
```

## Swap-out log

- **chalk** dropped: 130 TS records — too few for a viable small-ts bucket. Replaced with **axios** (first attempt).
- **axios** also dropped: 488 TS records (94% JS) — same problem as chalk. Replaced with **colinhacks/zod** (pure TypeScript, schema-validation library, strong style contrast with ky).
