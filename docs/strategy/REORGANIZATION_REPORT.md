# Strategy Corpus Reorganization — Report

Date: 2026-07-22. Task: create the founder manifesto and move the strategy corpus into
`docs/strategy/`, updating references. The strategy was frozen; no substantive conclusions changed.

## Files created

- `FOUNDER.md` (repo root) — one-page founder operating manifesto (~1,000 words). Consistent with the canonical strategy; contains the required "operating manifesto; canonical documents win if they conflict" note and the canonical-references block pointing to `docs/strategy/`.
- `docs/strategy/REORGANIZATION_REPORT.md` — this report.

## Files moved (root → `docs/strategy/`, no duplicates, no stale root copies)

- `ARGOT_STRATEGY.md`
- `ARGOT_STRATEGY.html`
- `ARGOT_STRATEGY_CARD.md`
- `ARGOT_CURRENT_REALITY.md`
- `ARGOT_PRODUCT_GAPS.md`
- `ARGOT_STRATEGY_CHANGELOG.md`

Canonical filenames preserved exactly. Moved with `mv` (the files were untracked; no history to
preserve). The moved files are the hardened versions (normative hierarchy, `ARGOT_CURRENT_REALITY.md`
authoritative on facts, Standing Decisions D1–D14, trusted-core definition, North Star measurability
conflict, requirements separated from reality, contributor decision test).

## Filenames normalized

None required. No `(1)`-suffixed or transfer-mangled filenames existed; the six files were already at
the canonical names.

## References updated

- **`FOUNDER.md`** — canonical-references block links to `./docs/strategy/<file>` (all six verified to resolve).
- **`docs/strategy/ARGOT_STRATEGY.md`** — inter-doc links to `./ARGOT_CURRENT_REALITY.md` and `./ARGOT_PRODUCT_GAPS.md` remain valid (siblings moved together). §0.1 wording updated: names `docs/strategy/ARGOT_STRATEGY.md` as canonical, `ARGOT_CURRENT_REALITY.md` as authoritative on current fact, and `FOUNDER.md` as a non-overriding manifesto shortcut. Markdown remains canonical; HTML and card remain derived.
- **`docs/strategy/ARGOT_STRATEGY.html`** — normative banner updated to note the canonical source path and that `FOUNDER.md` is a shortcut, not a replacement. Companion-file references are bare local sibling filenames (correct in the new directory). No external URLs changed.
- **`docs/strategy/ARGOT_STRATEGY_CARD.md`, `ARGOT_CURRENT_REALITY.md`, `ARGOT_PRODUCT_GAPS.md`, `ARGOT_STRATEGY_CHANGELOG.md`** — reference companion files by bare local sibling name; all remain valid after the group move; no changes needed.

A repo-wide search found **no other files** referencing the old root-level strategy paths, so no
further reference updates were required.

## Instruction file updated for discoverability

- **`CLAUDE.md`** (internal agent/contributor instructions) — added a short "Strategy & positioning" pointer directing readers to `FOUNDER.md`, `docs/strategy/ARGOT_STRATEGY.md`, and `docs/strategy/ARGOT_CURRENT_REALITY.md` before changing product, positioning, website, or public claims. Minimal addition; existing guidance unchanged. `AGENTS.md` was deliberately not modified (it is a published product surface).

## Verification performed

1. All six hardened files present in `docs/strategy/` — confirmed.
2. No stale root-level copies — confirmed (`ls ARGOT*` at root: none).
3. No `(1)` filenames anywhere — confirmed.
4. All relative Markdown links from `FOUNDER.md` and the moved docs resolve — confirmed by an automated resolver (all `OK`, zero `MISS`).
5. HTML companion-file references use correct local sibling filenames — confirmed.
6. Repo-wide search for obsolete old-path references — none found.
7. `FOUNDER.md` does not claim accept-time automatic checking exists ("not yet fully shipped") — confirmed.
8. `FOUNDER.md` does not imply direct retention measurement ("conceptual … cannot directly observe") — confirmed.
9. `FOUNDER.md` preserves the reality/decision/hypothesis/future-option distinctions — confirmed.
10. No application code, website copy, README marketing, CLI behavior, or branding altered — confirmed (`git status` scope limited to `FOUNDER.md`, `docs/strategy/`, `CLAUDE.md`).

## Unresolved / ambiguous references

None. All relative links resolve; no broken or ambiguous references remain.

## Confirmation

No product behavior, website, README marketing, CLI, application code, or branding was changed. The
only edits beyond the file move are: the new `FOUNDER.md`, minimal canonical-path/`FOUNDER` wording
in the moved `ARGOT_STRATEGY.md` and `ARGOT_STRATEGY.html`, and a small discoverability pointer in
`CLAUDE.md`. The strategy's substantive conclusions are unchanged.
