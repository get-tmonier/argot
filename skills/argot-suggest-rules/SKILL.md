---
name: argot-suggest-rules
description: Turn a convention argot has already *discovered* into a scripted custom rule. Runs `argot conventions`, surfaces the repo's mined placement conventions (where a kind of code lives — "validation in schema files", "DB access only in migrations", "business logic in the service layer, not views") and internal-API vocabulary, then codifies a chosen one as a `.argot/rules/<name>/` rule gated on a green fixture suite. Use when the user asks to "codify our conventions", "what conventions does argot see", "make a rule from that placement", or wants argot to propose rules from the repo instead of stating one by hand. Distinct from argot-write-rule (which starts from a convention the *user* states) and argot-check (which runs the existing rules).
---

# argot-suggest-rules

`argot-write-rule` starts from a convention **you** state. This skill starts from
what argot **already found** in the repo. `argot conventions` mines two kinds:

- **placement conventions** — *where* a kind of code lives: a feature (a call, an
  import) that concentrates in one location (a directory, a filename role, an
  extension) and is near-absent elsewhere. "validation lives in schema files",
  "`queryInterface` only in migrations", "business logic in the service layer,
  not in views". These are the team's structural rules.
- **vocabulary** — the repo's own most-used internal API (shared helpers, the
  objects everyone routes through).

The mining is corpus- and framework-agnostic (pure feature×location
association) and every candidate carries its evidence. You turn a chosen one
into a real scripted rule — same manifest + Rhai + fixture-gate shape as
`argot-write-rule`, same `custom` group. Full reference:
<https://argot.tmonier.com/docs/custom-rules/>.

This is a deliberate later workflow after audit and ordinary check activation;
it does not make a discovered convention a default CI or commit gate.

## Preconditions

