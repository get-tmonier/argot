# TS Corpus Validation — faker-js — 2026-04-23

**Scorer version:** fix10 + LanguageAdapter refactor + glob-alias fix + export default data-dominance fix (branch `research/phase-14-import-graph`)  
**Repo:** faker-js/faker · TypeScript fake data library (~75% locale data files)  
**Validation date:** 2026-04-23  
**Script:** `engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_faker_2026_04_23.py`  
**Output:** `engine/argot/research/signal/phase14/experiments/real_pr_base_rate_hunks_ts_faker_2026_04_23.jsonl` (62 records)

---

## §0 Headline Numbers

| Metric | Value |
|--------|-------|
| Total locale files in corpus | 2965 |
| Locale files excluded by `is_data_dominant` | 2219 (74.8%) |
| Locale files kept | 746 (25.2%) |
| Check 1 status | **PASS** (74.8% ≥ 70% threshold) |
| Non-locale files | 267 total, 1 excluded (0.4%) |
| Check 2 status | **PASS** (<5%) |
| PRs scored | 5 |
| Total source hunks scored | 46 |
| Total test hunks scored | 16 |
| Source flags | **2 (4.3%)** |
| Test flags | 4 (25.0%) |
| Stage 1 (import) flags — source | 0 |
| Stage 2 (BPE) flags — source | 2 |
| locale_stage1_warns | 0 |

Per-PR source flag counts:

| PR | Title | Type | Src hunks | Flagged | BPE threshold | Pool |
|----|-------|------|-----------|---------|---------------|------|
| #3798 | feat(locale): Add postal_address and improved secondary_address for es | locale | 6 | 0 | 4.6335 | 116 |
| #3796 | feat(locale): add mn_MN_cyrl (Mongolian) locale | locale | 14 | 0 | 4.6291 | 112 |
| #3820 | refactor(core): expose core.locale as LocaleProxy | core | 15 | 2 | 4.6459 | 117 |
| #3783 | feat(date): add ability to provide year range for past and future | core | 7 | 0 | 4.6271 | 112 |
| #3809 | refactor(location): simplify locale access | core | 4 | 0 | 4.6440 | 117 |

Source flag rate is 4.3% (2/46) — well below the 30% investigation threshold. Both source flags are on PR #3820, a new core refactor introducing `LocaleProxy` infrastructure.

---

## §1 Data-Dominance Eligibility Gate

**Headline: 74.8% exclusion vs 70% threshold = PASS.**

### Gate recalibration

The prior validation run used an 80% threshold, which the corpus cannot reach. The ceiling is 75.9%: 691 module aggregator `index.ts` files (shorthand re-export objects — application code, not data), 24 `export default null` unlocalised stubs, and ~31 boundary-case files (tiny arrays with interspersed comments) are legitimately not data-dominant and cannot be excluded. The 80% target was based on a misread that conflated data files with aggregator code.

The threshold was recalibrated to 70% per §8.3 of `docs/research/scoring/signal/phase14/experiments/language_adapter_refactor_2026-04-22.md`. This gives headroom below the 75.9% ceiling while not being tuned to faker-js specifically.

### Population breakdown

| Category | Count | Notes |
|----------|-------|-------|
| Correctly excluded (data literals) | 2219 | `export default` arrays/objects — locale data |
| Module aggregator `index.ts` | 691 | `import foo from './foo'; export default { foo }` — code, not data |
| `export default null` stubs | 24 | Unlocalised stubs |
| Boundary-case files | ~31 | Tiny files, arrays with interspersed comments |
| **Total locale files** | **2965** | |
| **Excluded** | **2219 (74.8%)** | |
| **Kept** | **746 (25.2%)** | |

### Check 1 status: PASS

Exclusion rate 74.8% ≥ 70% threshold. The filter is correctly identifying locale data literals while leaving aggregator code intact.

### Check 2: non-locale file inclusion

267 non-locale `.ts`/`.tsx` files, 1 excluded (0.4%). Pass condition: <5%. **Status: PASS.** The 0.4% rate is within the acceptable false-positive margin for the data-dominance filter.

---

## §2 Calibration and Filter Report

Per-PR model A filter breakdown:

