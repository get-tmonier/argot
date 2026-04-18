# Scoring Benchmark — Corpus (Phase 2 Pin)

> Living document. Updated as each bucket is verified. Final once all
> 9 repos are pinned.

**Scope:** 9 repositories (1 self + 2 per external bucket × 4 buckets) used
to answer "how does AUC scale with dataset size?" See
[`DESIGN.md`](DESIGN.md) §Phase 2 for the corpus rationale.

**Reproduce the extracts:**

```bash
# Per row below:
git clone <url> .argot/research/repos/<slug>
git -C .argot/research/repos/<slug> checkout <sha>
uv run --package argot-engine python -m argot.extract \
    .argot/research/repos/<slug> \
    --out .argot/research/datasets/<slug>.jsonl \
    --repo-name <slug>
```

## Buckets

| bucket  | slug     | url                                        | sha (pinned)                               | records | language |
|:--------|:---------|:-------------------------------------------|:-------------------------------------------|--------:|:---------|
| micro   | argot    | https://github.com/get-tmonier/argot       | fda874cee3d77563da347d8497ce330300bd8c0c   |     243 | ts+py    |
| small   | zod      | https://github.com/colinhacks/zod          | c7805073fef5b6b8857307c3d4b3597a70613bc2   |   14715 | ts       |
| small   | click    | https://github.com/pallets/click           | 04ef3a6f473deb2499721a8d11f92a7d2c0912f2   |    6334 | py       |
| medium  | vite     | https://github.com/vitejs/vite             | 0deadcde614fee4fe2a5ccc9f5321dc30bfcca2f   |    8252 | ts       |
| medium  | ruff     | https://github.com/astral-sh/ruff          | 172ac2c9a27040e4a60726f82cabed6166af094a   |    3343 | py       |
| large   | effect   | https://github.com/Effect-TS/effect        | b56a6ec05688c1461574b54a8044a849e7fe639c   |    9227 | ts       |
| large   | pydantic | https://github.com/pydantic/pydantic       | a6bf50b721c2dd1ed609c8bb402076e8ec0c43f3   |   13446 | py       |
| xlarge  | tsgo     | https://github.com/microsoft/tsgo          | <TBD task 5>                               |     TBD | ts       |
| xlarge  | django   | https://github.com/django/django           | <TBD task 5>                               |     TBD | py       |

## Bucket targets and composition

| bucket | target records (downsample) | repos             | notes                          |
|:-------|----------------------------:|:------------------|:-------------------------------|
| micro  |                         150 | argot             | single-repo; skipped in bench  |
| small  |                       1,500 | zod + click       |                                |
| medium |                       6,000 | vite + ruff       |                                |
| large  |                      20,000 | effect + pydantic |                                |
| xlarge |                      80,000 | tsgo + django     |                                |

## Swap-out log

| task | bucket | candidate | reason              | replacement |
|-----:|:-------|:----------|:--------------------|:------------|
|    3 | medium | vigie     | 1194 records < 2400 | vite v2.0.0 |
