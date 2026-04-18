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
| small   | zod      | https://github.com/colinhacks/zod          | <TBD task 2>                               |     TBD | ts       |
| small   | click    | https://github.com/pallets/click           | <TBD task 2>                               |     TBD | py       |
| medium  | vigie    | https://github.com/get-tmonier/vigie       | <TBD task 3>                               |     TBD | ts       |
| medium  | ruff     | https://github.com/astral-sh/ruff          | <TBD task 3>                               |     TBD | py       |
| large   | effect   | https://github.com/Effect-TS/effect        | <TBD task 4>                               |     TBD | ts       |
| large   | pydantic | https://github.com/pydantic/pydantic       | <TBD task 4>                               |     TBD | py       |
| xlarge  | tsgo     | https://github.com/microsoft/tsgo          | <TBD task 5>                               |     TBD | ts       |
| xlarge  | django   | https://github.com/django/django           | <TBD task 5>                               |     TBD | py       |

## Bucket targets and composition

| bucket | target records (downsample) | repos             | notes                          |
|:-------|----------------------------:|:------------------|:-------------------------------|
| micro  |                         150 | argot             | single-repo; skipped in bench  |
| small  |                       1,500 | zod + click       |                                |
| medium |                       6,000 | vigie + ruff      |                                |
| large  |                      20,000 | effect + pydantic |                                |
| xlarge |                      80,000 | tsgo + django     |                                |

## Swap-out log

_(empty — record any candidate that missed its bucket and was replaced)_
