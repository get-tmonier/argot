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

The Action installs argot and fits in CI, so this works on its own. But CI and
local setup are one decision, not two: a committed `argot.toml` — the excludes
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
       branches: [main]   # the run that fits the model every PR then reads

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
             fetch-depth: 0    # argot fits on the PR's base branch, so it needs history
         - uses: get-tmonier/argot@main
   ```

   Both triggers matter: the `push` run fits the model and publishes it, the
   `pull_request` run reads it and stays at seconds. See step 5.

   That's the whole workflow — the Action installs argot, caches and fetches
   the ~100 MB semantic embedding model itself (no manual cache step needed),
   fits, and scores. In a hand-rolled workflow, run `argot model fetch` once
   after installing argot instead.

3. If the repo already has an `argot.toml` (from local setup or by hand), leave
   it — the Action respects it. It's optional; for a monorepo with peripheral
   packages, running the **argot-setup** flow first to commit a good `argot.toml`
   (excludes + any project-specific generated-file markers) makes the CI voice
   sharper, but isn't required.

4. Commit and push the workflow. Pushing a `.github/workflows/*.yml` needs the
   `workflow` token scope — if `git push` is rejected with *"refusing to allow an
   OAuth App to … workflow … without 'workflow' scope"*, run
   `gh auth refresh -s workflow` (or push over SSH).

5. **Keep the `push:` trigger — it is what makes pull requests fast.** Fitting
   the model is almost the whole cost of a run; the check is seconds. The run on
   the default branch is the *producer*: it fits and publishes the model into a
   cache slot, after a merge, on nobody's critical path. A pull request is a
   *consumer*: it reads that slot and does not fit. Remove the `push` trigger and
   every pull request pays the fit instead — which is how a new tool gets
   uninstalled. A pull request only refits when no model exists yet (the first
   run, or the slot expired after seven idle days) or when the base's
   `argot.toml` changed.

6. **Say that the cache does not exist until this workflow is merged.** The
   producer run is a `push` to the default branch, so until the pull request
   adding the workflow is *merged*, there is no cache to read and **every run is
   a cold fit** — minutes, not seconds. Tell the user this before they judge the
   tool: the workflow's own PR, and any PR opened before it lands, are the slow
   ones by design. After the merge the next default-branch push fills the slot
   and pull requests drop to seconds.

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

9. If the user prefers a hand-rolled workflow over the Action (or already has
   one), the building blocks are: install argot, `argot model fetch` (cache
   `~/.cache/argot/models` to keep the download out of every run; also cache
   `~/.cache/argot/embeddings` with a loose restore-key so unchanged functions
   don't re-embed across runs), `argot fit`, then `argot check --format github` — the
   `github` format prints workflow commands that GitHub renders as inline PR
   annotations. For a strict setup, `--error-on-warnings` makes warn-severity
   hits fail the run too. If an existing workflow runs `argot extract && argot
   fit`, replace that with plain `argot fit` (fit includes extraction).

## Principles

- **Non-blocking by default.** Do NOT add `fail-on-hits: true` unless the user
  explicitly wants a hard merge gate.
- **Self-contained.** The Action installs argot, fits the model on the PR's
  **base** branch, and scores the PR against it — a dependency or idiom the PR
  introduces is judged as new, not self-certified. No local argot needed.
- **Workflow-scoped.** The Action can run automatically after its workflow is
  committed, but no skill or plugin installation schedules CI or a full local
  check by itself.
- Full options and the copy-paste workflow: the
  [CI guide](https://argot.tmonier.com/docs/ci/).
