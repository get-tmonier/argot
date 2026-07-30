---
title: How it works
description: A local statistical voice model plus optional semantic, architecture, integrity, and custom-rule checks — with explicit limits on what a clean result means.
group: Start
order: 3
---

argot composes a statistical **voice** pass with independently registered semantic,
architecture, integrity, and scripted-rule passes. The voice model and the fit-time
artifacts learn from repository history; custom rules are authored by the repo.
Each pass emits findings under named **rules** (`argot rules` lists the live
registry), and every configurable rule's severity is yours to set — see
[Configure](/docs/configure/#rules--rule-severities).

The **statistical voice model** is deliberately simple: **no neural network**, just two
token-frequency distributions and a maximum log-likelihood ratio. That's what catches *foreign*
patterns — a dependency or API the repo has never used (rules `foreign-import`,
`unfamiliar-callee`, `rare-tokens`) — and it's why the statistical pass fits in seconds and scores in milliseconds
on CPU.

## The everyday lifecycle

Argot is not a service that silently learns from every pull request. When setup or material accepted
drift calls for it, a team reviews and commits a **fit snapshot**: the learned repository voice and the indexes
that make checks reproducible. Think of it like updating a lockfile or a dependency: a small,
deliberate maintenance commit, not infrastructure that has to run on every PR.

<figure class="lifecycle-map" aria-label="Argot's lifecycle: learn locally, commit one reviewed baseline, use it for local and CI checks, then refresh it locally only after material accepted drift.">
  <div class="lifecycle-map-grid">
    <div class="lifecycle-map-step">
      <span class="lifecycle-map-number">1</span>
      <span class="lifecycle-map-kicker">LOCAL · ONCE</span>
      <strong>Learn the repository</strong>
      <code>argot init</code>
      <small>voice · semantic index · architecture · test signals</small>
    </div>
    <div class="lifecycle-map-step">
      <span class="lifecycle-map-number">2</span>
      <span class="lifecycle-map-kicker">REVIEWED · SHARED</span>
      <strong>Commit one baseline</strong>
      <code>argot.toml · .argot/</code>
      <small>repository-specific learned state; caches stay local</small>
    </div>
    <div class="lifecycle-map-step">
      <span class="lifecycle-map-number">3</span>
      <span class="lifecycle-map-kicker">LOCAL + PR</span>
      <strong>Check against that memory</strong>
      <code>agent · CLI · advisory CI</code>
      <small>CI reads the base snapshot; a PR cannot teach itself</small>
    </div>
    <div class="lifecycle-map-step lifecycle-map-refresh">
      <span class="lifecycle-map-number">4</span>
      <span class="lifecycle-map-kicker">ONLY WHEN USEFUL</span>
      <strong>Refresh deliberately</strong>
      <code>argot-refresh</code>
      <small>review scope + mutes · fit locally · recommit</small>
    </div>
  </div>
  <div class="lifecycle-map-loop"><span aria-hidden="true">↺</span> Material accepted source, function, or layout drift returns to the reviewed baseline. Docs churn does not. There is no default time or commit cadence.</div>
  <figcaption>One repository memory, reviewed in Git. Every check reads it; only a deliberate local refresh changes it.</figcaption>
</figure>

The snapshot is needed because it is the learned state, not a cache: it contains the calibrated
voice, semantic index, architecture and integrity artifacts, plus provenance that lets Argot tell
when the baseline is old. Without it, another clone — including CI — cannot make the same
repository-grounded comparison. For the exact files and refresh command, see
[Configure](/docs/configure/#which-files-live-where) and [CI](/docs/ci/#refreshing-it).

CI itself is optional: a team can use MCP context and a local pre-commit check only. When it does
add the Action, there is no separate service, cache, or fit runner to operate — it just reads the
same reviewed files already in Git and posts advisory evidence on the PR.

### Why commit the learned files?

`argot.toml` says **what** the team wants Argot to consider. The fit snapshot says **what Argot
learned from the repository at that point in time**. They are both inputs to a meaningful check.

| Committed part | Why it matters | Typical size |
| --- | --- | --- |
| Voice snapshot (`scorer-config.json` + baseline) | Contains the learned vocabulary and calibrated thresholds. Without it, another machine cannot tell whether an API or idiom is foreign without fitting again. | Usually hundreds of KB to a few MB. |
| Semantic index (`semantic-index.json`) | Contains the local map from functions to their nearest existing neighbours. Without it, `redundant` and `misplaced` cannot make the same “you already have this” comparison. | Usually the largest part: a few MB to a few tens of MB, depending on the number of functions. |
| Layering, integrity, health, manifest | Preserve the learned dependency/test signals and prove the snapshot matches the configuration and binary model; they also make a stale snapshot visible. | Usually KBs to low MBs. |

Committing them is therefore the lightweight alternative to a CI fit: every developer, agent, and
PR starts from the **same reviewed baseline**. A pull request cannot replace that baseline with its
own code, and CI remains a fast reader instead of a second training system. The snapshot changes
only when a human runs `argot-refresh`, approves any scope/mute maintenance, reviews the fit diff,
and commits the update.

As one concrete scale reference, a recent full setup produced a **21 MB** snapshot, including a
**16 MB** semantic index. Run `argot status` after fitting to see the exact size for your own
repository before committing it; there is no hidden CI storage or download.

The **supersession detector** rides the same fit: it replays up to 1,000 accepted first-parent
commits and mines replacement pairs — an import or callee removed while its replacement is added,
in the same file of the same commit, repeatedly, across files, in one direction. A survivor means
the repo is mid-migration: the replacement stops reading as foreign, and new code written the old
way raises `superseded` (warn by default), citing the migrating commits themselves. Migrations can
also be [declared in two lines of `argot.toml`](/docs/configure/#migration--declare-a-migration).
Pure git2 and tree-sitter — no model, no network.

The **reinvention** and **placement** checks share the one neural component: a per-repo
code-embedding index, built at fit with a small model that ships **inside the binary** (15.6 MB
of distilled weights — nothing to download, no cache to warm, works air-gapped). Reinvention
flags a new function that duplicates one the repo already has (rule `redundant`); placement flags
a function filed in the wrong module area (rule `misplaced`). No cloud, no text generation, no
GPU; turn a function into a vector, look up its neighbours.

The **architecture check** builds a module-dependency graph of your repo at fit and flags an
added internal import that reverses the repo's established layer direction (rule `layering`). Pure
graph analysis — no model, no network.

The **test-integrity check** reads *both sides* of a changeset's diff, builds a per-version test
inventory with the same tree-sitter parsers as the rest of argot, and diffs the two inventories into
events: a test deleted while the code it exercised survives (rule `test-deleted`), a skip/ignore
marker added or a test gutted (rule `test-disabled`), or an assertion excised, tautologized, or
loosened (rule `test-weakened`) — each only alongside a production-code change, never on a
tests-only commit. The gates for which events fire are learned per repo at fit, from a replay of the
repo's own accepted history. No model, no network — pure Rust and tree-sitter.

The embedding model is a static token-embedding table argot distilled from
[jina-embeddings-v2-base-code](https://huggingface.co/jinaai/jina-embeddings-v2-base-code) by Jina
AI (Apache-2.0), using the [model2vec](https://github.com/MinishLab/model2vec) technique (MIT). It
is a table lookup and an average, not a transformer — which is why it needs no C++ backend, no
accelerator and no download, and why embedding a repo takes seconds rather than tens of minutes.
The inference is argot's own Rust. Provenance and licenses: the repository
[`NOTICE`](https://github.com/get-tmonier/argot/blob/main/NOTICE). argot is not affiliated with
Jina AI.

## The mental model

> A regex catches what you can write down. A type checker catches what you can prove. **argot catches
> what your team has implicitly agreed on by repetition** — naming patterns, error-handling shapes,
> control-flow idioms, the difference between `response.raise_for_status()` and
> `if response.status_code >= 400: raise`.

It builds two distributions:

- **the repo distribution** — how tokens are used across *your* codebase's history, and
- **the generic baseline** — a broad open-source corpus baseline bundled with argot.

A hunk is suspicious when at least one of its tokens is far more likely under the generic baseline
than under your repo. High surprise means "this looks like generic open-source code, not code from
*here*."

## The engine: two phases

The pipeline splits into **fit** (run locally at setup, then only through a deliberate recommended
refresh) and **check** (run on every selected diff).

<figure class="diagram">
<svg viewBox="0 0 1080 384" role="img" aria-label="argot pipeline: a fit phase (extract, train, calibrate) producing the .argot artifacts (scorer config, semantic index, layering graph), and a check phase (diff hunk through typicality filter, import checker, BPE scorer, and the semantic and layering detectors) producing clean or flagged.">
  <defs>
    <marker id="ah" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--muted)"></path></marker>
  </defs>
  <text x="24" y="26" class="d-phase">FIT · local setup or deliberate refresh</text>
  <g class="d-node"><rect x="24" y="42" width="128" height="58" rx="11"></rect><text x="88" y="68" class="d-cmd">extract</text><text x="88" y="86" class="d-sub">git history</text></g>
  <g class="d-node"><rect x="196" y="42" width="128" height="58" rx="11"></rect><text x="260" y="68" class="d-cmd">train</text><text x="260" y="86" class="d-sub">two distributions</text></g>
  <g class="d-node"><rect x="368" y="42" width="140" height="58" rx="11"></rect><text x="438" y="68" class="d-cmd">calibrate</text><text x="438" y="86" class="d-sub">threshold t</text></g>
  <g class="d-node d-artifact"><rect x="556" y="42" width="300" height="58" rx="11"></rect><text x="706" y="67" class="d-file">.argot/</text><text x="706" y="85" class="d-sub">scorer config · semantic index · layering graph</text></g>
  <line class="d-link" x1="152" y1="71" x2="196" y2="71" marker-end="url(#ah)"></line>
  <line class="d-link" x1="324" y1="71" x2="368" y2="71" marker-end="url(#ah)"></line>
  <line class="d-link" x1="508" y1="71" x2="556" y2="71" marker-end="url(#ah)"></line>
  <line class="d-flow" x1="152" y1="71" x2="196" y2="71"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="324" y1="71" x2="368" y2="71"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.3s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="508" y1="71" x2="556" y2="71"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.6s" repeatCount="indefinite"></animate></line>
  <path class="d-thread" d="M620,100 L620,212" marker-end="url(#ah)"></path>
  <text x="630" y="158" class="d-thread-label">threshold t</text>
  <path class="d-thread" d="M820,100 C820,150 872,158 872,212" marker-end="url(#ah)"></path>
  <text x="882" y="158" class="d-thread-label">index + graph</text>
  <text x="24" y="196" class="d-phase">CHECK · every diff</text>
  <g class="d-node"><rect x="24" y="212" width="112" height="58" rx="11"></rect><text x="80" y="238" class="d-stage">diff hunk</text><text x="80" y="256" class="d-sub">changed code</text></g>
  <g class="d-node"><rect x="176" y="212" width="138" height="58" rx="11"></rect><text x="245" y="238" class="d-stage">typicality</text><text x="245" y="256" class="d-sub">skip data-dominant</text></g>
  <g class="d-node"><rect x="354" y="212" width="126" height="58" rx="11"></rect><text x="417" y="238" class="d-stage">imports</text><text x="417" y="256" class="d-sub">foreign import?</text></g>
  <g class="d-node d-accent"><rect x="520" y="212" width="150" height="58" rx="11"></rect><text x="595" y="238" class="d-stage">BPE + penalty</text><text x="595" y="256" class="d-sub">surprise vs threshold</text></g>
  <g class="d-node"><rect x="710" y="212" width="176" height="58" rx="11"></rect><text x="798" y="238" class="d-stage">semantic · layering</text><text x="798" y="256" class="d-sub">redundant · misplaced · layers</text></g>
  <g class="d-out d-ok"><rect x="936" y="210" width="108" height="26" rx="8"></rect><text x="990" y="227">✓ clean</text></g>
  <g class="d-out d-flag"><rect x="936" y="246" width="108" height="26" rx="8"></rect><text x="990" y="263">⚑ flagged</text></g>
  <line class="d-link" x1="136" y1="241" x2="176" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="314" y1="241" x2="354" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="480" y1="241" x2="520" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="670" y1="241" x2="710" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="886" y1="237" x2="936" y2="223" marker-end="url(#ah)"></line>
  <line class="d-link" x1="886" y1="245" x2="936" y2="259" marker-end="url(#ah)"></line>
  <line class="d-flow" x1="136" y1="241" x2="176" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="314" y1="241" x2="354" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.3s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="480" y1="241" x2="520" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.6s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="670" y1="241" x2="710" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.9s" repeatCount="indefinite"></animate></line>
</svg>
<figcaption>Run <code>extract → train → calibrate</code> once; the artifacts in <code>.argot/</code> feed every <code>check</code> — the calibrated threshold gates the voice rules, the index and graph power the semantic and layering rules, and the accepted-history replay gates the integrity rules.</figcaption>
</figure>

### Fit

1. **extract** — walks `git log`, slices each commit into hunks, and tokenizes every hunk and its
   surrounding context with a language-aware [tree-sitter](https://tree-sitter.github.io/tree-sitter/)
   tokenizer. Output: `.argot/dataset.jsonl`.
2. **train** — counts BPE tokens across the repo's non-test source files (the repo distribution) and
   loads the bundled generic baseline. Data-dominant files (locale tables, fixtures, generated code)
   are excluded so they don't pollute the distribution.
3. **calibrate** — samples representative top-level functions and classes from your repo, scores them,
   and sets the threshold to the maximum score over those "normal" hunks. Per-language repos get one
   threshold per language.

`argot fit` runs all three for you and writes `.argot/scorer-config.json`. It then builds the
**semantic index** — it embeds every function with the local code-embedding model and writes
`.argot/semantic-index.json`, the per-repo vector index the reinvention and placement checks query
at check time — and the **layering graph** (`.argot/layering.json`), the module-dependency graph
the architecture detector checks new imports against. (`scorer-config.json` is unchanged; each
artifact lives in its own file.) Turn the `semantic` rule group off and fit skips the embedding
work entirely — no embedding pass, no index.

### Check

For each changed hunk, argot runs a short pipeline:

1. **Typicality filter** — skip hunks that are structurally data-dominant (mostly literals) or live in
   a data-dominant file. The n-gram model would only see noise there.
2. **Import checker** — if a hunk imports a module that's foreign to the repo's own first-party import
   set, flag it immediately (rule `foreign-import`).
3. **BPE scorer** — compute the max-surprise score, adjusted by a small per-callee penalty (applied
   only when the hunk reaches into a module foreign to the repo), and flag the hunk if the adjusted
   score exceeds the calibrated threshold (rules `rare-tokens` and `unfamiliar-callee`).
4. **Semantic checks** — for each new function, argot embeds it and queries the index: is there
   already a near-identical function elsewhere (*reinvention*, rule `redundant`)? Do its nearest
   neighbours cluster in a different package (*placement*, rule `misplaced`)? Real repos hold real
   duplication and cross-cutting helpers, so both show you the nearest existing code and let you
   judge — and both are one config line to downgrade to `warn` or `off`.
5. **Architecture check** — the added lines' internal imports are resolved against the fit-time
   module-dependency graph; an edge that reverses an established layer direction or leaves a
   (near-)sink is flagged (rule `layering`, "crosses a module boundary").

The math for the base voice model, in one line:

$$
\text{surprise}(t) = \log P_\text{baseline}(t) - \log P_\text{repo}(t)
\qquad
\text{score}(\text{hunk}) = \max_{t \,\in\, \text{tokens}(\text{hunk})} \text{surprise}(t)
$$

A high score means at least one token is far more common in generic code than in this repo — a
reliable signal of foreign style. Comments and docstrings are blanked before scoring, so natural
language doesn't inflate the signal.

For the full scoring model — the call-receiver penalty, file clustering, and the per-corpus
auto-detect probe — see [The scoring model](/docs/the-scoring-model/).

For the boundaries that matter when interpreting a clean run — fit suitability,
masked or in-vocabulary changes, and the diff/net-range limits — see
[Limitations](/docs/limitations/).
