# TS Corpus Validation — faker-js — 2026-04-23

**Scorer version:** fix10 + LanguageAdapter refactor + glob-alias fix (branch `research/phase-14-import-graph`)  
**Repo:** faker-js/faker · TypeScript fake data library (~92% locale data files)  
**Validation date:** 2026-04-23  
**Script:** `engine/argot/research/signal/phase14/experiments/score_pr_hunks_ts_faker_2026_04_23.py`  
**Output:** none — Check 1 gate failure; JSONL not written

---

## §0 Headline Numbers

| Metric | Value |
|--------|-------|
| Total .ts files in corpus (locale) | 2965 |
| Locale files excluded by `is_data_dominant` | 76 (2.6%) |
| Locale files kept (not excluded) | 2889 (97.4%) |
| Check 1 status | **FAIL** (pass condition: ≥80% excluded) |
| Hunks scored | 0 (gate failure — not reached) |
| Source flags | N/A |
| JSONL written | No |

The script raised `SystemExit` immediately after Check 1. No calibration, stability probe, or PR scoring was attempted.

---

## §1 Data-Dominance Eligibility Gate

**This is the headline section. Check 1 FAILED.**

### Audit numbers

| Metric | Count |
|--------|-------|
| Total locale files (`"locales" in path parts`) | 2965 |
| Excluded (`is_data_dominant = True`) | 76 |
| Kept (`is_data_dominant = False`) | 2889 |
| Exclusion rate | **2.6%** |
| Pass threshold | ≥80% |
| Status | **FAIL** |

### Root cause of failure

The `is_data_dominant` method in `TypeScriptAdapter` is blind to `export default` arrays and objects. It only handles the `variable_declarator` node pattern — i.e., `export const x = [...]` / `export const x = {...}`. This is a content-pattern gap, not a path-based problem.

faker-js locale files use `export default` exclusively:

**Pattern 1 — array literal (1759 locale files):**
```typescript
// src/locales/en/location/city_name.ts
export default [
  'Abilene',
  'Akron',
  ...
];
```

**Pattern 2 — object literal with array values (408 locale files):**
```typescript
// src/locales/de/person/first_name.ts
export default {
  generic: [
    'Arda',
    ...
  ],
};
```

**Pattern 3 — small inline array (many locale files):**
```typescript
// src/locales/sk/person/suffix.ts
export default ['Phd.'];
```

**Pattern 4 — object with weighted entries (many locale files):**
```typescript
// src/locales/sk/person/last_name_pattern.ts
export default {
  female: [{ value: '{{person.last_name.female}}', weight: 1 }],
  male: [{ value: '{{person.last_name.male}}', weight: 1 }],
};
```

**Pattern 5 — object with nested arrays (many locale files):**
```typescript
// src/locales/en/commerce/product_name.ts
export default {
  adjective: [
    'Awesome',
    'Bespoke',
    'Electronic',
    ...
  ],
};
```

None of these files produce a `variable_declarator` node in the tree-sitter parse tree. They produce `export_statement` nodes with an array or object expression as the default export. Since `_collect_ts_data_rows()` in `TypeScriptAdapter` only queries for `variable_declarator`, it finds zero data rows in every locale file and therefore classifies them as `is_data_dominant = False`.

The remaining ~797 locale files are `index.ts` re-export aggregators (pattern: `import foo from './foo'; ... export default { foo, bar, ... }`). These import locale data and re-export it as a typed definition object — they are correctly not classified as data-dominant, and excluding them would be wrong.

### Implication

The fix is narrowly scoped: add `export_statement` node handling to `_collect_ts_data_rows()` in `TypeScriptAdapter`. Specifically, detect:

- `export_statement` nodes with an `array` child as the default export value
- `export_statement` nodes with an `object` child as the default export value, where the object's values are arrays or string/number literals

The Hono and Ink runs are unaffected — neither corpus has locale-style files.

---

## §2 Non-Locale File Inclusion

**Not reachable — Check 1 failed.**

Check 2 logic (verify that non-locale files are kept at a ≥95% rate) is implemented in `_print_eligibility_gate()` and would have run immediately after Check 1 if the gate had passed. Since Check 1 raised `SystemExit`, Check 2 was not executed.

---

## §3 Calibration and Stability Probe Results

**Not reachable — Check 1 failed.**

The mandatory 3-seed stability probe was specified and implemented (runs for every PR regardless of pool size, using N_CAL=300 first with escalation to N_CAL=500 if `rel_var > 0.10` or `jaccard < 0.80`). It was not executed. No thresholds or stability metrics are available for this run.

---

## §4 Stage 1 Behavior on Locale Files

**Not reachable — Check 1 failed.**

The script includes a Check 4 guard (`locale_stage1_warn`) that would have printed a warning for any locale-file hunk with `import_score > 0`. No hunks were scored.

