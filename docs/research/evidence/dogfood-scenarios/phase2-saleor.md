# Phase 2 — saleor (real Django e-commerce app, 4284 py files)

## Setup — CLEAN out of the box (contrast to fastapi's docs_src)
- `argot init` → **Ready** in ~14s. python threshold 6.91, 4215 candidates.
- **1432 Django migration files** → argot **already excludes** them (path rules;
  adding `**/migrations/` changed nothing). 4284 → 1139 source files included;
  tests + migrations + templates + deployment auto-dropped.
- `argot init --suggest` → nothing stood out (correct — generated code already
  excluded). On a real Django app the recommended set Just Works; no hand-tuning.
- Only friction: ~14s fit (vs ~2-7s on smaller repos) — fine for a one-time setup.

## Realistic foreign-reflex catch — WORKS
- saleor's HTTP stack: `requests` (7×), httpx 0×; task queue celery (149×);
  GraphQL graphene (482×).
- An agent adds a webhook sender with **aiohttp** (a different async HTTP client):
  `! foreign · aiohttp — 0 of 114 module specifiers in repo`, score 1.0, foreign.
  Correct — a foreign HTTP client on a repo that standardises on requests.

## Verdict
Real-app setup is smoother than the framework case (no example-code voice
dilution; migrations handled automatically). Foreign-dependency catches fire
cleanly at scale (114 module specifiers, 1139-file corpus). CI loop not re-run
here — proven end-to-end on hono (setup → PR card → fix → green).
