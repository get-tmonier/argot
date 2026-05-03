# Context Map

## Contexts

- [CLI](./cli/CONTEXT.md) — TypeScript/Bun layer that exposes the pipeline as commands and orchestrates the engine
- [Engine](./engine/CONTEXT.md) — Python scoring pipeline that learns a repo's voice and flags diverging hunks

## Relationships

- **CLI → Engine**: CLI spawns the engine as a subprocess via `BunEngineRunner`; all scoring logic lives in the engine
- **Shared concept**: **voice profile** — the CLI produces it (pipeline commands), the engine reads and writes the individual fit artifacts inside `.argot/`
