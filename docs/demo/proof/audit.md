## argot audit

**last 1 commits** (2026-01-01 → 2026-01-02) · 1 commit audited · **0%** carry AI markers (0 of 1) · **1 finding** argot would have raised before merge

| group | findings | note |
|---|---|---|
| voice | 1 | |
| semantic | — | skipped: not compiled into this build |
| architecture | — | skipped: not compiled into this build |
| integrity | — | skipped: not compiled into this build |

**Worst offender** — `src/receipt.py:L1-10` · rare-tokens · `705db9c` (human)
> "add authored foreign import"
> ↳ import (0×), class (0×), def (0×) (+4 more)

Merged code is accepted code — read each finding as "would have prompted review before merge", not a bug list.

Method: findings are patterns that survive the audited base-to-head change. AI-marker attribution is a floor, not a census; "human" means no marker was found.

Next: `argot init` fits today's voice so `argot check` raises these before they merge.

Then choose a recurring path you configure: [pre-commit](https://argot.tmonier.com/docs/ci/) runs automatically at commit time once configured; the GitHub Action runs automatically in CI once configured.
