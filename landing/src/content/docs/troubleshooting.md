---
title: Troubleshooting
description: Diagnose shallow history, unsupported files, fit health, offline models, integrations, and removal safely.
group: Help
order: 15
---

Use the smallest safe command that explains the state. Argot findings are
review prompts, not proof that code is wrong.

## “Not recommended” or too little history

**Symptom:** `argot inspect` says the fit is not recommended, or a fresh/shallow
clone has little useful history. **Cause:** the voice model has too little
eligible repository history to calibrate well. **Safe command:** run
`argot inspect`, then use a full clone and `argot init --suggest` to review what
belongs in the corpus. **Escalate:** use normal review until a suitable fit is
available; see [Health & freshness](/docs/health-and-freshness/).

## A file is not assessed

**Symptom:** a changed file produces no finding. **Cause:** it may be unsupported,
excluded, generated/data-dominant, or outside the chosen check range. **Safe
command:** run `argot rules --format json` to confirm enabled rules and inspect
`argot.toml` exclusions; use the exact intended range with `argot check`.
**Escalate:** use a [custom rule](/docs/custom-rules/) only for a deliberate
repository convention; do not broaden exclusions merely to quiet output.

## The semantic model is unavailable or offline

**Symptom:** `redundant` and `misplaced` are skipped with a diagnostic. **Cause:**
the local model is not cached, or `ARGOT_OFFLINE=1` forbids downloading it.
**Safe command:** either keep offline mode and accept the explicit degradation,
or provide a verified local model with `ARGOT_SEMANTIC_MODEL=<path>`. **Escalate:**
remove the offline restriction only when network access is permitted; the base
voice checks continue without the model.

## The fit is stale

**Symptom:** `status` or `check` reports drift, stale artifacts, or a config
change. **Cause:** accepted history or corpus configuration changed after the
fit. **Safe command:** run `argot status`, then `argot fit` when a refresh is
needed. **Escalate:** review generated/vendor/data exclusions before refitting;
see [Health & freshness](/docs/health-and-freshness/).

## Action, plugin, or pre-commit integration behaves unexpectedly

**Symptom:** a hosted check lacks history, semantic findings, or an expected
prompt. **Cause:** CI may use a shallow checkout, disable semantic downloads,
or apply its own configured rule policy. **Safe command:** compare the workflow
with [CI and pre-commit](/docs/ci/) and run the equivalent local command.
For the Claude plugin, use [Claude Code](/docs/plugin/). **Escalate:** attach
the command, range, `argot rules --format json`, and non-sensitive stderr to an
issue; never paste repository code or credentials.

## Update or uninstall

**Symptom:** an installed binary is outdated or must be removed. **Cause:** the
release/update path is separate from analysis. **Safe command:** run
`argot update` for the current release, or `argot uninstall` to see its complete
inventory before confirming removal. **Escalate:** use the release notes or open
an issue if the inventory is unexpected; do not manually delete tracked
`argot.toml` or authored `.argot/rules/` content.
