# Phase 14 Prompt S — Faker Cross-Corpus Validation (fix7)

**Date:** 2026-04-22  
**Branch:** research/phase-14-import-graph  
**Why:** Two-corpus validation (FastAPI + rich) is narrow. Faker is the third validated
corpus (click excluded as too small). Confirm fix7 generalises to a third Python library
with a different vocabulary: data generation, locale tables, provider-class inheritance.

**Config:** Threshold = `max(cal_scores)` (Prompt Q verdict). N_CAL = 500 (Prompt R
verdict — N=100 was unstable on FastAPI).

---

## §0 Three-Corpus Side-by-Side

| Metric | FastAPI | Rich | Faker |
|---|---|---|---|
| PRs scored | 35 | 37 | 50 |
| Source hunks scored | 1,316 | 194 | 130 |
| PR flag rate | 31.4% (11/35) | 13.5% (5/37) | **2.0% (1/50)** |
| Hunk flag rate | 1.5% (20/1316) | 9.8% (19/194) | **0.8% (1/130)** |
| Stage 1 (import) | 0 | 4 | 1 |
| Stage 2 (bpe) | 20 | 15 | 0 |
| Auto-gen suppressed | 0 | 21 | 1 |
| Cal threshold min | 4.1718 | 4.1055 | **8.1837** |
| Cal threshold median | 4.1810 | 4.4168 | **8.2004** |
| Cal threshold max | 4.3508 | 4.4213 | **8.2050** |
| N_CAL used | 100 | 100 | 500 |

> **Critical observation:** Faker's calibration threshold (8.18–8.21) is roughly 2× that
> of FastAPI and Rich (4.11–4.42). FastAPI and Rich used N_CAL=100; Faker used N_CAL=500
> per Prompt R. This difference is not solely a corpus property — see §4 for analysis.

---

## §1 Faker Aggregates

- **Corpus:** joke2k/faker, 50 merged PRs, 2025-07-15 → 2026-04-17
- **Source hunks scored:** 130 (across 50 PRs)
- **PR flag rate:** 2.0% — 1 PR flagged out of 50
- **Hunk flag rate:** 0.8% — 1 hunk flagged out of 130
- **Stage 1 (import):** 1 flag (false positive — see §3)
- **Stage 2 (bpe):** 0 flags
- **Auto-gen suppressed:** 1 (false positive — see §3)
- **Diffs failed:** 0
- **Files missing at merge commits:** 0

**BPE score distribution (source hunks, n=130):**

| Percentile | BPE score |
|---|---|
| max | 8.0152 |
| p99 | 6.4702 |
| p95 | 5.4227 |
| median | 2.5638 |

**Per-PR calibration threshold distribution (n=50):**

| Statistic | Value |
|---|---|
| min | 8.1837 |
| median | 8.2004 |
| p90 | 8.2039 |
| max | 8.2050 |

> The max observable BPE score in the test set (8.0152) is **below the minimum
> calibration threshold (8.1837)**. Stage 2 is structurally blind on faker at this
> configuration. No BPE score in any of the 130 source hunks can reach the per-PR
> threshold.

---

## §2 Per-PR Table

