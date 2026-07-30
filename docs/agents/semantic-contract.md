# The semantic layer's self-calibration contract

Companion to [`calibration-contract.md`](calibration-contract.md), which governs
the *base* statistical scorer. The semantic layer (feature `semantic`, crate
`crates/argot-rules-semantic/`) calibrates itself separately at fit time; this
file records the invariants a contributor must not break.

## The embedder

A **static token-embedding table compiled into the binary** — 15.6 MB of int8
weights under `crates/argot-rules-semantic/model/`, distilled from
jina-embeddings-v2-base-code with the model2vec technique, read by argot's own
forty lines of inference in `static_embedder.rs`. Embedding is a table lookup,
a sum in token order, an L2 normalise, and an f16 canonicalisation.

Three consequences a contributor should hold onto:

- **There is no download, no cache to warm, no accelerator.** A fit works
  air-gapped on the first run. Any code path that reintroduces a network
  dependency on the analysis side breaks this.
- **Pooling order is part of the determinism contract.** The sum runs in token
  order over a fixed row layout and the result is canonicalised through f16, so
  a cached vector and a freshly computed one are bit-identical. That identity is
  what lets `~/.cache/argot/embeddings/` be a pure accelerator.
- **It cannot represent order or structure.** A bag of token vectors is a weaker
  sense than the transformer it replaced. The layer is viable because the
  structural confirmation below carries the precision — not because the
  embedding is strong. Weakening that confirmation to "let the embedder decide"
  is the change most likely to quietly wreck this layer.

`ARGOT_STATIC_MODEL=<dir>` points the embedder at another model directory
(`model.safetensors` + `tokenizer.json`). It exists so a candidate model can be
swept before it is shipped; it is not a user knob and is not documented as one.

## The shape

One artifact, `.argot/semantic-index.json` (`SemanticArtifact`), built at fit:
per language, every corpus function's embedding (int8 codes with one shared
dequantisation scale, base64) plus two structural fingerprints per function —
its **callees** and its IDF-weighted **identifier subtokens** — and two
self-calibrated config blocks:

- `reinvention` (`ReinventionConfig`) — drives the `redundant` rule (F1).
- `placement` (`PlacementConfig`) — drives the `misplaced` rule (F2).

The artifact records the embedding model's identity (`ModelIdentity`:
name/sha256/dim) and a format `version`. Validation is split in two so a check
with nothing to score never pays a model load:

- `validate_format()` — on-disk format only, no embedder needed. Checked first.
- `validate_for(&dyn EmbeddingModel)` — against the embedder that will actually
  query the index, which is not necessarily the one the build pins
  (`ARGOT_STATIC_MODEL`). Checked once an embedder is loaded.
- `validate_current()` — convenience over the compiled-in model, for callers
  with no embedder in hand.

Never bypass those gates: cosines across two embedding spaces are silently
wrong, not approximately right.

Anything outside the Rust that reads the artifact is a **mirror** and goes stale
silently. `benchmarks/sem_analysis.py` decodes the vector blob itself; it
asserts the blob's byte width and exits non-zero when the layout moves, because
a mirror that guesses reports a plausible number for a space that does not
exist. Its driver (`sem_all.py`) checks every sub-script's exit code for the
same reason.

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
reinvention false-fire down — do not replace it with hardcoded per-repo knobs
(`CLAUDE.md`: no corpus-specific strings in prod).

## F2 · placement — learned granularity, honest abstention

At fit, argot walks the tree to the repo's *real* package granularity, merges
semantically-entangled packages (a header-only lib's `src/` + `include/` are
one area), and calibrates the k-NN vote via a transplant simulation on the
repo's own functions. **When no configuration reaches usable recall, placement
disables itself for that repo** — a flat single-package library gets silence,
not noise. Abstention is a feature; never "fix" a corpus by lowering the bar.

## Invariants (the contract)

1. **Degradation is loud.** No embedder, stale index, group off — every skip
   prints its reason. A semantic result of "no findings" must never be
   confusable with "the layer didn't run" (the bench once recorded fake zeros;
   see commit `c5dfff3e` — and did it again when a sub-script's exit code went
   unchecked).
2. **Base metric untouched.** Semantic findings are never folded into the base
   catch/over-fire numbers, and with the feature off the base build is
   byte-for-byte unchanged.
3. **Fit and check extract identically.** `functions_in_file` is the single
   extraction path for both; an indexed function and its check-time
   re-derivation must be the same bytes.
4. **Self-calibration over global thresholds.** Any new bar must be measured
   per-repo at fit against the repo's own history, not tuned globally on the
   bench and hardcoded.
5. **Model changes ride releases.** Re-distilling changes every vector and so
   every finding: it is a research change, not a dependency bump. Re-run
   `just bench-semantic`, record the evidence under `docs/research/evidence/`,
   and let `ModelIdentity` invalidate old indices. There is no runtime model
   choice by design. Provenance and license obligations live in the repository
   `NOTICE` and `crates/argot-rules-semantic/model/README.md`.

Evidence trail: `docs/research/evidence/semantic-*.md` (tuning, all-gates run,
F1 conservative mode, F2 self-calibration) and `static-embedder-*.md` (why the
transformer was replaced, and what it cost).
