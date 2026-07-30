# The embedded model

Two files, ~17.5 MB, committed:

- `embeddings.safetensors` — 15.6 MB. An int8 token-embedding table,
  61,053 × 256, plus one shared dequantisation scale. Its safetensors
  `__metadata__` carries the provenance.
- `tokenizer.json` — 1.9 MB. The teacher's byte-level BPE tokenizer.

## Why they are in git

`static_embedder.rs` reads them with `include_bytes!`, which needs the bytes at
compile time. There is no way to embed a file in the binary that the compiler
cannot see.

## What was chosen against

A `build.rs` that downloads the weights during the build. It would keep the
repository small and break the thing this change was for: `cargo build` would
need a network, an offline or air-gapped build from source would fail, and
distribution packagers — who build in sandboxes with no egress by policy —
could not build argot at all. Moving a download out of the user's first run and
into the contributor's build is a worse trade.

Git LFS was also considered and declined: it trades a fixed 17.5 MB for a
hosting feature, a second auth path, and a clone that silently produces a repo
that will not build when LFS is missing.

## What it costs

Every clone pays ~17.5 MB once, including clones that never build the
`semantic` feature — the dev and CI base loops build with no features at all.
The weights are frozen: they change on a re-distill, not per release.

## Replacing them

Re-distilling changes every vector and therefore every semantic finding. That
is a research change: re-run the semantic bench and record the evidence under
`docs/research/evidence/`. A stale `.argot/semantic-index.json` is rejected
loudly rather than silently mis-scored — the artifact records the model's name,
fingerprint and dimensionality, and `check` validates them against the embedder
that is about to query it.

Provenance and license: see the repository `NOTICE`. These are derived from
jina-embeddings-v2-base-code (Apache-2.0) via model2vec (MIT).
