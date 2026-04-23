# TypeScript validation: faker-js (locale-heavy data library)

## Setup

Third TS validation after hono and ink. faker-js is the adversarial case
for the `LanguageAdapter`: of 2965 locale files in the repo,
~75% are `export default` data arrays/objects — pure fixture content
that would flood the BPE calibration pool if admitted. The
`is_data_dominant` filter was introduced for exactly this shape, and
this run validates it end to end. Same `SequentialImportBpeScorer`
core as hono and ink, same protocol: 5 recent PRs scored against each
PR's pre-merge snapshot. PR mix is deliberately split — two
locale-only PRs (#3796 Mongolian Cyrillic, #3798 Spanish postal) and
three core PRs (#3820 LocaleProxy refactor, #3783 date range, #3809
locale access simplification) — to separate data-touching from
source-touching behaviour.

## Results

**2 source flags across 46 source hunks = 4.3% flag rate** (well below
the 30% investigation threshold). Both flags are on PR #3820, the
`LocaleProxy` core-module introduction.

| PR | Type | src hunks | flagged | BPE threshold | pool |
|---|---|---:|---:|---:|---:|
| #3798 es postal | locale | 6 | 0 | 4.6335 | 116 |
| #3796 mn_MN_cyrl | locale | 14 | 0 | 4.6291 | 112 |
| #3820 LocaleProxy | core | 15 | 2 | 4.6459 | 117 |
| #3783 date range | core | 7 | 0 | 4.6271 | 112 |
| #3809 locale access | core | 4 | 0 | 4.6440 | 117 |

Data-dominance excluded **2219/2965 locale files (74.8%)** — PASS on
the 70% gate. The 25.2% kept (746 files) are module-aggregator
`index.ts` re-exports, `export default null` stubs, and boundary-case
files with interspersed comments — legitimately not data-dominant.
Non-locale false-exclusion rate was 1/267 (0.4%) — PASS on the <5%
gate. Stage 1 fired 0 times on locale-file hunks (`locale_stage1_warn = 0`):
relative-path locale imports correctly classified as intra-repo.
Calibration pool capped at 112–117 after filter. BPE threshold band
4.6271–4.6459 (Δ = 0.019) — tightest of the three TS corpora,
reflecting the very uniform core-module subset that survives filtering.
3-seed stability probe returned identical thresholds (rel_var = 0.000,
jaccard = 1.000) on every PR. Split flag rates tell the story
cleanly: **locale PRs 0.0% (0/20)**, **core PRs 7.7% (2/26)**. Both
flagged hunks are in the new `src/internal/locale-proxy.ts` file
introducing novel TypeScript proxy infrastructure (mapped types with
`-?:`, `Symbol`-keyed properties, `??=` memoisation) — all judged
INTENTIONAL_STYLE_INTRO, zero FALSE_POSITIVE.

## Interpretation

The `is_data_dominant` filter is what makes faker-js tractable: without
it, 2219 locale data arrays would drown the calibration pool and
suppress the threshold into over-firing. With it, calibration runs on
~115 clean core application files and the scorer behaves as expected.
Locale-touching PRs produce zero source flags; core-touching PRs flag
only the genuinely novel feature content. The 4.3% overall flag rate
— on a repo whose content is 75% data by file count — validates the
data filter as the last missing piece of the TypeScript seam.
