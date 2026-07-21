# Team-convention mining — feature × location lift (generalizes across repos)

**Date:** 2026-07-21 · **Branch:** feat/rules-creation ·
**Feeds:** [`convention-mining-and-report.md`](../decisions/convention-mining-and-report.md)

## Question

Beyond a repo's *vocabulary* (which helpers it uses), teams enforce **placement
/ structural conventions**: *DB access only in the migration layer*, *validation
only at the API boundary*, *React hooks only in view components*, *business logic
never in components*. Can argot detect these — and with a **corpus-agnostic**
signal, not framework-specific rules?

## Signal: a feature that concentrates in one location, absent from others

A team convention is an **association between a code-feature and a code-location**:

- **feature** — a call (`queryInterface.addColumn`), a bare callee (`useState`),
  or an import. Extracted via the language adapters (`non_none_callees`) — no
  framework knowledge.
- **location** — a directory segment (`migrations/`), a filename role
  (`x.service.ts` → `service`), or an extension (`.tsx`). All derived from the
  path — corpus-agnostic.

Measure **lift** = `P(feature | location) / P(feature)` with support gates. A
convention is a `(feature, location)` pair with high lift, enough support, and
high **concentration** (the feature appears *almost only* in that location).
Probe: `crates/argot-bench/examples/team_conv.rs` (deleted after this writeup).

## Result: strong, and repo-independent (4 corpora, 3 languages)

Every corpus surfaced its real team conventions as high-lift location
signatures. Representative (open-source corpora):

| corpus | location | signature features (lift, % of feature's files here) |
|---|---|---|
| outline | `dir:migrations` / `.js` | `queryInterface.addColumn` (×7, 100%), `.removeColumn`, `.addIndex` — **DB schema only in migrations** |
| outline | `dir:api` + `dir:routes` | `z.object` (×13), `router.post`, `validate`, `BaseSchema.extend` — **validation at the API boundary** |
| outline | `dir:editor` | `state`, `tr`, `dispatch`, `Plugin`, `TextSelection` (×8) — **editor logic isolated** |
| outline | `.tsx` / `dir:components` | `styled`, `useTranslation`, `useStores`, `observer` — **UI concerns in components** |
| laravel | `dir:Console` | `$this.option/info/error/newLine` (×8, 100%) — **command API only in Console** |
| laravel | `dir:Eloquent` | `$model`, `getQuery`, `setRelation`, `newCollection` (×10–14) — **ORM layer** |
| laravel | `dir:Http` | `$next`, `$request`, `$response` — **middleware layer** |
| dagster | `dir:_core` (py) | `check.inst_param`, `check.failed`, `isinstance` — **param-checking in the core** |
| dagster | `.tsx` | `useMemo`, `useState`, `gql`, `useQuery` — **React/GraphQL in the JS UI** |

The lift + concentration filters are what make it precise: a feature at ×7 lift
and 100%-concentration in `migrations/` is a genuine placement rule; a
ubiquitous helper (lift ≈ 1) is not.

## Categories of team convention this finds

1. **Layer placement** — a call-family concentrates in a directory (DB in
   `migrations/`, HTTP in `Http/`, validation in `api/`).
2. **File-role placement** — a call-family concentrates in a file-kind (React
   hooks in `.tsx`, param-checks in the `.py` core, console API in command files).
3. **The avoidance dual** (the enforceable rule) — the same signal flipped:
   *feature F is near-absent from location W while common elsewhere* →
   "F doesn't belong in W" (business logic not in components; `queryInterface`
   not outside migrations).

## Detection algorithm (corpus-agnostic, validated)

1. Partition each file by location labels: directory segments + filename role +
   extension. (Drop over-generic segments: `src`, `lib`, `app`, …)
2. Extract features per file: bare callees + qualified-callee receiver leads +
   `receiver.method` + imports.
3. For every `(feature, location)` with support ≥ K and local count ≥ M:
   `lift = (local/loc_files) / (global/total)`; `concentration = local/global`.
4. A **convention candidate** = high lift (≥ 2) × high concentration (feature
   lives ≥ ~80% in one location) × real support. Rank by `lift × local`.
5. The **rule** is the contrapositive: scope a scripted rule to files *outside*
   the home location (`exclude`/`include` path globs) and `ts_query` for the
   feature → "this call belongs in `<home>`, not here." Back-test against
   accepted history; the in-home occurrences are the corpus, the out-of-home
   ones are the (rare) violations or exceptions to mute.

No framework literals anywhere — the signal is pure feature×location
association, so it works the same on a Laravel PHP monorepo, a Dagster
Python/TS monorepo, or a Rust workspace.

## Two corrections the measured run forced (not hand-tuning)

Running the production miner (`argot_rules_voice::placement`) across 12 corpora
exposed two things eyeballing one repo would not:

1. **Raw `(feature, location)` pairs explode** — 2515 on Dagster, 819 on Guava
   — because every method of a confined receiver is its own pair. Fix:
   **aggregate by location**, receiver-dedupe (`queryInterface` present ⇒ drop
   `queryInterface.addColumn`), cap to the strongest ~24 places × ~6 signature
   features. Result: a usable **6–24 places/repo**, each a readable convention.
2. **No hardcoded lists** — the miner must stay corpus/framework-agnostic, so it
   carries **none**: (a) file **roles** are the last stem segment, and only
   become a place when the *name recurs* across the repo (a repo with 20
   `capsule.ts` files gets a `role:capsule` convention, learned, not declared);
   (b) universal directories (`src/`, `app/`) **self-filter** — their base rate
   ≈ 1 makes every lift there ≈ 1 (< threshold), so no "generic directory" list
   is needed; (c) language noise (`self`/`this` plus a language's builtins)
   comes from the adapter's `identifier_noise()`, not a list in the miner;
   (d) test/tooling exclusion is the repo's `argot.toml [exclude]` job — the
   same mechanism as the voice fit — not a hardcoded `TEST_MARKERS` list.

## Noise / limits (honest)

- Generic receiver leads (`Object`, `$`, `f`, single letters) and framework
  globals still surface; the same GENERIC/builtin filters the vocabulary
  listing uses apply here.
- Very flat repos (no directory structure, uniform file kinds) yield weak
  location contrast — the signal is proportional to how much structure the team
  actually imposes (which is the right behavior).
- Filename-role extraction is heuristic (last dot-segment / known suffixes);
  directory-segment partitioning is the most robust axis.

## Where this goes

This is the **placement-mining template** for the convention miner — the piece
that turns the reliable *vocabulary* listing into reliable *structural rules*.
Flow: mine `(feature, location)` conventions → present the ranked candidates
with their lift/concentration evidence → the human confirms the real ones →
argot scaffolds the scoped scripted rule + back-tests it. It directly answers
"can argot detect team conventions like *business logic goes in the service
layer, not the view*" — yes, and the signal is real and repo-independent.
