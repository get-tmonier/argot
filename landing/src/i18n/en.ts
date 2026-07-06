import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — lint the rules you never wrote down',
    description:
      'argot is a guardrail for AI-written code. It learns your repo’s patterns from its own git history, then flags the dependencies, APIs, and constructs it has never seen — the “unknown to this repo” code an AI agent reaches for when it doesn’t know your stack. No model, no cloud, no GPU.',
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
      'argot learns your repo’s patterns from its own git history, then flags code foreign to your codebase — the dependencies and APIs an AI agent drags in that your repo has never used. [[No model. No cloud. No GPU.]]',
    ctaPrimary: 'Read the docs',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · single static binary · macOS & Linux · no runtime deps',
  },
  demo: {
    label: 'The second question',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'Linters and type checkers answer “is this valid?” They can’t answer “is this how this team writes things?” That used to live in code review — until an LLM could bury it under a hundred clean, type-correct PRs in an afternoon. [[argot is the layer that asks it back.]]',
    codeTitle: 'routers/receipts.py',
    caption:
      'Every other endpoint in this repo is a typed FastAPI function with Depends. This one is a Django class-based view — View, JsonResponse, HttpResponseNotFound. Valid Python; mypy and ruff are happy. No linter knows this repo never writes Django. [[argot flags the foreign paradigm.]]',
  },
  catches: {
    label: 'What it catches',
    title: 'Technically fine. Socially foreign.',
    body: 'argot does not replace ESLint, ruff, or your type checker. It catches what they can’t articulate: a dependency, API, or whole paradigm [[the repo has never used]] — the code an agent reaches for when it doesn’t know your stack. And it’s honest about the one line it won’t cross.',
    items: [
      {
        title: 'A foreign dependency',
        desc: 'An import — a package, module, or header — the repo has never used. The clearest signal, and the one argot catches most reliably.',
      },
      {
        title: 'A foreign API',
        desc: 'A call into a library the codebase standardises away from — a different HTTP client, ORM, or serializer than the rest of the repo reaches for. The tell is the call, not just the import.',
      },
      {
        title: 'A foreign paradigm',
        desc: 'A whole idiom from another framework — a Django-style class view, a Flask route, hand-rolled validation — dropped into a codebase that has never written that way.',
      },
      {
        title: 'The line it won’t cross',
        desc: 'A wrong exception or value where [[every token is already yours]] — a choice, not a foreign pattern. argot surfaces these only sometimes, never gates on them, and tells you so.',
      },
    ],
  },
  proof: {
    label: 'Measured, not promised',
    title: 'Honest numbers, leak-free by construction.',
    stats: [
      {
        value: '99%',
        title: 'visible-foreign catch',
        desc: 'The one signal argot is built for — a foreign import, API, or dependency your repo has never used. When it shows in the code, argot catches [[522 of 527]], spliced into real files and judged by the real fit → check pipeline.',
      },
      {
        value: '0.23%',
        title: 'false alarms on real edits',
        desc: 'How often argot fires on your repo’s [[own existing code]] — replaying 27 repos’ commits it never trained on. Every corpus stays ≤ 0.98%. A fire on a genuinely new dependency is a [[detection]], not an alarm.',
      },
      {
        value: '150ms',
        title: 'to check a change',
        desc: 'Fast enough for a [[pre-commit hook]], on a 34k-file repo, laptop CPU. The one-time fit that learns your repo’s voice takes ~7 s. [[No GPU, no cloud.]]',
      },
      {
        value: '10',
        title: 'languages, one binary',
        desc: 'Python, TypeScript, Go, Rust, Java, C#, C, C++, Ruby, PHP — from a [[single static binary]], nothing to install. Mixed monorepos get [[one threshold per language]].',
      },
    ],
    finePrint:
      'Leak-free protocol (issue #92): recall from fixtures planted into real files and judged by the shipped binary; false alarms from a temporal holdout with commit-level bootstrap confidence intervals. Full per-repo numbers and methodology on the benchmarks page.',
  },
  setup: {
    label: 'Setup in one command',
    title: 'From clone to calibrated in one line.',
    body: 'argot init learns your repo’s voice and tells you if it’s ready — no config, no annotations. Messy repo? An AI agent (or argot init --suggest) picks out the generated and vendored directories that shouldn’t shape your voice. [[The model is a build artifact — argot keeps it out of git for you.]]',
    caption: 'One command. It even keeps the rebuildable model out of your git history.',
    ctaLocal: 'Set it up with one prompt',
    ctaCi: 'or one prompt for CI',
  },
  agents: {
    label: 'Built for AI agents',
    title: 'Your agent writes the code. argot keeps it in voice.',
    body: 'Most code argot judges is now written by an agent — so give the agent the guardrail, locally and in CI. Three skills wire it in; each surfaces anything foreign — [[advisory, never blocking]] — and MCP feeds it your repo’s idioms before it writes a line.',
    cards: [
      {
        title: 'argot-setup · local',
        desc: 'Fits the voice model, and picks out what shouldn’t shape it.',
      },
      {
        title: 'argot-check · local',
        desc: 'Scores the diff as the agent works — advisory, never blocking.',
      },
      {
        title: 'argot-ci · CI',
        desc: 'Wires the GitHub Action — a voice score on every PR, no local setup.',
      },
      {
        title: 'MCP · voice_context',
        desc: 'Feeds the repo’s idioms before the agent generates a line.',
      },
    ],
    caption:
      'Local or CI, it never blocks a commit or rewrites your code. It surfaces — you decide.',
  },
  ciScore: {
    label: 'In CI, without the friction',
    title: 'A voice score on every PR. Never a merge gate.',
    body: 'Like a security check, argot decorates each pull request with a visual score and the hot-spots — [[advisory by default]]. Intentional? One argot mute accepts it, with an audit trail. The reviewer always has the last word.',
    caption:
      'The same score lands in the Actions summary, a sticky PR comment, and the Security tab.',
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
        desc: 'Runs on every PR and posts a [[visual voice score]] — advisory by default, never a merge gate. Job summary, sticky comment, and SARIF annotations.',
      },
      {
        title: 'Incremental, not a rewrite',
        desc: 'Point it at a repo, run [[argot init]] once, then check forever. No annotations, no config to get started.',
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