---

## §5 Glob Alias FP Check

**Result: CLEAN — no alias risk.**

The tsconfig paths check ran as part of script initialization (before the locale gate, Check 5 is pre-scored at the script level). faker-js's `tsconfig.json` has no `paths` entries:

```
tsconfig.json: no paths entries
```

No `@/*` or similar aliases are defined. There is no glob alias FP risk for faker-js. Stage 1 import scoring would not be distorted by alias mismatches.

---

## §6 Core PRs vs Locale PRs Flag-Rate Divergence

**Not reachable — Check 1 failed.**

The 5 selected PRs were hand-picked to include both locale and core changes so the flag-rate divergence between the two PR types could be measured. For reference when the `is_data_dominant` fix is applied and this validation is re-run:

| # | PR | Title | Type |
|---|-----|-------|------|
| 1 | #3798 | feat(locale): Add postal_address and improved secondary_address for es | locale |
| 2 | #3796 | feat(locale): add mn_MN_cyrl (Mongolian) locale | locale |
| 3 | #3820 | refactor(core): expose core.locale as LocaleProxy | core |
| 4 | #3783 | feat(date): add ability to provide year range for past and future | core |
| 5 | #3809 | refactor(location): simplify locale access | core |

Expected behavior after fix: locale PRs (#3798, #3796) should have their locale-file hunks excluded from model A (they are data-dominant), and any non-locale hunks in those PRs (e.g., index re-exports) should be scored normally. Core PRs (#3820, #3783, #3809) touch application logic and should be scored against a clean model A.

---

## §7 Per-Flag Table

**Not applicable — no hunks were scored (Check 1 gate failure).**

---

## §8 Category Breakdown

**Not applicable — no hunks were scored (Check 1 gate failure).**

---

## §9 Three-Corpus Comparison (Hono, Ink, faker-js)

| Metric | Hono | Ink | faker-js |
|--------|------|-----|----------|
| TS files in corpus | 196 | 56–64 | ~3200+ (incl. locales) |
| Locale files | 0 | 0 | 2965 |
| Locale exclusion rate | 0% (N/A) | 0% (N/A) | **2.6% (FAIL)** |
| Check 1 status | N/A (no locale files) | N/A (no locale files) | **FAIL** |
| Stability probe | Not triggered (pool > 400) | Passed (Δ = 0.000, pool-capped) | Not reached |
| BPE threshold range | 5.8487–5.8628 (Δ = 0.014) | 3.5422–3.7528 (Δ = 0.211) | N/A |
| Flag rate (source) | 0.0% (0/22) | 21.4% (3/14) | N/A |
| Flag rate (test) | 42.9% (3/7) | 100% (4/4) | N/A |
| All flags INTENTIONAL? | Yes | Yes | N/A |

**Narrative:**

Hono and Ink are clean corpora with no locale data files. In both cases `is_data_dominant` fired zero times on non-data source files — confirming the filter is not over-aggressive on normal TypeScript code. Its 0% exclusion of Hono/Ink source files is the correct baseline.

faker-js is the first corpus to stress the locale-exclusion logic, and it revealed a missing pattern. The filter's 2.6% exclusion rate on locale files means the scorer sees 2889 locale data arrays as valid calibration candidates, polluting the BPE model A with string-literal frequency distributions that are not representative of real application code. A calibration corpus dominated by locale arrays would produce an artificially low BPE threshold, increasing false positives on real source hunks.

The failure on faker-js is specifically a missing tree-sitter query in `_collect_ts_data_rows()` — the `export_statement` with array/object default pattern is not handled. This is a targeted fix, not a fundamental architecture problem. Hono and Ink required no changes to behave correctly, and the fix will not affect them.

---

## §10 Verdict

- **Scorer handles locale-dominant corpus: NO — needs tuning**
- **Specific gap:** `is_data_dominant()` in `TypeScriptAdapter` does not handle `export default [array]` or `export default {object}` patterns. It only handles `export const x = [...]` via `variable_declarator` nodes. faker-js locale files use `export_statement` nodes with array/object defaults exclusively.
- **Fix scope:** Narrow and targeted — add `export_statement` node handling to `_collect_ts_data_rows()` in `TypeScriptAdapter`. The fix does not touch calibration logic, the BPE scorer, or the import scorer.
- **Risk:** Low — Hono and Ink are unaffected (they have no locale-style files). The fix adds a new query branch for a pattern those corpora do not use.
- **Next step:** Apply targeted fix to `TypeScriptAdapter._collect_ts_data_rows()`, then re-run the faker-js validation (`score_pr_hunks_ts_faker_2026_04_23.py`) to confirm Check 1 passes at ≥80% locale exclusion.