| PR# | Date | Title (truncated) | Hunks | Flagged | St1 | St2 | AutoGen | Threshold |
|---|---|---|---|---|---|---|---|---|
| #2352 | 2026-04-17 | Add job providers for ar\_DZ and fr\_DZ | 2 | 0 | 0 | 0 | 0 | 8.2050 |
| #2351 | 2026-04-17 | Add company providers for ar\_DZ and fr\_DZ | 2 | 0 | 0 | 0 | 0 | 8.2046 |
| #2350 | 2026-04-17 | Add geo providers for ar\_DZ and fr\_DZ | 2 | 0 | 0 | 0 | 0 | 8.2040 |
| #2349 | 2026-04-17 | Add currency providers for ar\_DZ and fr\_DZ | 2 | 0 | 0 | 0 | 0 | 8.2039 |
| #2348 | 2026-04-17 | add date\_time provider for ar\_DZ | 1 | 0 | 0 | 0 | 0 | 8.2039 |
| #2347 | 2026-04-17 | Add ssn providers for ar\_DZ and fr\_DZ | 2 | 0 | 0 | 0 | 0 | 8.2037 |
| #2362 | 2026-04-17 | Fix UnicodeEncodeError in CLI docs | 3 | 0 | 0 | 0 | 0 | 8.2037 |
| #2364 | 2026-04-17 | fix: update image placeholder service | 1 | 0 | 0 | 0 | 0 | 8.2037 |
| #2358 | 2026-04-06 | Follow-up to #2287: Add requested tests | 2 | 0 | 0 | 0 | 0 | 8.2037 |
| #2287 | 2026-04-06 | Fix: Ensure deterministic locale selection | 1 | 0 | 0 | 0 | 0 | 8.2037 |
| #2341 | 2026-03-30 | Add address providers for ar\_DZ and fr\_DZ | 2 | 0 | 0 | 0 | 0 | 8.2019 |
| #2340 | 2026-03-23 | Fix deepcopy proxy bindings | 1 | 0 | 0 | 0 | 0 | 8.2019 |
| #2337 | 2026-03-13 | Add major Swiss banks to de\_CH | 1 | 0 | 0 | 0 | 0 | 8.2018 |
| #2324 | 2026-03-13 | Fix: mixed-gender names in es\_MX | 4 | 0 | 0 | 0 | 0 | 8.2018 |
| #2326 | 2026-03-13 | Fix pt\_PT postalcode format | 1 | 0 | 0 | 0 | 0 | 8.2018 |
| #2330 | 2026-03-04 | Fix: Add free email domains to hu\_HU | 2 | 0 | 0 | 0 | 0 | 8.2017 |
| #2316 | 2026-03-04 | Improve Polish address grammar | 15 | 0 | 0 | 0 | 0 | 8.2016 |
| #2314 | 2026-03-04 | Add country names to he\_IL | 1 | 0 | 0 | 0 | 0 | 8.2007 |
| #2327 | 2026-02-23 | Fix \_get\_local\_timezone() missing return | 1 | 0 | 0 | 0 | 0 | 8.2007 |
| #2318 | 2026-02-23 | fix: add missing formats in user\_name | 7 | 0 | 0 | 0 | 0 | 8.2007 |
| #2310 | 2026-02-06 | Fix/italian identity documents | 3 | 0 | 0 | 0 | 0 | 8.2006 |
| #2306 | 2026-02-06 | Add major GR banks to el\_GR | 1 | 0 | 0 | 0 | 0 | 8.2005 |
| #2304 | 2026-02-06 | feat(pt\_BR): Improve internet provider | 2 | 0 | 0 | 0 | 0 | 8.2005 |
| #2302 | 2026-02-06 | fix: pyfloat TypeError with positive=True | 1 | 0 | 0 | 0 | 0 | 8.2005 |
| #2294 | 2026-01-13 | Fix: Make tzdata conditionally required | 1 | 0 | 0 | 0 | 0 | 8.2005 |
| #2309 | 2026-01-13 | Fix broken parameter grouping for -i | 2 | 0 | 0 | 0 | 0 | 8.2005 |
| #2299 | 2025-12-29 | feat: Add selective uniqueness with exclude\_types | 2 | 0 | 0 | 0 | 0 | 8.2004 |
| #2291 | 2025-12-29 | Fix capitalise en\_GB address street suffixes | 1 | 0 | 0 | 0 | 0 | 8.2003 |
| #2276 | 2025-12-17 | translate: adding french female variantes for jobs | 1 | 0 | 0 | 0 | 0 | 8.1978 |
| #2275 | 2025-12-17 | Update \_\_init\_\_.py | 18 | 0 | 0 | 0 | 0 | 8.1979 |
| #2264 | 2025-12-17 | feat: add french company RCS number | 2 | 0 | 0 | 0 | 0 | 8.1978 |
| #2279 | 2025-11-19 | Implement localized UniqueProxy | 1 | 0 | 0 | 0 | 0 | 8.1978 |
| #2271 | 2025-11-19 | Add ar\_DZ | 1 | 0 | 0 | 0 | 0 | 8.1970 |
| #2270 | 2025-11-19 | Add fr\_DZ | 3 | 0 | 0 | 0 | 0 | 8.1960 |
| #2272 | 2025-11-05 | Add support for Python 3.14 | 3 | 0 | 0 | 0 | 0 | 8.1960 |
| #2265 | 2025-10-24 | feat: add french company VAT number | 1 | 0 | 0 | 0 | 0 | 8.1959 |
| #2267 | 2025-10-07 | Fix bank dry violation | 9 | 0 | 0 | 0 | **1** | 8.1960 |
| #2263 | 2025-10-07 | feat: add french company APE code | 2 | 0 | 0 | 0 | 0 | 8.1935 |
| #2255 | 2025-10-07 | Add names generation to Kenya locale | 2 | 0 | 0 | 0 | 0 | 8.1869 |
| #2251 | 2025-09-15 | Add Automotive providers for ja\_JP | 1 | 0 | 0 | 0 | 0 | 8.1866 |
| #2259 | 2025-09-15 | fix: fix minor grammar typo | 1 | **1** | **1** | 0 | 0 | 8.1865 |
| #2256 | 2025-09-15 | Add Nigerian locales (yo\_NG, ha\_NG, ig\_NG, en\_NG) | 3 | 0 | 0 | 0 | 0 | 8.1860 |
| #2246 | 2025-08-26 | Add Automotive providers for ko\_KR | 1 | 0 | 0 | 0 | 0 | 8.1860 |
| #2243 | 2025-07-30 | fix(pydecimal): allow Decimal for min/max\_value | 2 | 0 | 0 | 0 | 0 | 8.1859 |
| #2232 | 2025-07-30 | Fix Turkish TCKN provider | 1 | 0 | 0 | 0 | 0 | 8.1859 |
| #2230 | 2025-07-30 | Fix unnatural Korean company names in ko\_KR | 2 | 0 | 0 | 0 | 0 | 8.1855 |
| #2211 | 2025-07-30 | Add Spanish lorem provider | 3 | 0 | 0 | 0 | 0 | 8.1837 |
| #2225 | 2025-07-30 | Provide male names correctly | 1 | 0 | 0 | 0 | 0 | 8.1837 |
| #2220 | 2025-07-15 | Refactoring `faker/utils` | 5 | 0 | 0 | 0 | 0 | 8.1837 |
| #2214 | 2025-07-15 | Changed VIN generation function | 1 | 0 | 0 | 0 | 0 | 8.1837 |

