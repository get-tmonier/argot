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
    body: 'Linters and type checkers answer “is this valid?” They can’t answer “is this how this team writes things?” That used to live in code review — until an LLM could bury it under a hundred clean, type-correct PRs in an afternoon. [[argot is the layer that asks it back.]]',
    codeTitle: 'routers/receipts.py',
    caption:
      'Every other endpoint in this repo is a typed FastAPI function with Depends. This one is a Django class-based view — View, JsonResponse, HttpResponseNotFound. Valid Python; mypy and ruff are happy. No linter knows this repo never writes Django. [[argot flags the foreign paradigm.]]',
    seeLive: 'See it work on real repos — FastAPI, Saleor, Hono',
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
