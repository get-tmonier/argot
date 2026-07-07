---
title: Languages
description: Tree-sitter tokenization, supported languages, and per-language calibration for monorepos.
group: Reference
order: 11
---

argot is **language-agnostic by design**. All language-specific logic — import extraction, callee
extraction, prose masking, sampleable-range enumeration — is encapsulated in `LanguageAdapter`
implementations. Nothing in the scorer hardcodes a framework, a library, or a corpus.

## Tree-sitter tokenization

Hunks are tokenized with [tree-sitter](https://tree-sitter.github.io/tree-sitter/), an incremental,
error-tolerant parser. Two properties matter here:

- It works on **partial, syntactically invalid fragments** — essential, because a diff hunk is almost
  always a mid-block slice that doesn't parse on its own.
- It gives a **single uniform interface** for every supported language, so adding a language is a
  matter of writing an adapter, not a new pipeline.

## Supported out of the box

Python, TypeScript, JavaScript, Go, Rust, Java, C#, C, C++, Ruby, and PHP —
eleven languages, each with its own tree-sitter adapter, each benchmarked on a
real open-source corpus. TypeScript and JavaScript are **separate** adapters and
separate models: `.ts`/`.tsx` and `.js`/`.jsx` are written differently, so argot
learns each voice on its own (and treats a TypeScript repo's transpiled `.js`
output as generated, not authored). We publish the leak-free per-corpus numbers —
catch rate and false alarms, with commit-level confidence intervals — on the
[benchmarks page](/benchmarks). Nothing is hidden.

More languages are adapter-shaped work — the model and pipeline don't change.

## Per-language calibration

A mixed Python + TypeScript monorepo would, with a single threshold, calibrate against a *joint*
distribution dominated by whichever language has broader token diversity. argot instead emits **one
threshold per language** present in the repo, and `check` dispatches each hunk by file extension.

So a TypeScript hunk is judged against the repo's TypeScript voice, and a Python hunk against its
Python voice — no cross-language bleed. This is automatic; there's nothing to configure.

## What gets excluded

The token model only makes sense over code, so a few things are kept out of both training and
scoring:

- **Data-dominant files** — modules that are ≥80% top-level array/object literals (locale tables,
  fixtures, generated lookups). The same structural predicate runs at fit and check time.
- **Comments and docstrings** — blanked before scoring, so prose doesn't inflate the surprise signal.
- **Test files and conventional directories** — skipped by the built-in `argot:recommended` set, which
  you can extend or replace with `argot.toml`'s `[exclude]`. See [Configure](/docs/configure/).
