# Supersession mining probe — replacement pairs from accepted history

**Date:** 2026-07-21 · **Status:** positive — signal is real and filterable;
production miner validated (see final section)
**Question:** can we mine "this repo systematically removes X and adds Y"
(replacement pairs) from git history with enough precision to power a
`superseded` rule (flag new code using the legacy side of an in-progress
migration) and to stop flagging the rising replacement as foreign?

## Method (dirty probe, ~100 lines of Python)

For the last 4 000 non-merge commits of each repo, parse `git log -p`
import-line changes per file. Per (commit, file): truly-removed imports
(removed and not re-added in the same file) × truly-added imports → candidate
pairs. Filters: skip churn-heavy files (>6 removed or >6 added imports),
keep pairs seen in **≥3 distinct commits** and **≥3 distinct files**, with
**asymmetric direction** (forward ≥ 2× reverse). For survivors, count HEAD
files still containing X (`git grep`) — the "migration leftover" measure.

Corpora: fresh shallow clones (depth 4 000) — the `benchmarks/data/`
snapshots have **no `.git`** and cannot be used for history mining.

## Results

| corpus | raw pairs | candidates after filters | verdict per candidate |
|---|---|---|---|
| fastapi | 486 | 4 | **`typing_extensions → typing`** (3c/19f, 10 leftover files): textbook real migration with leftovers. 3× `X → importlib` (pytest, fastapi.testclient, dirty_equals): real systematic test refactor but mislabeled as supersession — *role mismatch* noise class. |
| excalidraw | 2 654 | 5 | **`react-dom → react-dom/client`** (React 18 migration, 24 leftovers): real. **`./bounds → @excalidraw/common`** (monorepo package extraction): real path migration. 3 relative-path pairs (`../components/App → ./App`, `../i18n → ../../i18n`, …): *unresolved-relative-import* noise class — same string means different targets at different depths. |
| composer | 149 | 1 | **`Symfony\…\InputOption → Composer\Console\Input\InputOption`** (4c/24f, 9 leftovers): textbook — repo introduced its own subclass, migrated commands, forgot 9 files. |
| wagtail | 336 | 1 | **`django.template.response → django.views.generic`** (4c/4f, 18 leftovers): plausible view-refactor systematic edit; needs eyeballing. |

Precision after three trivial filters: 6–7 of 11 candidates are genuinely
useful migrations (several with actionable leftover lists); the rest fall
into exactly **two noise classes**, both mechanically addressable:

1. **Role mismatch** (`pytest → importlib`): X and Y co-occur in a systematic
   edit but Y does not play X's role. Guard: require specifier-class match
   (external↔external at same ecosystem depth, internal↔internal), and/or
   require Y's repo-wide usage to be *rising* where X's is falling.
2. **Unresolved relative imports** (TS/JS): must canonicalize internal
   specifiers to repo-relative module identity before pairing
   (`argot-lang` `internal_import_bindings` / resolver machinery already
   does this for the arch layer).

Volume behaviour is healthy: 149–2 654 raw pairs collapse to 1–5 candidates
per repo, i.e. the filters do the work and the survivors are few enough for
per-candidate evidence (commit shas, dates, counts) to be rendered in full.

## Cost

Whole probe: seconds per repo (regex over `git log -p`, 4 000 commits).
A production miner riding the integrity-style two-sided replay
(`two_sided::collect_two_sided_per_commit`, bounded window, parallel,
`argot-lang` extractors on both blob sides) is the same order of work as the
integrity mini-replay (~34 s single-threaded / 1.4 k-file corpus at
150 commits) — but migrations span months, so the miner likely wants a
larger window (500–1 000 first-parent commits) with hunk-level import diffs
rather than full blob re-extraction where possible.

## Probe v2 — production guards, 16 corpora

