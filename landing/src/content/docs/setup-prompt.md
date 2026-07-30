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
   Tell me setup will write argot.toml (committed) and .argot/ (gitignored,
   rebuildable local fit artifacts). Do not add .argot/ to the repository;
   CI caches it after default-branch fits.

1. PROOF FIRST
   Run `argot audit --format json`, SAVE IT TO A FILE, and show me a readable
   summary. It needs no configuration and changes nothing.
   TELL ME THE COST BEFORE YOU START IT, or I will kill it thinking it hung.
   Audit fits the whole corpus once, so a large repo costs minutes and
   gigabytes, not seconds — 924k lines of Object Pascal took 2 min 38 s and
   3.6 GB. A narrower `--commits` does NOT make it cheaper: the window only
   moves the base commit, the fit still reads every file. The lever is
   [exclude], which is phase 2 — so on a repo that size, offer to do the
   obvious exclusions first and audit second.
   Then CHECK IT PRODUCED SOMETHING: exit status 0 and a non-empty JSON file.
   A run that dies leaves a 0-byte file, and an empty audit reads exactly like
   a clean one.
   If it says the window touched no supported source, widen it with
   `--commits 200` or `--since 6m`. Phase 6 reads this same file, so do not
   run it twice.

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
   Run `argot init`. Then run `argot inspect --format json` and read it:
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

7. DO NOT WRITE CUSTOM RULES DURING CORE SETUP
   `argot conventions` is useful above to choose a deliberately foreign import,
   but do not mine conventions or create custom rules yet. First finish scope,
   fit, baseline, tuning, and integrations. A rule written before those choices
   settle is usually a frozen accident. The only custom-rule exploration is the
   explicit opt-in at the very end (step 11).

8. WHERE IT RUNS — ask me about local and CI together
   Local: the pre-write guardrail hook (already included if I use the Claude
   Code plugin), pre-commit (argot-check advisory, argot-check-gate blocking),
   and the MCP server (`argot mcp`) for agent context.
   CI: the GitHub Action, non-blocking by default. Two things you must tell me:
     - the workflow needs BOTH `pull_request:` and `push:` to the default
       branch. The push run fits and publishes the resulting `.argot/` artifacts
       to a cache; pull requests read that cache and stay fast. Without it every
       PR pays the fit.
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

11. OPTIONAL: EXPLORE ONLY THE CONVENTIONS ARGOT SHOULD ENFORCE
    Only after the summary, ask me exactly once whether I want a read-only,
    opt-in exploration for zero to three team conventions that are genuinely
    worth custom rules. Make clear that saying no is normal, the exploration
    creates no files, and selecting a candidate is a separate approval to write
    a rule.

    If I say yes:
      - use `argot conventions --format json`, the audit saved in step 1,
        `argot rules`, argot.toml, existing .argot/rules/, canonical source
        examples, and targeted git history to find durable team decisions;
      - reject generic syntax/style/import-order/deprecated-API/security rules
        that OXLint, ESLint, or another normal linter can own; reject built-in
        Argot duplicates; turn a mined migration into [[migration]], not a
        script; reject weak placement, rules that need type inference or
        cross-file binding resolution, and anything without a one-sentence
        canonical remedy;
      - keep a candidate only if Argot has a real contextual advantage: the
        pre-image (`ts_query_old` / file.old_text), the other paths in this
        changeset (`changeset_paths()`), a committed contract or its siblings
        (`read_repo_file()` / `repo_paths()`), the fitted-history allowlist
        (`import_attested()` / `callee_attested()`), or a strongly concentrated
        learned placement convention (feature F belongs in location L, so F
        outside L is the violation);
      - show at most three candidates, or zero if none clear the bar. For each
        show the counts and locations supporting it (concentration, home_files,
        out_files, audit/history evidence), one canonical good example, one
        plausible bad change, the exact Argot-only context, a syntactically
        detectable shape, proposed scope, and why current leaks are legitimate
        exceptions, debt, or a reason to reject the idea;
      - ask me separately which candidate, if any, to codify. Do NOT create a
        .argot/rules/ directory merely because I opted into exploration.

    For each candidate I explicitly approve, use argot-suggest-rules when it
    came from a mined placement convention; use argot-write-rule when I stated
    the convention. Write fixtures before the script: at least one firing case
    and one silent canonical case, plus old.<ext> or sibling fixture files when
    a pre-image or repository read matters. Loop `argot rules test <name>` until
    green, then prove it on a throwaway real diff and revert the diff. The
    fixture harness has no fitted model, so import_attested/callee_attested are
    false there: test the unattested path in fixtures and the attested path live.
    Start every new rule at warn; promote it to error or lock it only after real
    PRs show that no legitimate exception exists.

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
    "N large hunk(s) scored above the flat threshold but not above the one
    for their size"     the score is a max over the hunk's tokens, so a big
                        hunk scores higher for free; the bar rises with size
                        past this repo's own p90. Those were judged against a
                        higher bar, not skipped. Review them by hand if they
                        are rewrites.

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
