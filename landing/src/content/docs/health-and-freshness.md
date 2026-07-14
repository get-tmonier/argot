---
title: Health & freshness
description: How argot judges its own fit (the Ready / Ready-with-notes / Not recommended verdict), keeps the model fresh in the background, and detects drift — so you never have to guess when to refit.
group: Guide
order: 6
---

A fitted voice is a snapshot, and snapshots age: the repo merges new modules,
you edit `argot.toml`, a generated directory appears. argot treats keeping the
model trustworthy as **its own job** — every fit grades itself, every check
reads that grade, and staleness triggers a background refresh. This page is
the one place that explains the whole loop.

<figure class="diagram">
<svg viewBox="0 0 920 250" role="img" aria-label="argot's freshness loop: fit writes the voice artifacts and health.json into .argot; check reads them and prints one-line notes; when accepted history moved past the fit or the config changed, check spawns a background refit that runs fit again.">
  <defs>
    <marker id="ahf" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--muted)"></path></marker>
  </defs>
  <g class="d-node"><rect x="24" y="60" width="150" height="58" rx="11"></rect><text x="99" y="86" class="d-cmd">fit / init</text><text x="99" y="104" class="d-sub">grades its own corpus</text></g>
  <g class="d-node d-artifact"><rect x="250" y="60" width="240" height="58" rx="11"></rect><text x="370" y="85" class="d-file">.argot/</text><text x="370" y="103" class="d-sub">voice artifacts + health.json</text></g>
  <g class="d-node d-accent"><rect x="566" y="60" width="150" height="58" rx="11"></rect><text x="641" y="86" class="d-cmd">check</text><text x="641" y="104" class="d-sub">every diff</text></g>
  <g class="d-out d-ok"><rect x="782" y="62" width="114" height="26" rx="8"></rect><text x="839" y="79">score + notes</text></g>
  <line class="d-link" x1="174" y1="89" x2="250" y2="89" marker-end="url(#ahf)"></line>
  <line class="d-link" x1="490" y1="89" x2="566" y2="89" marker-end="url(#ahf)"></line>
  <line class="d-link" x1="716" y1="75" x2="782" y2="75" marker-end="url(#ahf)"></line>
  <line class="d-flow" x1="174" y1="89" x2="250" y2="89"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="490" y1="89" x2="566" y2="89"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.3s" repeatCount="indefinite"></animate></line>
  <path class="d-thread" d="M641,118 C641,196 200,196 105,124" marker-end="url(#ahf)"></path>
  <text x="370" y="182" class="d-thread-label" text-anchor="middle">stale? (accepted source moved · argot.toml changed) → background refit</text>
</svg>
<figcaption><code>fit</code> writes the voice and its own health record; <code>check</code> reads both — and when the snapshot goes stale, it quietly schedules the next <code>fit</code> itself.</figcaption>
</figure>

## The verdict — Ready / Ready with notes / Not recommended

Every `fit`/`init` ends with a verdict on the corpus it just learned from. It
is a statement about **calibration confidence**, not about your code:

