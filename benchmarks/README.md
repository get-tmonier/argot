# argot-bench

Benchmark harness for argot's production scorer. Reproduces the validation
numbers claimed in the root README across all six corpora.

## Run

```
just bench           # full 6-corpus run (~1.5h first time, ~20min cached)
just bench-quick     # ~8 min — 1 PR + 1 fixture per category
just verify-bench    # ruff + mypy + pytest on benchmarks/
```

Outputs land in `benchmarks/results/<timestamp>/` — one `report.md` +
one JSON per corpus.
