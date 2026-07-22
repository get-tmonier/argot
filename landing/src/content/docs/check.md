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
argot mute <hash> --reason "intentional parallel implementation"
```

See [Reading the output](/docs/reading-the-output/) for the field reference and
[Configure](/docs/configure/) for severities, inline suppressions, and locked rules.
