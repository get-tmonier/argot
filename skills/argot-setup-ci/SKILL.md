---
name: argot-setup-ci
description: Wire argot into a repository's GitHub Actions as a non-blocking configured check on every pull request — a job summary plus code-scanning annotations. Use when the user wants argot "in CI", "on PRs", "as a GitHub Action", or asks to "set up argot CI". Distinct from argot-setup (local checking) and argot-review-pr (reviewing one PR on demand).
---

# argot-setup-ci

Add argot to a repository's CI as a **non-blocking** pattern check on every
pull request. Never make it block the merge unless the user explicitly asks for
a gate. This is user-wired automation: the Action runs only at the GitHub event
in the workflow the repository commits; it does not install or claim an agent
end-of-turn or acceptance lifecycle.

The Action installs Argot but **never fits in CI**. CI and local setup are one
decision: a committed `argot.toml` plus a complete committed `.argot/` fit
snapshot — the excludes and learned detector data that keep every check
reproducible. A repository without that snapshot must run the full
**argot-setup** flow first. The excludes
that keep vendored, generated and demo trees out of the voice — makes the CI
voice sharper, and argot is only as good as that scoping. **If the repository
has no `argot.toml` yet, offer the full [argot-setup](../argot-setup/SKILL.md)
flow first**, which covers CI as one of its phases. Come here when the user
wants CI specifically, or already has a configured repo.

## Steps

1. Confirm the repo is on GitHub. (Forks: enable Actions in Settings first.)

2. Write `.github/workflows/argot.yml`:

   ```yaml
   name: argot
   on:
     pull_request:
     push:
       branches: [main]

   permissions:
     contents: read
     pull-requests: write     # the sticky score comment
     security-events: write   # SARIF code-scanning annotations

   jobs:
     voice:
       runs-on: ubuntu-latest
       steps:
         - uses: actions/checkout@v4
           with:
             fetch-depth: 0    # Action reads the committed snapshot from the PR base
         - uses: get-tmonier/argot@main
   ```

   That's the whole workflow — the Action installs Argot, reads the reviewed
   base snapshot, and scores. It does not fit, cache `.argot/`, or download a
   semantic model.
   There is no model to fetch: the embedder ships inside the binary, so an
   air-gapped runner needs no special handling.

3. If the repo already has an `argot.toml` (from local setup or by hand), leave
   it — the Action respects it. It's optional; for a monorepo with peripheral
   packages, running the **argot-setup** flow first to commit a good `argot.toml`
   (excludes + any project-specific generated-file markers) makes the CI voice
   sharper, but isn't required.

4. Commit and push the workflow. Pushing a `.github/workflows/*.yml` needs the
   `workflow` token scope — if `git push` is rejected with *"refusing to allow an
   OAuth App to … workflow … without 'workflow' scope"*, run
   `gh auth refresh -s workflow` (or push over SSH).

5. **Validate the precondition before committing the workflow:** `argot status
   --format json` must show `snapshot.complete: true` and `snapshot.committed:
   true`. If it is stale, run `argot fit` locally on the accepted branch, review
   and commit `.argot/`; never add a CI fit as a workaround.

6. Explain the scorecard: it is advisory when a complete snapshot is old, but a
   missing/incomplete/config-mismatched base snapshot is an explicit setup error
   because a partial check must not pretend to cover semantic, layering, or
   integrity rules.

7. Tell the user what they'll get on each configured PR workflow: a
   **non-blocking** job summary, optional sticky PR comment, and inline
   code-scanning annotations. Findings do not fail the Action by default;
   operational workflow failures can still fail it.

8. **Offer a live README badge.** If the user wants one, add `contents: write`
   to `permissions` and `publish-badge: true` under the action's `with:`. On
   each push to the default branch the Action publishes the in-voice score to a
   `badges` branch; give the user the snippet to paste in their README:

   ```md
   [![argot](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/OWNER/REPO/badges/argot.json)](https://argot.tmonier.com)
   ```

   It renders `argot | N% in-voice`, green when in voice. (For a static badge
   with no shields.io round-trip: `argot voice-diff <range> --format svg`.)

9. For a hand-rolled workflow, install Argot, extract the base commit's tracked
   `.argot/` snapshot into a temporary directory, then run `argot check
   --argot-dir <snapshot> --format github`. Do not cache or run `argot fit` in
   CI. For a strict findings policy, `--error-on-warnings` makes warn-severity
   hits fail too.

## Principles

- **Non-blocking by default.** Do NOT add `fail-on-hits: true` unless the user
  explicitly wants a hard merge gate.
- **Base-snapshot only.** The Action reads the fit snapshot committed on the PR
  base and scores the PR against it — a dependency, config, or artifact the PR
  introduces cannot self-certify. Local setup is required to produce updates.
- **Workflow-scoped.** The Action can run automatically after its workflow is
  committed, but no skill or plugin installation schedules CI or a full local
  check by itself.
- Full options and the copy-paste workflow: the
  [CI guide](https://argot.tmonier.com/docs/ci/).