---

## §3 Flag and Suppression Judgment

Only 2 events occurred (1 flag + 1 auto-gen suppression). A 30-flag stratified sample is
not possible at this flag volume. Both events are documented in full.

### Event 1 — Flag: PR #2259 `faker/factory.py` hunk#0

**Category: FALSE\_POSITIVE**

- **PR title:** "fix: fix minor grammar typo"
- **Stage:** Stage 1 (import), `import_score=1.0`, `bpe_score=4.4956`
- **Threshold:** 8.1865
- **Foreign modules:** `[]`
- **Diff content:**

```diff
@@ -107,8 +107,7 @@
             if locale:
                 logger.debug(
                     "Provider `%s` does not feature localization. "
-                    "Specified locale `%s` is not utilized for this "
-                    "provider.",
+                    "Specified locale `%s` is not used for this provider.",
                     provider_module.__name__,
                     locale,
                 )
```

- **Why it fired:** Stage 1 uses `file_imports | hunk_imports` when `file_source` is
  provided (line 250–254 of `sequential_import_bpe_scorer.py`). `faker/factory.py` imports
  `logging`, `importlib.import_module`, `typing`, `sys` — all stdlib modules not present
  in faker's `_repo_modules` set. The hunk itself has no import changes (pure string
  edit), but file-level foreign imports make `import_score = float(len(foreign)) ≥ 1.0`.
