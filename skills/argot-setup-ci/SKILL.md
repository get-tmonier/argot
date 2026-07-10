---
name: argot-setup-ci
description: Wire argot into a repository's GitHub Actions as a non-blocking voice check on every pull request — a visual voice-score card plus code-scanning annotations. Use when the user wants argot "in CI", "on PRs", "as a GitHub Action", or asks to "set up argot CI". Distinct from argot-setup (local checking) and argot-review-pr (reviewing one PR on demand).
---

# argot-setup-ci

Add argot to a repository's CI as a **non-blocking** pattern check on every
pull request. You do **not** need to set argot up locally first — the Action
installs argot and fits the model in CI. Never make it block the merge unless
the user explicitly asks for a gate.

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

5. Tell the user what they'll get on each PR: a **non-blocking** voice-score card
   (a sticky PR comment + the Actions job summary) and inline code-scanning
   annotations. It never fails the build.

6. If the user prefers a hand-rolled workflow over the Action (or already has
   one), the building blocks are: install argot, `argot model fetch` (cache
   `~/.cache/argot/models` to keep the download out of every run), `argot fit`,
   then `argot check --format github` — the
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
- Full options and the copy-paste workflow: the
  [CI guide](https://argot.tmonier.com/docs/ci/).