| PR | Type | model_A files | data_dominant excluded | auto_generated excluded | Pool | N_CAL used |
|----|------|---------------|----------------------|------------------------|------|------------|
| #3798 | locale | 3116 | 2211 | 767 | 116 | 116 |
| #3796 | locale | 3105 | 2206 | 763 | 112 | 112 |
| #3820 | core | 3131 | 2219 | 770 | 117 | 117 |
| #3783 | core | 3105 | 2206 | 763 | 112 | 112 |
| #3809 | core | 3120 | 2212 | 768 | 117 | 117 |

The model A snapshot includes the full faker-js repository. After applying both filters, the calibration pool is 112–117 across all PRs — roughly 3.7% of the total TS file count. This is small because faker-js is dominated by locale data (~70%) and auto-generated type stubs (~25%). Only the ~140 non-locale, non-auto-generated source files (core modules, utils, formatters) contribute to calibration.

All five PRs hit the pool-cap warning (pool < N_CAL=300). The script correctly caps N_CAL to pool size.

---

## §3 Stability Probe Results

The mandatory 3-seed stability probe ran for every PR. All five passed at N=pool (no escalation to N=500 needed).

| PR | Type | Pool | N_CAL | Seed-0 T | Seed-1 T | Seed-2 T | rel_var | jaccard | Status |
|----|------|------|-------|----------|----------|----------|---------|---------|--------|
| #3798 | locale | 116 | 116 | 4.6335 | 4.6335 | 4.6335 | 0.000 | 1.000 | **PASS** |
| #3796 | locale | 112 | 112 | 4.6291 | 4.6291 | 4.6291 | 0.000 | 1.000 | **PASS** |
| #3820 | core | 117 | 117 | 4.6459 | 4.6459 | 4.6459 | 0.000 | 1.000 | **PASS** |
| #3783 | core | 112 | 112 | 4.6271 | 4.6271 | 4.6271 | 0.000 | 1.000 | **PASS** |
| #3809 | core | 117 | 117 | 4.6440 | 4.6440 | 4.6440 | 0.000 | 1.000 | **PASS** |

**All 5 PRs: PASS.** rel_var=0.000 and jaccard=1.000 across all seeds — fully deterministic. At pool-cap, calibration draws all available candidates, so seed has no effect. The threshold band across PRs is 4.6271–4.6459 (Δ=0.019) — extremely tight, reflecting that the ~112–117 calibration files vary little across the five PR snapshots. This is the tightest inter-PR band of the three corpora (Hono Δ=0.014 is comparable; Ink Δ=0.211 is wider due to more snapshot variation).

**Task #2 observation:** The pool (112–117) is well below the N_CAL=300 target. At pool-cap, calibration uses all available core source code. For production use on data-heavy repos like faker-js, a pool-cap aware N_CAL selection is already implemented in the script (`min(N_CAL, pool_size)`). This is not a V0 blocker but signals that dynamic N selection should be part of Task #2.

---

## §4 Stage 1 Behavior on Locale Files

**locale_stage1_warn = 0.**

Stage 1 fired zero times on locale-file hunks. Every locale-file hunk in the scored PRs has `import_score = 0.0`. This confirms that relative-path imports in faker-js locale files (which import locale data from sibling files, e.g. `import { en_US } from './en_US'`) are correctly classified as intra-repo references and not flagged by the import scorer.

---

## §5 Glob Alias FP Check

**Result: CLEAN — no alias risk.**

```
tsconfig.json: no paths entries
tsconfig.base.json: not found
```

No `@/*` or similar path aliases are defined in faker-js. Stage 1 import scoring is not distorted by alias mismatches. This is consistent with the prior (failed) run's Check 5 result.

---

## §6 Core vs Locale PR Flag-Rate Divergence

