---
title: Set up with your agent
description: One copy-pasteable prompt that walks any coding agent through setting argot up properly — audit, scope, fit, verify, tune, wire.
group: Start
order: 3
---

Argot learns how *your* repository writes code. Everything downstream depends on
one judgment — **which code is the voice** — and getting it wrong fails quietly:
false alarms people learn to scroll past, or real problems never flagged.

If your agent supports skills, install them and ask for `argot-setup`:

```sh
npx skills add get-tmonier/argot
```

If it does not, or you would rather not install anything, **paste the prompt
below into any coding agent** in your repository. It is the same procedure.

## The prompt

Copy everything in this block.

```text
Set up argot in this repository. Argot learns how this repo writes code and
flags what looks foreign to it, so the quality of every future result depends
on scoping it correctly now. Work through the phases below in order. At each
one, show me the evidence and let me decide — do not apply changes you cannot
justify in one sentence. Any phase can be skipped if I say so.

0. PREFLIGHT
   Run `argot --version` (if missing, stop and point me at
   https://argot.tmonier.com/docs/getting-started/). Confirm this is a git repo
   with real history, that I am on the default branch, and that the tree is
   clean — fitting a feature branch bakes unmerged commits into the voice.
   Tell me setup will write argot.toml (committed) and .argot/ (gitignored).

1. PROOF FIRST
   Run `argot audit --format json`, SAVE IT TO A FILE, and show me a readable
   summary. It needs no configuration and changes nothing. If it says the
   window touched no supported source, widen it with `--commits 200` or
   `--since 6m`. Audit takes minutes on a large repo — phase 6 reads this same
   file, so do not run it twice.

2. SCOPE
   Decide what should NOT shape the voice. Two sources now, a third after the
   fit in phase 3:
     - `argot init --suggest --format json` (generated / data-heavy dirs)
     - your own read of the tree: vendored code, demo/example/playground trees,
       peripheral monorepo packages, generated clients, transpiled JS in a TS
       repo, committed backup/old snapshots, large data and fixtures
   Show me `argot inspect --corpus` before we commit to it. Put what we agree
   in argot.toml [exclude].paths, one per line, each with a trailing # reason.
   Phase 3 prints more evidence and sends you back here — that is expected,
   scoping is two passes, not one.

3. FIT AND CHECK HEALTH
   Run `argot init`. Warn me first that the first run downloads a ~100 MB
   embedding model. Then run `argot inspect --format json` and read it:
     - verdict "not_recommended" -> go back to phase 2, do not continue
     - a reason with signal "voice_not_where_the_work_is" -> argot found a
       directory that shapes much of the voice but takes almost none of the
       recent changes. That is a mis-scope. Take it back to phase 2.
   Also read what `argot init` PRINTED, it is evidence phase 2 could not have:
     - "N files argot:recommended would exclude are shaping the voice (...)" ->
       add every path it names to [exclude].paths
     - "<language>: N files - too few to learn a voice" -> those files are NOT
       checked at all. Tell me. If an extension is routed to the wrong language
       here, exclude it.
   Yellow notes on a small repo are normal. Do not chase a perfect verdict.

4. VERIFY IT CATCHES SOMETHING
   Pick the package deliberately, do not guess: run `argot conventions` (or
   read .argot/repo-corpus.txt) to see what this repo actually imports, then
   choose something plainly outside it. In a real source file — primary source,
   not a test or an example — add that import plus a line using it. Run
   `argot check`, confirm it fires, then revert.
   If it does NOT fire, DO NOT conclude the tool is weak. Run the diagnosis
   below before anything else.

5. ADOPTION
   Ask me directly: baseline or clean slate?
     - baseline: `argot check --add-ignores` writes an inline ignore above
       every finding that exists today, so only new code is judged from here
     - clean slate: we fix or mute the existing findings now
   If I baseline, remind me those ignores are a snapshot and should be
   re-scored later, or the baseline silently becomes permanent.

6. TUNE FROM MY OWN HISTORY
   Re-read the audit JSON you saved in phase 1 and look at `over_firing` — rules tripping more
   than 2% of scanned hunks. A healthy repo sits at 0.3-0.7% per rule, so
   anything above that is describing this repository rather than flagging it.
   For each, propose in this order and never jump to "off":
     1. scope it to the tree that trips it, e.g.
        foreign-import = { severity = "error", exclude = ["**/*.bench.ts"] }
     2. soften it to "warn"
     3. accept it with a standing [[mute]] and a reason
   If the history shows one dependency replacing another, or I tell you we are
   mid-migration, propose a [[migration]] entry (from / to / reason).

7. CONVENTIONS
   Run `argot conventions --format json`. Do NOT dump it. Show me at most a
   handful, ranked by whether a rule for it would have caught something in the
   audit window. For any I pick, offer to write it as a custom rule.

8. WHERE IT RUNS — ask me about local and CI together
   Local: the pre-write guardrail hook (already included if I use the Claude
   Code plugin), pre-commit (argot-check advisory, argot-check-gate blocking),
   and the MCP server (`argot mcp`) for agent context.
   CI: the GitHub Action, non-blocking by default. Two things you must tell me:
     - the workflow needs BOTH `pull_request:` and `push:` to the default
       branch. The push run fits the model and publishes it to a cache; pull
       requests read that cache and stay fast. Without it every PR pays the fit.
     - that cache does not exist until the workflow is MERGED to the default
       branch. Until then every run is a cold fit and looks slow. Say so before
       I judge the tool on its first PR.

9. PROVE IT
   Give me the phase-4 fixture as a repeatable local command, so I can re-run
   the catch check myself after any config change. Do not push branches or open
   pull requests.

10. SUMMARIZE
    What the audit found, what we excluded and why, the health verdict, what
    was wired, and what my team sees on their next PR. Tell me refits are
    automatic but re-scoping is not: when a new generated or vendored tree
    appears, argot names it at the next fit and I should act on it.

DIAGNOSIS — read this before concluding anything about argot's quality.
If it seems noisy, or seems to catch nothing, it is far more often mis-scoped
than wrong. A mis-configured argot fails SILENTLY: no error, just false alarms
I learn to scroll past, or real problems never flagged. You cannot tell by
looking at findings. Run these:

  argot inspect --format json
    .verdict            "not_recommended" -> do not trust any finding yet
    .reasons[].signal
      voice_not_where_the_work_is
                        a directory shapes much of the voice but takes almost
                        none of the recent changes. THE most common cause of
                        both noise and silence: the model learned from code
                        nobody edits and is judging the code everybody does.
                        The message names the directory and both shares. Fix
                        it in phase 2, do not tune rules around it.
      polyglot_mix      several languages share the repo; expect weaker signal
      any other reason  relay it verbatim, do not summarise it away

  argot audit --format json
    .over_firing[]      rules tripping >2% of MY OWN accepted history. A
                        healthy repo sits at 0.3-0.7% per rule. Above that the
                        rule is describing this repository rather than flagging
                        it — that is noise caused by scope or by the rule being
                        wrong here, NOT by argot being inaccurate.

  what `argot init` printed
    unlearnable_languages / "too few to learn a voice"
                        those files are NOT CHECKED. Silence there means not
                        looked at, not clean.
    "N hunk(s) over ... lines were not judged"
                        a rewrite that big is not one pattern being introduced.
                        Those hunks were skipped, not passed.

Throughout: if argot reports it did NOT judge something — a language with too
few files, an oversized hunk, a disabled rule — relay it to me. Silence that
means "not checked" must never look like silence that means "nothing found".
```

## Why it is a conversation and not a command

Almost every step above is a measurement argot can make on its own. The one it
cannot make is the judgment: *is this directory our voice, or is it demos we
happen to ship?* That is why setup asks rather than assumes, and why every
`[exclude].paths` entry carries a written reason — so the decision is reviewable
in the pull request that adds it.

## After setup

- [Check a changeset](/docs/check/) — the daily review loop
- [Configure](/docs/configure/) — the full configuration reference
- [CI and pre-commit](/docs/ci/) — what the Action does and what it costs
- [Health and freshness](/docs/health-and-freshness/) — when to re-scope
