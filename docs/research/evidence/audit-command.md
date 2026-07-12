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

| corpus | lang | card | json | findings | commits (AI) | indep. AI recount | runtime |
|---|---|---|---|---|---|---|---|
| fastapi | Python | OK | OK | 2 | 50 (0) | 0 ✓ | 27 s |
| faker-js | TS | OK | OK | (see json) | 50 (1) | 1 ✓ | 17 s |
| express | JS | OK | OK | (see json) | 50 (1) | 1 ✓ | 5 s |
| bat | Rust | OK | OK | (see json) | 147 (2) | 2 ✓ | 37 s |
| guava | Java | — | — | — | — | — | — |
| jellyfin | C# | — | — | — | — | — | — |
| laravel | PHP | — | — | — | — | — | — |
| rubocop | Ruby | — | — | — | — | — | — |
| curl | C | — | — | — | — | — | — |
| rocksdb | C++ | — | — | — | — | — | — |
| hugo | Go | — | — | — | — | — | — |
| dagster | monorepo Py+TS | — | — | — | — | — | — |
| excalidraw | TS | — | — | — | — | — | — |

*(table completed below as batches landed — see final numbers)*

**Attribution spot-check (the 0-false-`ai-assisted` gate):** every commit the
classifier marked AI across bat / express / faker-js / argot-itself was
opened and eyeballed — all carried genuine markers:
`Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>` (bat),
`Co-Authored-By: Claude Opus 4.6 <…>` (bat, express),
`Co-authored-by: Copilot <175728472+Copilot@users.noreply.github.com>`
(faker-js). The model-name variants confirm matching on the **email**, not
the display name, was the right call. fastapi's window (Apr 2026) reported
0/50 — independently confirmed.

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

## Example artifacts

Real cards captured during validation (fresh clones, unedited output) live
with the run logs; the README carries the argot-self-audit terminal card, and
`landing/src/components/Audit.astro` plays the same run abridged.
