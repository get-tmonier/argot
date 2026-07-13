# Evidence — `argot replay` → `argot audit` (history scorecard + AI attribution)

**Date:** 2026-07-12 · **Branch:** `feat/audit` · **Status:** shipped on branch
**PRD:** `.scratch/audit-command/PRD.md` · **Marker table:**
`.scratch/audit-command/ATTRIBUTION-MARKERS.md`

## What changed

`replay` became `audit` — a clean reshape (pre-prod, no alias). The replay
mechanics stayed as the engine (fit at base in a temp worktree, score
`base..HEAD`, window auto-shrink); on top of it:

- **Windows:** `--commits N` (default 50) + new `--since <duration|date>`
  (`90d`/`12w`/`6m`/`1y` or `YYYY-MM-DD`). Hard cap 1,000 commits; every
  clamp (cap / history-or-shallow / fit-shrink) is loud on stderr *and*
  recorded on the card (`window.clamp`, `clamp_note`).
- **AI attribution:** every commit in the range and every finding's
  *introducing commit* is classified `ai-assisted` / `human` / `unknown` from
  **concrete markers only** (agent `Co-authored-by` trailers matched by
  email, agent GitHub-bot authors matched by slug with the numeric-ID prefix
  stripped, agent footer lines, aider's name suffix). Allowlist only — no
  generic `[bot]` rule, so dependabot/renovate/github-actions stay out.
  Style is never evidence; `unknown` = the introducing commit can't be
  resolved (blame failure / shallow horizon), never folded into `human`.
  Introducing commits come from `git2` blame bounded to `base..HEAD`
  (most-lines-wins per finding span) — the old replay could only show the
  range head SHA; audit shows the true culprit commit.
- **Formats:** terminal card (default) · `json` (stable `schema_version: 1`)
  · `markdown` (PR-pasteable) · `html` (single self-contained file, inline
  CSS, zero external requests, light+dark).
- **Zero-setup:** works on a fresh clone with no `.argot/`/`argot.toml`; the
  semantic model downloads on first use with progress; under
  `ARGOT_OFFLINE=1` the semantic group is **marked skipped** on the card,
  never rendered as a silent zero.
- One engine-adjacent fix: `fit_repo`'s "Step 1/2 / 2/2" progress lines moved
  stdout → stderr (they contaminated `audit --format json`'s document;
  status never belongs on a machine format's stdout).

## Validation (fresh local clones, zero config)

Per-corpus harness: clone `benchmarks/data/<c>/.repo` → bare `argot audit`
(terminal) + `--format json` → assert exit 0, card/JSON count consistency,
80-col layout, no ANSI when piped, every `ai-assisted` finding's markers
re-verified against `git log` in the clone, plus an *independent* regex
recount of AI-marked commits over the whole range.

All rows: exit 0 twice, card↔json counts consistent, ≤80-col layout, no ANSI
piped, independent AI recount = reported. Runtimes are the terminal-card run
(fresh clone, full fit incl. semantic embed, NO seed) and most ran **under
3-way CPU contention** — see "runtime" below for the solo gate measurement.

| corpus | lang | findings (v/s/a/i) | hunks | commits (AI) | indep. ✓ | runtime |
|---|---|---|---|---|---|---|
| fastapi | Python | 2 (2/0/0/0) | 3 | 50 (0) | ✓ | 27 s |
| faker-js | TS | 0 | 1057 | 50 (1) | ✓ | 17 s |
| express | JS | 0 | 14 | 50 (1) | ✓ | 5 s |
| bat | Rust | 3 (3/0/0/0) | 59 | 147 (2) | ✓ | 37 s |
| guava | Java | 34 (2/32/0/0) | 573 | 50 (0) | ✓ | 1156 s* |
| jellyfin | C# | 1 (0/1/0/0) | 115 | 116 (2) | ✓ | 397 s* |
| laravel | PHP | 11 (0/8/0/3) | 178 | 60 (3) | ✓ | 471 s* |
| rubocop | Ruby | 1 (0/1/0/0) | 124 | 102 (0) | ✓ | 202 s* |
| curl | C | 4 (0/4/0/0) | 150 | 50 (0) | ✓ | 271 s* |
| rocksdb | C++ | 89 (18/71/0/0) | 840 | 50 (0) | ✓ | 1721 s* |
| hugo | Go | 6 (2/2/0/2) | 191 | 51 (8) | ✓ | 246 s* |
| dagster | monorepo Py+TS | 9 (2/5/2/0) | 321 | 50 (9) | ✓ | 554 s |
| excalidraw | TS | 11 (4/5/0/2) | 375 | 50 (3) | ✓ | 113 s |

\* under 2–3 concurrent corpus fits on one machine.

Marker classes confirmed on real history: Claude Code trailers (5 model-name
variants), `🤖 Generated with [Claude Code]` footers (dagster), Copilot
GitHub trailer (faker-js), VS Code Copilot `copilot@github.com` trailer
(excalidraw), Cursor `cursoragent@cursor.com` trailer (excalidraw).

### Runtime gate (target: ≤~2 min default window on the largest corpus)

Solo measurements on rocksdb (the slowest corpus, 4k+ C++ files, 840 hunks
in the window):

| path | time |
|---|---|
| fresh clone, zero setup (full semantic embed of the base tree) | **1188 s** |
| `argot fit` alone on the same clone (HEAD) | 966 s |
| audit after `argot init` (semantic index seeds the worktree fit) | **368 s** |

**Verdict: met on ordinary repos, honestly missed on giant ones.** 10 of 13
corpora audit fresh in ≤ ~4 min under contention (fastapi 27 s, express 5 s,
faker-js 17 s, bat 37 s, excalidraw 113 s solo…); rocksdb and guava are
embed-dominated — ~80% of the fresh cost is `fit`'s semantic index build,
which audit inherits and which shows live progress throughout. The seed cuts
repeat runs 3.2× and the fresh and seeded cards are byte-identical (the seed
changes speed, never findings). This is the price of zero-setup with the embedding layer
on repo-scale outliers, published red rather than gamed (no hunk caps, no
silent semantic skip).

**Follow-up (delivered):** the runtime was then instrumented and attacked —
a machine-wide content-addressed embedding cache (repeat encounters reuse
vectors across checkouts), plus deterministic parallelism for the three
phases the split surfaced (placement calibrate, integrity replay, check-time
scoring). Findings stay byte-identical; sequence batching was implemented,
measured, and **rejected** for flipping a cosine tie. Full phase-split table,
final numbers, and the B4 anchoring rejection: [`audit-runtime.md`](audit-runtime.md).

Negative control caught in the wild: a dagster commit whose message says
"auto-reordered with Claude" in *prose* was correctly left `human` — prose
mentions are not concrete markers — while the window's 9 genuinely-marked
commits (trailers + `🤖 Generated with [Claude Code]` footers) were all
caught.

### Fix loop (real defects the validation surfaced, each fixed + pinned by a test)

1. **`fit` progress on stdout** contaminated `--format json`'s document →
   moved to stderr (status never belongs on a machine format's stdout).
2. **guava parallel trees** (`guava/` + `android/guava/` ship identical
   files): left-ellipsized paths erased exactly the distinguishing prefix and
   rendered two distinct findings as identical rows → middle-ellipsis.
3. **laravel deleted-test mis-attribution**: span-blame credited `9c01c82`
   where `git log -S` proves `4a03574` deleted the tests (blame can only see
   surviving neighbour lines) → `test-deleted` now resolves by content
   (pickaxe-style walk on the structured `symbol` field added to check's
   JSON hits), with a provable sole-touching-commit fallback, else honest
   `unknown`.
4. **hugo 83-col overflow**: `unfamiliar-callee` (17 chars) pushed rest-rows
   past 80 → the location column flexes around the widest rule/attribution.

**Attribution spot-check (the 0-false-`ai-assisted` gate):** every commit
the classifier marked AI across bat / express / faker-js / jellyfin /
laravel / hugo / argot-itself (17 marked commits) was opened and eyeballed —
all carried genuine markers: Claude trailers in several model-name variants
(`Claude Opus 4.8 (1M context)`, `Claude Opus 4.6`, `Claude Sonnet 4.6` —
all `<noreply@anthropic.com>`, confirming email-matching over name-matching)
and `Co-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>`
(faker-js). Zero false `ai-assisted` labels. The 0-AI windows (fastapi,
guava, rubocop, curl, rocksdb) were independently reconfirmed by a separate
regex recount over every commit in each range.

**Dogfood (argot's own history):** 50 commits, **52% AI-marked (26/50)**,
1 finding, worst offender correctly attributed to an `ai-assisted` commit
(`cae8349`, real Claude trailer) — the card embedded in the README is this
run, unedited.

## Degradation matrix (all verified)

| case | result |
|---|---|
| offline + empty model cache (`ARGOT_OFFLINE=1`, fresh `XDG_CACHE_HOME`) | exit 0; card shows `semantic — skipped: embedding model not available (offline?)`; voice findings intact |
| shallow clone (`--depth 10`, window 50) | exit 0; loud clamp; card headline honest ("9 commits audited") |
| short history (3 commits) | exit 0; clamp to root |
| docs-only window | exit 0; "touched no supported source files (docs-only?)" + widen hint only when unclamped |
| no source anywhere | exit 2 with the actionable `[exclude]` message |
| `--since 2015-01-01` (≫ cap) | exit 0; clamped to 1,000 with loud note in stderr + card + json (`"clamp": "cap"`) |
| `--commits 5000` | same cap behaviour |
| `--since 60d` on a stale snapshot (HEAD older than cutoff) | exit 2, honest "newest commit (2026-04-21) is older than --since 60d (2026-05-13)" |
| `--commits` + `--since` together | clap conflict, exit 2 |

## Design decisions of record

- **`human` = "no AI markers found"** — the card and every format say so
  explicitly ("the AI share is a floor, not a census"). An unmarked agent
  commit is indistinguishable from a human one; claiming otherwise would be
  style inference, which is banned.
- **Tools with no verified default marker are absent** from the tables
  (Amazon Q, Windsurf, Cline, Roo Code, Goose, Gemini CLI), as are
  LLM-powered *review commentary* bots that don't author code (coderabbitai,
  Qodo, Gemini Code Assist, copilot-pull-request-reviewer). One false
  `ai-assisted` costs more than several misses.
- **No MCP audit tool**: `argot mcp` serves the per-diff agent loop
  (check/explain/voice_context/fit_status); audit is a human-facing report.
  Nothing stale to rename there (verified).
- **Cap = 1,000 commits**: past that the "base voice is still the same repo"
  premise breaks; the clamp is loud in three places (stderr, card, json).
- Core-internal "replay" vocabulary (calibration replay, integrity
  mini-replay, auto-refresh worktrees) is intentionally untouched — those
  really are replays.

## Example artifacts (unedited output)

Working copies live in `.scratch/audit-command/examples/` (local); the
README carries the argot-self-audit card and
`landing/src/components/Audit.astro` plays the same run abridged. Durable
copies of the three most telling cards:

argot's own history (the README card — real Claude trailer attributed):

```
━━ argot audit ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  last 50 commits · 2026-06-01 → 2026-07-11 · 50 commits audited
  52% carry AI markers (26 of 50) · 1 finding would have met review

  voice    1  code foreign to how this repo writes

  Worst offender — commit cae8349 · ai-assisted
  ! landing/src/pages/llms-full.txt.ts:L1-32 · foreign-import
      "feat: v1 launch polish — on…" — Co-Authored-By: Claude Opus 4.8
      ↳ astro (L1), astro:content (L2), #lib/site (L3) — 0 of 49 module s…

  Merged code is accepted code — read each finding as "would have
  prompted review before merge", not a bug list. "human" means no AI
  markers were found; the AI share is a floor, not a census.
  Next: argot init fits today's voice so argot check raises these
  before they merge.
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
```

dagster (monorepo, three rule groups firing, 18% AI-marked):

```
━━ argot audit ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
  last 50 commits · 2026-04-30 → 2026-05-04 · 50 commits audited
  18% carry AI markers (9 of 50) · 9 findings would have met review

  voice           2  code foreign to how this repo writes
  semantic        5  functions you already had, or code filed oddly
  architecture    2  imports that break the repo's layering

  Worst offender — commit 648cd19 · human
  ! python_modules/l…ter-soda/dagster_soda/__init__.py:L1-10 · foreign-import
  …
```

offline degradation (semantic marked skipped, never a silent zero):

```
  voice       2  code foreign to how this repo writes
  semantic    —  skipped: embedding model not available (offline?)
```
