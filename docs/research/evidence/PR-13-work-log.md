# PR-13 work log — landing gates

**Worktree:** `/Users/damienmeur/projects/argot-wt-pr-13-20260722t1554`
**Branch:** `codex/pr-13-landing-gates-20260722t1554`
**Base:** `origin/main` `4616bc18776d48b340d6210ffbeb443d9b209b09`
**Date:** 2026-07-22

## Lease and exclusions

This PR owns landing navigation/modal accessibility, landing test/build tooling,
the landing-only CI job, and orphan landing/demo assets. Product copy, README
work, and general release workflows are excluded. The removed film component is
not recreated.

## Issue record

| Issue | Goal / files | Dependency and acceptance | Validation |
| --- | --- | --- | --- |
| LD-13 | Skip target/link, small-screen navigation semantics, focus restoration, and global focus styling in the landing layouts, Nav, and global CSS. | LD-06 merged; all primary navigation is keyboard reachable at 320px and skip bypasses navigation. | Playwright keyboard/mobile and responsive smoke. |
| LD-14 | Retained-film modal accessibility. | DR-11 selected removal and PR-12 removed `Film.astro`; not applicable. | Confirm no retained film component. |
| LD-15 | Built-output route, Markdown-twin, locale/hreflang, and sitemap gates in landing tooling and CI. | LD-01–12 merged; seeded broken route fails. | Landing check/build and Bun route test. |
| LD-16 | Representative axe/Lighthouse and responsive/reduced-motion checks, with a visual matrix. | LD-13–15 and DOC-01–14 merged; no serious desktop axe findings and reports are retained in CI. | Playwright, Lighthouse, matrix receipt. |
| AS-05 | Consumer inventory and orphan cleanup for landing/demo visuals and render instructions. | AS-03/04 and LD-11 merged; only README-owned `docs/demo/demo.gif` remains. | Reference search and render-script syntax smoke. |

## Boundaries

No capability claims, product copy, README wording, or general release workflow
is changed. LD-14 is recorded as not applicable because the film is removed.
