# Domain Docs

## Before exploring, read these

- **`CONTEXT-MAP.md`** at the repo root — points to one `CONTEXT.md` per context. Read each one relevant to the topic.
- **`docs/research/`** — serves as ADR for this repo. Read entries that touch the area you're about to work in.

If any of these files don't exist, proceed silently. Don't flag their absence upfront.

## Layout (multi-context)

```
/
├── CONTEXT-MAP.md
├── docs/research/          ← system-wide decisions (replaces docs/adr/)
├── cli/
│   └── CONTEXT.md          ← TypeScript/Bun CLI context
└── engine/
    └── CONTEXT.md          ← Python engine context
```

## Use the glossary's vocabulary

When your output names a domain concept, use the term as defined in the relevant `CONTEXT.md`. Don't drift to synonyms the glossary explicitly avoids.

## Flag research conflicts

If your output contradicts a finding in `docs/research/`, surface it explicitly rather than silently overriding:

> _Contradicts research/04-import-graph-breakthrough.md — but worth revisiting because…_
