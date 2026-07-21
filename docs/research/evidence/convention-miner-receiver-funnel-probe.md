# Convention miner — receiver-funnel probe (negative, reshapes the design)

**Date:** 2026-07-21 · **Branch:** feat/rules-creation ·
**Decision it feeds:** [`convention-mining-and-report.md`](../decisions/convention-mining-and-report.md)

## Question

The convention-miner design (`argot rules suggest`) assumed the strongest
taxonomy-free template was **receiver-funnel**: mine methods `M` that the repo
calls overwhelmingly through one receiver `R` ("all HTTP goes through
`apiClient.*`; bare `fetch` is near-zero") and propose them as candidate rules.
Before building it, a throwaway prototype tested the decisive question:

> Does raw AST receiver-funnel dominance surface **rule-worthy conventions** at
> a low junk rate on real repos?

Prototype: `crates/argot-bench/examples/convention_probe.rs` (deleted after this
writeup). Uses argot's real `argot_lang::callees::non_none_callees` extraction —
not throwaway regex — so the signal is representative. No fit, no back-test yet:
this only measures the *raw dominance* signal's precision.

## Result: no. Raw dominance can't tell a convention from library-API usage.

### v1 — dominance ≥ 85%, ≥ 4 files, ≥ 8 calls

6 clean corpora (Python/JS/TS/PHP/Java). Candidate counts: fastapi **22**,
rich **10**, express **0**, hono **48**, laravel **86**, guava **82**. The
lists are dominated by four junk classes, not conventions:

- **Test-framework assertions** (scoping leak — `.test.`/`.spec.` files inside
  `src/`): hono `toBe`(116 files), `toEqual`, `toHaveBeenCalledWith`; guava
  `createTestSuite`, `addEqualityGroup`; laravel `PHPUnit.assertEmpty`.
- **Stdlib / language built-ins**: `JSON.stringify`, `Object.keys/entries`,
  `Math.random`, `Array.isArray`, `Arrays.fill`, `System.nanoTime`,
  `Class.forName`, `re.compile`, `os.path.dirname`, `datetime.now`.
- **Third-party library API**: `torch.cat`(678 files), `nn.Linear`,
  `playwright.chromium.launch`, `page.goto`, `parser.add_argument`,
  `Container.getInstance`.
- **Self-scope / fluent chains**: laravel is almost entirely `$this.*`
  (`$this.option` 74 files, `$this.argument`, `$this.newLine`); everywhere
  `<call>.method` (builder chains, receiver unknown); Python `<call>.__init__`.

Eyeballed rule-worthy yield: **~5–12%**. The genuine conventions that *do*
appear (hono `c.req.param`/`c.redirect`; laravel `Str.snake`/`Str.camel`) are
(a) rare, (b) mostly "use this library's idiom," not "our repo enforces this,"
and (c) **indistinguishable from junk by dominance** — `torch.cat` is 99%
concentrated on `torch` for the same reason `Str.snake` is on `Str`: it's a
namespaced API call, not a repo choice.

### v2 — contested band [80%, 97%], minority ≥ 3, drop self/`$this`/`<call>`/Capitalized-static, skip test paths

Rationale: a real convention is **contested** — dominant but with a real,
recurring minority (the would-be violation). A ~100% share means "no bypass
exists" → vacuous (`torch.cat`). Capitalized-lead receivers proxy for
class/module statics (library/stdlib); lowercase receivers proxy for repo-local
singletons (`db`, `log`, `app`, `c`). 11 corpora:

| corpus | fastapi | rich | scrapy | express | hono | outline | laravel | composer | guava | cobra | ripgrep |
|---|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|--:|
| candidates | 2 | 0 | 3 | 0 | 0 | 8 | 0 | 0 | 8 | 0 | 0 |

**7 of 11 corpora yield zero.** The ~21 survivors are still ~25–30% precision
and still mostly library idioms, not repo rules: outline `z.uuid`/`z.array`
(Zod), `document.createElement`, `process.exit`; scrapy `logging.getLogger`,
`warnings.warn`; guava `stream.readObject`/`writeObject` (Java serialization).
The tightening that removes junk also removes the real conventions (hono's
`c.req.param` dropped out; laravel went to zero).

## Verdict

**Receiver-funnel on AST dominance is not a viable candidate-rule signal** —
tighten the filters and recall goes to zero, loosen them and junk floods.
There is no threshold that yields acceptable precision *and* recall across
repos. This is the **same failure shape** as the foreign-structure AST signal
([`project_foreign_structure_ast_signal`] / `docs/research/` structural work):
a real signal that is fundamentally ungatable.

**Root cause.** A codifiable convention is a *sanctioned form vs. an avoided
substitute* ("use `session.execute`, not a raw cursor"). AST co-occurrence sees
"method M concentrates on receiver R" — which is true for **every** namespaced
API call, whether or not the repo faced a choice. Dominance cannot see that
`cat` has no substitute (not a rule) while a raw cursor is the avoided
substitute for `session.execute` (a rule). The missing ingredient — "these two
forms do the same job; the repo picks one" — is semantic and is in neither the
AST nor the fitted model's stored facts.

## Reshape (not a kill of the feature)

1. **`argot report` (the dashboard) is unaffected and ships** — it *shows*
   learned facts (imports, naming, syntax idioms, layering, calibration). No
   precision problem: it visualizes, it doesn't propose rules.
2. **Drop auto receiver-funnel as the miner's primary signal.** Demote it to at
   most a *ranking hint inside a template the human already chose* ("you asked
   to funnel a helper — here are the receivers that already concentrate calls"),
   never an autonomous proposer.
3. **Pivot the auto-suggest to the one place "the repo made a choice" is
   observable: history removals.** Mine forms the repo *deleted* across accepted
   history (bare SQL later routed through a builder; a client swapped out) via
   pickaxe / `ts_query_old`. The removal **is** the evidence of an enforced
   convention; it's back-testable and non-vacuous. Novel — no linter mines
   "what you migrated away from." This is the auto-suggest worth building.
4. **Make `argot-suggest-rules` evidence-assisted authoring, not a blind
   miner.** The skill offers convention *shapes*; the human supplies the
   "these are substitutes" semantics the miner can't; argot supplies the
   evidence (counts, canonical examples, the accepted-history back-test) and
   auto-scaffolds + gates. This is `argot-write-rule` upgraded with the report's
   evidence, which is what the maintainer actually asked for ("it's hard to know
   what the conventions are") — served by *surfacing* + *assisting*, not by an
   AST miner that can't reach precision.

---

# Part 2 — history-substitution mining (the salvage signal)

Raw dominance can't see "the repo made a choice." History can: a commit that
removes form A and adds form B **is** that evidence. Probe:
`benchmarks/conv_history_probe.py` (deleted after this writeup) — mines, over
each repo's full git history, (1) **import-swaps** (a file removes `import R`,
adds `import A` in one commit) and (2) **receiver-swaps** (a hunk removes
`R.m(`, adds `S.m(` — same method, receiver changed), ranked by distinct
commits, with a **liveness gate** (does the old form still appear at HEAD →
would it creep back → is the rule still enforceable?).

Full-history clones (blob:none partial clones silently truncate `git log -p` —
must clone with blobs): scrapy (9085 commits parsed), fastapi (7514), rich
(8909).

## Result: a real signal — but thin and noisy, not an autonomous miner.

**The signal is real** (unlike raw dominance). Best hits, cleaned
(deprecation-shims / oscillating pairs / relative-import & regex noise removed):

- scrapy **`settings.getbool → crawler.settings.getbool`** (7 commits, **LIVE**)
  and `settings.getint → crawler.settings.getint` (5c, LIVE) — a genuine,
  still-enforceable architectural convention (read settings via
  `crawler.settings`). A human keeps these instantly.
- scrapy `collections → collections.abc` (4c), `six.moves.urllib.parse →
  urllib.parse` (4c, correctly flagged **stale** — a completed py2→3 migration,
  the old form is gone, so it's *not* a live rule).
- rich `shutil.get_terminal_size → os.get_terminal_size` (real API sub, stale).

**But precision is still low and yield is thin:**

- **import-swaps** are dominated by *hub artifacts* — a popular new import
  (`scrapy.exceptions`, `fastapi.responses`) paired with whatever else the
  commit happened to remove — plus regex false matches (`typing → example`,
  `typing → content`: string/dict keys, not imports). fastapi's cleaned
  import list is ~all noise.
- **receiver-swaps** are cleaner but mixed with meaningless variable renames
  (`d.addErrback → d2.addErrback`, `c.print → console.print`).
- Genuinely valuable, LIVE conventions found across 3 large repos: **~2–3
  total.** The signal is real but sparse.

**The liveness gate is a keeper.** It correctly separated "stale completed
migration" (`six.moves→urllib`) from "live enforceable convention"
(`settings.getbool→crawler.settings.getbool`). This is the discriminator that
tells a *rule* from *history that already happened*.

## Consolidated verdict across all experiments

| signal | precision | yield | verdict |
|---|---|---|---|
| raw receiver-funnel (dominance) | ~5–12% | high (junk) | dead — vacuous, conflates convention with API usage |
| gated receiver-funnel (contested band) | ~25–30% | 7/11 corpora empty | dead — tighten→0 recall, loosen→flood |
| history import-swap | ~10–20% | moderate | weak — hub artifacts + regex noise |
| history receiver-swap + liveness | ~30–40% | ~2–3 live/repo | **the salvage** — real but thin & noisy |

**No autonomous convention-miner reaches clean precision.** But the
history-receiver-swap + liveness signal is a strong *evidence input for
human-assisted authoring*: it surfaces the handful of substitutions a repo
actually made and that are still live; a human curates them in one glance
(keep `settings→crawler.settings`, drop `d→d2`) — a judgment trivial for a
person and impossible for the miner. This is exactly the reshaped design: the
report visualizes, the `argot-suggest-rules` skill presents these ranked
candidates, the human disposes, argot scaffolds + back-tests + gates.

---

# Part 3 — AST-quality history mining, quantified across 9 repos

The regex probe (Part 2) had two weaknesses: string-literal false matches and
hub artifacts. A sharper Rust probe (`crates/argot-bench/examples/conv_hist.rs`,
deleted after this writeup) walks history via **git2**, extracts callees +
imports from the before/after blobs with the **real argot-lang tree-sitter
extraction** (no string/comment leakage), and adds two precision gates:

- **receiver-funnel + path-containment** — keep a receiver-swap `R.m → S.m` only
  when the new receiver reaches the old through a longer path (`settings` →
  `crawler.settings`), a real "access via a wrapper/parent" convention; drop
  everything else as a variable rename (`d → d2`, `c → console`).
- **import-swap tight-delta + anti-hub** — only files with a small import delta,
  dropping modules that pair with many partners.

Plus the liveness gate (old form still at HEAD = enforceable rule vs. completed
migration). Full-history blobful clones; django/laravel capped at 12k commits.

## Quantified result (9 repos)

**Receiver-funnels (the precision engine):**

| repo | funnels | renames dropped | what fired |
|---|--:|--:|---|
| scrapy | 6 | 28 | `settings.get* → spider/crawler.settings.get*` (14c!) — one real convention |
| flask | 4 | 14 | `session_interface.* → app.session_interface.*`, `url_prefix → state.url_prefix` |
| requests | 2 | 28 | `r.close → r.raw.close`, `scheme.lower → parsed.scheme.lower` |
| fastapi, rich, django, laravel, cobra, hono | 0 | — | — |

- **Fires on 3/9 repos. Precision ≈ 100% of what fires is a real, LIVE
  convention** — the path-containment gate auto-drops ~90 variable renames
  across the set. Yield 2–6 conventions/repo when it fires.
- **Recall is repo-dependent and often zero** (5/9 silent): a repo only yields
  funnels if its history contains "route access through a wrapper" refactors.

**Import-swaps (secondary, noisier, complementary):**

- Fires on 6/9. **Precision ≈ 40–50%** — real migrations mixed with
  deprecation/type-stub/hub noise (the anti-hub gate at 4 partners was too loose
  to fire; the `→warnings`/`→__future__`/`→_typeshed` noise survived).
- Genuine gems it alone catches (cross-library, which funnels can't):
  laravel `Exception → Throwable`, flask `jinja2 → markupsafe`, rich
  `commonmark → markdown_it`, scrapy `sha → hashlib` / `pkg_resources →
  importlib`, django `os → pathlib`.

## Final verdict — the auto-suggest IS worth building, framed honestly

The history signal is real and, with AST extraction + the containment/liveness
gates, produces **genuinely valuable, high-precision candidates** — but it is
**opportunistic, not universal**: it fires on ~1/3 of repos and needs a human
glance to curate (trivial for a person). Concretely:

1. **Receiver-funnel + path-containment + liveness** is the precision engine
   (~100% when it fires). Ship it as the primary history template.
2. **Import-swap** is a complementary, lower-precision signal that catches
   cross-library migrations; keep it, ranked below, clearly "needs review".
3. Neither is autonomous. Both feed `argot-suggest-rules` as ranked candidates;
   the human keeps the real ones, argot scaffolds + back-tests + gates.
4. **`argot report` remains the universal piece** — it de-black-boxes every
   repo, including the 5/9 where history mining is silent.

Net: the miner is a *sometimes-finds-gold, always-honest* suggester, not a
"every repo gets N rules" engine. That is the correct product framing.

---

# Part 4 — the RELIABLE signal: internal-API fan-in (LISTING, not rule-firing)

The maintainer's actual ask is a **reliable way to LIST** a repo's conventions —
one that fires on *every* repo, not the ~1/3 where history mining works. Reframe:
*listing* is more tractable than *generating a firing rule*. And the convention
that matters most and is present in **every** repo is its **own shared API** —
the helpers/objects everyone routes through (`db.session`, `log`, a query
builder, `context`). Listing those IS listing the conventions.

Probe (`crates/argot-bench/examples/conv_list.rs`, deleted after this writeup)
uses the adapter's precise repo-internal signals — no fit, no history:

- `internal_import_bindings` — names imported from **relative/internal** modules
  (`from .db import session` → `session`): the repo's deliberate shared API.
- `callable_definitions` — symbols the repo itself defines.
- `import_bindings` — third-party binding names, to *exclude* library receivers.

Two ranked views by cross-file fan-in: **internal shared helpers** (imported
repo symbols) and **internal receiver-funnels** (calls on a repo-local receiver).

## Result: reliable, and the top of each list IS the repo's convention

12 corpora, 6 languages. Substantive output on **9–10/12** (the empties —
express 7 files, cobra 37 files — are genuinely tiny). The top entries per repo
read as exactly the repo's conventions:

| repo | top of the listing |
|---|---|
| fastapi | funnel `app` (332 files) |
| hono | funnel `c` (79), `c.req` (41); helpers `Hono`, `Context`, `MiddlewareHandler` |
| dagster | funnel `context` (179), `instance` (318), `context.log` (115); helpers `gql`, `RepoAddress`, `useQuery` |
| laravel | funnel `Str` (181 files / 98 methods), `Arr` (185), `Carbon` |
| guava | funnel `ImmutableList`, `Preconditions`, `Ordering`, `ImmutableMap` |
| scrapy | funnel `logger` (48), `request.meta` |
| rich | helpers `Console`, `Text`, `Style`, `Segment`; funnel `console` (45) |
| redis | helpers `server`, `zmalloc`, `util`, `cluster` |
| ripgrep | helpers `Error`, `Searcher`, `Sink`; funnel `matcher`, `searcher`, `builder` |
| outline | helpers `User`, `Document`, `config`, `RootStore`; funnel `Logger` (145) |

**Why it's reliable where everything else wasn't:** it ranks by cross-file
fan-in over the repo's *own* API, grounded in hard adapter facts
(`callable_definitions` + `internal_import_bindings`), not dominance and not a
migration. Every non-trivial repo has a most-used internal API.

**Residual noise (honest):** some generic local-var receivers (`result`, `f`,
`s`, `m`, `err`) and JS globals (`JSON`, `Math`, `Array`, `z`) leak into the
funnel view via name-collision in the `defined` set. But the real conventions
concentrate at the **top of a short ranked list**, so a human glance (or the
report's presentation) filters them trivially. The **shared-helpers view**
(pure `internal_import_bindings`) is essentially noise-free; the **funnel view**
is noisier but catches the routed param-objects (`c`, `context`, `logger`) that
imports miss. The two are complementary.

## Part 4b — refined + quantified (tiered, 20 repos / 12 languages)

The v1 funnel view mixed real conventions with generic locals (`result`, `f`)
and JS globals (`JSON`, `z`) via `defined`-set name collision. v2 structures the
output into confidence tiers so precision is measurable:

- **TIER 1** — internal-import bindings by fan-in **+** funnel receivers that are
  a **Capitalized** defined/imported type (`Str`, `Console`, `Preconditions`,
  `ImmutableList`, `TOrmModel`). Nearly noise-free.
- **TIER 2** — lowercase/param internal receivers (`c`, `context`, `logger`,
  `db`), minus a language-builtins list and a small generic-throwaway list,
  short names gated by a fan-in floor. The judgment zone.

**Result across 20 repos, all 12 languages:**

- **TIER 1: ~90% precision, substantive output on 17–18/20 repos.** Almost every
  entry is a real convention/type — laravel `Str`/`Arr`/`Carbon`, guava
  `ImmutableList`/`Preconditions`, hono `Hono`/`Context`, dagster
  `gql`/`RepoAddress`/`useQuery`, curl `curl_setup`/`urldata`, mormot2
  `TSynLog.Add`/`TRestHttpServer`, jellyfin `RequestHelpers`/`LibraryManager`.
  Empty only on tiny (cobra 37 files) or Go (weak classification).
- **TIER 2: ~50–60% precision.** Catches the valuable param-funnels (`context`,
  `logger`, `c`, `request.meta`) but mixed with locals; needs human review.

The tiering **resolves the precision problem**: TIER 1 (Capitalized-type +
imported-vocab) is clean enough to present *as* the conventions; TIER 2 is the
"routing objects, confirm these" list.

**Adapter gaps this surfaced (relevant to the production feature):**

- `import_bindings` returns empty for **TS/JS/Go** (default trait impl) — so
  third-party binding detection is blind there and `z` (zod) etc. leak into
  TIER 2. Filling it per-language is the clean fix, not a denylist.
- Vendored subtrees (jemalloc in redis, OpenAL `al*` in castle-engine) classify
  as "internal" — argot's fit-time ignore/exclude system handles this in
  production; the probe doesn't.
- A per-language **builtin-globals** set (JS `JSON`/`Math`/…) belongs in each
  adapter (today only a small `identifier_noise` set exists).

## Where this leaves the design

The reliable **convention LIST** is a composite, all computable at fit with no
new heavy machinery:

1. **Naming + syntax idioms** — already in `ConventionModel` (`ident_shapes`,
   `node_kinds`); present in every repo, just needs surfacing.
2. **Familiar imports** — already in `import_modules`.
3. **Internal-API funnels + shared helpers** — this Part-4 signal; reliable,
   fires on ~every repo, top-of-list precision high.
4. **Historical live substitutions** — Parts 2–3; opportunistic bonus (~1/3 of
   repos), high precision when present.

(1)–(3) make the listing **reliable on every repo**; (4) is the opportunistic
extra. A candidate becomes a *rule* only after human curation + back-test + the
`argot rules test` gate. This directly answers the ask: a dependable convention
list first, rule-generation as the curated second step.

## Reproduction

- receiver-funnel dominance: `cargo run -p argot-bench --example convention_probe`
- regex history: `python3 benchmarks/conv_history_probe.py <full-clone>...`
- AST history: `cargo run -p argot-bench --example conv_hist -- <full-clone>...`
- internal-API listing: `cargo run -p argot-bench --example conv_list -- <repo>...`

All examples deleted after this writeup (restore from this commit's parent).
History probes need **blobful** clones — `blob:none` silently truncates
`git log -p` / tree diffs. `benchmarks/data/` corpora are bare checkouts (no
`.git`, fine for the non-history probes).
