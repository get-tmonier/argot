# CLI Context

The TypeScript/Bun layer that exposes argot's four pipeline steps as commands and orchestrates the Python engine. Contains no scoring logic — it is a thin orchestrator over the engine.

## Language

**Voice**:
The observable style patterns of a repo — its idiomatic token choices, call conventions, and import vocabulary.
_Avoid_: style, patterns, conventions

**Voice profile**:
The set of fit artifacts produced by the pipeline that capture a repo's learned voice. Lives in `.argot/`.
_Avoid_: model, artifacts, trained state

**Pipeline**:
The four-step sequence — `extract → train → calibrate → check` — that first builds a voice profile, then scores hunks against it.
_Avoid_: workflow, commands, process

**Hunk**:
The unit of code the pipeline scores — a diff slice during `check`, or a sampled top-level function/class during `calibrate`.
_Avoid_: chunk, block, snippet

## Relationships

- A **pipeline** produces a **voice profile** (via `extract`, `train`, `calibrate`)
- A **pipeline** consumes a **voice profile** to score **hunks** (via `check`)
- Each pipeline step maps to one CLI module: `extract-dataset`, `train-model`, `calibrate`, `check-voice`

## Example dialogue

> **Dev:** "Should the CLI validate that the voice profile exists before running `check`?"
> **Domain expert:** "Yes — `check` is meaningless without a calibrated voice profile. If `.argot/scorer-config.json` is missing, fail fast with a clear message."

## Flagged ambiguities

- "style" was used in module names — resolved: **voice** is the domain term; module is now `check-voice`.
