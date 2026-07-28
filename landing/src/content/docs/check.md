---
title: Check a changeset
description: Select the changes to score, interpret severity and confidence separately, and keep the human decision explicit.
group: Use
order: 2
---

`argot check` scores a selected changeset against a fitted repository voice. It is the full local
check: skills, MCP, hooks, and CI do not silently broaden a selected changeset into an automatic
acceptance-time check.

```bash
argot check                         # all uncommitted modified, staged, and untracked changes
argot check --staged                # staged changes only
argot check --unstaged              # unstaged changes only
argot check HEAD~5                  # current state compared with a ref
argot check HEAD~5..HEAD            # commits in a range
argot check --commit abc1234        # one commit
argot check --format json           # stable machine-readable document
```

`--only` and `--exclude` narrow the selected paths; `--exclude` wins when both match. Machine
formats are `json`, `sarif`, and `github`; human output is the default. The JSON schema is versioned
with the release, so consumers should read the schema version rather than infer one from prose.

## Read the result

Exit 0 means no error-severity finding was reported. Exit 1 means at least one error-severity
finding needs review. Exit 2 means setup or command usage failed. `--error-on-warnings` makes
warning-severity findings exit non-zero for a deliberately strict invocation.

Severity controls the exit behavior. Confidence (`unusual`, `suspicious`, or `foreign`) describes
the strength of the evidence for display and filtering; it does not decide whether the command
fails. A clean run means none of the configured checks fired, not that every choice matches the
repository’s conventions.

For every hit, read its rule and evidence. In particular, compare a `redundant` finding with the
named existing function, and treat an intentional exception as a human decision. A durable mute
requires a meaningful reason:

```bash
argot mute <hash> --reason "intentional parallel implementation"     # this hit only
argot mute --path 'src/legacy/**' --rule redundant --reason "…"      # a standing rule
```

A hash mute covers **that hit alone** — the same finding in a sibling file has its own hash. Reach
for `--path` when the decision covers a tree, or you will be committing one mute per file.

## Adopting on a codebase that already has findings

Fitting a repository with history means `check` will surface findings that predate your decision to
use Argot. Choose a starting line deliberately:

```bash
argot check --ignore-existing     # working tree only
```

This writes an inline ignore comment above every finding that exists today, so only new code is
judged from here. It is the right move for a mature codebase adopting Argot without a cleanup
project first. The alternative is to fix or mute what is there now, which suits a smaller or
younger repository.

Those ignores are a **snapshot, not a verdict**. Re-score them periodically — Argot can report which
suppressions no longer fire — or the baseline quietly becomes permanent and hides regressions in
the code it covered.

See [Configure](/docs/configure/) for severities, machine-format fields, inline suppressions, and
locked rules.
