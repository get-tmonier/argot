# Convention mining → candidate rules, and the `argot report` dashboard

> **TL;DR.** argot already *learns* a repo's conventions (attested imports,
> call clusters, naming morphology, syntax idioms, `defined_symbols`) but
> keeps most of them locked inside statistical scoring. Two additions turn
> that latent knowledge into something a human can act on. **(1) A convention
> miner** — `argot rules suggest` — walks the fitted corpus, extracts the
> near-universal patterns the repo follows, and emits them as *candidate
> scripted rules*, each **back-tested against the repo's own accepted history**
> so a noisy candidate is demoted before it's ever proposed. **(2) A
> self-contained HTML dashboard** — `argot report --html` — renders the
> candidates (with evidence + a copy-pasteable Rhai scaffold), the voice
> fingerprint including the naming/syntax conventions no command surfaces
> today, the layering graph, and calibration health, so the model stops
> reading as a black box. The report is **pure visualization**; the actual
> accept → codify → gate loop is driven by a new skill, `argot-suggest-rules`.
> argot never installs a rule on its own.
>
> UX mockup (design-locked): the `argot report` dashboard —
> conventions-candidates as the hero, dominance meters + back-test chips,
> expandable scaffolds, interactive layering graph.

## Update — 2026-07-21: a throwaway prototype reshaped the miner

Before building the miner, a prototype tested its strongest template
(receiver-funnel) on 11 real corpora. **Negative result** — evidence in
[`../evidence/convention-miner-receiver-funnel-probe.md`](../evidence/convention-miner-receiver-funnel-probe.md).
Raw AST dominance cannot separate a *convention* (sanctioned form vs. avoided
substitute) from ordinary *namespaced library-API usage* (`torch.cat` is 99%
on `torch` for the same reason a real rule's receiver is dominant): tighten the
filters → recall goes to zero (7/11 corpora empty), loosen → junk floods (test
asserts, stdlib built-ins, library API, `$this`/fluent chains). Same
ungatable-AST-signal shape as the foreign-structure work.

**What survives, reshaped:**

- The **`argot report` dashboard is unaffected and ships** — it visualizes
  learned facts, no rule-proposal precision problem.
- **Receiver-funnel is demoted** from autonomous proposer to at most a ranking
  hint inside a template the human already picked.
- **The auto-suggest pivots to history-substitution mining** (probes in the
  evidence doc, Parts 2–3; AST-quality run quantified over **9 repos**): mine
  `A→B` substitutions the repo actually made across git history, gated by
  **path-containment** (new receiver reaches the old through a longer path —
  `settings → crawler.settings`, a real funnel; everything else is a variable
  rename, auto-dropped) and **liveness** (old form still at HEAD → enforceable
  vs. a completed migration → stale). Quantified result: **receiver-funnel +
  containment fires on ~1/3 of repos at ~100% precision** when it fires
  (scrapy `settings→spider/crawler.settings`, flask `session_interface→
  app.session_interface`, requests `r.close→r.raw.close`; ~90 renames
  auto-dropped across the set), **zero on the other ~2/3**. A secondary
  **import-swap** signal (~40–50% precision) complements it with cross-library
  migrations the funnel can't see (laravel `Exception→Throwable`, flask
  `jinja2→markupsafe`, rich `commonmark→markdown_it`). So the suggester is
  **opportunistic and high-precision, not universal** — an input for human
  curation, never an autonomous installer.
- **`argot-suggest-rules` becomes evidence-assisted authoring**, not a blind
  miner: argot presents the ranked live-substitution candidates + the report's
  facts (counts, canonical examples); the **human curates in one glance** (keep
  `settings→crawler.settings`, drop the `d→d2` variable rename — trivial for a
  person, impossible for the miner); argot then scaffolds + runs the
  accepted-history back-test + gates on `argot rules test`.