Second pass adding the guards the v1 noise classes called for:
canonicalized relative imports (`int:`/`ext:` classes, pairs must match
class), trend measured **since the pair's first commit** (X net-declining,
Y net-rising — kills deliberately dual-era patterns like fastapi's
`typing_extensions`, the conservative call; `[[migration]]` covers the
partial-migration case by declaration), and a **refactor-sink guard**
(a Y absorbing ≥ 3 distinct X across the raw pairs — fastapi's `importlib`
test refactor — is a systematic edit, not a replacement; likewise a ≥ 2×
dominant-Y requirement when X pairs with several Y).

| corpus | candidates | detail |
|---|---|---|
| composer | 1 | Symfony InputOption → own subclass (9 leftovers) ✓ |
| excalidraw | 1 | react-dom → react-dom/client (React 18) ✓ |
| wagtail | 1 | django.template.response → django.views.generic ✓ |
| scrapy | 2 | six.moves.urllib.parse → urllib.parse (0 leftovers, completed); pkg_resources → packaging.version ✓✓ |
| ripgrep | 1 | std::os::unix::ffi::OsStrExt → bstr (6 leftovers) ✓ |
| fastapi, rich, eslint, hono, cobra, hugo, laravel/framework, rubocop, junit5, outline, curl | **0 each** | no over-fire on 11 no-migration corpora |

Every candidate that survives is a real systematic migration; every corpus
without one is silent. Completed migrations (X gone from HEAD) carry no
enforcement value → production drops pairs with zero leftovers.

**Callee-level probe** (crude regex callees, 5 corpora): the signal is real —
scrapy `iteritems → items`, `callLater → call_later`, `LogCapture →
at_level`, fastapi `insert_assert → snapshot`, hugo `LastChange → Lastmod` —
but generic names are an over-fire hazard (`File` → PathInfo with `File` in
425 HEAD files; `stop → stop_async`, 65). Production takes callee pairs with
distinctiveness guards (name length, leftover cap, non-ubiquity), to be
confirmed or restricted to import-only by the release bench.

## Production validation (release binary, real fits)

The shipped miner (`argot-rules-voice/src/scoring/supersede.rs`, window
1 000 first-parent commits, adapters + tree-sitter on both blob sides,
unique-blob dedup, parallel) was run via `argot fit` on 10 fresh clones:

- **Over-fire: 0.** fastapi, hono, cobra, hugo, rich (and the languages of
  every other corpus without a live migration) mined nothing.
- **Catches:** ripgrep `regex → regex_automata` (3 commits / 8 files,
  2023-06-15..2023-09-28) — the evidence sha resolves to the commit literally
  titled "regex: migrate grep-regex to regex-automata"; scrapy internal
  receiver refactor `spider.crawler.stats.inc_value →
  self.crawler.stats.inc_value` (callee kind, 1 leftover).
- **Fire test:** on the fitted ripgrep, a new untracked file with
  `use regex::Regex;` raises exactly one `superseded` finding (warn, exit 0)
  with the mined evidence line; nothing else fires.
- **Cost:** mining 0.9–4 s typical, worst 17.6 s (wagtail, JS+TS+Python) —
  a bounded, fit-time-only cost, degraded to zero on repos without history.
- **Scope notes vs the probe:** import supersessions are module-level (the
  adapters' surface), so sub-path moves (`react-dom → react-dom/client`,
  intra-vendor class paths) are out of scope by construction; migrations
  whose last activity predates the 1 000-commit window age out (scrapy's
  2022 `pkg_resources` cleanup). Both are accepted restrictions: the marquee
  case is the dependency/API swap in the repo's living history, and the
  probe's noise-free precision carries over intact.

## Design consequences recorded

- The pair signal (co-removal/co-addition in the same file+commit, k-commit
  support, file diversity, direction asymmetry, X-still-in-HEAD) is the right
  primitive — *not* age or raw dominance (see
  `convention-miner-receiver-funnel-probe.md` for why dominance alone was
  abandoned; removal evidence was the retained pivot).
- Evidence is intrinsically transparent: every surviving pair carries real
  commits/dates/counts, renderable via `RenderEvidence` as-is.
- Probe script was throwaway (scratchpad), per research workflow.
