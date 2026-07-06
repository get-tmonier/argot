import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — lint the rules you never wrote down',
    description:
      'argot is a guardrail for AI-written code. It learns your repo’s patterns from its own git history, then flags the dependencies, APIs, and constructs it has never used — the “unknown to this repo” code an AI agent reaches for. No model, no cloud, no GPU.',
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
      'It flags code foreign to your repo — the dependencies and idioms an AI writes that your codebase has [[never used]].',
    ctaPrimary: 'Read the docs',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · single static binary · macOS & Linux · no runtime deps',
  },
  demo: {
    label: 'The second question',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'Linters ask “is this valid?” — never “is this how we write things?” An LLM buries that under clean, type-correct PRs. [[argot asks it back.]]',
    caption: 'A Django view in an all-FastAPI repo. mypy and ruff pass — [[argot flags it in ~150 ms.]]',
    seeLive: 'See it on real repos',
  },
  catches: {
    label: 'What it catches',
    title: 'Technically fine. Socially foreign.',
    body: 'What ESLint, ruff, and type checkers can’t articulate: a dependency, API, or paradigm [[the repo has never used]].',
    items: [
      {
        title: 'A foreign dependency',
        desc: 'An import the repo has never used. The clearest signal — caught most reliably.',
      },
      {
        title: 'A foreign API',
        desc: 'A call into a library the rest of the codebase avoids.',
      },
      {
        title: 'A foreign paradigm',
        desc: 'A whole idiom from elsewhere — a Django view in a FastAPI repo.',
      },
      {
        title: 'The line it won’t cross',
        desc: 'A wrong exception where [[every token is already yours]]. argot won’t gate on a choice — and says so.',
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
        desc: 'Foreign imports and APIs spliced into real files, judged by the shipped binary: [[522 of 527]].',
      },
      {
        value: '0.23%',
        title: 'false alarms on real edits',
        desc: 'How often argot fires on your repo’s [[own existing code]] — 27 repos, every one ≤ 0.98%.',
      },
      {
        value: '150ms',
        title: 'to check a change',
        desc: 'On a 34k-file repo, laptop CPU. The one-time fit takes ~7 s. [[No GPU, no cloud.]]',
      },
      {
        value: '10',
        title: 'languages, one binary',
        desc: 'Python · TypeScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP — one [[static binary]].',
      },
    ],
    finePrint:
      'Leak-free protocol (issue #92): recall from fixtures planted into real files and judged by the shipped binary; false alarms from a temporal holdout with commit-level bootstrap CIs. Full per-repo numbers on the benchmarks page.',
  },
  setup: {
    label: 'Setup in one command',
    title: 'From clone to calibrated in one line.',
    body: 'argot init learns your repo and tells you if it’s ready — no config, no annotations. [[argot init --suggest]] handles the messy ones.',
    caption: 'One command. The model stays out of your git history.',
    ctaLocal: 'Set it up with one prompt',
    ctaCi: 'or one prompt for CI',
  },
  agents: {
    label: 'Built for AI agents',
    title: 'Your agent writes the code. argot keeps it in voice.',
    body: 'The code argot judges is written by agents — so give the agent the guardrail. [[Advisory, never blocking.]]',
    cards: [
      {
        title: 'argot-setup · local',
        desc: 'Fits the model; picks what shouldn’t shape it.',
      },
      {
        title: 'argot-check · local',
        desc: 'Scores the diff as the agent works.',
      },
      {
        title: 'argot-ci · CI',
        desc: 'Wires the Action — a score on every PR.',
      },
      {
        title: 'MCP · voice_context',
        desc: 'Feeds your idioms before the agent writes.',
      },
    ],
    caption: 'It surfaces — you decide. Never blocks, never rewrites.',
  },
  ciScore: {
    label: 'In CI, without the friction',
    title: 'A voice score on every PR. Never a merge gate.',
    body: 'A visual score and the hot-spots on each PR — [[advisory by default]]. Intentional? One argot mute, committed as an audit trail.',
    caption: 'Lands in the Actions summary, a sticky PR comment, and the Security tab.',
  },
  cta: {
    title: 'Add the layer your CI is missing.',
    body: 'MIT · alpha. Calibrate on your repo in two minutes, then see what it flags.',
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
