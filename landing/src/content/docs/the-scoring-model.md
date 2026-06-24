---
title: The scoring model
description: The BPE log-ratio, the call-receiver penalty, file clustering, and per-corpus auto-detect.
group: Reference
order: 6
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

## Reason attribution

A hunk is flagged if the import checker fires (foreign import) **or** the adjusted BPE score exceeds
the threshold. The reason is `call_receiver` when the penalty pushed a below-threshold BPE over the
line, and `bpe` when raw BPE already crossed it. Scores and reasons are always included in the output.
