---
title: How it works
description: Two frequency tables, a max log-ratio, and a threshold calibrated on your own code.
group: Start
order: 2
---

argot is deliberately simple. There is **no neural network** at scoring time — the model is two
token-frequency distributions and a maximum log-likelihood ratio. That's the whole idea, and it's
why argot fits in seconds and scores in milliseconds, entirely on CPU.

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
<svg viewBox="0 0 920 384" role="img" aria-label="argot pipeline: a fit phase (extract, train, calibrate) producing a calibrated scorer config, and a check phase (diff hunk through typicality filter, import checker, and BPE scorer) producing clean or flagged.">
  <defs>
    <marker id="ah" viewBox="0 0 10 10" refX="8.5" refY="5" markerWidth="7" markerHeight="7" orient="auto-start-reverse"><path d="M0,0 L10,5 L0,10 z" fill="var(--muted)"></path></marker>
  </defs>
  <text x="24" y="26" class="d-phase">FIT · once per repo</text>
  <g class="d-node"><rect x="24" y="42" width="128" height="58" rx="11"></rect><text x="88" y="68" class="d-cmd">extract</text><text x="88" y="86" class="d-sub">git history</text></g>
  <g class="d-node"><rect x="196" y="42" width="128" height="58" rx="11"></rect><text x="260" y="68" class="d-cmd">train</text><text x="260" y="86" class="d-sub">two distributions</text></g>
  <g class="d-node"><rect x="368" y="42" width="140" height="58" rx="11"></rect><text x="438" y="68" class="d-cmd">calibrate</text><text x="438" y="86" class="d-sub">threshold t</text></g>
  <g class="d-node d-artifact"><rect x="556" y="42" width="200" height="58" rx="11"></rect><text x="656" y="67" class="d-file">scorer-config.json</text><text x="656" y="85" class="d-sub">the shipped model</text></g>
  <line class="d-link" x1="152" y1="71" x2="196" y2="71" marker-end="url(#ah)"></line>
  <line class="d-link" x1="324" y1="71" x2="368" y2="71" marker-end="url(#ah)"></line>
  <line class="d-link" x1="508" y1="71" x2="556" y2="71" marker-end="url(#ah)"></line>
  <line class="d-flow" x1="152" y1="71" x2="196" y2="71"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="324" y1="71" x2="368" y2="71"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.3s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="508" y1="71" x2="556" y2="71"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.6s" repeatCount="indefinite"></animate></line>
  <path class="d-thread" d="M656,100 L656,212" marker-end="url(#ah)"></path>
  <text x="666" y="158" class="d-thread-label">threshold t</text>
  <text x="24" y="196" class="d-phase">CHECK · every diff</text>
  <g class="d-node"><rect x="24" y="212" width="120" height="58" rx="11"></rect><text x="84" y="238" class="d-stage">diff hunk</text><text x="84" y="256" class="d-sub">changed code</text></g>
  <g class="d-node"><rect x="188" y="212" width="150" height="58" rx="11"></rect><text x="263" y="238" class="d-stage">typicality</text><text x="263" y="256" class="d-sub">skip data-dominant</text></g>
  <g class="d-node"><rect x="382" y="212" width="150" height="58" rx="11"></rect><text x="457" y="238" class="d-stage">imports</text><text x="457" y="256" class="d-sub">foreign import?</text></g>
  <g class="d-node d-accent"><rect x="576" y="212" width="160" height="58" rx="11"></rect><text x="656" y="238" class="d-stage">BPE + penalty</text><text x="656" y="256" class="d-sub">surprise vs threshold</text></g>
  <g class="d-out d-ok"><rect x="788" y="210" width="108" height="26" rx="8"></rect><text x="842" y="227">✓ clean</text></g>
  <g class="d-out d-flag"><rect x="788" y="246" width="108" height="26" rx="8"></rect><text x="842" y="263">⚑ flagged</text></g>
  <line class="d-link" x1="144" y1="241" x2="188" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="338" y1="241" x2="382" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="532" y1="241" x2="576" y2="241" marker-end="url(#ah)"></line>
  <line class="d-link" x1="736" y1="237" x2="788" y2="223" marker-end="url(#ah)"></line>
  <line class="d-link" x1="736" y1="245" x2="788" y2="259" marker-end="url(#ah)"></line>
  <line class="d-flow" x1="144" y1="241" x2="188" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="338" y1="241" x2="382" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.3s" repeatCount="indefinite"></animate></line>
  <line class="d-flow" x1="532" y1="241" x2="576" y2="241"><animate attributeName="stroke-dashoffset" from="23" to="0" dur="1.1s" begin="0.6s" repeatCount="indefinite"></animate></line>
</svg>
<figcaption>Run <code>extract → train → calibrate</code> once; the calibrated threshold feeds every <code>check</code>.</figcaption>
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

`argot fit` runs all three for you and writes `.argot/scorer-config.json`.

### Check

For each changed hunk, argot runs a short pipeline:

1. **Typicality filter** — skip hunks that are structurally data-dominant (mostly literals) or live in
   a data-dominant file. The n-gram model would only see noise there.
2. **Import checker** — if a hunk imports a module that's foreign to the repo's own first-party import
   set, flag it immediately (`reason: import`).
3. **BPE scorer** — compute the max-surprise score, adjusted by a small per-callee penalty (applied
   only when the hunk reaches into a module foreign to the repo), and flag the hunk if the adjusted
   score exceeds the calibrated threshold.

The math, in one line:

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
