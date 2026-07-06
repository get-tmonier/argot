# V1 launch dogfood — findings & unclear scenarios

**Date:** 2026-07-06 · **Binary under test:** public `@tmonier/argot@0.2.54` (npm latest), not a dev build.

## Method

Six autonomous subagents each played a **brand-new, skeptical user + their AI agent**
evaluating argot for the first time, on a real forked corpus — one per language:

| corpus | language | verdict on `init` | star verdict |
|---|---|---|---|
| fastapi | Python | Ready | marginal-YES |
| hono | TypeScript | Ready | NO¹ |
| ripgrep | Rust | Ready | YES (caveat) |
| gh-cli | Go | Ready | NO¹ |
| rubocop | Ruby | Ready | YES (caveat) |
| fmt | C++ | **Not recommended** | YES (qualified) |

Each ran the full loop: read the public docs → install → clone → `argot init` (+`--suggest`) →
plant a foreign dependency in real source → `argot check` → negative test (in-voice change stays
quiet) → documented-limit test (in-vocab convention break stays silent) → CI-range simulation
(`fit` on base, `check base..HEAD`, then a fix commit) → `argot mute` lifecycle.

¹ The hono/gh-cli "NO" verdicts were **methodology artifacts of bug #2 below**, not detection
failures — see "Root cause" — but they show how easily a real user trips the same wire.

## Consolidated findings

### Tier 1 — trust-breakers (FIXED on `fix/dogfood-v1`)

**#1 — Mutes were silently not committed.** `argot init` writes `.argot/.gitignore` as a blanket
`*`, which also ignores `.argot/suppressions.yaml`. But the docs promise a mute is a *committed,
shared audit trail* ("the rule is committed… the noise never comes back"). As shipped, a mute made
locally never reached a teammate or CI — the same hit reappeared. **All six agents** hit or
independently verified this (`git check-ignore -v` / fresh-clone test). *Fixed:* generated ignore
re-includes `!suppressions.yaml`; `argot mute` now prints "→ commit .argot/suppressions.yaml to
share this decision." Verified: `git add` now accepts the file; the model stays ignored.

**#2 — `argot fit`/`init` learns from the dirty working tree.** Calibration reads files as they
are on disk, so an *uncommitted* foreign change is folded into the learned voice and then read as
familiar — with no warning. Confirmed deterministically on a clean 12-commit repo: baseline
`import_modules = [json, typing]`; add `import requests` **uncommitted** + `argot fit` → vocab
becomes `[json, requests, typing]` → the foreign import now scores **0 hits**. This is worst-case
for the north-star scenario (an agent runs `argot init` mid-task while its own foreign code sits
uncommitted) — and it explains hono's "axios missed" (the agent had *committed* the plant on the
branch it later fit over) and gh-cli's "gorm missed" (gorm still sat in the working tree at fit
time). Corroborated by ripgrep (explicit repro: unstaged `use reqwest;` + fit → `reqwest` enters
`import_modules`) and fastapi. *Fixed (guard):* `fit`/`init` now warns, listing the dirty source
paths and telling the user to commit/stash first. Bench-neutral (the benchmark fits clean pinned
SHAs). *Deeper fix recommended below.*

### Tier 2 — real, needs a decision (NOT yet changed)

**#3 — `.h` headers are classified as C, not C++.** In fmt, all 15 `include/fmt/*.h` files landed
under language "c" and only the 4 `src/*.cc` under "cpp" — a pure extension split. Header-centric
C++ (templates/logic in `.h`, a very common convention) is trained/scored under the *C* model,
starving both buckets and producing the discouraging "Not recommended" verdict on a famous,
high-quality C++ library. *Recommendation:* when a repo contains `.cc`/`.cpp`/`.cxx`, treat `.h`
as C++ (or content-sniff). Needs a small heuristic + parity/bench check.

**#4 — Deeper fix for #2: calibrate from committed HEAD, not the working tree.** The warning is a
guard; the root fix is to build the vocab + call-receiver corpus from HEAD blobs (like the BPE path
already does via `git_walk`). Provably bench-neutral on clean checkouts, but it's a semantic change
to the calibration corpus that deserves an explicit nod + a parity/golden run before shipping.

