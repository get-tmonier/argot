---
title: Audit accepted history
description: Review what Argot would have surfaced before changes were merged, without changing your working tree.
group: Use
order: 1
---

`argot audit` is the evidence-first starting point. It fits a voice at a historical base in a
temporary worktree, scores the surviving base-to-head change, and attributes findings only when
there are explicit commit markers. Your checkout and its `.argot/` directory are not changed.

```bash
argot audit
argot audit --commits 200
argot audit --since 6m
argot audit --format markdown
```

## Scope and output

The default window is the last 50 commits on the first-parent line, capped at 1,000 commits. If
today’s configured source scope did not exist that far back, Audit shrinks the window and says so.
It scores the net surviving diff from the fitted base to HEAD rather than replaying every interim
state. A finding therefore means “would have prompted review before merge,” not “this commit is a
bug.”

Audit can render `terminal`, `json`, `markdown`, or `html`. Successful audits always exit 0;
failure to set up or inspect the history exits 2. It runs entirely offline — the embedding model
the semantic rules use ships inside the binary.

## Use the result as a habit

Read the finding evidence and the rule groups, then make a deliberate next choice:

- Set up a reviewed voice with [Init and Fit](/docs/init-and-fit/).
- Score the change you are currently making with [Check](/docs/check/).
- Wire the same review signal into [CI or pre-commit](/docs/ci/) if your team wants it at a
  configured workflow or commit event.

Audit is informational. It is useful even when it is quiet: that says the selected history yielded
no current findings under the chosen configuration, not that every historical decision was correct.
