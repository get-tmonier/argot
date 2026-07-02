<!-- Thanks for the PR! Keep it focused — one change per PR. -->

## What changed

<!-- The change in a sentence or two. -->

## Why

<!-- The motivation. Link the issue it closes: "Closes #NNN". -->

## Checklist

- [ ] `just verify` passes (fmt + clippy `-D warnings` + tests)
- [ ] Tests added/updated alongside the change (behaviour-focused)
- [ ] If it touches the scorer's output: benchmarked, with evidence recorded under `docs/research/evidence/`
- [ ] If it touches the landing site (`landing/`): `just landing-check` passes
- [ ] No research-era jargon (`era`, `phase`) in production code, CLI help, or user-facing docs
