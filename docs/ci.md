# Run argot in CI

Three ways to wire argot into a pipeline: the GitHub Action (with SARIF upload
to code scanning), a pre-commit hook, and raw `argot check --format` for any
other CI system.

## GitHub Action

The repo root ships a composite action (`action.yml`). It downloads the
prebuilt `argot` binary from GitHub Releases, caches the fitted `.argot/`
model directory (re-fitting only when tracked sources change), runs
`argot check`, and uploads SARIF results to GitHub code scanning so hits show
up as PR annotations.

```yaml
name: argot
on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read
  security-events: write   # required for the SARIF upload

jobs:
  argot:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0   # argot fits on git history and checks ref ranges
      - uses: get-tmonier/argot@main
```

Inputs (all optional):

| Input | Default | Meaning |
|---|---|---|
| `path` | `.` | Repository path to check |
| `argot-version` | `latest` | Release to install (`latest` or e.g. `0.2.43`) |
| `format` | `sarif` | `sarif`, `json`, or `human` |
| `output-file` | `argot-results.sarif` | Where the check document is written |
| `ref` | auto | Ref/range to check. Empty = `origin/<base>..HEAD` on PRs, the head commit on pushes |
| `cache` | `true` | Cache `.argot/` between runs (keyed on sources + argot version) |
| `upload-sarif` | `true` | Upload to code scanning (only when `format: sarif`) |
| `fail-on-hits` | `true` | Fail the job when hits are found (exit 1) |

Outputs: `exit-code` (0 clean, 1 hits) and `results-file`.

Runner support: Linux (`ubuntu-*` x64 and `ubuntu-*-arm` arm64), macOS
(arm64 and Intel x64), and Windows (`windows-*` x64) — the targets argot
publishes prebuilt binaries for.

Example: report hits without failing the build, keeping the annotations:

```yaml
      - uses: get-tmonier/argot@main
        with:
          fail-on-hits: "false"
```

## pre-commit hook

The repo root also ships `.pre-commit-hooks.yaml`, so argot registers with the
[pre-commit](https://pre-commit.com) framework. The hook runs
`argot check --staged` — staged changes only, so it stays fast and scoped to
what the commit will contain.

```yaml
# .pre-commit-config.yaml
repos:
  - repo: https://github.com/get-tmonier/argot
    rev: v0.2.43   # any released tag
    hooks:
      - id: argot-check
```

Two honest caveats, because the hook is `language: system`:

1. **pre-commit does not install argot.** The `argot` binary must be on PATH —
   install it first via the curl installer or `npm install -g @tmonier/argot`
   (see the [README](../README.md#installation)).
2. **The repo must be fitted.** Run `argot extract && argot fit` once per
   clone; an unfitted repo makes the hook fail with a "run `argot fit`" hint.

## Raw `--format` output (any CI)

`argot check` takes `--format {human,json,sarif}`. The machine formats write
exactly one document to stdout (warnings go to stderr) and keep the usual exit
semantics: `0` clean, `1` hits found, `2` usage/setup error.

```sh
argot check origin/main..HEAD --format sarif > argot-results.sarif
argot check --staged --format json | jq '.hits[] | {path, line_start, severity, score}'
```

SARIF (2.1.0) maps argot's severity tiers to standard levels — `unusual` →
`note`, `suspicious` → `warning`, `foreign` → `error` — with one rule per
scorer reason code (`bpe`, `import`, `call_receiver`) and per-hit evidence in
`results[].properties.evidence`. Any SARIF consumer (GitHub code scanning,
Azure DevOps, VS Code SARIF viewer) can ingest it.

The `json` format is argot's own stable schema:

```json
{
  "tool": { "name": "argot", "version": "0.2.43" },
  "repo": ".",
  "scanned": "workdir",
  "hunks_scanned": 12,
  "hits": [
    {
      "path": "src/utils/http-helpers.ts",
      "line_start": 42,
      "line_end": 48,
      "score": 8.21,
      "threshold": 6.75,
      "severity": "foreign",
      "reason": "import",
      "reason_label": "foreign import",
      "source": "workdir",
      "evidence": ["↳ axios — 0 of 47 module specifiers in repo"]
    }
  ]
}
```