1. `argot --version` — if missing, tell the user how to install it
   (<https://argot.tmonier.com/docs/getting-started/>) and stop.
2. `argot conventions` needs the repo **fitted** (`.argot/scorer-config.json`).
   If missing, run the **argot-setup** skill (or `argot init`) first.

## Workflow

1. **Surface what argot found.** Run `argot conventions --format json`. The
   `placement` array is the spine — each entry is:
   ```json
   { "location": "role:schema", "files": 52,
     "location_globs": ["**/schema.*", "**/*.schema.*"],
     "signature": [ { "feature": "z.object", "home_files": 51, "out_files": 5,
                      "lift": 36.0, "concentration": 0.91 }, … ] }
   ```
   Read it as: *"`z.object` lives in schema files (51 here, 5 elsewhere) — 91%
   confined."* Present the strongest few to the user in plain words ("argot sees
   that validation via `z.*` concentrates in your schema files — codify that?").

   The catalog also carries `migrations[]` (patterns argot's history-mining
   found the repo has replaced) and `declared_migrations[]` (already declared
   in `argot.toml`). **A mined migration is not a rule to write** — codify it
   as a `[[migration]]` entry instead, asking the user for the `reason`:
   ```json
   // languages.<lang>.migrations[]
   { "old": "moment", "new": "date-fns", "kind": "import",
     "commits": 4, "files": 4, "leftover_count": 9 }
   ```
   ```toml
   # argot.toml
   [[migration]]
   from = "moment"
   to = "date-fns"
   reason = "Q2 date-handling refactor"   # ask the user for this
   ```
   No fixture gate needed — `[[migration]]` takes effect immediately, no
   refit required.

2. **Pick a candidate with the user.** Prefer high `concentration` and low
   `out_files` (few existing leaks). A convention that's only 60% confined isn't
   a rule yet — say so. The `out_files` count is the current number of
   violations already in the repo: name it, so the user knows what enabling the
   rule will surface.

3. **State the rule as the contrapositive.** A placement convention "feature F
   lives in location L" becomes the rule **"F outside L is a violation"**:
   - **Scope** to files *outside* the home: `exclude = <location_globs>` in the
     manifest. The rule then only checks files that aren't the home.
   - **Detect** F in `check.rhai` with `ts_query`. Write the query for what F is:
     a member namespace (`z.*`) is a `(member_expression)` whose text starts with
     `z.`; a bare callee (`useState`) is a `(call_expression function:
     (identifier))`; a specific method is a member call. **Use judgment** — if F
     needs type inference or cross-file resolution to detect, say so and stop
     (same honesty as argot-write-rule).
   Name the rule after the convention, kebab-case (`validate-in-schema`,
   `db-only-in-migrations`, `no-business-logic-in-views`).

4. **Fixtures first, then the gate.** Create
   `.argot/rules/<name>/tests/<case>/{input.<ext>, expected.json}`: at least one
   **fires** case (a file using F) and one **silent** case (a file that doesn't).
   The silent case is what proves the rule isn't noisy. Loop
   `argot rules test <name>` until green — this is the gate, not optional polish.

5. **Verify live + report the leak.** Run `argot check` on a real (throwaway)
   diff that uses F outside the home; confirm the finding points at the home
   (`this belongs in <L>`). Then tell the user the `out_files` count — the
   existing violations — and offer `argot mute <hash> --reason "…"` for the
   legitimate exceptions rather than weakening the rule.

6. **Finish.** Re-run `argot rules test <name>` if the script changed since the
   last green run. Show `argot rules` so the user sees the new `custom` entry.
   The rule ships in `.argot/rules/<name>/`, committed — every contributor and
   CI run gets it on pull, no argot rebuild.

## Worked example: `validate-in-schema`

`argot conventions` reports `role:schema → z.object` (51 files home, 5 out, 91%
confined), `location_globs = ["**/schema.*", "**/*.schema.*"]`. Codify "validation
belongs in schema files":

```toml
# .argot/rules/validate-in-schema/rule.toml
[rule]
schema = 1
name = "validate-in-schema"
description = "validation (z.*) belongs in a schema file — not here"
severity = "warn"
languages = ["typescript"]
exclude = ["**/schema.*", "**/*.schema.*"]   # the home — from location_globs
```

```rhai
// .argot/rules/validate-in-schema/check.rhai
for m in ts_query("(member_expression) @e") {
    if m.text.starts_with("z.") {
        report(m.line, "validation via z.* belongs in a schema file (role:schema), not here");
    }
}
```

Fixtures — fires (a non-schema file using `z.*`) and silent (no `z.`):

```ts
// tests/fires-outside-schema/input.ts
export const User = z.object({});
```
```json
// tests/fires-outside-schema/expected.json
[{"line": 1, "message": "validation via z.* belongs in a schema file (role:schema), not here"}]
```
```ts
// tests/silent-plain/input.ts
export function add(a: number, b: number) { return a + b; }
```
```json
// tests/silent-plain/expected.json
[]
```

```bash
argot rules test validate-in-schema
# ok    validate-in-schema :: fires-outside-schema
# ok    validate-in-schema :: silent-plain
# 2 case(s), 0 failed
```

Green — the rule is live. (Custom findings always display at `suspicious`
confidence, the `?` glyph — see argot-check.)

## Make it un-gameable (optional, for load-bearing conventions)

A placement convention a team treats as a hard boundary can be locked in the
committed `argot.toml` so a later agent can't `off` it, mute it, or rewrite the
script — see the "Make it un-gameable" section of the **argot-write-rule** skill
and <https://argot.tmonier.com/docs/configure/#locked-rules--the-agent-cant-turn-off-the-alarm>.
Offer it only when the user frames the placement as a hard invariant.

## Hard rules

- **Never hand off a rule without a green `argot rules test <name>`** covering a
  fires case and a silent case.
- **Only codify a confined convention.** If `concentration` is low or `out_files`
  is large, the placement isn't enforced yet — surface it, don't rule it.
- **Never invent a placement the mining didn't find.** This skill codifies what
  `argot conventions` reports; to state a convention argot didn't discover, use
  **argot-write-rule** instead.
- **Never turn a mined migration into a scripted rule.** A `migrations[]`
  entry is codified as `[[migration]]` in `argot.toml`, not a `.argot/rules/`
  script.
- **Never ship a rule for a feature you can't detect syntactically.** If it needs
  type inference or cross-file resolution, say so and stop.
- Existing violations (`out_files`) are muted per-hit with a reason, not
  designed around by loosening the script.

## If the CLI and this document disagree

Trust the binary: `argot conventions --help` for the mining surface, `argot
rules` for the registry, `argot rules test --help` for the harness. The
[custom rules reference](https://argot.tmonier.com/docs/custom-rules/) is the
fuller doc; this skill may lag behind both.