- **FP class:** **File-level import contamination** — stdlib imports in the surrounding
  file trigger Stage 1 regardless of hunk content. Not a new class; previously documented
  as a Stage 1 FP mechanism. In practice, `faker/factory.py` (the main factory
  orchestrator) is the most import-heavy file in the repo — it is disproportionately
  likely to attract this FP.

---

### Event 2 — Auto-gen suppression: PR #2267 `faker/providers/bank/__init__.py` hunk#0

**Category: FALSE\_POSITIVE (auto-gen filter)**

- **PR title:** "Fix bank dry violation"
- **Diff content:** Adds a `bank()` base method to the `Provider` class that raises
  `AttributeError` if the subclass has no `banks` tuple — a legitimate manual refactor
  to eliminate duplicated `bank()` methods across locale subclasses.
- **Why it fired:** `is_auto_generated()` scans the first 40 lines of the file for
  known markers. Line 18 of `faker/providers/bank/__init__.py` reads:

  ```
  Bank codes, account numbers, and other ID's generated by this provider
  ```

  The phrase **"generated by"** matches the generic marker in `_GENERIC_MARKERS`. This
  is a docstring description of what the class produces, not a generation provenance
  marker.
- **FP class:** **Docstring phrase collision** — a class docstring that describes what
  data the class generates (a very common phrasing in faker's provider docs) matches a
  generic "generated by" marker. This is a new FP class not seen in FastAPI or Rich,
  where provider classes don't describe themselves as "generators of X."

---

## §4 Cross-Corpus Comparison and Faker-Specific Findings

### 4.1 Threshold Inflation — Root Cause

Faker's calibration threshold (8.18–8.21) is roughly 2× FastAPI's and Rich's (4.11–4.42).
The cause is the interaction of three factors:

1. **`max(cal_scores)` aggregation:** The threshold is the single highest BPE score in
   the calibration sample. It is a worst-case bound.
2. **N_CAL = 500:** With 500 random hunks drawn from faker's source tree, the sampler
   is highly likely to hit a hunk from a locale provider file containing a large
   string table. These tables (names, addresses, dates in 70+ locales) have high BPE
   cross-entropy under the generic model B because they contain non-English strings,
   abbreviated codes, and phonetic data not present in the generic corpus.
3. **Faker's vocabulary distribution:** A substantial fraction of faker's source is
   locale data. Unlike FastAPI (mostly application logic) or Rich (mostly rendering
   logic), faker has many files like `faker/providers/address/ar_AA/__init__.py`
   containing long tuples of Arabic/Korean/Hebrew strings. These push the `max`
   calibration score far above what PR hunks typically score.

The result: the calibration ceiling (8.2050) is higher than the max BPE score
observable in any of the 130 PR hunks (8.0152). Stage 2 is **structurally blind** on
faker under this configuration.

**Important nuance:** The N_CAL=500 stability result from Prompt R was measured on
FastAPI PRs, not on faker. The stability guarantee (N=500 → max\_rel\_var 1.4%,
Jaccard 100%) was corpus-specific. On faker, higher N makes the threshold more
deterministic but also inflates it further (more samples → higher chance of hitting a
locale string outlier). The stability test has not been run on faker at N=100 vs N=500.

### 4.2 Locale Provider Files — No FP Class Found

Faker has hundreds of locale provider files under `faker/providers/*/<locale>/__init__.py`.
These files contain large string tables (names, cities, postal codes, etc.). The question
was whether they would:
(a) trigger auto-gen suppression, or
(b) be flagged directly

**Result: Neither.** Inspection of sampled locale headers shows clean Python class
definitions with no auto-gen markers. A representative header:

```python
from typing import Tuple
from .. import Provider as AddressProvider

class Provider(AddressProvider):
    """Address provider for fr_DZ locale."""
    # Source: https://fr.wikipedia.org/wiki/Wilayas_d%27Algérie
    wilayas: Tuple[str, ...] = (
```

No "generated by", "do not edit", or similar markers appear in locale file headers.
The locale string tables do, however, drive calibration threshold inflation (§4.1):
they inflate `max(cal_scores)` without being directly flagged.

PRs #2271 (Add ar\_DZ), #2270 (Add fr\_DZ), #2256 (Add Nigerian locales) added
whole new locale trees and were scored with 0 flags — correct behaviour, since
adding locale data is expected churn for faker, not foreign style.

### 4.3 Provider-Class Inheritance — No Distortion

Faker's architecture relies on many thin subclasses that override a few attributes
(`banks = (...)`, `bban_format = "..."`) while inheriting provider logic from a base
class. These patterns scored cleanly: BPE scores for such hunks were in the 1–4 range,
well below the calibration threshold. The inheritance pattern did not confuse the scorer.

### 4.4 New FP Class: Docstring Phrase Collision in Auto-Gen Filter

The auto-gen filter uses broad string matching including the generic marker `"generated by"`.
In faker's provider ecosystem, class docstrings routinely describe the provider as
generating data: "Generated by this provider", "values generated using this method",
etc. This is structurally different from auto-generation provenance markers (e.g.,
"Generated by protoc"). The fix7 auto-gen filter does not distinguish between them.

**Scope of exposure in faker:** A spot-check of the base bank, address, and person
providers shows this phrasing appears in multiple class docstrings. The hunk in PR #2267
happened to be suppressed because its file matched first. Other provider base files
may have similar docstrings and would also be suppressed.

**FastAPI/Rich exposure:** Neither FastAPI nor Rich has this problem because they do
not describe themselves as "generating" outputs in their class-level docstrings.

---

## §5 Verdict

**Fix7 does NOT cleanly generalise to faker under the current configuration.** Two
distinct failure modes were observed:

### Failure 1 — Stage 2 Structural Blindness (Known Limit)

`max(cal_scores)` with N_CAL=500 on faker's locale-heavy corpus produces a calibration
ceiling (~8.20) above the maximum BPE score observable in PR hunks (~8.02). Stage 2
fires on **zero hunks** across 50 PRs.

- **Root cause:** Locale string tables in the calibration sample drive the max BPE
  score well above the "application code" ceiling. The threshold strategy that works
  for FastAPI/Rich (small, uniform codebases) degenerates on a mixed-purpose codebase
  where a significant fraction of source files are data files.
- **Decision: Document as known limit, no fix8.** This is a property of the
  `max(cal_scores)` threshold strategy, not a bug in the scorer. Options for future
  work: (a) exclude data-heavy files from calibration sampling, (b) use a percentile
  (p99/p95) threshold, (c) corpus-specific calibration. None of these are needed for
  the current PR campaign targets (FastAPI, Rich), where the scorer is working.

### Failure 2 — Auto-Gen Docstring Phrase Collision (New FP Class)

The auto-gen filter fires on `faker/providers/bank/__init__.py` because the class
docstring contains "generated by this provider" — a legitimate description of what
the class does, not a machine-authorship marker.

- **Root cause:** The generic `"generated by"` marker in `_GENERIC_MARKERS` is too
  broad. It cannot distinguish "this file was generated by tool X" from "this class
  generates values of type Y."
- **Decision: Document as known limit, no fix8.** The false suppression is benign
  (it causes a real PR change to be skipped, not flagged). On faker specifically, the
  structural blindness of Stage 2 (Failure 1) already means this file would not have
  been flagged by BPE anyway. Fixing this marker would require context-aware parsing
  (e.g., "generated by" only in comments/strings, not in docstrings) — out of scope.
  Mark as a known limit specific to data-generation libraries.

### Summary

| Issue | FP class | Decision |
|---|---|---|
| Stage 2 blind (`max` + N=500 on locale corpus) | Known limit — threshold strategy × corpus type | Document; no fix8 |
| Auto-gen filter: "generated by this provider" | New — docstring phrase collision | Document; no fix8 |
| Stage 1: stdlib imports in `factory.py` | Known — file-level import contamination | Pre-existing, no change |

**For the PR campaign:** Faker is not a target corpus. FastAPI and Rich are unaffected
by the faker-specific findings. Fix7 remains the production config for FastAPI and Rich
validation. No fix8 is warranted.
