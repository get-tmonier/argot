# Example scripted rules

Working rules you can copy into a repository's `.argot/rules/`, one directory
each. They exist to be read: every one is a shape that a generic linter cannot
express, and between them they exercise the whole host API.

```sh
cp -r examples/rules/route-documented /path/to/repo/.argot/rules/
cd /path/to/repo && argot rules test route-documented
```

Their fixtures run in argot's own test suite
(`crates/argot-core/tests/example_rules.rs`), so an API change that breaks one
of them breaks the build — an example that no longer works is worse than no
example.

| rule | language | API | shows |
|---|---|---|---|
| [`route-documented`](route-documented/) | typescript | 2 | `read_repo_file` — a route must appear in the committed API description |
| [`contract-answered`](contract-answered/) | pascal | 2 | `read_repo_file` + `repo_paths` + `ts_query_old` — a member added to a contract must be answered by every implementation of it |

## What these are not

They are not a rule library to depend on. Copy one, then make it yours: the
paths, the severity and the message all belong to the repository that runs it.
A rule whose message does not point at *your* canonical example is a rule
people will mute.

## Writing your own

Start from the [Custom rules guide](https://argot.tmonier.com/docs/custom-rules/),
or ask a coding agent with the `argot-write-rule` skill. Two things the guide
says that the examples show:

- **Fixtures before script.** A rule that has only ever been proven to fire has
  never been checked for false alarms.
- **Scope in the manifest, not the script.** `include`/`exclude`/`languages`
  say which files run; `check.rhai` says what the pattern is.
