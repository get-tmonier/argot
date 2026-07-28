---
title: Limitations
description: "What argot deliberately does not prove: fit suitability, in-vocabulary choices, masked content, and changes outside the checked range."
group: Reference
order: 14
---

argot is a probabilistic review guardrail, not a correctness oracle. A finding
is evidence to inspect; a clean run means only that no configured detector found
a pattern in the checked changeset.

## The model needs a suitable fit

`argot inspect` reports whether the repository has enough eligible history and
whether the fitted corpus is suitable. A **Not recommended** verdict means the
learned model is not calibrated enough to treat its hits as strong evidence.
Generated, vendored, data-heavy, unsupported, or shallow-history repositories
can reduce what the model learns. Use `argot init --suggest`, review exclusions,
or use conventional review until the repository has a sound fit.

## Familiar vocabulary can still be wrong

Argot is strongest when a change introduces a pattern foreign to the repository.
It does not reliably catch an error built entirely from vocabulary already used
there — for example, choosing the wrong familiar exception type or calling an
existing API in the wrong place. The semantic and architecture checks narrow
different gaps; neither turns a clean check into proof of correctness.

## Some input is intentionally masked or outside scope

The voice scorer masks prose such as comments and docstrings, and its normal
source-language scope excludes configured generated, data, and non-source
content. Custom rules can deliberately cover additional paths. Argot also judges
the diff or range you give it: unchanged code, an omitted ref range, and events
outside that range are not retrospectively validated.

## Optional capability limits

Semantic checks need a local embedding model and the semantic feature in the
binary. With `ARGOT_OFFLINE=1` and no cached/local model, `redundant` and
`misplaced` are skipped with a diagnostic while the remaining checks continue.
Architecture and integrity checks likewise depend on their shipped feature and
fit artifacts. Confirm the live rule registry and fit health with `argot rules`
and `argot inspect` before relying on a specific detector.

`misplaced` also abstains where it cannot answer honestly: on a body that calls
nothing (a property setter is written out of the names it assigns, so it reads
as whatever unit owns them, not as code in the wrong place), and on any function
recovered from inside a region the parser could not read. The second is not
rare on every grammar — on one real 924k-line Object Pascal tree a single
unparsed construct put 23% of the repository's functions out of reach for both
`misplaced` and `redundant`. Neither rule reports a guess in that state, so
"nothing found" there means "not judged", not "judged clean".
