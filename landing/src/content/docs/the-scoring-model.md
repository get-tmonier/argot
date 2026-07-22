---
title: The scoring model
description: The BPE log-ratio, the call-receiver penalty, file clustering, per-corpus auto-detect, the semantic layer, the architecture graph, and the integrity detector.
group: Reference
order: 10
---

This page is the deep end — the full scoring model. For the plain-English version, start with
[How it works](/docs/how-it-works/).

The whole score, in one line — a BPE surprise term plus a call-receiver penalty:

$$
\text{score}(\text{hunk})
\;=\;
\underbrace{\max_{t \,\in\, \text{tokens}(\text{hunk})}
\log \frac{P_\text{baseline}(t)}{P_\text{repo}(t)}}_{\text{BPE surprise}}
\;+\;
\underbrace{\min\!\Big(\textstyle\sum_{c} w(c),\; \text{cap}\Big)}_{\text{call-receiver penalty}}
$$

## BPE surprise

argot tokenizes each hunk with the [UnixCoder](https://huggingface.co/microsoft/unixcoder-base) BPE
tokenizer — only the *vocabulary* is used, not the neural network. It then compares two smoothed
distributions, the repo ($A$) and the generic baseline ($B$):

$$
P_A(t) = \frac{\text{count}_A(t)}{\text{total}_A} + \varepsilon
\qquad
P_B(t) = \frac{\text{count}_B(t)}{\text{total}_B} + \varepsilon
$$

$$
\text{surprise}(t) = \log P_B(t) - \log P_A(t)
\qquad
\text{score}(\text{hunk}) = \max_{t \,\in\, \text{tokens}(\text{hunk})} \text{surprise}(t)
$$

A high score means at least one token is far more common in generic open-source code than in this
repo. Prose lines (comments, docstrings) are blanked before scoring so natural language doesn't
inflate the signal.

## The call-receiver penalty

The raw BPE score is adjusted by a small per-callee penalty over the hunk's distinct dotted callees
$c$:

$$
\text{adjusted} = \text{bpe} + \min\!\Big(\sum_{c} w(c),\; \text{cap}\Big)
$$

The weight $w(c)$ depends on how the callee relates to the repo and to its file's cluster:

$$
w(c) =
\begin{cases}
\alpha + r & c \text{ unattested, but its \textbf{root} is known} \;(\texttt{req.send}\text{ vs }\texttt{req.get}) \\[2pt]
\alpha & c \text{ and its root are both unattested} \\[2pt]
\beta & c \text{ is attested, but absent from its file's \textbf{cluster}} \\[2pt]
\beta & c \text{ is \textbf{cluster-rare}, in } \le \tau \text{ cluster files (auto-detected)}
\end{cases}
$$

Shipping config: $\alpha = 2.0,\; r = 2.0,\; \beta = 5.0,\; \text{cap} = 5.0,\; K = 8,\; \tau = 2$.

The cluster-conditional term targets context-dependent breaks — a known callee showing up in a file
kind it never belongs to (think `Math.random` inside a deterministic faker provider, even though
`Math.random` exists elsewhere in the repo's tests).

The whole penalty is **gated by foreign reach**: it applies only when the hunk's file reaches into a
module foreign to the repo (a foreign namespace-qualified or bare-foreign callee somewhere in the
file). In files that stay entirely within the repo's own vocabulary the penalty is suppressed, so an
unattested-callee soft signal can't tip in-voice code over the line — the gate that cut
`unfamiliar-callee` false alarms in the #92 pass.

## File clustering

At fit time, every non-data-dominant source file is reduced to its **callee bag** (the set of dotted
call expressions, via tree-sitter), encoded as a 128-perm MinHash signature, and clustered into
`K = 8` groups with KMeans. Each cluster's attested set is the union of its files' callees. At score
time a hunk's file is mapped to its cluster (or the Jaccard-nearest one if it's new). The clustering
is derived purely from callee statistics — no path patterns, no per-corpus heuristics.

## Calibration and auto-detect

Calibration samples up to 500 representative top-level functions and classes, scores them, and sets
the BPE threshold to the **max score over those normal hunks**. Because calibration hunks come from
files already in the corpus, their callees are subsets of their cluster's attested set — so
calibration scores are invariant under both $\alpha$ and $\beta$. The threshold is set against raw BPE; the
penalty exists only to push genuinely anomalous hunks past it at score time.

Calibration also runs a **per-corpus auto-detect probe**: it loads ~1000 diff hunks and measures the
fire rate of the cluster-rare rule. If the rule fires on < 5% of hunks it's informative and stays
enabled; otherwise it's disabled to avoid Zipf-tail false-positive floods. This is what keeps the
same config honest across very different repos.

## Rule attribution

A hunk is flagged by the base voice model if the import checker fires **or** the adjusted BPE score
exceeds the threshold. The finding carries a stable rule name: `foreign-import` for a foreign
import, `unfamiliar-callee` when the penalty pushed a below-threshold BPE over the line, and
`rare-tokens` when raw BPE already crossed it. The semantic layer adds two more rules — `redundant`
and `misplaced` — the architecture graph adds `layering`, and the integrity detector adds
`test-deleted`, `test-disabled`, and `test-weakened` (all below). Scores and rule names are always
included in the output, and every rule's severity is configurable — see
[Configure](/docs/configure/#rules--rule-severities).

## The semantic layer

Separate from the BPE model above, argot keeps a per-repo **code-embedding index**. At fit it embeds
every function with a small local model (`jina-embeddings-v2-base-code`, Q4 GGUF, ~100 MB, statically
linked via llama.cpp — CPU-first, Metal-accelerated on macOS; fetched once to a local cache on first
use, ~250 MB peak RAM while a check embeds) and stores the vectors in `.argot/semantic-index.json`.
It turns a function into a vector — no prompt, no generation, nothing leaves your machine; offline,
the layer no-ops and the base guardrail still runs.

At check, each new function is embedded and matched against the index:

- **`redundant`** — the function's nearest cross-file neighbour is a near-duplicate above a similarity
  margin. Evidence: `↳ duplicates <symbol> (path:line) — similarity 0.86`.
- **`misplaced`** — the function's nearest semantic neighbours concentrate in a different package or
  area than the one it was filed under. Evidence: `↳ looks like <area> code filed under <actual-area>`.

Both findings are pinned to the `unusual` **confidence** tier (the evidence is a similarity lookup)
and carry severity `error` by default — they fail the check like any other rule, and they're one
`[rules]` line to downgrade (`redundant = "warn"`, or `semantic = "off"` for the whole group; with
the group off, fit and check skip the model download and the index entirely). Real repos hold real
duplication and cross-cutting code, so argot shows the nearest existing function and lets you
judge. This channel is separate from the foreign-catch metric — it does not change the base model's
catch or false-alarm numbers.

## The architecture graph

The fourth detector is pure graph analysis — no model, no scoring math. At fit, argot resolves
every internal import into a module-dependency graph, derives the repo's layer directions from it,
and persists the result as `.argot/layering.json`. At check, the *added* lines' internal imports
are resolved against that graph; an edge that reverses an established layer direction (a
transitive reversal counts), closes a cycle, or leaves a (near-)sink module is flagged under rule
**`layering`** ("crosses a module boundary"), pinned to the `unusual` confidence tier, severity
`error` by default.

Benchmarked on 25 corpora across all 12 supported languages: **264/272 (97.1%)** planted violations
caught, **0/148** false positives on control edits, worst-case over-fire 2.7%. The check-time
import resolver covers Python in v1.

## The integrity detector

The fifth detector is event-based, not a scored threshold — no model, no BPE, no similarity
margin. At fit, argot diffs each accepted commit's test inventory (per-language: test cases,
assertion sites, skip/disable markers, expected literals) against its parent, reduces the diffs to
gaming events, and replays 150 accepted commits to learn which events this repo's own history
trips often enough to be noisy — those are disabled for that repo, and the result is persisted as
`.argot/integrity.json`. At check, the same diff-to-events reduction runs on the changeset; an
event fires only when the changeset also modifies production source (a tests-only commit is suite
curation, not gaming) and its per-repo gate is open. Findings are flagged under **`test-deleted`**,
**`test-disabled`**, and **`test-weakened`**, all pinned to the `suspicious` confidence tier; the
first two default to severity `error`, `test-weakened` defaults to `warn`.

The canonical result manifest records **155/164 (94.5%)** authored gaming
fixtures caught across 23 corpora / 12 languages, **0/106** legitimate-refactor
controls fired, and **45/3,602 (1.25%)** accepted-history test-touching commits
flagged at gating severity. These are separate detector-specific measures, not a
product-wide accuracy rate. Full numbers:
[`docs/research/evidence/test-integrity-capstone.md`](https://github.com/get-tmonier/argot/blob/main/docs/research/evidence/test-integrity-capstone.md).
