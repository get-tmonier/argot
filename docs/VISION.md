# Vision

`argot` is a style linter that learns the unwritten conventions of a codebase from its own git history, and scores new code by how far it diverges from them. It exists because linters catch syntax, type checkers catch types, security scanners catch vulnerabilities — but nothing catches "this doesn't sound like us."

In the age of LLM-assisted coding, that gap matters more than ever: the failure mode isn't broken code, it's code that works but feels wrong.

## What `argot` is

A CLI that runs anywhere linters run: on demand in a terminal, in CI, in a pre-commit hook, or later via an editor extension. The output is a ranked list of hunks sorted by surprise score, plus an exit code for automation.

Pre-commit is one integration mode among many. It is not the identity of the tool.

## Non-goals

- Replacing linters, formatters, type checkers, or security scanners.
- Detecting bugs, vulnerabilities, or performance issues.
- Cloud-hosted analysis. The model is local. Always.
- Cross-repo learning in v0 — one model per repo, period.
- Support for every language under the sun. TypeScript first, Python second, others later.
- Auto-fixing. `argot` flags; the human decides.

## Principles

1. **Local by default, cloud never.** The model, the training data, and the inference all stay on the user's machine.
2. **A linter, not a gate.** Same UX as `eslint`, `knip`, `ruff`. Outputs a score, returns an exit code, prints a ranked list. Any gating (pre-commit, CI) is the user's choice, not the tool's assumption.
3. **Fast to install, fast to train, fast to run.** If `train` takes more than 30 min on a consumer GPU or `check` takes more than 5 seconds on a typical diff, we're doing it wrong.
4. **Bounded scope, bounded output.** `check` returns at most N hunks. Devs don't read more than that anyway.
5. **Honest about uncertainty.** Surprise is a signal, not a verdict. The tool warns; the human decides.
6. **Reproducible.** Same repo + same seed = same model. Same model + same diff = same score.

## Roadmap

### v0 — Proof of signal (target: 2 weekends)

Goal: show that the surprise signal is non-trivially useful on a real repo.

- [ ] Dataset extraction from git log (TS + Python files, tokenized with tree-sitter)
- [ ] JEPA training loop, adapted from LeWM
- [ ] CLI: `argot train`, `argot check` (staged diff + arbitrary git range)
- [ ] JSON output mode for machine consumption
- [ ] Manual validation on 3 repos: Vigie, spot-the-glitch, one public repo

### v0.5 — Usable daily (target: +1 weekend)

- [ ] Threshold calibration based on rolling percentile
- [ ] Pre-commit hook installer (`argot install-hook`) — one integration path among others
- [ ] Cold-start handling (refuse gracefully on repos with too little history)
- [ ] Config file (`.argot/config.json`)
- [ ] Exit codes: 0 = clean, 1 = flagged hunks, 2 = error

### v1 — Sharable (target: public OSS release)

- [ ] Demo GIF in the style of `spot-the-glitch` — simulate an LLM paste-through, show the spike
- [ ] Benchmark script: "on this public repo, these were the last 10 flagged hunks — here's why"
- [ ] Support for Python (in addition to TS)
- [ ] Homebrew formula, npm global install
- [ ] GitHub Action template in the README

### v1.5 — Team mode

- [ ] Mono-project, multi-author model: learns the repo's voice, not an individual's
- [ ] Per-author mode as a flag, not the default

### v2 — Editor + GitHub App (optional, only if v1 gets traction)

- [ ] VS Code extension showing surprise scores inline
- [ ] GitHub App that trains a model per repo and comments on PRs
- [ ] Opt-in paid tier for private repos; local mode remains free forever

## Success criteria

- v0: On Vigie, `argot check` on a Claude-Code-generated commit flags at least one real stylistic outlier a human reviewer agrees with.
- v0.5: I use it daily for 2 weeks and don't disable it.
- v1: 3 external users report it caught something useful.
- v2 (only if v1 succeeds): 20+ repos using the GitHub App or the VS Code extension.