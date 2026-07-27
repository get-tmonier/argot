---
name: argot-setup-ci
description: Wire argot into a repository's GitHub Actions as a non-blocking configured check on every pull request — a job summary plus code-scanning annotations. Use when the user wants argot "in CI", "on PRs", "as a GitHub Action", or asks to "set up argot CI". Distinct from argot-setup (local checking) and argot-review-pr (reviewing one PR on demand).
---

# argot-setup-ci

Add argot to a repository's CI as a **non-blocking** pattern check on every
pull request. You do **not** need to set argot up locally first — the Action
installs argot and fits the model in CI. Never make it block the merge unless
the user explicitly asks for a gate. This is user-wired automation: the Action
runs only at the GitHub event in the workflow the repository commits; it does
not install or claim an agent end-of-turn or acceptance lifecycle.

## Steps

1. Confirm the repo is on GitHub. (Forks: enable Actions in Settings first.)

2. Write `.github/workflows/argot.yml`:

   ```yaml
   name: argot
   on: pull_request

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

5. **Know where a run's time goes.** Fitting the base is almost the whole cost;
   the check itself is seconds. The model cache is keyed on the base commit, and
   because an active branch moves, the Action falls back to the nearest cached
   model and refits only when it is more than `max-staleness` accepted commits
   behind (default 10, mirroring argot's local `[fit] refresh-after`) or when
   `argot.toml` changed. The job summary says which path the run took. Two
   knobs if a repo needs them: `max-staleness: 0` to demand the exact base
   commit, and `semantic: false` — the embedding index is what makes a fit
   expensive.

6. Tell the user what they'll get on each configured PR workflow: a
   **non-blocking** job summary, optional sticky PR comment, and inline
   code-scanning annotations. Findings do not fail the Action by default;
   operational workflow failures can still fail it.

7. **Offer a live README badge.** If the user wants one, add `contents: write`
   to `permissions` and `publish-badge: true` under the action's `with:`. On
   each push to the default branch the Action publishes the in-voice score to a
   `badges` branch; give the user the snippet to paste in their README:

   ```md
   [![argot](https://img.shields.io/endpoint?url=https://raw.githubusercontent.com/OWNER/REPO/badges/argot.json)](https://argot.tmonier.com)
   ```

   It renders `argot | N% in-voice`, green when in voice. (For a static badge
   with no shields.io round-trip: `argot voice-diff <range> --format svg`.)

8. If the user prefers a hand-rolled workflow over the Action (or already has
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