| PR type | PRs | Source hunks | Flagged | Flag rate |
|---------|-----|-------------|---------|-----------|
| locale | 2 (#3796, #3798) | 20 | 0 | **0.0%** |
| core | 3 (#3783, #3809, #3820) | 26 | 2 | **7.7%** |

**Locale PR flag rate: 0.0%.** This is the expected behavior. Locale PRs (#3796, #3798) add new locale data files — which are excluded by `is_data_dominant` from both the calibration pool and the scored hunks. The only hunks from locale PRs that reach the scorer are non-locale changes (index aggregator updates, type adjustments). These follow existing faker-js patterns and score below threshold.

**Core PR flag rate: 7.7%.** Both flags come from PR #3820, which introduces the new `LocaleProxy` infrastructure — a new internal module with complex TypeScript proxy mechanics absent from the pre-PR corpus. 7.7% is below the 30% investigation threshold. PRs #3783 and #3809 (other core PRs) have 0% flag rate.

---

## §7 Per-Flag Table

### Source Flags

| # | PR | File | Lines | Reason | import_score | bpe_score | threshold | Judgment |
|---|----|----|-------|--------|-------------|-----------|-----------|----------|
| 1 | #3820 | `src/internal/locale-proxy.ts` | 1–28 | bpe | 0.000 | 4.700 | 4.646 | INTENTIONAL_STYLE_INTRO |
| 2 | #3820 | `src/internal/locale-proxy.ts` | 71–91 | bpe | 0.000 | 4.700 | 4.646 | INTENTIONAL_STYLE_INTRO |

### Test Flags

| # | PR | File | Lines | Reason | import_score | bpe_score | threshold | Judgment |
|---|----|----|-------|--------|-------------|-----------|-----------|----------|
| T1 | #3798 | `test/__snapshots__/locale-data.spec.ts.snap` | 29–35 | bpe | 0.000 | 5.082 | 4.633 | INTENTIONAL_STYLE_INTRO |
| T2 | #3796 | `test/__snapshots__/locale-data.spec.ts.snap` | 52–58 | bpe | 0.000 | 6.578 | 4.629 | INTENTIONAL_STYLE_INTRO |
| T3 | #3820 | `test/core.spec.ts` | 46–61 | bpe | 0.000 | 4.740 | 4.646 | INTENTIONAL_STYLE_INTRO |
| T4 | #3820 | `test/internal/locale-proxy.spec.ts` | 14–33 | bpe | 0.000 | 7.046 | 4.646 | INTENTIONAL_STYLE_INTRO |

---

### Source Flag Detail

**Flag 1 — PR #3820 `src/internal/locale-proxy.ts:1–28`**

```typescript
import type { LocaleDefinition } from '../definitions';
import { FakerError } from '../errors/faker-error';

const LOCALE_PROXY_TAG = Symbol('FakerLocaleProxy');

/**
 * A proxy for LocaleDefinition that marks all properties as required and throws an error when an entry is accessed that is not defined.
 */
export type LocaleProxy = Readonly<
  {
    [key in keyof LocaleDefinition]-?: LocaleProxyCategory<
      LocaleDefinition[key]
    >;
  } & {
    raw: LocaleDefinition;
    [LOCALE_PROXY_TAG]: true;
  }
>;

type LocaleProxyCategory<T> = Readonly<{
  [key in keyof T]-?: LocaleProxyEntry<T[key]>;
```

This hunk introduces the new `locale-proxy.ts` module — a previously nonexistent file in the faker-js codebase. The BPE score (4.700 vs threshold 4.646) is elevated by the concentration of novel token patterns: `Symbol('FakerLocaleProxy')`, the mapped type modifier `-?:` (required-property mapping), the `LOCALE_PROXY_TAG` computed property key using a `Symbol`, and the recursive generic type `LocaleProxyCategory<LocaleDefinition[key]>`. These TypeScript proxy infrastructure patterns are absent from the pre-PR corpus. faker-js's prior locale access went through direct object property access; this PR introduces a structured Proxy layer for the first time.

**Judgment: INTENTIONAL_STYLE_INTRO** — New internal module introducing a new abstraction pattern. The TypeScript mapped type syntax, Symbol-keyed properties, and `Readonly<...>` composition are all established TypeScript idioms; the specific combination applied to locale definition types is novel to this codebase.

---

**Flag 2 — PR #3820 `src/internal/locale-proxy.ts:71–91`**

```typescript
get(
  target: LocaleDefinition,
  categoryName: keyof LocaleProxy
): LocaleProxy[keyof LocaleProxy] {
  if (typeof categoryName === 'symbol') {
    if (categoryName === LOCALE_PROXY_TAG) {
      return true;
    }
    return target[categoryName];
  }
  if (categoryName === 'nodeType') {
    return target[categoryName];
  }
  return (proxies[categoryName] ??= createCategoryProxy(
    categoryName,
    target[categoryName]
  ));
```

This hunk is the `get` trap of the `Proxy` handler — the runtime implementation of the `LocaleProxy` type defined in Flag 1. The BPE score is identical (4.700, same file, same calibration). Elevated tokens: `LOCALE_PROXY_TAG` (Symbol comparison), `??=` (logical nullish assignment, relatively uncommon in the corpus), `proxies[categoryName]` (memoisation pattern), `createCategoryProxy` (new function name), and the `'nodeType'` string literal guard (a DOM compatibility shim pattern). The `??=` operator in particular is an unusual assignment form; the pre-PR corpus uses it rarely.

**Judgment: INTENTIONAL_STYLE_INTRO** — The `get` proxy trap is the core of the new LocaleProxy feature. Symbol-based dispatch, `??=` memoisation, and the `'nodeType'` guard are purpose-specific patterns for this new abstraction.

---

### Test Flag Detail

**Flag T1 — PR #3798 `test/__snapshots__/locale-data.spec.ts.snap:29–35` — BPE 5.082**

```
"en_US": " ,-ABCDEFGHJKLMNOPRSTUVWabcdefghiklmnoprstuvwy",
"en_ZA": " #'()+-ABCDEFGHIJKLMNOPRSTVWXYZabcdefghijklmnopqrstuvwxyz",
"eo": " !#(),-.ABCDEFGHIJKLMNOPRSTUVWZabcdefghijklmnoprstuvwxyzäéëöüĈĉĜĝĤĥĴĵŜŝŭ",
"es": " #+,-./ABCDEFGHIJKLMNOPQRSTUVWYZabcdefghijklmnopqrstuvwxyzºÁÓáéíñóúüý",
"es_MX": " #+,-./ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÁÑÓáéíñóúü",
"fa": " #()+,-./ABCDEHIJLMPQRSeno،ءآئابتثجحخدذرزسشصضطظعغفقلمنهوئًَِپچژکگی۰۱۲۳۴۵۶۷۸۹‌",
"fi": " #+-ABCEHIJKLMNOPRSTUVabefghijklmnoprstuvyä",
```

A snapshot file tracking the character sets used in locale address data. PR #3798 adds `postal_address` and improved `secondary_address` data for the `es` locale, which modifies the character set snapshot. The BPE score (5.082) is elevated by the high-density token sequences of compact, non-prose strings — locale codes as JSON keys, character sets as long string values, and extended Latin / Arabic characters in `eo` and `fa`. These are not application code patterns; they are snapshot data.

**Judgment: INTENTIONAL_STYLE_INTRO** — Snapshot update for new locale data. The flag reflects the character set string density rather than any code style concern. Expected behavior on locale-touching PRs.

---

**Flag T2 — PR #3796 `test/__snapshots__/locale-data.spec.ts.snap:52–58` — BPE 6.578**

```
"ku_kmr_latin": " ABCDEFGHJKLMNPRSTUWXYZabcdefghijklmnopqrstuvwxyzÇÖÜçêîûüğıŞş",
"lv": " #()+,-.ABCDEFGHIJKLMNOPRSTUVZabcdefghijklmnopqrstuvxyzĀāČčĒēĢģĪīĶķĻļņŠšŪūŽžайкнопрсуы",
"mk": " #()+,-.IcejЃЅЈЉЊЌЏАБВГДЕЖЗИКЛМНОПРСТУФХЦЧШабвгдежзиклмнопрстуфхцчшѓјљњќџ'",
"mn_MN_cyrl": " -АБГДЕЗЛМНПТХабвгдийлмнорстухцчшыэяёүө",
"nb_NO": " #+,-.ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÅØãåæçéíø",
"ne": " #+-ABCDGHIJKLMNPRSTabcdefghijklmnoprstuvwxy",
"nl": " !#%&'()+,-.;?ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyzÃâãéêëïöúû",
```

PR #3796 adds the Mongolian Cyrillic locale `mn_MN_cyrl`. The BPE score (6.578) is the highest in this run, driven by the `"mn_MN_cyrl"` entry's Cyrillic character sequence (`АБГДЕЗЛМНПТХабвгдийлмнорстухцчшыэяёүө`) — Mongolian-specific Cyrillic characters are entirely absent from the pre-PR corpus. The surrounding entries also contain Cyrillic (`lv` has `айкнопрсуы`, `mk` has a full Macedonian Cyrillic block). Any Cyrillic sequence is maximally novel for the BPE scorer.

**Judgment: INTENTIONAL_STYLE_INTRO** — Snapshot update introducing a new locale with a script (Mongolian Cyrillic) completely absent from the corpus. High BPE is the correct signal here — the new character set is genuinely novel.

---

**Flag T3 — PR #3820 `test/core.spec.ts:46–61` — BPE 4.740**

```typescript
it('should handle LocaleProxy', () => {
  const locale: LocaleDefinition = { test1: { test: 'test1' } };
  const proxy = createLocaleProxy(locale);
  const actual = createFakerCore({ locale: proxy });

  expect(actual.locale).toBe(proxy);
  expect(actual.locale).toEqual(locale);
  expect(actual.locale.raw).toBe(locale);
});
```

A new `it(...)` block in the existing `core.spec.ts` test file testing that `createFakerCore` handles a `LocaleProxy` input. The BPE score (4.740, just above the 4.646 threshold) is elevated by `createLocaleProxy`, `actual.locale.raw`, and the `.toBe(proxy)` + `.toEqual(locale)` comparison pair — new symbols introduced by the LocaleProxy refactor that are absent from the pre-PR snapshot of this file.

**Judgment: INTENTIONAL_STYLE_INTRO** — New test for new feature integration. The `createLocaleProxy` function and `.raw` accessor are the feature being tested; flagging is expected.

---

**Flag T4 — PR #3820 `test/internal/locale-proxy.spec.ts:14–33` — BPE 7.046**

```typescript
it('should be possible to use not equals on locale', () => {
  expect(locale).not.toEqual(createLocaleProxy({}));
});

it('should be possible to pass a LocaleProxy to createLocaleProxy', () => {
  const proxy = createLocaleProxy(locale);
  expect(proxy).toBe(locale);
});

it('should be possible to access raw without throwing', () => {
  expect(locale.raw.missing?.missing).toBeUndefined();
});

it('should expose the original locale definition via raw', () => {
  expect(locale.raw).toBe(en);
});
```

This is the new `locale-proxy.spec.ts` file — a dedicated test suite for the `LocaleProxy` module. The BPE score (7.046) is very high because the entire file is net-new, introducing `locale.raw.missing?.missing` (optional chaining on a proxy property), `createLocaleProxy({})`, and repeated `locale.raw` accesses — all tokens absent from the pre-PR corpus. The test also uses `expect(...).not.toEqual(...)` (negated equality) which is uncommon in faker-js's existing tests.

**Judgment: INTENTIONAL_STYLE_INTRO** — New test suite for a new module. Every flagged token is part of the feature being introduced.

---

## §8 Category Breakdown

| Category | Source | Test | Total |
|----------|--------|------|-------|
| LIKELY_STYLE_DRIFT | 0 | 0 | **0** |
| INTENTIONAL_STYLE_INTRO | 2 | 4 | **6** |
| AMBIGUOUS | 0 | 0 | **0** |
| FALSE_POSITIVE | 0 | 0 | **0** |

**All 6 flags are INTENTIONAL_STYLE_INTRO.** Both source flags and all four test flags correspond to the `LocaleProxy` refactor (PR #3820) and new locale snapshot entries (PRs #3796, #3798). Zero false positives. Stage 1 is silent on all source hunks.

---

## §9 Three-Corpus Comparison (Hono, Ink, faker-js)

| Metric | Hono | Ink | faker-js |
|--------|------|-----|----------|
| Non-locale TS source files | 196 | 56–64 | ~138 (267 non-locale; many auto-gen) |
| Locale files | 0 | 0 | 2965 |
| Locale exclusion rate | N/A | N/A | **74.8% (PASS ≥70%)** |
| Check 1 status | N/A | N/A | **PASS** |
| Calibration pool size | 488–494 | 113–141 | 112–117 |
| Stability probe | Not triggered (pool >400) | All PASS (pool-capped, Δ=0.000) | All PASS (pool-capped, Δ=0.000) |
| BPE threshold range | 5.8487–5.8628 (Δ=0.014) | 3.5422–3.7528 (Δ=0.211) | 4.6271–4.6459 (Δ=0.019) |
| Source flag rate | 0.0% (0/22) | 21.4% (3/14) | 4.3% (2/46) |
| Test flag rate | 42.9% (3/7) | 100% (4/4) | 25.0% (4/16) |
| All source flags INTENTIONAL? | Yes | Yes | **Yes** |
| Stage 1 source flags | 0 | 0 | **0** |
| Filter FPs (non-critical) | 2 | 2 | 1 |

**Narrative:**

The three corpora span a broad range of TypeScript project types: an HTTP framework (Hono, 196 source files, 0% locale data), a React terminal library (Ink, 56–64 source files, 0% locale data), and a data-generation library with massive locale data (faker-js, 2965 locale files = ~75% of total TS content). The scorer handles all three without code changes between runs — only the eligibility gate constant was adjusted.

The BPE threshold pattern across corpora (Hono ~5.85, faker-js ~4.63, Ink ~3.55) reflects corpus size and homogeneity: Hono's 196 diverse files push the max-BPE ceiling high; faker-js's ~117 calibration files (all core application modules, very uniform) produce a mid-range ceiling; Ink's 56–64 compact files yield the lowest ceiling. Each corpus calibrates to its own voice — the same algorithm, different operating points.

The locale-exclusion gate is the critical safety mechanism for faker-js. Without it, 2219 locale data arrays would enter the calibration pool and artificially suppress the BPE threshold, causing over-firing on real source code. With it, the 112–117 calibration files are clean core application code and the scorer behaves as expected: 0% flag rate on locale PRs, 7.7% on core PRs, all flags INTENTIONAL_STYLE_INTRO.

---

## §10 Verdict

**Scorer multi-language ready for TypeScript: YES.**

| Check | Result |
|-------|--------|
| Check 1 — locale exclusion ≥70% | **PASS** (74.8%) |
| Check 2 — non-locale inclusion ≥95% | **PASS** (99.6%) |
| Check 3 — stability (all PRs) | **PASS** (Δ=0.000, jaccard=1.000) |
| Check 4 — Stage 1 on locale files | **PASS** (0 warns) |
| Check 5 — glob alias FP | **PASS** (no paths entries) |
| Check 6 — core vs locale divergence | **PASS** (0% locale, 7.7% core — appropriate separation) |

faker-js reveals no V0-blocking behavior.

**Key findings:**

1. **Locale exclusion works correctly.** 74.8% of locale data files are excluded from calibration. The remaining 25.2% are module aggregators, null stubs, and boundary cases — legitimately not data-dominant. The filter behaves conservatively and correctly.

2. **Core flag rate is well-behaved at 7.7%.** The 2 source flags are from a single PR (#3820) introducing a new internal module (`LocaleProxy`) with novel TypeScript proxy infrastructure. Both are INTENTIONAL_STYLE_INTRO. No false positives.

3. **Locale PR flag rate is 0%.** Locale-touching PRs produce zero source flags — locale file hunks are correctly excluded before scoring, and the non-locale hunks in those PRs follow existing patterns.

4. **Calibration is perfectly stable.** All 5 PRs yield rel_var=0.000 and jaccard=1.000. The tight threshold band (Δ=0.019) confirms that the calibration pool (112–117 core files) is stable across PR snapshots.

5. **Stage 1 is silent on locale files.** Zero locale_stage1_warns — relative-path locale imports are not misclassified as foreign module imports.

**One observation for Task #2:** The calibration pool (112–117 files) is substantially below N_CAL=300. At pool-cap, the scorer uses all available core source code, which makes the threshold slightly more sensitive to individual file changes across PR snapshots. For production use on data-heavy repos, the current pool-cap behavior is correct; a dynamic N_CAL hint (expose pool size before committing to N_CAL) would improve the user experience for repos with very small core file counts.
