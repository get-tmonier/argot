# Era 15 — The production path: close the gap between the bench and `argot check`

Era 14 closed with strong bench numbers (108/115 libraries, 45/56
applications, FP ≤ 2%) that did not describe the shipped tool. Era 15's brief:
make `argot check` itself earn those numbers — subtle out-of-voice catches on
real repos at near-zero FP — with a production-path bench mode as the new
headline, and a live gauntlet on a real workspace (moneta, TypeScript/Effect,
~34k files) as the acceptance test.

## The three measured root causes (all confirmed in code)

1. **Self-attestation (issue #79).** `check` rebuilt the callee attestation
   and the BPE token distribution from the corpus files *on disk at check
   time*, so new code attested its own callees and diluted its own token
   surprise. The unattested-callee branches essentially never fired on
   exactly the code check exists to judge. The import stage was immune via
   its fit-time `import_modules` snapshot.
2. **No cluster routing at check time.** `check` passed `file_path=None`
   into scoring, taking the non-cluster contribution branch — while bench
   fixtures and calibration hunks were cluster-routed and could earn +5
   bonuses.
3. **Cal/check threshold asymmetry.** Calibration candidates earned cluster
   bonuses into the max-of-sample threshold; the check path could never earn
   them. On moneta the threshold sat at 11.30 while the strongest import-free
   hunk peaked near 9 — plain-language verdict "Ready", actual behaviour: an
   import tripwire.

## Fixes shipped (production code)

### The model artifact (scorer-config v3)

Calibrate now persists the fit-time model per language: BPE token counts,
callee attestation, cluster partition (repo-relative file keys), convention
frequencies + bars, plus a deterministic `model_hash` (same corpus + config →
byte-identical artifact; the #63 reproducibility slice). Check loads scorers
entirely from the artifact — no corpus re-read, no live-tree attestation.
Side effects: moneta `fit` runs in ~7 s and `check` startup no longer
re-tokenizes the corpus.

Also folded: `resolve_repo_modules` into the production import snapshot
(the bench always had it; check didn't), and repo-dir canonicalization in
calibrate — the un-canonicalized rglob paths missed `file_to_cluster` and
sent calibration hunks through Jaccard-nearest routing, which is where
moneta's spurious +5s (threshold 11.30) came from. After the fix the moneta
threshold calibrates to 6.30.

### Check scores with the calibration's signal surface

`check` passes the hunk's repo-relative path; fitted files route to their
fit-time cluster. Cluster-conditional branches apply **only** to fitted
files — Jaccard-guessing a cluster for an unknown file handed its own staples
wrong-cluster bonuses (a React file routed into an Effect-heavy cluster was
the dominant FP driver on new-feature commits; measured directly, below).

### Row-granular data scope

Check dropped data-dominant files wholesale and typicality's file-level
fallback zeroed hunks in data-heavy files — so *code planted inside data
files* was invisible (3 of 16 faker fixtures in production mode). Both
file-level vetoes are replaced by one row gate in `score_hunk`: a hunk whose
non-blank rows sit mostly (>0.65) inside the host's data-literal spans is
skipped as data; code rows in the same file are judged. faker production
recall 12/16 → 15/16 at FP 0/27 control hunks.

### Parse-error host fallback ON (era-14 phase D, re-litigated)

Git picks hunk boundaries, not the parser: staged hunks routinely open
mid-construct (`}` + blank + new code), so bare-fragment parse errors are the
*norm* at check time — and the call-receiver contributed 0 on exactly those
hunks, while the calibration side always applied the host fallback. Era 14
gated the fallback off from catalog-mode FP measured with a forced
cluster-rare rule; production auto-detects that rule per corpus. Enabling it
took the moneta gauntlet from 3/12 to 6/12. The bench flag is now
`--no-parse-error-fallback` (era-14 baseline reproduction).

## The convention-rarity stage (new signal family)

The gauntlet's remaining misses were *conventional*, not lexical: snake_case
transliteration, `var` declarations, `class` lifecycle in a functional
codebase. Scout evidence (moneta, 496-hunk calibration sample vs the 12
breaks):

- token-surprise aggregations (max, top-10 mean, fraction above a mild bar)
  do **not** separate — the breaks sit inside the calibration body (the
  tokens of `total_sum = total_sum + values[item_index]` are all common);
  the sustained-surprise family is refuted for this population;
- **AST node-kind surprisal** separates: `var` statements score 12.51 and
  `public` modifiers 11.26 vs calibration max 10.72;
- **identifier-shape surprisal** (abstract morphology: camel / pascal /
  snake / scream / flat — character classes only, no words) separates:
  snake_case in the camelCase corpus scores 3.41 vs calibration max 1.78.

Shipped as `scoring/conventions.rs`: corpus-derived frequencies, max
surprisal per hunk, firing bars calibrated as the max feature value over the
multi-seed calibration sample, suppressed on the calibration side per the
contract (asymmetric by construction), +5.0 bonus, new reason code
`convention`. This is *presence of atypical convention* — a different family
from era-14 phase C's refuted *absence of typical callees* primitives.

## Neighbourhood attestation (the FP arm)

Fixing #79 exposed the alpha branch's FP nature: the self-attestation bug had
been acting as the FP suppressor. On 30 real moneta commits the fresh-fit
check produced 80 hits; instrumenting the contribution events showed in-voice
new code firing through three holes, each now closed on principle:

- `setRenaming:Unattested` — a `useState` destructured binding invoked bare.
  **Value bindings** (destructured names, consts, parameters) now attest
  bare calls; a dotted method on them (`xhr.open`) still carries voice.
- `widgetTitles.join:Unattested` — dotted callees are keyed by receiver
  variable name, so any fresh receiver made corpus-universal methods look
  unattested. **Corpus-known method segments** no longer alpha-fire
  (derived from the attested set; no artifact change).
- Callables the change defines (changeset-wide), and names imported from
  **relative specifiers**, attest bare + rooted calls — code naming its own
  neighbourhood is not foreign voice.

FP control trajectory on the same 30 commits: 80 → 63 (no cluster-guessing)
→ 41 (neighbourhood attestation). Recall cost: one gauntlet break (XHR —
its `open`/`send` methods are corpus-known); 9/12 → 8/12.

## Moneta gauntlet (acceptance)

12 subtle NO-import breaks planted into real corpus files (plant → `git add`
→ `argot-local check --staged` → restore from HEAD; never committed):

| break | outcome | reason |
|:---|:---|:---|
| snake_case transliteration | **caught** | convention (ident morphology) |
| callback pyramid | **caught** | call_receiver |
| jQuery-style DOM mutation | **caught** | call_receiver |
| raw SQL string building | **caught** | bpe |
| legacy class lifecycle | miss | this-rooted calls invisible; syntax 7.56 < bar 10.71 |
| Java getter/setter bean | **caught** | bpe |
| promise chains in Effect code | **caught** | call_receiver |
| XMLHttpRequest ES5 | miss | open/send are corpus-known methods (traded for FP) |
| ES5 var+concat loop | **caught** | bpe |
| global event registry (var) | **caught** | convention (node kinds) |
| Go-style error tuple | miss | nothing lexically or conventionally rare enough |
| Python-docstring comments | miss | prose-blanked by design; ident 1.65 < bar 2.15 |

**8/12 flagged** (G3 recall gate: ≥ 8 ✓). Era start: 0/12 would have crossed
the 11.30 threshold (import tripwire only).

FP controls on real moneta history:

- **fit@HEAD, check last 30 commits**: 41 hits / 1656 hunks (2.5% of hunks,
  11/30 commits). Residue is dominated by churn drift — the window contains
  an actively-developed feature area, and each commit's intermediate file
  state differs from the fit (`ExpandControl (0×)` was renamed before HEAD;
  `versions.find` cluster-absent because the file evolved). This control
  replays a month of churn against today's model, which is not the product
  flow.
- **fit@HEAD~31, check the next 30 commits** (stale fit): 561 hits — a new
  feature epoch against a month-old model. Staleness is the dominant noise
  axis; motivates #60 (freshness warning) and per-change fitting (fit is 7 s).
- **fit at each commit's parent, check that commit** (the product flow):
  see below.

## Threshold mechanism (task: heterogeneous corpora)

Measured before building anything: moneta's calibration distribution is
P50 1.46 / P95 3.32 / P99 4.42 / max 6.30, while the pure-phrasing breaks
score 2.3–4.1 — **inside the calibration body**. No sub-max aggregation can
separate them; the failure was signal, not thresholding. The convention
stage (above) is the mechanism that actually unlocked this class. Sub-max
thresholding for its own sake remains un-shipped; max-of-sample stays, with
the phrasing-headroom report making its consequences visible per repo.

## G4: phrasing headroom in `argot inspect`

`inspect` now reports, per language, the BPE ceiling (max reachable token
surprise under the model), the callee cap, and
`phrasing_headroom = ceiling + cap − threshold`. Headroom ≤ 0 → red
`phrasing_detection_dead` (import tripwire only; verdict not_recommended);
threshold above the BPE ceiling alone → yellow `phrasing_needs_callees`.
moneta after the era-15 fixes: threshold 6.30, ceiling 10.41, headroom +9.11
— phrasing detection alive, and the report says so instead of a bare
"Ready".

## Production-path bench mode

`argot-bench --mode production` (now the default) plants every catalog
fixture into its host file on disk, stages it with real git, and judges it
with `run_check --staged` against a real `argot fit` artifact; the FP
control replays the corpus's recent commits through `check --commit`.
`--mode both` reports the catalog↔production recall gap as a tracked metric.
Catalog mode (`--mode catalog`) stays for continuity; `--no-conventions` and
`--no-parse-error-fallback` reproduce the era-14 configuration.

<!-- RESULTS: final both-mode numbers land here -->

## Reproducibility notes

- Gauntlet scripts and per-run JSON: session scratchpad (plant/restore, per
  break results, FP-control breakdowns). Break fixtures are ephemeral by
  design (moneta is a private repo); the recipe is documented above.
- All scoring changes are corpus-derived; no language or framework literals
  entered production code (CLAUDE.md G6 discipline).