| Signal | Verdict |
|---|---|
| A modeled language has **under 50 candidate hunks** to calibrate on | Not recommended — the threshold would be unstable |
| Under **200 candidate hunks** | Ready — with notes: usable, but the threshold is seed-sensitive |
| **No supported source files** at all | Not recommended |
| Two+ languages each hold ≥ 20% of the corpus | Ready — with notes: consider [per-language thresholds](/docs/the-commands/#sliced-calibration-per-subdirectory--per-author) (argot calibrates one per language automatically) |
| The calibrated threshold leaves no headroom for phrasing detection | Not recommended — argot would be an import tripwire only on this fit |

Only languages that meaningfully shape the corpus (≥ 20% of modeled files)
drive the verdict — a stray `.c` file in a TypeScript repo can't turn it red.
**Notes are expected on small repos**: they are tuning hints, not blockers —
the goal is a corpus that reflects how the team writes, not a spotless label.
When the verdict isn't a clean Ready, the fix loop is
[Setup — if the verdict isn't a clean Ready](/docs/setup/#if-the-verdict-isnt-a-clean-ready).

## health.json — the fit's self-record

Alongside the model, every fit writes `.argot/health.json` with three facts:

- **`fit_sha`** — the commit the voice was fitted at,
- **`config_fingerprint`** — a stable hash of `[exclude]` + `[detect]` at fit
  time (only the sections that change *what is learned*; `[rules]` and
  `[update]` don't count),
- **`drift_candidates`** — directories that look generated or data-heavy but
  aren't excluded yet.

Nothing reads it but argot itself: `check` prints a one-line note when the
record says the model is out of step, and `argot status` renders it as the
one-stop health view (fitted SHA, commits behind, config in sync or not,
unexcluded noisy directories).

## Staying fresh — the background auto-refresh

Staleness is measured against **accepted history**: the merge-base of your
HEAD with the default branch. A feature branch's own commits never count —
they're the code argot is judging, and must never become the voice it judges
against. And not every accepted commit counts either: only ones that touch
**in-scope source** (per your excludes) age the voice — a docs sprint or CI
churn doesn't.

When accepted history gains **`refresh-after` such commits** (default 10) since
the fit — or the fit is a week old with any such drift, or **`argot.toml`
changed since the fit** — `check` spawns a detached background refit and tells
you in one dim line:

```text
argot: voice model is 10+ accepted source commit(s) behind — refitting in the
background; your next check uses the fresh voice ([fit] auto-refresh = false to disable)
```

The check you just ran still used the old model — zero added latency; the next
one scores against the fresh voice. Guardrails, so this never surprises you:

- the refit **fits at the accepted anchor in a throwaway worktree** whenever
  HEAD isn't that anchor or the tree is dirty — unmerged branch commits and
  uncommitted edits never train the voice,
- at most **one attempt per day**, one refit at a time (a lock file),
- the semantic index reuses embeddings of unchanged functions, so a routine
  refresh costs seconds,
- the integrity gates (`.argot/integrity.json`) refresh the same way as the
  rest of the voice — a stale mini-replay never keeps judging you against
  history the repo has since moved past,
- **never runs in CI** (the Action refits per base advance instead),
- a failing refit doesn't retry silently — the next check says
  `the last background refit failed — run argot fit to see why`,
- and none of it weighs on `check`: the fresh case costs one commit-graph
  query, and the deeper scan stops the moment the threshold is crossed.

All three knobs live in `argot.toml`, written explicitly by `init`
([Configure](/docs/configure/#fit--the-background-auto-refresh)):

```toml
[fit]
auto-refresh = true
refresh-after = 10                # accepted in-scope commits before a refresh
refresh-from = "default-branch"   # auto-detects the trunk (origin/HEAD → main → master);
                                  # name one ("develop"), or "current-branch" to opt out
```

A **manual** `argot fit` follows your branch — that's your call to make. When
that call would fold unmerged branch commits into the voice, fit says so in a
quantified warning (never a prompt — agents and hooks drive fit too) and stays
silent when the branch adds nothing in scope or you've set
`refresh-from = "current-branch"`.

Opt out with `[fit] auto-refresh = false` and drive `argot fit` yourself; a
fit older than 90 days then earns a one-line nudge, nothing more.

## Drift — when the tree outgrows the config

Freshness is "the model is behind the history". **Drift** is "the tree grew
something the voice shouldn't learn" — a new `gen/` directory, a vendored SDK,
a wave of data files. Every fit re-scans for it (that's `drift_candidates`),
and `check` surfaces it:

```text
[argot] 2 directories look generated or data-heavy and are shaping the voice — review `argot init --suggest`
```

`argot init --suggest` shows the evidence; you decide what goes into
`[exclude].paths` ([Configure](/docs/configure/#exclude--set-the-scope)), then
re-fit. The suggestion only names directories *not already excluded*, so a
well-configured repo stays quiet.

## Staleness argot refuses to guess about

Two artifacts carry their own identity so a mismatch fails loudly instead of
scoring wrong:

- **`semantic-index.json`** records the embedding model that built it. An
  index from a different model or argot version is rejected — `check` prints
  `semantic index … — run argot fit to rebuild` and skips the
  `redundant`/`misplaced` rules for that run rather than comparing vectors
  from different spaces.
- **`manifest.json`** records hashes of everything learned (per-language model
  hashes, the fit commit, corpus size) — two fits of the same corpus and
  config are provably identical. `argot inspect --model` reads it.

## Updates — the binary and the model

argot never updates itself unasked. At most once a day it makes one cached
GET to `version.json` and prints a dim notice when a newer release exists;
`argot update` performs the self-update (curl installs) or points npm installs
at `npm install -g @tmonier/argot@latest`. When a release pins a **new
embedding model**, `argot update` says so: the next `fit` downloads it
(~100 MB, sha256-verified) and rebuilds the semantic index from scratch.
Opt out of the notice with `[update] check = false`
([Configure](/docs/configure/#update--the-passive-update-notice)).
