---
title: Output reference
description: A compatibility route for check output details; use Check a changeset for the canonical guide.
group: Understand
order: 7
---

This legacy reference route is retained for bookmarks and incoming links. [Check a changeset](/docs/check/)
is the canonical guide to selecting a diff, separating severity from confidence, and reading exit
codes and evidence.

For rule configuration and machine formats, see [Configure](/docs/configure/). When a finding is
deliberate, use one explicit lifecycle: inspect its evidence, either change the code or run
`argot mute <hash> --reason "…"` for that one hit (or `argot mute --path <glob> --reason "…"` when
the decision covers a tree), commit that reason for review, then periodically run
`argot list-mutes` and `argot review-mutes --prune`. Inline and path suppressions are documented
there too. Locked rules refuse every suppression surface; they must be resolved through the
reviewed rule policy, never silently softened. A finding is review evidence; deciding how a team
responds to it remains a human policy choice.
