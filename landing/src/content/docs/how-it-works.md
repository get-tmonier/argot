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

Argot is not a service that silently learns from every pull request. A team periodically
reviews and commits a small **fit snapshot**: the learned repository voice and the indexes
that make checks reproducible. Think of it like updating a lockfile or a dependency: a small,
deliberate maintenance commit, not infrastructure that has to run on every PR.

<figure class="diagram lifecycle-diagram">
<svg viewBox="0 0 1080 538" role="img" aria-label="Argot's lifecycle: initial local fit and commit, optional custom rules, local MCP and pre-commit help, advisory pull-request CI using the base snapshot, then a local refresh after accepted changes.">
  <defs>
    <marker id="lifecycle-arrow" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto"><path d="M0,0 L10,5 L0,10 z" fill="var(--muted)"></path></marker>
  </defs>
  <text x="26" y="29" class="d-phase">1 · SET UP ON AN ACCEPTED BRANCH</text>
  <g class="d-node"><rect x="26" y="47" width="170" height="72" rx="11"></rect><text x="111" y="76" class="d-stage">audit the history</text><text x="111" y="96" class="d-sub">choose exclusions · scope</text></g>
  <g class="d-node d-accent"><rect x="242" y="47" width="170" height="72" rx="11"></rect><text x="327" y="75" class="d-cmd">argot init</text><text x="327" y="96" class="d-sub">fit voice · index · graph</text></g>
  <g class="d-node d-optional"><rect x="458" y="47" width="178" height="72" rx="11"></rect><text x="547" y="74" class="d-stage">optional: custom rules</text><text x="547" y="95" class="d-sub">only high-value conventions</text><text x="547" y="109" class="d-sub">fixtures prove no false positives</text></g>
  <g class="d-node d-artifact"><rect x="682" y="47" width="358" height="72" rx="11"></rect><text x="861" y="73" class="d-file">review + commit</text><text x="861" y="95" class="d-sub">argot.toml · .argot/ snapshot · optional rules</text></g>
  <line class="d-link" x1="196" y1="83" x2="242" y2="83" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-link" x1="412" y1="83" x2="458" y2="83" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-link" x1="636" y1="83" x2="682" y2="83" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-flow d-flow-1" x1="196" y1="83" x2="242" y2="83"></line>
  <line class="d-flow d-flow-2" x1="412" y1="83" x2="458" y2="83"></line>
  <line class="d-flow d-flow-3" x1="636" y1="83" x2="682" y2="83"></line>
  <text x="26" y="183" class="d-phase">2 · WRITE WITH CONTEXT, THEN OPEN A PR</text>
  <g class="d-node"><rect x="26" y="201" width="202" height="72" rx="11"></rect><text x="127" y="229" class="d-stage">before writing</text><text x="127" y="249" class="d-sub">MCP shares familiar APIs</text><text x="127" y="263" class="d-sub">and migrations with the agent</text></g>
  <g class="d-node"><rect x="274" y="201" width="186" height="72" rx="11"></rect><text x="367" y="229" class="d-stage">while developing</text><text x="367" y="249" class="d-sub">optional pre-commit check</text><text x="367" y="263" class="d-sub">surfaces, never decides</text></g>
  <g class="d-node d-accent"><rect x="506" y="201" width="156" height="72" rx="11"></rect><text x="584" y="229" class="d-stage">pull request</text><text x="584" y="249" class="d-sub">code under review</text></g>
  <g class="d-node d-ci"><rect x="708" y="201" width="332" height="72" rx="11"></rect><text x="874" y="226" class="d-stage">advisory CI · ArgoScore</text><text x="874" y="247" class="d-sub">reads the committed base snapshot</text><text x="874" y="262" class="d-sub">findings explain; default is not a merge gate</text></g>
  <line class="d-link" x1="228" y1="237" x2="274" y2="237" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-link" x1="460" y1="237" x2="506" y2="237" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-link" x1="662" y1="237" x2="708" y2="237" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-flow d-flow-1" x1="228" y1="237" x2="274" y2="237"></line>
  <line class="d-flow d-flow-2" x1="460" y1="237" x2="506" y2="237"></line>
  <line class="d-flow d-flow-3" x1="662" y1="237" x2="708" y2="237"></line>
  <path class="d-thread" d="M862,119 L862,201" marker-end="url(#lifecycle-arrow)"></path>
  <text x="875" y="162" class="d-thread-label">same approved baseline</text>
  <text x="26" y="338" class="d-phase">3 · REFRESH DELIBERATELY, NOT IN CI</text>
  <g class="d-node d-merge"><rect x="26" y="356" width="202" height="72" rx="11"></rect><text x="127" y="384" class="d-stage">accepted code merges</text><text x="127" y="404" class="d-sub">the repository keeps moving</text></g>
  <g class="d-node d-warn"><rect x="294" y="356" width="252" height="72" rx="11"></rect><text x="420" y="383" class="d-stage">status / CI says “refresh due”</text><text x="420" y="404" class="d-sub">after ~10 accepted source commits*</text></g>
  <g class="d-node d-accent"><rect x="612" y="356" width="188" height="72" rx="11"></rect><text x="706" y="384" class="d-cmd">argot fit</text><text x="706" y="404" class="d-sub">locally on the accepted branch</text></g>
  <g class="d-node d-artifact"><rect x="846" y="356" width="194" height="72" rx="11"></rect><text x="943" y="383" class="d-stage">review + recommit</text><text x="943" y="404" class="d-sub">the refreshed snapshot</text><text x="943" y="418" class="d-sub">next PRs read it</text></g>
  <line class="d-link" x1="228" y1="392" x2="294" y2="392" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-link" x1="546" y1="392" x2="612" y2="392" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-link" x1="800" y1="392" x2="846" y2="392" marker-end="url(#lifecycle-arrow)"></line>
  <line class="d-flow d-flow-1" x1="228" y1="392" x2="294" y2="392"></line>
  <line class="d-flow d-flow-2" x1="546" y1="392" x2="612" y2="392"></line>
  <line class="d-flow d-flow-3" x1="800" y1="392" x2="846" y2="392"></line>
  <text x="580" y="514" class="d-thread-label">* configurable freshness threshold · CI never runs the fit</text>
</svg>
<figcaption>One deliberate snapshot gives local tools and CI the same learned baseline. Custom rules are opt-in source code; CI is advisory by default; a freshness reminder asks for a small local fit-and-commit update rather than doing work behind your back.</figcaption>
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

| Committed part | Why it matters |
| --- | --- |
| Voice snapshot (`scorer-config.json` + baseline) | Contains the learned vocabulary and calibrated thresholds. Without it, another machine cannot tell whether an API or idiom is foreign without fitting again. |
| Semantic index (`semantic-index.json`) | Contains the local map from functions to their nearest existing neighbours. Without it, `redundant` and `misplaced` cannot make the same “you already have this” comparison. |
| Layering, integrity, health, manifest | Preserve the learned dependency/test signals and prove the snapshot matches the configuration and binary model; they also make a stale snapshot visible. |

Committing them is therefore the lightweight alternative to a CI fit: every developer, agent, and
PR starts from the **same reviewed baseline**. A pull request cannot replace that baseline with its
own code, and CI remains a fast reader instead of a second training system. The snapshot changes
only when a human deliberately runs `argot fit`, reviews the diff, and commits the update.

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
