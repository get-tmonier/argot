---
name: argot-refresh
description: Refresh Argot's committed fit snapshot safely — first diagnose why maintenance is recommended, re-audit corpus scope and structural path changes, review stale mutes and policy entries with the user, then fit locally, verify, and prepare the reviewed `.argot/` update. Use when `argot status`, `argot check`, MCP, or CI reports `recommended`, `strongly_recommended`, `config_changed`, or `review_scope_then_fit`; when the user asks to "refresh Argot", "refit Argot", "update the semantic index/voice snapshot", or to clean up Argot exclusions and mutes after a repository reorganization.
---

# Argot refresh

Treat a refresh like dependency maintenance, not a blind rebuild. The repository
may have changed enough that its corpus scope, paths, and suppressions need
review before the next fit freezes them into the shared snapshot.

Keep three concerns separate:

- `[exclude]` and `[detect]` shape what the fit learns. Changing them requires a
  new fit.
- `[rules]`, `[[migration]]`, and mutes shape check policy. They do not train the
  model, but stale policy can hide useful findings and belongs in the same
  maintenance review.
- `.argot/` is the generated, committed fit snapshot. CI only reads the base
  branch copy; it never fits or certifies a snapshot supplied by a PR.

## Non-negotiable contract

- Work on the accepted/default branch with a clean source tree. If either is
  false, show the evidence and stop before fitting.
- If the repository has never been fitted, stop and hand off to
  **argot-setup**. Refresh maintains a reviewed baseline; it does not replace
  first-time corpus selection and setup verification.
- Never fit in CI, in the background, or merely because a commit count or age
  is high.
- Run every audit below read-only first. Do not edit `argot.toml`, prune a mute,
  fit, stage, or commit while still assembling the proposal.
- Ask for one explicit confirmation covering the proposed scope and mute
  changes. A user may accept some rows and reject others.
- Never remove an exclusion or standing mute just because it currently matches
  nothing; it may be a deliberate future guardrail. State the uncertainty.
- Never claim a hash mute on a still-present file is stale. Argot cannot recover
  its original hunk from the hash.

## 0 · Preflight

Run:

```sh
argot --version
git status --short
git branch --show-current
argot status --format json
```

If Argot is missing, stop and point to
<https://argot.tmonier.com/docs/getting-started/>. Read these status fields:

- `snapshot.complete` and `snapshot.committed`
- `refresh.compatibility`
- `refresh.recommendation`, `score`, and `reasons`
- `refresh.next_action`
- `refresh.fit_sha` and `accepted_sha`

Confirm that the current branch is the configured accepted/default branch. If
`accepted_sha` is absent because compatibility was rejected before history was
measured, use the clean accepted branch's `git rev-parse HEAD` as the audit end
point. Never substitute a feature-branch HEAD.

Route the work:

| `next_action` | Meaning | Route |
|---|---|---|
| `none` | No material accepted drift | Explain that no fit is due; offer the read-only maintenance audit only if the user still wants it. |
| `monitor` | Early drift (`watch`) | Do not nag or fit. Offer the read-only audit and stop unless the user asks to continue. |
| `fit` | Material content/function drift | Audit scope and mutes, then fit after confirmation. |
| `review_scope_then_fit` | New/moved areas, a language surface, layout drift, or fit-relevant config changed | Treat the scope audit as required before fitting. |
| `inspect_history` | The clone cannot compare `fit_sha` with accepted history | Repair/fetch history or move to a full clone; do not guess freshness. |

For `profile_missing` or `lineage_diverged`, explain the compatibility problem
and continue only on the accepted branch.

## 1 · Re-audit corpus scope without fitting

Run both views and keep their output:

```sh
argot init --suggest --format json
argot inspect --corpus
```

Then inspect `argot.toml`, the repository tree, and the accepted tree delta from
`refresh.fit_sha` to the accepted SHA resolved in preflight. Start with the
scopes named by `refresh.reasons[]`; do not rescan unrelated code blindly.

Build one proposal list from four checks:

1. **New exclusions.** Use `init --suggest` evidence for generated,
   data-dominant, vendored, or not-authored-here directories. Record path,
   supported files, source lines, included real files, edit ratio, and reason.
   Also inspect new generated clients, transpiled output, committed snapshots,
   peripheral packages, and vendor drops that the statistical scan cannot know
   by intent.