**The reliable LISTING signal (evidence Part 4).** The maintainer's core ask is
a *reliable way to list conventions* — fires on every repo, not the ~1/3 where
history speaks. Reframe: *listing* is more tractable than *auto-generating a
firing rule*, and the convention present in every repo is its **own most-used
internal API**. Ranking the repo's internal-import bindings + internal
receiver-funnels by cross-file fan-in (grounded in the adapter's
`callable_definitions` + `internal_import_bindings`, no fit/history) fires with
substantive output on **9–10/12 corpora**, and the **top of each list is the
repo's actual convention**: fastapi `app`, hono `c`/`c.req`/`Context`, dagster
`context`/`instance`/`context.log`, laravel `Str`/`Arr`, guava
`ImmutableList`/`Preconditions`, scrapy `logger`, redis `server`/`zmalloc`.
Residual noise (generic locals, JS globals) sits below the real conventions in a
short ranked list → trivially human-filtered.

**So the reliable convention list is a composite** (all computable at fit):
(1) naming + syntax idioms (`ConventionModel`, every repo), (2) familiar imports
(`import_modules`), (3) internal-API funnels + shared helpers (Part 4, ~every
repo), (4) historical live substitutions (Parts 2–3, opportunistic). (1)–(3)
make the LIST reliable everywhere; (4) is the bonus. A listed convention becomes
a *rule* only after human curation + back-test + the `argot rules test` gate.

This resolves the priority tension the maintainer flagged: **the reliable
convention *list* comes first** (this is the product's spine, not the HTML
report), rule-generation is the curated second step, and `argot report`/any
surface is just how the list is shown.

The sections below are the **original** design; read them through this update —
the reliable *listing* signal (Part 4) is the spine, the history suggester is
the opportunistic bonus, and the dominance-based `rules suggest` is dead.

## Context

Custom scripted rules (`.argot/rules/<name>/`, `--features script`) are the
sanctioned way to codify a repo convention the eleven built-ins don't cover
("HTTP through the shared client", "no raw SQL", "retries always back off").
The `argot-write-rule` skill and the Rhai host API (`ts_query`,
`import_attested`, `report`, …) make *writing* one tractable.

The gap the maintainer named: **you have to already know what the convention
is** before you can write a rule for it. Meanwhile the voice model has, at fit
time, computed exactly this kind of knowledge — and hides it:

| Learned knowledge | Persisted | Human-readable today |
|---|---|---|
| Familiar imports (`import_modules`) | yes | yes (`describe-voice`, `inspect --model`) |
| Call clusters + top callees | yes (`call_receiver.clusters`) | yes (`describe-voice`) |
| **Naming morphology** (snake/camel/pascal…) | yes (`ConventionModel.ident_shapes`, `model.rs`) | **no** |
| **Syntax idioms** (normal tree-sitter node kinds) | yes (`ConventionModel.node_kinds`) | **no** |
| **The repo's declared API** (`defined_symbols`) | yes (`call_receiver.defined_symbols`) | **no** |

So the maintainer is asked to reverse-engineer, by hand, conventions the tool
already knows. And separately: the fitted artifact, the cluster stats, and the
arch layer's `.argot/layering.json` graph are legible only to someone who
knows the internals — the tool reads as a black box.

Two reusable building blocks already exist and de-risk both additions:

- **`argot audit --format html`** (`crates/argot-cli/src/audit/html.rs`) proves
  a **single self-contained HTML file** — inline CSS, zero external requests,
  light/dark, screenshot-ready — is already in the binary's idiom. No server,
  no runtime dep, consistent with the single-static-binary invariant.
- **The audit/check replay path** already scores a window of history against a
  voice fitted just before it. That is exactly the machinery a back-test needs.

## Decision

Ship three pieces that compose: a miner that *proposes*, a report that *shows
and lets you review*, a skill that *codifies your choices*.

### 1. `argot rules suggest` — the convention miner

A new `RulesAction::Suggest` (sibling of the existing `Test`). Not a
`Detector`: it **produces** candidate rules, it doesn't apply any. It walks the
fitted corpus (same file set as the fit, honoring `[exclude]`) and runs a small
set of **convention templates**:

- **import-funnel** — a module ubiquitous across the corpus while a competing
  module is (near-)absent ("everything goes through `httpx`, never
  `requests`"). Cheap, from the import surface.
- **receiver-funnel** — a member-access receiver / callee that concentrates a
  category of operations ("all HTTP goes through `apiClient.*`; bare `fetch` is
  near-zero"). Needs an AST walk over the corpus; highest rule-value.
- **naming** — the dominant identifier morphology per language, already in
  `ConventionModel`. **Report-only** by default (see "Why not"): the voice
  layer already scores naming adaptively, so a hard rule adds little; its value
  is in the fingerprint, not as a `.argot/rules/` entry.

For every candidate the miner emits (`--json` and as report input):
`{kind, dominance (e.g. 61:0), backtest_fires, languages, ts_query,
canonical_example (file:line), violation_shape, suggested_severity, name,
description}`. The `ts_query` + examples are what let the skill scaffold a real
rule without guessing.

**The back-test is what keeps this fitted to argot's north star (catch @ low
false-alarm).** Each candidate is materialized as an *ephemeral scripted rule*
and run over a window of the repo's **already-accepted** commits via the
existing check/audit replay. The number of fires on accepted code **is** the
candidate's estimated false-alarm rate. argot dogfoods its own scripted-rules
engine to validate its own suggestions.

Threshold policy — **balanced** (maintainer's call):

- dominance ≥ 95% **and** 0 back-test fires → `ready`.
- 85% ≤ dominance < 95%, or a small number of back-test fires that the scaffold
  can `exclude`-scope → `verify` ("à vérifier"), surfaced but flagged.
- dominance < 85%, or back-test fires that can't be cleanly scoped → dropped
  (logged, not silently — the count of dropped candidates is reported).

A candidate must be expressible as a `ts_query` shape, or it isn't proposed —
the same detectability honesty `argot-write-rule` already enforces.

### 2. `argot report --html <file>` — the dashboard

A new top-level command emitting **one self-contained HTML file** (inline
CSS + vanilla JS, zero server, zero dep; extends the `audit/html.rs` pattern,
with JS added for the interactive graph and the expand/collapse). One page,
summary before detail:

1. **Conventions candidates** (the hero) — one card per candidate: severity
   stripe (`ready`/`verify`), dominance meter, back-test chip, and an
   expandable body with the evidence and the **full copy-pasteable rule
   scaffold** (`rule.toml` + `check.rhai`). This is the review surface.
2. **Voice fingerprint** — familiar imports, and finally the **naming
   morphology + syntax idioms** the model hid. This is the de-black-box.
3. **Layering graph** — interactive, from `.argot/layering.json`; a reversal
   edge is drawn distinctly to tie back to the `layering` rule.
4. **Calibration health** — per-language threshold table, from `inspect`.

The report is a **read-only artifact**: a static file in a browser can't write
to disk. It shows and recommends; it does not install. `--serve` is explicitly
rejected (would add a web-server dep, breaking the single-binary invariant).

### 3. `argot-suggest-rules` skill — the codify loop

Sibling of `argot-write-rule`. Flow: `argot rules suggest --json` → present the
ranked candidates with evidence → for each candidate the **user** accepts,
scaffold `.argot/rules/<name>/` (manifest + `check.rhai` + fixtures **derived
from the miner's `ts_query` + canonical/violation examples**, so no guessing) →
green the `argot rules test <name>` gate → hand off. `argot-write-rule` stays
for bespoke conventions; `argot-suggest-rules` starts from what argot found.

A thin `RulesAction::New { from: <candidate> }` scaffolding helper is the clean
seam for the skill to call (proposed; does not exist yet).

### Crate placement

The miner orchestrates several slices at once — the voice model
(`import_modules`, `ConventionModel`, `defined_symbols`), a corpus AST walk, and
a check-replay for back-testing. A rule crate never imports another rule crate,
so the miner is **not** a rule slice: it's a **facade / CLI-level concern**
(argot-core or argot-cli), consuming the engine's `ModelFacts` port and the
existing walk/replay plumbing. The report command is CLI-level for the same
reason. Surfacing the hidden `ConventionModel` / `defined_symbols` fields flows
through `LanguageModelView` (`inspect.rs`) so `describe-voice`, `inspect`, and
`report` all read the one view.

## Why not the alternatives

- **Auto-install the mined rules.** Rejected. A miner that installs rules from
  statistical dominance is a false-positive machine — the exact failure argot
  exists to avoid. Human-in-the-loop via the skill is the safeguard; the
  back-test only *ranks*, it doesn't earn the right to auto-commit. (This was
  the maintainer's explicit call.)
- **Mine the soft/statistical conventions (naming, imports) into hard
  `.argot/rules/` entries.** Rejected as the default. The voice layer already
  scores these adaptively; a frozen Rhai rule duplicating "foreign import"
  would be redundant and would go stale. These belong in the *fingerprint*
  (report), and only graduate to a rule when the maintainer wants a hard,
  locked invariant.
- **`argot report --serve` (ephemeral localhost).** Rejected. Adds a
  web-server dependency and a running-process model to a tool whose whole
  identity is a single static binary with no runtime. A self-contained HTML
  file is interactive enough and stays dependency-free.
- **Fold the report into `describe-voice --format html`.** Rejected.
  `describe-voice` is the focused STYLE.md generator; the report aggregates
  voice + arch + candidates + calibration into a dashboard. Different job,
  different command. `describe-voice` stays as-is.
- **Skip the back-test, rank on dominance alone.** Rejected. Dominance measures
  "common", not "a rule". A pattern can be 90% dominant and still have dozens of
  legitimate exceptions in history. The accepted-history back-test is the only
  thing that separates a convention from a habit.

## Consequences

- **Honest scope limit.** "Detect *all* conventions" is not achievable, and
  saying so is correct. A convention that needs type inference or cross-file
  binding resolution is not syntactically minable. The miner surfaces the
  *`ts_query`-expressible, statistically-dominant, history-validated* subset —
  which is exactly the subset that makes *good* rules.
- **Validation gate before ship.** The miner is the riskiest piece (per
  `feedback_validate_on_real_corpora`). It must be run against argot itself +
  several bench corpora and eyeballed **before** it ships — the metric is
  "candidates proposed that a maintainer would actually keep, at near-zero
  junk". A throwaway prototype (import-funnel + receiver-funnel) over 2–3
  corpora is the next research step, ahead of freezing the templates.
- **Agent-facing surfaces move together** (`CLAUDE.md` "keep in sync" list): a
  new command + skill means updates to `skills/` (the new `argot-suggest-rules`,
  bundled in `.claude-plugin/`), `crates/argot-cli/src/mcp.rs`, `AGENTS.md`,
  and `landing/` docs + i18n. Not in this decision's scope, but tracked.
- **Back-test cost.** Replaying a history window per candidate has a wall-time
  cost. The window size and whether candidates share one replay pass are build
  choices; default to a bounded window and report it.
- **New surfaced fields.** `ConventionModel.{ident_shapes,node_kinds}` and
  `call_receiver.defined_symbols` gain a human-readable path through
  `LanguageModelView`; `describe-voice` and `inspect` can adopt the same
  fields, closing the "learned but never shown" rows in the table above.

## Open questions (resolve during build)

- The **receiver-funnel heuristic**: how to cluster "a category of operations
  behind one receiver", and how to name the *competing alternative* the model
  doesn't currently store as a first-class fact (today it only knows what's
  attested, not what's the sanctioned substitute for what).
- **Back-test window**: fixed commit count vs since-last-fit vs the audit
  window; and whether a candidate's fires are attributed (like `audit`) to help
  the maintainer judge exceptions.
- Whether **naming** ever graduates from report-only to an opt-in rule template
  for repos that want a hard casing invariant.