**#5 — `argot check` exits 1 on `unusual` (lowest tier).** A naive hand-rolled CI wiring
(`argot check || fail`) would gate merges on merely-`unusual` noise, contradicting the repeated
"never block" philosophy. (The shipped GitHub Action is already non-blocking; this bites people who
wire the CLI by hand.) *Recommendation:* document the intended `--min-severity` for gating, and/or
reconsider the default exit-code tier.

### Tier 3 — polish (docs/wording/UX)

- **#6 — "0 commit(s) (master..HEAD)"** printed when a range nets to an empty diff but has real
  commits (add-then-revert). Correct behavior, misleading wording. (fastapi, rubocop, fmt.)
- **#7 — getting-started still uses the vague "voice / token distribution diverges / doesn't sound
  like anyone" framing** — overpromises the subtle in-vocab catches and contradicts the sharpened,
  honest landing positioning. (Confirmed directly; ripgrep/rich noted the `--help` tagline is
  likewise loose.) *Doc-alignment fix.*
- **#8 — `fit` and `mute` aren't discoverable from getting-started** (rubocop, fmt); the full
  lifecycle only surfaces via `argot --help`.
- **#9 — `.argotignore` (source scope) vs `.argot/.gitignore` (model) are easy to confuse**, and the
  landing never mentions `.argotignore` (gh-cli).
- **#10 — `argot init --suggest --format json` drops the reasoning** the human format gives (hono);
  JSON is `{"candidates": []}` with no explanation.
- **#11 — no machine-readable "suppressed" signal in check JSON** — the count is stderr-only (fmt).
- **#12 — install naming:** npm `@tmonier/argot` vs GitHub `get-tmonier/argot` reads as two names
  for one project (fastapi).

## What genuinely worked (the trust signal)

- **Evidence quality** on a true foreign hit is excellent and consistent across languages:
  `↳ reqwest (L12) — never seen in repo` / `common here: std (51×), bstr (17×)…`. Names the foreign
  symbol, quantifies novelty, shows what the repo reaches for instead.
- **Zero false positives** across every agent's negative test (in-voice change) *and* documented-limit
  test (in-vocab convention break stayed silent, exactly as designed). The honesty claim held.
- **Speed:** clone + `init`/`fit` were seconds on every corpus (e.g. ripgrep ~5s, fmt ~7s).
- **Setup safety:** `--suggest` never once proposed excluding real primary source; generated/vendored
  dirs (gh-cli's gRPC stubs) were caught.
- **Net-diff range check** correctly cancels add-then-revert to a clean result.
- **Content-hash mutes** survive rebases and re-fits.
- Rust, Ruby, and Go all felt first-class (adapters, calibration, generated-file detection). Only
  C++ (via #3) felt second-class.

## Unclear scenarios to polish for V1 (the punch-list)

1. **Dirty-tree fit** (#2/#4) — the setup skill literally says "run `argot init` first," which an
   agent may do mid-task. Guarded now; decide on the deeper fix.
2. **The "clean run" trust trap** — a clean `argot check` means "no foreign pattern found," *not*
   "idiomatic." Must read consistently everywhere (landing ✓, skill ✓, but getting-started #7 ✗).
3. **Mute lifecycle end-to-end** — create → commit → survives re-fit → teammate/CI sees it. Fixed #1
   makes this true; worth an explicit doc walkthrough.
4. **Header-only C++ and other header/extension ambiguities** (#3).
5. **Hand-rolled CI vs the Action** (#5) — the exit-code/severity contract for gating.
6. **Small / shallow-history repos** — graceful, honest "Not recommended," with an actionable next
   step (fmt's "Not recommended" dead-ended: `--suggest` can't fix a low-hunk-count verdict).
7. **First-value latency & lifecycle discoverability** (#8) — `init → check` is discoverable;
   `fit`/`mute`/CI should be one click away from the front door.
8. **Two-name confusion** (#12) and `.argotignore` vs `.argot/.gitignore` (#9).

## Shipped this pass

- `fix/dogfood-v1`: #1 + #2 (guard) with unit tests; `just verify` green.
- Doc-alignment pass: #7, #8, #9 (see the docs commits).

## Recommended, pending a nod

- #3 (`.h`→C++), #4 (calibrate-from-HEAD), #5 (exit-code/min-severity), #6 (wording), #10–#12.
