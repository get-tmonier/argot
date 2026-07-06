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
    body: 'Linters answer “is this valid?” — never “is this how we write things?” That lived in code review, until an LLM could bury it under a hundred clean, type-correct PRs. [[argot asks it back.]]',
    caption:
      'A Django-style view in an all-FastAPI repo — a framework this codebase has never imported. mypy and ruff pass; no linter says a word. [[argot flags it in ~150 ms.]]',
    seeLive: 'See it work on real repos — FastAPI, Saleor, Hono',
  },
  catches: {
    label: 'What it catches',
    title: 'Technically fine. Socially foreign.',
    body: 'Not a replacement for ESLint, ruff, or your type checker — it catches what they can’t articulate: a dependency, API, or paradigm [[the repo has never used]]. And it’s honest about the one line it won’t cross.',
    items: [
      {
        title: 'A foreign dependency',
        desc: 'An import the repo has never used. The clearest signal — and the one argot catches most reliably.',
      },
      {
        title: 'A foreign API',
        desc: 'A call into a library the codebase avoids — a different HTTP client, ORM, or serializer than the rest of the repo. The tell is the call, not just the import.',
      },
      {
        title: 'A foreign paradigm',
        desc: 'A whole idiom from elsewhere — a Django-style view, a Flask route, hand-rolled validation — in a codebase that never writes that way.',
      },
      {
        title: 'The line it won’t cross',
        desc: 'A wrong exception where [[every token is already yours]] — a choice, not a foreign pattern. argot won’t reliably catch these, never gates on them, and says so.',
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
        desc: 'The signal argot is built for. Foreign imports and APIs spliced into real files, judged by the shipped binary: [[522 of 527]] caught.',
      },
      {
        value: '0.23%',
        title: 'false alarms on real edits',
        desc: 'How often argot fires on your repo’s [[own existing code]] — replaying 27 repos it never trained on. Every corpus stays ≤ 0.98%.',
      },
      {
        value: '150ms',
        title: 'to check a change',
        desc: 'On a 34k-file repo, laptop CPU — fast enough for a [[pre-commit hook]]. The one-time fit takes ~7 s. [[No GPU, no cloud.]]',
      },
      {
        value: '10',
        title: 'languages, one binary',
        desc: 'Python, TypeScript, Go, Rust, Java, C#, C, C++, Ruby, PHP — one [[static binary]]. Mixed monorepos get [[a threshold per language]].',
      },
    ],
    finePrint:
      'Leak-free protocol (issue #92): recall from fixtures planted into real files and judged by the shipped binary; false alarms from a temporal holdout with commit-level bootstrap confidence intervals. Full per-repo numbers and methodology on the benchmarks page.',
  },
  setup: {
    label: 'Setup in one command',
    title: 'From clone to calibrated in one line.',
    body: 'argot init learns your repo and tells you if it’s ready — no config, no annotations. Messy repo? argot init --suggest (or an agent) picks out the generated and vendored dirs that shouldn’t shape it. [[The model is a build artifact — argot keeps it out of git for you.]]',
    caption: 'One command. It even keeps the rebuildable model out of your git history.',
    ctaLocal: 'Set it up with one prompt',
    ctaCi: 'or one prompt for CI',
  },
  agents: {
    label: 'Built for AI agents',
    title: 'Your agent writes the code. argot keeps it in voice.',
    body: 'Most code argot judges is now written by an agent — so give the agent the guardrail. Three skills wire it in, [[advisory, never blocking]]; MCP feeds it your repo’s idioms before it writes a line.',
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
    body: 'Like a security check, argot decorates each PR with a visual score and the hot-spots — [[advisory by default]]. Intentional? One argot mute accepts it, with a committed audit trail. The reviewer has the last word.',
    caption:
      'The same score lands in the Actions summary, a sticky PR comment, and the Security tab.',
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
