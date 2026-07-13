---
title: Custom rules
description: Write your own rule — a rule.toml manifest plus a sandboxed Rhai script under .argot/rules/<name>/ — versioned with the repo, discovered fresh on every run, no recompiling argot. Same severities, suppressions, and output surfaces as every built-in.
group: Guide
order: 12
---

argot's ten built-in rules cover the patterns every repo eventually cares about — foreign
imports, reinvented functions, layering, gamed tests. Some conventions are yours alone, though:
"never call this deprecated internal helper," "route handlers must not touch the ORM directly,"
"this repo's one rule about raw SQL." For those, write a **custom rule**.

A custom rule is a directory under `.argot/rules/<name>/`, committed with the rest of the repo:
a small `rule.toml` manifest plus a sandboxed [Rhai](https://rhai.rs/) script. `argot check`
discovers it fresh on every run — no recompiling argot, no plugin build step, no restart. Its
findings behave exactly like a built-in's: the same rule name in every output format, the same
`[rules]`/`--rule` severity knobs, the same inline-comment and `[[mute]]` suppression surfaces —
all under one new group, `custom`.

## Layout

```text
.argot/rules/no-raw-sql/
  rule.toml          # identity, severity, language scope, host-API generation
  check.rhai         # the detection logic — sandboxed Rhai, host API v1
  tests/             # fixtures for `argot rules test` (see below)
    fires-on-execute/
      input.py
      expected.json
    silent-on-builder/
      input.py
      expected.json
```

One rule per directory. `argot check` and `argot rules` scan every directory under
`.argot/rules/`; a repo with no such directory — the common case — is unaffected.

## `rule.toml` reference

```toml
[rule]
schema = 1                 # required — manifest schema generation (currently 1)
name = "no-print"          # required — must equal the directory name
label = "no print calls"   # optional — shown next to a finding; defaults to `name`
description = "this repo logs, never prints"   # optional — shown by `argot rules`
severity = "error"         # optional — error | warn | off; defaults to warn
languages = ["python"]     # optional — scoring language names; empty/omitted = every language

[engine]
api = 1                    # optional — host-API generation the script targets; defaults to 1
script = "check.rhai"      # optional — script path, relative to the rule dir; defaults to check.rhai
```

| Field | Required | Default | Notes |
|---|---|---|---|
| `rule.schema` | yes | — | The manifest schema generation. A schema newer than this argot understands is skipped with a clear message. |
| `rule.name` | yes | — | Must equal the directory name — a rule is addressable by its path. |
| `rule.label` | no | `name` | Short label shown next to a finding. |
| `rule.description` | no | `""` | One-line description, shown by `argot rules`. |
| `rule.severity` | no | `warn` | `error` / `warn` / `off`. **Note the default differs from built-ins**, which default to `error` — a rule just dropped into a repo reports before it gates. |
| `rule.languages` | no | every language | Scoring language names — `python`, `typescript`, `javascript`, `go`, `rust`, `java`, `csharp`, `php`, `cpp`, `ruby`, `c` (see [Languages](/docs/languages/)). |
| `engine.api` | no | `1` | The host-API generation the script targets. A script asking for a newer generation than the binary provides is skipped, never half-run. |
| `engine.script` | no | `check.rhai` | Script file, relative to the rule directory. |

A manifest that fails to parse, names a schema or host-API generation this argot doesn't
understand, or whose `name` doesn't match its directory is **skipped with a warning on
stderr** — discovery degrades per rule, never for the whole run.

## Host API v1

The script's top-level statements run once per in-scope changed file. Two read-only bindings
are in scope:

- **`file`** — a map: `path` (repo-relative), `language` (scoring name), `new_text` (the
  post-image source), `old_text` (the pre-image source; `()` for an added file or when the
  mode can't resolve it).
- **`hunks`** — an array of maps, one per changed range: `start`, `end` (1-indexed, inclusive
  line numbers), `text` (the hunk's source).

And the host functions:

| Function | Returns | Does |
|---|---|---|
| `ts_query(query)` | array of `#{capture, text, line, end_line}` | Runs a [tree-sitter](https://tree-sitter.github.io/tree-sitter/) query against the file; one map per capture, lines 1-indexed. An invalid query or unsupported language returns an empty array. |
| `ts_query_old(query)` | same | The same query against the **pre-image** (`file.old_text`) — for rules about what a change *removed*. Empty when there is no old side. |
| `import_attested(module)` | bool | Did the fitted voice model see this module imported anywhere in this language, at fit time? |
| `callee_attested(name)` | bool | Same, for a called name. |
| `changeset_paths()` | array of strings | Every path in the current changeset — for rules that need cross-file context (e.g. "flag X unless a sibling test file also changed"). |
| `report(line, message)` | — | Records one finding on a single line. |
| `report_span(start, end, message, opts)` | — | Records one finding over a line range. `opts` is a map: optional `evidence` (array of strings, shown as the finding's evidence lines) and optional `symbol` (string). |

`import_attested`/`callee_attested` reflect the fitted voice model — in `argot rules test`
there is no fitted model, so both **always return `false`**; test the unattested path there
and the attested path live.

## Worked example: `no-print`

Ban `print()` calls in Python — this repo logs, never prints.

```toml
# .argot/rules/no-print/rule.toml
[rule]
schema = 1
name = "no-print"
languages = ["python"]
severity = "error"
```

```rhai
// .argot/rules/no-print/check.rhai
for m in ts_query("(call function: (identifier) @fn)") {
    if m.capture == "fn" && m.text == "print" {
        report_span(m.line, m.end_line, "print() call — this repo logs, never prints", #{
            evidence: ["use the logger instead"],
        });
    }
}
```

Add a fixture and run the authoring loop before the rule ever sees a real diff:

```python
# .argot/rules/no-print/tests/fires-on-print/input.py
print("debug")
```

```json
// .argot/rules/no-print/tests/fires-on-print/expected.json
[{"line": 1, "message": "print() call — this repo logs, never prints"}]
```

```bash
argot rules test no-print
# ok    no-print :: fires-on-print
#
# 1 case(s), 0 failed
```

Once it's green, it's live — the very next `argot check` runs it over real changes:

```bash
argot check
# ! src/app.py:12   no-print — print() call — this repo logs, never prints
#                   ↳ use the logger instead
```

## What only argot can express

Two host calls have no equivalent in any classic linter:

**The pre-image.** A linter sees one version of one file; argot hands your rule both sides of
the diff. `ts_query_old` runs the same tree-sitter query against the file as it was *before* the
change — so you can write rules about what a diff **removed**:

```rhai
// no-dropped-endpoints — a route that existed before this change, gone now
const ROUTES = "(call_expression function: (member_expression property: (property_identifier) @verb)
                arguments: (arguments (string (string_fragment) @path)))";
let now = [];
for m in ts_query(ROUTES) { if m.capture == "path" { now.push(m.text); } }
for m in ts_query_old(ROUTES) {
    if m.capture == "path" && !now.contains(m.text) {
        report(m.line, "endpoint '" + m.text + "' removed without a deprecation cycle — see docs/api-lifecycle.md");
    }
}
```

Pair it with an `old.ts` fixture in the harness (see below) and severity `error`: silently
dropping a public route now fails the check with the route's name.

**The learned model.** `import_attested(module)` / `callee_attested(name)` consult the fitted
voice model — your own git history as the allowlist. *"Flag any date library this repo has never
used"* needs no hardcoded list and never goes stale: the repo's history **is** the list.

## Severity, suppression, output — identical to a built-in

A custom finding's internal reason is `custom:<name>`, but every user-facing surface treats it
exactly like a built-in rule:

- **Severity:** `[rules] no-print = "warn"` (one rule) or `[rules] custom = "off"` (the whole
  group) in `argot.toml`, or `argot check --rule no-print=warn` per run.
- **Inline suppression:** `# argot: ignore-next-line rule=no-print — legacy debug shim`.
- **Durable mute:** `argot mute <hash> --reason "…"`, or a hand-written `[[mute]]` with
  `rule = "no-print"`.
- **Output:** the rule name appears in human output, `--format json`'s `rule` field, and
  SARIF's `ruleId` — same as `foreign-import` or `redundant`.
- **Confidence:** every custom finding displays at `suspicious` confidence — a discrete,
  evidenced event, the same tier as the `integrity` rules — never `unusual` or `foreign`.
- **Listing:** `argot rules` lists custom rules after the built-ins, group `custom`, with their
  source directory (`.argot/rules/no-print`); `--format json` adds a `source` field to those
  entries.

See [Configure](/docs/configure/#rules--rule-severities) for the full severity and suppression
reference — nothing here is a new mechanism.

## Sandbox guarantees

The script runs in a stripped-down Rhai engine, not a general-purpose scripting environment:

- **No filesystem, no network, no module imports** — Rhai's core language has none of these,
  and the script only ever sees the one file and hunk data passed in.
- **`print`/`debug` are captured, never reach stdout** — a script can't pollute a machine
  output format by accident.
- **Hard caps per (rule, file):** 1,000,000 operations, a call-depth of 32 (recursion guard), a
  1 MB max string, a 100,000-entry max array, a 10,000-entry max map, and a 100 ms wall-clock
  budget.
- **Degrade, never fail:** a script that fails to compile, trips a cap, or errors at runtime is
  **disabled for the rest of the run** with one diagnostic on stderr (`custom rule <name>: … —
  rule disabled for this run`) — it never takes down `check` itself, and never silently.

## The `argot rules test` harness

Fixtures live inside the rule directory, one subdirectory per case:

```text
.argot/rules/no-raw-sql/tests/
  fires-on-execute/
    input.py          # the file the rule runs over — the whole file is one hunk
    expected.json      # [{"line": 2, "message": "raw SQL — …"}]
  silent-on-builder/
    input.py
    expected.json      # []
```

`input.<ext>` picks the case's language from its extension; `expected.json` is the exact list
of `{line, message}` pairs the script should (or, for a silent case, shouldn't) report —
compared order-independently.

```bash
argot rules test              # every discovered rule, every case
argot rules test no-print     # one rule
```

Exit codes: `0` every case passed, `1` at least one failure, `2` a setup problem (an unknown
rule name, a script that fails to compile, or no `tests/` directory at all — add one case
before shipping the rule).

An optional `old.<ext>` sibling in a case directory supplies the pre-image for
`file.old_text` / `ts_query_old` (absent = the rule sees an added file).

Because the harness has no fitted model, `import_attested`/`callee_attested` return `false` in
every fixture — write a case for the unattested branch here, and rely on a real `argot check`
run to exercise the attested one live.
