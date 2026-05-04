# Calibration scope and check-time scope must match

> **TL;DR.** What argot learns from at fit time governs what it can score
> at check time. Pre-PR, calibration excluded test files / data-dominant
> files / non-language extensions, but check-time happily ran the trained
> scorers on those files anyway, producing structural false positives
> ("test-register words look foreign because the model never saw test
> code"). Fix: apply the calibration filter set at check time too. This
> branch also locks down two scope leaks the calibration filter alone
> doesn't catch: VSCode's `.history/` backup dir and the import-graph
> scorer re-deriving its module set from current file contents on every
> run.

## Context

Argot's scorers (BPE n-gram, call-receiver, import-graph) are voice-of-
the-repo statistical models. Each is fitted on a curated subset of the
repo's files — `engine/argot/scoring/calibration/random_hunk_sampler.py`
filters to "production code only" by skipping:

- `test/`, `tests/`, `__tests__/`, `*.spec.*`, `*.test.*`, `test_*.py`,
  `conftest.py`
- `docs/`, `examples/`, `migrations/`, `scripts/`, `build/`, `dist/`,
  `__pycache__/`, `.tox/`, `.eggs/`, `.git/`
- `*.config.*` (vite / vitest / tsup / jest / rollup / …) and
  `.<x>rc.<y>` dotfiles (`.eslintrc.js`, `.prettierrc.json`, …)
- Files outside the calibration adapter's `file_extensions` (e.g. `.js`
  in a TypeScript-fitted repo)
- `adapter.is_data_dominant` files (≥80% top-level array / object
  literals — locale tables, generated lookups)

But `argot check` had no equivalent gate. It scored every changed file
through the trained scorer, even if the file's class was never in the
training distribution. On faker-js this produced three classes of
structural FPs in the wild:

1. **Test-register hits** — `expect`, `describe`, `actual`, `wrapper`
   look like foreign vocabulary against a production-trained BPE model.
   Three flagged hunks at `HEAD~20`, all in `*.spec.ts`.
2. **Locale data hits** — `src/locales/es/company/name_pattern.ts` is
   `export default ['{{person.lastName}}…', …]`. The BPE model fires on
   string-literal payloads it was never trained to tokenise.
3. **Build-config hits** — `tsup.config.ts`, `.prettierrc.js` flagged
   purely on calibration-vocabulary novelty.

## Decision

Apply the same file-level filter set at check time as at fit time. The
new `_is_out_of_scope(file_path, content, repo_root, language_extensions)`
helper in `check.py` mirrors `is_excluded_path` plus
`adapter.is_data_dominant` plus the language-extension gate. Hits in
out-of-scope files are dropped before scoring.

A second leak surfaced on WAR (a TypeScript monorepo): VSCode's
local-history extension dumps file backups under `.history/`. Those
copies were sneaking into both the calibration corpus *and* the
scorer's foreign-import surface — when the user added `import mongoose`
to a tracked file, VSCode immediately wrote a `.history/` backup
containing the same import, and the scorer absorbed mongoose into its
known-modules set. Add `.history` to the default-exclude set.

A third leak — subtler and more architectural — was the import-graph
scorer's `fit()` method, which read every file in `repo_corpus_files`
*at scorer construction time*. At check time the scorer is reconstructed
from the calibration JSON, so `fit()` re-reads current file contents.
Adding a brand-new foreign import to a tracked file polluted the model
on the very hunk that introduced it. Fix: snapshot the foreign-module
set + prefix set at fit time, persist them to `scorer-config.json` under
`import_modules` / `import_module_prefixes`, and restore via
`ImportGraphScorer.load_snapshot` at check load. Foreignness now reflects
what was known at fit, not what's on disk now.

## Why not configurable

The exclusion list is currently hardcoded. Two reasons:

1. **Lock-step with calibration.** If users could exclude `test/` from
   check but not from fit (or vice versa) the FP class returns. Both
   paths must read the same source of truth.
2. **`.argotignore` is the right home.** Issue
   [#57](https://github.com/get-tmonier/argot/issues/57) plans a unified
   suppression surface — `.argotignore` + inline comments + CLI mute. The
   right move when that lands is to ship `argot:recommended` as the
   default rule set and let users opt out per-repo. Until then the
   hardcoded list is a placeholder, not an architectural choice. See the
   issue thread for the design hooks.

## Consequences

- **No change to ergonomics on production code.** The classes filtered
  out are exactly the classes argot couldn't speak meaningfully about
  — the FPs were the only signal there.
- **Test files get no lint coverage today.** Acceptable: argot's voice
  model was never trained on tests. If users want test coverage, the
  honest path is to widen the calibration corpus, not paper over the
  scope mismatch at check time.
- **Re-fit required to pick up the import snapshot.** Older configs
  fall through to the legacy re-derive path with a warning-free
  fallback. The first re-fit after this branch merges populates the
  snapshot.
- **Behaviour stays in lock-step.** Whatever filter changes land next
  (e.g. via `.argotignore`) must be applied identically at fit and
  check. The two paths share `is_excluded_path`; that's the contract.

## See also

- Issue [#57](https://github.com/get-tmonier/argot/issues/57) — `.argotignore`
- `docs/agents/domain.md` — repo-wide doc conventions
- `engine/argot/scoring/calibration/random_hunk_sampler.py:is_excluded_path`
- `engine/argot/check.py:_is_out_of_scope`
