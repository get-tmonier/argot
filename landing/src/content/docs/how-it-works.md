---
title: How it works
description: Four detectors — a statistical voice model, two embedding-based checks (reinvention, placement), and a module-dependency architecture graph — all learned from your git history.
group: Start
order: 3
---

argot has **four detectors**, all learned entirely from your git history. Each one emits findings
under a named **rule** (`argot rules` lists them all), and every rule's severity is yours to
configure — see [Configure](/docs/configure/#rules--rule-severities).

The **statistical voice model** is deliberately simple: **no neural network**, just two
token-frequency distributions and a maximum log-likelihood ratio. That's what catches *foreign*
patterns — a dependency or API the repo has never used (rules `foreign-import`,
`unfamiliar-callee`, `rare-tokens`) — and it's why the statistical pass fits in seconds and scores in milliseconds
on CPU.

The **reinvention** and **placement** detectors share the one neural component: a per-repo
code-embedding index, built at fit with a small local model (`jina-code`, ~100 MB, statically
linked via llama.cpp — CPU-first, Metal-accelerated on macOS, fetched once to a local cache on
first use). Reinvention flags a new function that duplicates one the repo already has (rule
`redundant`); placement flags a function filed in the wrong module area (rule `misplaced`). No
cloud, no GPU, no text generation; turn a function into a vector, look up its neighbours. Offline,
the download is skipped with a printed note — never silently — and the other detectors still run.

The **architecture detector** builds a module-dependency graph of your repo at fit and flags an
added internal import that reverses the repo's established layer direction (rule `layering`). Pure
graph analysis — no model, no network.

The embedding model is [jina-embeddings-v2-base-code](https://huggingface.co/jinaai/jina-embeddings-v2-base-code)
by Jina AI (Apache-2.0), run via [llama.cpp](https://github.com/ggml-org/llama.cpp) (MIT). argot
is not affiliated with Jina AI.

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

## Two phases

The pipeline splits into **fit** (run once per repo, and after major refactors) and **check** (run on
every diff).

<figure class="diagram">
<svg viewBox="0 0 1080 384" role="img" aria-label="argot pipeline: a fit phase (extract, train, calibrate) producing the .argot artifacts (scorer config, semantic index, layering graph), and a check phase (diff hunk through typicality filter, import checker, BPE scorer, and the semantic and layering detectors) producing clean or flagged.">
  <defs>
    <marker id="ah" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--muted)"></path></marker>
  </defs>
  <text x="24" y="26" class="d-phase">FIT · once per repo</text>
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
<figcaption>Run <code>extract → train → calibrate</code> once; the artifacts in <code>.argot/</code> feed every <code>check</code> — the calibrated threshold gates the voice rules, the index and graph power the semantic and layering rules.</figcaption>
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
work entirely — no model download, no index.

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
