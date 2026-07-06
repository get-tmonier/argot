# Phase 1a — fastapi setup (from scratch)

Repo: fastapi @ 88021c3d · argot v0.2.52 · fresh clone to ~/projects/fastapi

## What happened
- `argot init` → **Ready** in ~2.6s. python threshold 7.79, 367 candidate hunks.
- `argot init --suggest` → "nothing stood out" (no generated/data-heavy dirs).

## GOOD
- Install→Ready in one command, seconds. `.argot/.gitignore` auto-written.
- Correct language detection (100% python; 4 stray TS files handled).

## PAIN / judgment call (documentable)
- **The voice is ~90% example code, not library source.** `fastapi/` (authored
  library) = 48 files; `docs_src/` (tutorial examples) = 454; tests = 581.
  Default include = 496 → dropping `docs_src/` collapses it to exactly 48.
- `--suggest` can't catch this — docs_src isn't generated or data-heavy, it's a
  *semantic* call (is example code part of "our voice"?). This is precisely the
  LLM/human-decides-exclusions moment.
- Guidance: on a **framework/library** repo with a large examples/tutorial tree,
  a contributor to the library should exclude it (`docs_src/`) so the voice is
  the library's internals; an app author using the framework would keep it.
  Foreign-dependency catches fire either way; the exclusion mainly sharpens the
  false-alarm rate on library-style code.
