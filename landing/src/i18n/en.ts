import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — lint the rules you never wrote down',
    description:
      'argot is a voice linter. It learns your repo’s voice from its own git history, then flags the hunks that don’t sound like anyone on your team wrote them. No model, no cloud, no GPU.',
  },
  nav: {
    demo: 'Demo',
    catches: 'What it catches',
    local: 'Local',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Local · single Rust binary · no LLM',
    titleLead: 'Lint the rules',
    titleGradient: 'you never wrote down.',
    subtitle:
      'argot learns your repo’s voice from its own git history, then flags the hunks that don’t sound like anyone on your team wrote them. [[No model. No cloud. No GPU.]]',
    ctaPrimary: 'Read the docs',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · single static binary · macOS & Linux · no runtime deps',
  },
  demo: {
    label: 'The second question',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'Linters and type checkers answer “is this valid?” They can’t answer “is this how this team writes things?” That used to live in code review — until an LLM could bury it under a hundred clean, type-correct PRs in an afternoon. [[argot is the layer that asks it back.]]',
    codeTitle: 'routers/users.py',
    caption:
      'Decorators, Depends, the typed return — all idiomatic FastAPI. The one break is a bare ValueError where this repo always raises HTTPException. mypy is happy. The linter has nothing. [[argot flags the line.]]',
  },
  catches: {
    label: 'What it catches',
    title: 'Technically fine. Socially wrong.',
    body: 'argot does not replace ESLint, ruff, or your type checker. It catches the things they can’t articulate — the patterns your team agreed on [[by repetition]], never by writing them down.',
    items: [
      {
        title: 'LLM paste-through',
        desc: 'A block whose style diverges sharply from the surrounding file — fluent in the average voice of every public repo, not yours.',
      },
      {
        title: 'Convention drift',
        desc: 'Error handling, logging, or control-flow shapes that don’t match how the rest of the codebase does it.',
      },
      {
        title: 'Foreign paradigm',
        desc: 'Class-based OOP dropped into a functional codebase. A sync def on a hot async path. The wrong import for the job.',
      },
      {
        title: 'Stylistic outlier',
        desc: 'Code that’s correct, typed, and lint-clean — but doesn’t sound like anyone on this team wrote it.',
      },
    ],
  },
  local: {
    label: 'How it stays honest',
    title: 'Two frequency tables. No neural network.',
    body: 'argot builds two token distributions — one from your repo, one from a generic open-source baseline — and flags hunks far more likely under the generic one. [[That is the whole model.]] It fits on CPU in seconds and ships its threshold per repo.',
    points: [
      {
        title: 'Nothing leaves your machine',
        desc: 'No GPU, no cloud, no telemetry. The model is two frequency tables and a max log-ratio — it fits in seconds and scores in milliseconds.',
      },
      {
        title: 'Calibrated per repo',
        desc: 'The threshold is set from your own code, so “normal” means normal here — not the average of every public repo a model trained on.',
      },
      {
        title: 'Language-aware, not language-locked',
        desc: 'A tree-sitter tokenizer parses partial, invalid hunks. Python and TypeScript out of the box; mixed monorepos get one threshold per language.',
      },
      {
        title: 'Evidence, not vibes',
        desc: 'Every flag names the tokens that carried the score, how often they appear in your repo, and what’s common here instead.',
      },
    ],
  },
  features: {
    label: 'Why argot',
    title: 'Reads like a linter. Thinks like a reviewer.',
    items: [
      {
        title: 'One fast Rust binary',
        desc: 'A single statically-linked binary — no Python, no Node, no runtime to install, no model to download. [[extract 5× faster, check ~23× faster]] than the previous engine, with instant startup and byte-for-byte identical results.',
      },
      {
        title: 'Drops into CI',
        desc: 'argot check runs on every commit, groups hits by file, and [[exits non-zero]] when something diverges. Wire it in like ESLint.',
      },
      {
        title: 'Incremental, not a rewrite',
        desc: 'Point it at a repo, run extract → fit [[once]], then check forever. No annotations, no config to get started.',
      },
      {
        title: 'Per-hunk evidence',
        desc: 'Each hit shows the offending tokens with their repo attestation — startedAt (0×) vs use (88×) — and the repo’s typical vocabulary instead.',
      },
      {
        title: 'Severity you can tune',
        desc: 'unusual · suspicious · foreign, relative to the calibrated threshold. Filter the noise with [[--min-severity]].',
      },
      {
        title: 'Per-language calibration',
        desc: 'A Python + TypeScript monorepo gets one threshold per language, dispatched by file extension. No single distribution to dominate the other.',
      },
      {
        title: 'Honest about itself',
        desc: 'Public benchmarks, a 35-doc research log, and a [[probabilistic-linter]] disclaimer printed in every run. Verify before you act.',
      },
    ],
  },
  cta: {
    title: 'Add the layer your CI is missing.',
    body: 'argot is MIT and alpha. Calibrate it on your repo in a couple of minutes, then see what it flags.',
    primary: 'Get started',
    secondary: 'View on GitHub',
  },
  footer: {
    tagline: 'A voice linter for the unwritten rules.',
    builtBy: 'Built by Damien Meur',
    docs: 'Docs',
    npm: 'npm',
  },
};

export default en;
