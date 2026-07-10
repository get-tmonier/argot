# The semantic layer's self-calibration contract

Companion to [`calibration-contract.md`](calibration-contract.md), which governs
the *base* statistical scorer. The semantic layer (feature `semantic`,
`crates/argot-core/src/scoring/semantic/`) calibrates itself separately at fit
time; this file records the invariants a contributor must not break.

## The shape

One artifact, `.argot/semantic-index.json` (`SemanticArtifact`), built at fit:
per language, every corpus function's embedding (f16, base64) plus two
structural fingerprints per function — its **callees** and its IDF-weighted
**identifier subtokens** — and two self-calibrated config blocks:

- `reinvention` (`ReinventionConfig`) — drives the `redundant` rule (F1).
- `placement` (`PlacementConfig`) — drives the `misplaced` rule (F2).

The artifact records the embedding model's identity (`ModelIdentity`:
name/sha256/dim) and a format `version`; `validate_current()` rejects an index
built by another model or format **loudly** at check ("run `argot fit` to
rebuild"). Never bypass that gate: cosines across two embedding spaces are
silently wrong, not approximately right.

## F1 · reinvention — retrieval fires, structure confirms

Embeddings retrieve the nearest cross-file function; **cheap structural
agreement confirms** (code embeddings are anisotropic — cosine alone over-fires):

- normal tier: `cos ≥ 0.78` + moderate callee/subtoken overlap;
- strong (rescue) tier: `cos ≥ 0.70` + high structural agreement — the
  heavily-reworded reinvention.

**Fit-time self-calibration (mini-replay):** the fit replays the repo's own
recently added functions against the index and estimates the false-fire rate.
Repos that practice *systematic parallel implementation* (per-locale providers,
checkout/order mirrors, protocol-variant families) trip a **twin-rate guard**
and get a stricter **conservative mode** automatically. This is what holds
false-fire ≤ 2.8%/hunk on all 31 corpora with no per-corpus exceptions — do not
replace it with hardcoded per-repo knobs (`CLAUDE.md`: no corpus-specific
strings in prod).

## F2 · placement — learned granularity, honest abstention

At fit, argot walks the tree to the repo's *real* package granularity, merges
semantically-entangled packages (a header-only lib's `src/` + `include/` are
one area), and calibrates the k-NN vote via a transplant simulation on the
repo's own functions. **When no configuration reaches usable recall, placement
disables itself for that repo** — a flat single-package library gets silence,
not noise. Abstention is a feature; never "fix" a corpus by lowering the bar.

## Invariants (the contract)

1. **Degradation is loud.** No embedder, stale index, offline — every skip
   prints its reason. A semantic result of "no findings" must never be
   confusable with "the layer didn't run" (the bench once recorded fake zeros;
   see commit `c5dfff3e`).
2. **Base metric untouched.** Semantic findings are never folded into the base
   catch/over-fire numbers, and with the feature off the base build is
   byte-for-byte unchanged.
3. **Fit and check extract identically.** `functions_in_file` is the single
   extraction path for both; an indexed function and its check-time
   re-derivation must be the same bytes.
4. **Self-calibration over global thresholds.** Any new bar must be measured
   per-repo at fit against the repo's own history, not tuned globally on the
   bench and hardcoded.
5. **Model changes ride releases.** New model ⇒ new pinned constants + release
   tag; `ARTIFACT_VERSION`/`ModelIdentity` invalidate old indices. There is no
   runtime model choice by design.

Evidence trail: `docs/research/evidence/semantic-*.md` (tuning, all-gates run,
F1 conservative mode, F2 self-calibration).
