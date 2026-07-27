---
title: Custom rules
description: Write your own rule — a rule.toml manifest plus a sandboxed Rhai script under .argot/rules/<name>/ — versioned with the repo, discovered fresh on every run, no recompiling argot. Same severities, suppressions, and output surfaces as every built-in.
group: Guide
order: 13
---

argot's twelve built-in rules cover the patterns every repo eventually cares about — foreign
imports, reinvented functions, layering, gamed tests, migration leftovers. Some conventions are yours alone, though:
"never call this deprecated internal helper," "route handlers must not touch the ORM directly,"
"this repo's one rule about raw SQL." For those, write a **custom rule**.

A custom rule is a directory under `.argot/rules/<name>/`, committed with the rest of the repo:
a small `rule.toml` manifest plus a sandboxed [Rhai](https://rhai.rs/) script. `argot check`
discovers it fresh on every run — no recompiling argot, no plugin build step, no restart. Its
findings behave exactly like a built-in's: the same rule name in every output format, the same
`[rules]`/`--rule` severity knobs, the same inline-comment and `[[mute]]` suppression surfaces —
all under one new group, `custom`.

## Start from a working example

The repository ships rules you can copy, one directory each, under
[`examples/rules/`](https://github.com/get-tmonier/argot/tree/main/examples/rules):

| rule | language | shows |
|---|---|---|
| `route-documented` | typescript | `read_repo_file` — a route must appear in the committed `openapi.yaml` |
| `contract-answered` | pascal | `read_repo_file` + `repo_paths` + `ts_query_old` — a member added to a shared contract must be answered by every implementation of it |

```sh
cp -r examples/rules/route-documented /path/to/repo/.argot/rules/
cd /path/to/repo && argot rules test route-documented
```

Their fixtures run in argot's own test suite, so they cannot rot: a host-API
change that breaks one breaks the build. Copy, then make it yours — the paths,
the severity and the message belong to the repository that runs it.

## Layout

```text
.argot/rules/no-raw-sql/
  rule.toml          # identity, severity, language scope, host-API generation
  check.rhai         # the detection logic — sandboxed Rhai, host API v2
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
languages = ["python"]     # optional — restrict to these scored languages (does NOT reach
                           #   unscored files like .env — use `include` for that)
include = []               # optional — path globs; runs the rule on ANY matching file,
                           #   even extensions argot doesn't score (.env, .yml, lockfiles…)
exclude = []               # optional — path globs subtracted from the scope (e.g. tests)

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
| `rule.languages` | no | every language | Restrict to these **scored** languages — `python`, `typescript`, `javascript`, `go`, `rust`, `java`, `csharp`, `php`, `cpp`, `ruby`, `c`, `pascal` (see [Languages](/docs/languages/)). This gate is over *supported source files only*; it can't reach a `.env` — that's what `include` is for. |
| `rule.include` | no | (none) | Repo-relative **path globs** (dialect: `*`/`**` cross `/`, `?`, `[abc]`). When set, the rule runs on any matching path — **including extensions argot doesn't score**. See *Which files a rule runs on*. |
| `rule.exclude` | no | (none) | Path globs subtracted from the scope — even from the default language scope, so an `include`-less rule can still skip `**/*.test.ts`. |
| `engine.api` | no | `1` | The host-API generation the script targets. A script asking for a newer generation than the binary provides is skipped, never half-run. `read_repo_file` / `repo_paths` need `api = 2`. |
| `engine.script` | no | `check.rhai` | Script file, relative to the rule directory. |

A manifest that fails to parse, names a schema or host-API generation this argot doesn't
understand, or whose `name` doesn't match its directory is **skipped with a warning on
stderr** — discovery degrades per rule, never for the whole run.

## Which files a rule runs on

By **default a rule sees the same files `check` scores** — the source files of the languages
argot supports (`.py`, `.ts`, `.go`, …), minus anything excluded by `[exclude]` or the
`argot:recommended` set. No configuration needed, which is why the built-in rules carry none.

A custom rule's **manifest** narrows or widens that scope with three fields:

- **`languages`** — restrict to a subset of the *scored* languages. `languages = ["typescript"]`
  runs the rule only on changed `.ts`/`.tsx` files. This gate is over supported source files —
  **it can't reach a `.env` or a YAML config**; those aren't a "language" argot scores.
- **`include`** — repo-relative **path globs**, and the escape hatch from the language gate: a
  rule with `include` runs on **any changed file that matches, including extensions argot doesn't
  score at all**. `include = ["*.env"]` reaches your env files; `include = [".github/workflows/*.yml"]`
  CI config; `include = ["**/*.eslintrc*"]` lint configs. For an unscored file the script still
  gets `file.path`, `file.ext`, `file.new_text`/`old_text`, and `hunks` — everything except a
  tree-sitter `language` (it's `""`, and `ts_query` returns nothing, since there's no grammar).
- **`exclude`** — path globs subtracted from whatever the above admit, so even an `include`-less,
  language-gated rule can carve out `**/*.test.ts` or `**/__tests__/**`.

`include` and `languages` **intersect** (`include = ["src/api/**"]` + `languages = ["typescript"]`
= *changed TypeScript under `src/api/`*); `exclude` always wins. The glob dialect is exactly
`[[mute]].path`'s (see [Configure](/docs/configure/#the-mute-format)): `*` and `**` cross `/`,
`?` is one character, `[abc]` / `[a-z]` are character classes.

> **Scoping any rule, not just custom ones.** The `include`/`exclude` above are a custom rule's
> *own* scope, declared by its author (and the only way to reach unscored files). A **repo owner**
> can additionally restrict *any* rule — built-in or custom — to paths from `argot.toml`'s
> `[rules]`, e.g. `layering = { include = ["src/**"] }` or `convention = { exclude = ["legacy/**"] }`.
> That's a config-side filter on findings, covered in
> [Configure → path-scoping a rule](/docs/configure/#path-scoping-a-rule).

## Host API

The script's top-level statements run once per in-scope changed file (see *Which files a rule
runs on* above). Two read-only bindings are in scope:

- **`file`** — a map: `path` (repo-relative), `language` (scoring name, or `""` for a file the
  rule claimed via `include` that argot doesn't score), `ext` (the file extension, e.g. `.env`),
  `new_text` (the post-image source), `old_text` (the pre-image source; `()` for an added file
  or when the mode can't resolve it).
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
| `read_repo_file(path)` | string or `()` | **API 2.** The text of another file in the repository, repo-relative. `()` when it is missing, escapes the root, is not UTF-8, or exceeds 1 MiB. |
| `repo_paths(glob)` | array of strings | **API 2.** Repo-relative paths the repository *contains* (git's index when the root is a repo, else a bounded walk) matching `glob` — the same dialect as `[[mute]].path`. Sorted. |
| `report(line, message)` | — | Records one finding on a single line. |
| `report_span(start, end, message, opts)` | — | Records one finding over a line range. `opts` is a map: optional `evidence` (array of strings, shown as the finding's evidence lines) and optional `symbol` (string). |

`import_attested`/`callee_attested` reflect the fitted voice model — in `argot rules test`
there is no fitted model, so both **always return `false`**; test the unattested path there
and the attested path live.

### Reading the rest of the repository (API 2)

`ts_query` and `hunks` see the changed file. A whole family of conventions is about *two*
files, though — a contract and the implementations that must answer it, a migration and the
schema it belongs to, a route and its entry in the API description. Those need
`read_repo_file` and `repo_paths`, so declare `api = 2` in the manifest.

```rhai
// Every backend under kernel/<platform>/ must answer every member of the contract.
let contract = read_repo_file("lib/common/kernel/mseguiintf.inc");
if contract != () {
    let missing = [];
    for line in contract.split("\n") {
        // … collect the members the contract declares …
    }
}
for backend in repo_paths("lib/common/kernel/*/mseguiintf.pas") {
    // … and compare each sibling against them.
}
```

The sandbox stays closed where it matters: reads are **read-only**, refused outside the
repository root (`..`, absolute paths, and symlinks that leave it), capped at 1 MiB per file,
and metered per checked file — 64 reads, 4 MiB, 16 listings. Past the budget the calls return
`()` / `[]` and the rule keeps running, exactly like an unresolvable `ts_query`. A rule can
read nothing its author's own clone does not already hold.

Unlike the model facts, **these work in `argot rules test`**: repository access is rooted at
the fixture case directory, so a case can ship the sibling files its rule reads next to
`input.<ext>` — the cross-file analogue of `old.<ext>`.

## Worked example: `domain-imports-stay-inward`

The convention: the pure domain layer (`src/domain/`) must not reach outward into infrastructure
(`src/infra/`) — dependencies point inward, ports/adapters keep it testable. Everyone agrees in
review; nothing enforces it. In ESLint this is an afternoon with a boundaries plugin and a config
file nobody wants to own. Here it's ten lines that live next to the convention's own README:

```toml
# .argot/rules/domain-imports-stay-inward/rule.toml
[rule]
schema = 1
name = "domain-imports-stay-inward"
description = "src/domain must not import src/infra — dependencies point inward"
languages = ["typescript"]
severity = "error"
include = ["src/domain/**"]     # scope lives in the manifest, not the script
```

```rhai
// .argot/rules/domain-imports-stay-inward/check.rhai
for m in ts_query("(import_statement source: (string (string_fragment) @from))") {
    if m.capture == "from" && m.text.contains("/infra/") {
        report_span(m.line, m.end_line, "domain imports infrastructure — invert the dependency", #{
            evidence: ["depend on a port defined in the domain (see src/domain/README.md)"],
        });
    }
}
```

Write both fixtures before polishing the script — the silent case is what protects the team
from a noisy rule:

```ts
// .argot/rules/domain-imports-stay-inward/tests/fires-on-infra-import/input.ts
import { PgClient } from '../../infra/postgres';
export const load = (c: PgClient) => c.query('...');
```

```json
// .argot/rules/domain-imports-stay-inward/tests/fires-on-infra-import/expected.json
[{"line": 1, "message": "domain imports infrastructure — invert the dependency"}]
```

```ts
// .argot/rules/domain-imports-stay-inward/tests/silent-on-port/input.ts
import type { Store } from './ports';
export const load = (s: Store) => s.get();
```

```json
// .argot/rules/domain-imports-stay-inward/tests/silent-on-port/expected.json
[]
```

```bash
argot rules test domain-imports-stay-inward
# ok    domain-imports-stay-inward :: fires-on-infra-import
# ok    domain-imports-stay-inward :: silent-on-port
#
# 2 case(s), 0 failed
```

Once it's green, it's live — the very next `argot check` runs it over real changes:

```bash
argot check
# ! src/domain/orders.ts:1   domain-imports-stay-inward — domain imports infrastructure — invert the dependency
#                            ↳ depend on a port defined in the domain (see src/domain/README.md)
```

## Shapes you'll actually write

**Composite call shapes** — the pattern is two calls nested, which is exactly where flat
lint rules give up and a tree query doesn't. *"Files are parsed through `lib/config` — its
loader validates and applies defaults; a raw `JSON.parse` over a file read skips both":*

```toml
# rule.toml — the loader itself is allowed; scope it out in the manifest
exclude = ["lib/config/**"]
```

```rhai
// parse-through-loader — JSON.parse directly over a file read
for m in ts_query("(call_expression function: (member_expression) @f
                    arguments: (arguments (call_expression function: (identifier) @inner)))") {
    if m.capture == "inner" && m.text.contains("readFile") {
        report(m.line, "parse through lib/config — a raw JSON.parse skips schema validation and defaults");
    }
}
```

**History-parameterized rules** — the allowlist is the repo's own git log, so the *same rule
file* is correct in every repo that adopts it, with zero configuration. *"One HTTP client per
repo — whichever one history already knows":*

```rhai
// one-http-client — flag any HTTP client this repo has never used
for m in ts_query("(import_statement source: (string (string_fragment) @mod))") {
    if m.capture == "mod"
        && ["axios", "got", "ky", "undici", "superagent"].contains(m.text)
        && !import_attested(m.text) {
        report(m.line, m.text + " — this repo already has an HTTP client; history knows which one");
    }
}
```

Drop that rule into a `got` shop and it fires on `axios`; drop it into an `axios` shop and it
fires on `got`. Nobody edits the rule — each repo's fitted history is the configuration. That's
what makes custom rules **shareable**: a rule pack can encode the *category* and let every
repo's own voice supply the allowlist.

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
- **Durable mute:** `argot mute <hash> --reason "…"` for one hit,
  `argot mute --path <glob> --rule <name> --reason "…"` for a standing one, or a `[[mute]]` with
  `rule = "domain-imports-stay-inward"`.
- **Output:** the rule name appears in human output, `--format json`'s `rule` field, and
  SARIF's `ruleId` — same as `foreign-import` or `redundant`.
- **Confidence:** every custom finding displays at `suspicious` confidence — a discrete,
  evidenced event, the same tier as the `integrity` rules — never `unusual` or `foreign`.
- **Listing:** `argot rules` lists custom rules after the built-ins, group `custom`, with their
  source directory (`.argot/rules/domain-imports-stay-inward`); `--format json` adds a `source` field to those
  entries.

See [Configure](/docs/configure/#rules--rule-severities) for the full severity and suppression
reference — nothing here is a new mechanism.

The optional pre-write hook consults the same effective `[rules]` policy, but it
does not execute custom rules and has no custom-rule-specific configuration.

One addition worth knowing: a custom rule can be **locked** —
`"domain-imports-stay-inward" = { severity = "error", locked = true }` in the committed
`argot.toml`. A locked rule's findings refuse every suppression surface, and a diff that edits
the rule's own script or manifest fires `rule-tampered` (error, unsuppressable) — so an agent
can't "fix" a failing check by rewriting the rule that caught it. See
[Locked rules](/docs/configure/#locked-rules--the-agent-cant-turn-off-the-alarm).

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

## Two things that will bite you

**`trim()`, `replace()` and friends mutate in place and return `()`.** This is
Rhai, not Rust:

```rhai
let t = line.to_lower().trim();   // t is (), and every later call on it fails
let t = line.to_lower(); t.trim(); // what you meant
```

**A cross-file rule costs real operations.** The sandbox stops a script at 1M
operations per file, and a per-character scan of a large file will hit it — on a
7 450-line source, stripping comments character by character does. Work line by
line, and reach for `contains` / `index_of` (one native op) over interpreted
loops. When a rule does trip a cap, argot says so on stderr and disables it for
the run — if a rule you expect goes quiet, read stderr before believing the
silence.

## The `argot rules test` harness

Fixtures live inside the rule directory, one subdirectory per case:

```text
.argot/rules/domain-imports-stay-inward/tests/
  fires-on-infra-import/
    input.ts           # the file the rule runs over — the whole file is one hunk
    expected.json      # [{"line": 1, "message": "domain imports infrastructure — …"}]
  silent-on-port/
    input.ts
    expected.json      # []
```

`input.<ext>` picks the case's language from its extension; `expected.json` is the exact list
of `{line, message}` pairs the script should (or, for a silent case, shouldn't) report —
compared order-independently.

```bash
argot rules test              # every discovered rule, every case
argot rules test domain-imports-stay-inward     # one rule
```

Exit codes: `0` every case passed, `1` at least one failure, `2` a setup problem (an unknown
rule name, a script that fails to compile, or no `tests/` directory at all — add one case
before shipping the rule).

An optional `old.<ext>` sibling in a case directory supplies the pre-image for
`file.old_text` / `ts_query_old` (absent = the rule sees an added file).

Because the harness has no fitted model, `import_attested`/`callee_attested` return `false` in
every fixture — write a case for the unattested branch here, and rely on a real `argot check`
run to exercise the attested one live.