2. **Existing exclusions.** Resolve every `[exclude].paths` pattern against the
   current tracked tree. Flag paths that disappeared, were renamed, became too
   broad, or now omit a replacement directory. Do not propose removal without
   reading its comment and Git history.
3. **Changed architecture.** For `new_area_surface`, `new_language_surface`, or
   `layout_turnover`, inspect the named area and its move/rename history. Decide
   whether it is authored voice, a separate monorepo slice, check-only code, or
   material that should be excluded. A new package is not automatically noise.
4. **Corpus sanity.** Compare `argot inspect --corpus` with the intended primary
   source tree. Look for duplicated generated/compiled source, missing primary
   packages, and languages with too few real files to learn.

Every proposed config edit must carry a one-sentence reason and a measurable
effect: files/lines added to or removed from the corpus. Prefer no edit over a
weakly justified one.

## 2 · Review mutes and policy separately

Run:

```sh
argot list-mutes
argot review-mutes
```

Classify findings without changing anything:

- **Safe dead hash mute:** `review-mutes` says its concrete file is gone. It can
  be pruned after confirmation.
- **Expired entry:** show its expiry and reason; propose removal, renewal, or
  replacement, but do not choose for the user.
- **Hash mute, file present:** keep by default. The original diff hunk is not
  recoverable from the hash, so automatic stale detection would be unsound.
- **Standing path/glob mute:** list what it matches now. Review whether the
  exception still describes a live architectural boundary or was tied to an
  old path. A zero-match glob is evidence to discuss, not proof of rot.
- **Inline suppression:** name the file and rule. Inspect only when the
  structural changes touched that area; never rewrite source merely to tidy a
  maintenance pass.
- **Rule severity or migration:** surface policies whose path/rationale no
  longer exists. Remember that changing them does not itself require a fit.

## 3 · Ask once, with the complete proposal

Present a compact table before any write:

| Decision | Proposed change | Evidence | Corpus/check effect |
|---|---|---|---|
| scope | add/remove/update one path | files, lines, history, structural reason | what enters/leaves the fit |
| mute | prune/keep/review one entry | gone file, expiry, current matches, reason | what becomes visible again |
| policy | update one rule/migration entry | obsolete path or rationale | check behavior only |

Ask the user to confirm the selected rows once. Preserve every rejected row and
record that no change was made. If there are no defensible edits, say so and do
not manufacture maintenance work.

## 4 · Apply only approved maintenance

- Edit `argot.toml` minimally. Keep a trailing reason comment on every custom
  exclusion.
- For approved dead hash mutes, run `argot review-mutes --prune` only after
  verifying its read-only report still names the same dead set. It prunes all
  safely dead hash mutes, not an arbitrary subset.
- Remove or renew expired/path mutes manually only when explicitly approved.
- Re-run `argot init --suggest --format json` after scope edits. An accepted
  candidate must disappear; rejected candidates may remain.
- Re-run `argot inspect --corpus` and report the before/after file count plus
  the important packages/languages added or removed.

If only mutes or rule policy changed and status did not recommend a fit, stop:
there is no model work to do. Otherwise continue.

## 5 · Fit deliberately

State the expected cost before starting when the corpus is large. Then run:

```sh
argot fit
```

Do not use `argot init` for routine maintenance: `fit` refreshes artifacts
without scaffolding repository configuration. Never hide warnings about a dirty
tree, feature branch, unlearnable language, or suspicious corpus composition.

## 6 · Verify the refreshed snapshot

Run:

```sh
argot status --format json
argot inspect --format json
git status --short
git diff -- argot.toml .argot/
```

Require:

- `snapshot.complete: true`
- `refresh.compatibility: "ready"`
- `refresh.recommendation: "fresh"`
- `refresh.next_action: "none"`
- an inspect verdict that is not `not_recommended`
- no unexpected source changes

Show the exact changed artifacts and total snapshot size. Explain any detector
that legitimately abstained. If health is not recommended or suggestions still
show an accepted scope problem, return to the proposal; do not paper over it.

Finish with the exact maintenance diff and offer:

```sh
git add argot.toml .argot/
git commit -m "chore(argot): refresh fit snapshot"
```

Do not stage, commit, or push unless the user explicitly asks. Remind them that
the commit makes the same reviewed snapshot available to local agents and CI.

## If the CLI disagrees

Trust `argot <command> --help`, `argot status --format json`, and the live
`argot review-mutes` report over this skill. Keep the conservative path: surface
uncertainty, preserve policy, and ask.
