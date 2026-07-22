---
name: argot-write-rule
description: Codify a repo convention argot's built-ins don't cover into a scripted custom rule — a `.argot/rules/<name>/rule.toml` manifest plus a sandboxed Rhai script that fires exactly like a built-in, gated on a green fixture suite before it ever sees a real diff. Use when the user asks to "write an argot rule for X", "codify this convention", "ban <pattern> with argot", or wants a repo-specific check beyond the twelve built-ins. Distinct from argot-check (running the existing rules) and argot-setup (fitting the voice model).
---

# argot-write-rule

argot's built-in rules cover patterns most repos eventually hit — foreign
imports, reinvented functions, layering breaks, gamed tests. Some conventions
are the repo's own: "route SQL through the query builder, never a
string-concatenated query," "HTTP calls go through the shared client, not a
bare one-off request," "retries always back off, never a hard-coded sleep
loop." For those, write a **custom rule**: a small `rule.toml` manifest plus a
sandboxed [Rhai](https://rhai.rs/) script under `.argot/rules/<name>/`,
committed with the rest of the repo. `argot check` discovers it fresh every
run — no recompiling argot. Its findings behave exactly like a built-in's:
same severities, same suppressions, same output formats, one more group,
`custom`. Full reference: <https://argot.tmonier.com/docs/custom-rules/>.

Rule authoring is a later, explicit workflow: use audit and ordinary checks to
establish current behavior first. A custom rule must never turn a probabilistic
finding into an unreviewed default gate.

## The gate: `argot rules test <name>` must be green

A custom rule is not done until its fixture suite is green. This is not
polish at the end — it is the loop:

1. Write fixtures **before** touching the script: at minimum one case that
   should fire and one that should stay silent. The silent case is what
   protects the team from a noisy rule — a rule that only ever proves it
   *can* fire has never been checked for false alarms.
2. Run `argot rules test <name>` with no script yet (or a stub) and watch it
   **fail**.
3. Write or fix `check.rhai` until every case passes.
4. Every time you touch the script afterward — while iterating, while
   narrowing a false alarm, right before you hand it off — re-run
   `argot rules test <name>` first. A script you haven't re-tested since your
   last edit is a script you don't know still works.

```
argot rules test no-direct-env
# ok    no-direct-env :: fires-on-raw-env
# ok    no-direct-env :: silent-on-loadconfig
#
# 2 case(s), 0 failed
```

Only once this is green does a live `argot check` (below) earn its keep — it
confirms the rule reaches a real diff and points at the right file. It never
substitutes for the fixture gate: the harness is what catches over-fire
*before* it reaches someone's PR.

## Preconditions

1. `argot --version` — if missing, tell the user how to install it (see
   <https://argot.tmonier.com/docs/getting-started/>) and stop.
2. `argot rules test` needs no fitted model — it runs fixtures straight off
   disk, so you can write and green a rule in an unfitted repo. A live
   `argot check` (the last step) does need the repo fitted
   (`.argot/scorer-config.json`); if it's missing, run the **argot-setup**
   skill (or `argot init`) first.

## Workflow

1. **Capture the convention precisely.** Get it from the user's own words,
   then ground it in the repo: grep 2-3 real examples — the canonical *good*
   pattern and at least one place someone wrote the violation (or would have,
   before the convention existed). Name the rule after the convention,
   kebab-case (`no-direct-env`, `no-raw-sql`, `http-through-client`).

2. **Decide detectability honestly.** A convention is a shape tree-sitter can
   query — a call, an import, a member access, a specific argument pattern —
   or it isn't. If catching it needs type inference or resolving a binding
   across files, **say so and stop**: a rule that guesses at that will either
   miss constantly or misfire on unrelated code, and a noisy rule is worse
   than no rule. Default a new rule's severity to `warn` and only promote it
   to `error` once it has survived a few real PRs without a false alarm —
   `error` from day one is for conventions with zero legitimate exceptions.

   The host API is more than `ts_query`, and some conventions only become
   detectable with the rest of it — reach for these before concluding a rule
   isn't writable (full list: the [reference](https://argot.tmonier.com/docs/custom-rules/#host-api-v1)):
   - **`ts_query_old(q)`** queries the file's *pre-image* — the only way to
     write a rule about what a change **removed** (a route deleted without a
     deprecation cycle, a lint config entry dropped). No linter has a "before".
   - **`import_attested(m)` / `callee_attested(n)`** ask the fitted voice
     model — the repo's own git history as the allowlist. *"Flag any HTTP
     client this repo has never used"* needs no hardcoded list and never goes
     stale. (These return `false` in the fixture harness, which has no model —
     test the unattested path in fixtures, the attested path live.)
   - **`file.old_text` / `changeset_paths()`** give the pre-image text and the
     other paths in the same change, for coupled-file rules (*"this file
     changed but its sibling didn't"*).

3. **Scaffold `.argot/rules/<name>/`** — `rule.toml` (`schema = 1`, `name`
   matching the directory) and `check.rhai`. **Scope in the manifest, not the
   script**: `languages` for the language gate; `include` / `exclude` path
   globs when the convention is area-specific (`include = ["src/domain/**"]`,
   `exclude = ["**/*.test.ts"]`). `include` can also target files argot
   doesn't score at all — `.env`, CI configs, lockfiles. Keep `check.rhai`
   about the *pattern*, never about *which files run* — that belongs in the
   manifest, where a reader sees the scope at a glance.

4. **Fixtures first, then the script** — this is the gate above. Create
   `tests/<case>/{input.<ext>, expected.json}` for the firing case and the
   silent case (add an `old.<ext>` sibling too if the rule reads
   `ts_query_old` for what a change *removed*). Loop
   `argot rules test <name>` until every case is green.

5. **Verify live.** Reproduce the violation in a real (throwaway) diff and
   run `argot check`. Confirm the finding's message points at the repo's
   canonical example — the fix should be one hop away, not a guess. Then
   revert the throwaway diff.

6. **Finish.** Run `argot rules test <name>` one last time if the script
   changed since the last green run. Show the user `argot rules` so they see
   the new entry in the repo's vocabulary, group `custom`. Tell them the rule
   ships in `.argot/rules/<name>/` — committed, so every contributor and CI
   run gets it the moment they pull, with no argot rebuild.

## Worked example: `no-direct-env`

Convention: "config is read through the repo's `loadConfig()` helper; raw
`process.env` reads skip validation and defaults." Detectable — it's a
member-access shape tree-sitter can query. Scope: TypeScript/JavaScript, and
the config module itself is exempt — declared with `exclude` in the manifest,
so the script stays purely about the pattern.

```toml
# .argot/rules/no-direct-env/rule.toml
[rule]
schema = 1
name = "no-direct-env"
description = "config is read through loadConfig() — raw process.env reads skip validation and defaults"
severity = "warn"
languages = ["typescript", "javascript"]
exclude = ["src/config/**"]     # the loader itself is allowed to read process.env
```

```rhai
// .argot/rules/no-direct-env/check.rhai
for m in ts_query("(member_expression) @e") {
    if m.text.starts_with("process.env") {
        report(m.line, "read config through loadConfig() — raw process.env skips validation and defaults (see src/config.ts)");
    }
}
```

Fixtures — the firing case:

```ts
// .argot/rules/no-direct-env/tests/fires-on-raw-env/input.ts
export function getPort() {
  return process.env.PORT;
}
```

```json
// .argot/rules/no-direct-env/tests/fires-on-raw-env/expected.json
[{"line": 2, "message": "read config through loadConfig() — raw process.env skips validation and defaults (see src/config.ts)"}]
```

The silent case — the same read, done the repo's way:

```ts
// .argot/rules/no-direct-env/tests/silent-on-loadconfig/input.ts
import { loadConfig } from "../config";

export function getPort() {
  return loadConfig().port;
}
```

```json
// .argot/rules/no-direct-env/tests/silent-on-loadconfig/expected.json
[]
```

Run the gate:

```bash
argot rules test no-direct-env
# ok    no-direct-env :: fires-on-raw-env
# ok    no-direct-env :: silent-on-loadconfig
#
# 2 case(s), 0 failed
```

Green — now it's live. The very next `argot check` runs it over real changes:

```bash
argot check
# ? src/http.ts:14   no-direct-env — read config through loadConfig() — raw
#                    process.env skips validation and defaults (see src/config.ts)
```

(Custom findings always display at `suspicious` confidence — the `?` glyph —
never `unusual` or `foreign`; see [argot-check](../argot-check/SKILL.md).)

## Make it un-gameable (optional, for load-bearing conventions)

A custom rule is configured and suppressed like any built-in — which means, by
default, a future agent that can't satisfy it can also `off` it, mute it, or
just rewrite the script that caught it. For a convention with **zero legitimate
exceptions**, lock it in the committed `argot.toml`:

```toml
[rules]
"no-raw-sql" = { severity = "error", locked = true }
# or lock every repo-local rule at once:
custom       = { severity = "error", locked = true }
```

A locked rule's severity is frozen (local config and `--rule` are refused), and
**no suppression surface applies to its findings** — no inline ignore, no
`[[mute]]`, no `[exclude]`. Weakening the lock, or editing a locked rule's own
script, is itself reported by the `rule-tampered` rule (pinned `error`,
unsuppressable) — so the rule you just wrote can't be quietly undone in a later
diff. Only a committed `argot.toml` change a human reviews can relax it. See
[Locked rules](https://argot.tmonier.com/docs/configure/#locked-rules--the-agent-cant-turn-off-the-alarm).

Offer this when the user frames the convention as a hard invariant ("must
never", "always", a security or correctness boundary) — not for a soft style
preference, where `warn` and a normal mute are the right ergonomics.

## Hard rules

- **Never hand off a rule without a green `argot rules test <name>`** covering
  at least one firing case and one silent case. The silent case is the only
  thing standing between "this works" and "this is noisy."
- **Never treat a live `argot check` run as a substitute for the fixture
  gate.** One real diff proves the rule *can* fire; it says nothing about
  over-fire, which is exactly what the silent-case fixture is for.
- **Never ship a rule for a convention you can't state as a syntactic
  shape.** If it needs type inference or cross-file binding resolution, say
  so and stop rather than ship something that guesses.
- **Never default a new rule to `error`** unless the convention has zero
  legitimate exceptions — start at `warn` and let it prove itself.
- False positives happen even in scripted rules. If the user says a hit is
  fine, offer `argot mute <hash> --reason "…"` rather than loosening the
  script's logic on the spot to make one case go away.

## If the CLI and this document disagree

If the binary's manifest fields, host functions, or harness behavior don't
match this document, trust the binary: `argot rules` for the registry and
`argot rules test --help` for the harness's flags — the CLI is the source of
truth, and the [custom rules reference](https://argot.tmonier.com/docs/custom-rules/)
is the fuller doc; this skill may lag behind both.
