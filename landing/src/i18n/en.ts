import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — catch the AI code that doesn’t fit your codebase',
    description:
      'argot is a local guardrail for AI-written code. It learns your repo’s patterns from its own git history, then flags code that doesn’t belong — a dependency you’ve never used, a function you already wrote, logic in the wrong place, an import that breaks your layering. Backed by a code-embedding model that runs on your laptop. No LLM, no cloud, no GPU.',
  },
  nav: {
    demo: 'Demo',
    catches: 'What it catches',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Local embedding model · Rust single binary · no LLM',
    titleLead: 'Lint the rules',
    titleGradient: 'you never wrote down.',
    subtitle:
      'It flags the AI code that doesn’t fit your repo — a dependency you’ve never used, a function you [[already have]], logic in the wrong place, an import that breaks your layering.',
    ctaPrimary: 'Read the docs',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · single static binary · macOS · Linux · Windows · 100% local',
    installAlt: 'or install without npm',
  },
  demo: {
    label: 'The second question',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'Linters ask “is this valid?” — never “is this how we write things?” An LLM buries that under clean, type-correct PRs. [[argot asks it back.]]',
    caption:
      'Two real checks: a foreign framework, and a function the repo [[already had]]. mypy and ruff pass both — argot doesn’t.',
    seeLive: 'See it on real repos',
  },
  catches: {
    label: 'What it catches',
    title: 'It compiles. It’s typed. It still doesn’t fit.',
    body: 'Four detectors, all learned from your git history — none of this visible to ESLint, ruff, or a type checker. And one honest line it won’t cross: a wrong choice made only of tokens that are [[already yours]] is a choice, not a foreign pattern.',
    items: [
      {
        title: 'Foreign',
        desc: 'A dependency, API, or idiom [[the repo has never used]]. The clearest signal — caught most reliably.',
      },
      {
        title: 'Redundant',
        desc: 'A new function that [[reinvents one you already have]]. argot finds the original and shows you where it lives.',
      },
      {
        title: 'Misplaced',
        desc: 'The right code, filed in the [[wrong package]] — its nearest kin all live somewhere else.',
      },
      {
        title: 'Layering',
        desc: 'An internal import that [[reverses your architecture]] — a boundary this repo never crosses. 96.8% caught, zero false positives.',
      },
    ],
  },
  proof: {
    label: 'Measured, not promised',
    title: 'Honest numbers, leak-free by construction.',
    stats: [
      {
        value: '98%',
        title: 'visible-foreign catch',
        desc: 'When the foreign symbol is [[visible]] in the diff — an explicit import or call — judged by the shipped binary: 604 of 618.',
      },
      {
        value: '96.8%',
        title: 'architecture violations caught',
        desc: 'Planted layering violations (244 of 252) at [[zero false positives]] — 0 of 140 control edits flagged.',
      },
      {
        value: '0.22%',
        title: 'false alarms on real edits',
        desc: 'How often argot fires on your repo’s [[own existing code]] — 49 of 22,785 real hunks; 31 repos, every one ≤ 1.17%.',
      },
      {
        value: '150ms',
        title: 'to check a change',
        desc: 'On a 34k-file repo, laptop CPU. The one-time fit takes ~7 s. [[No GPU, no cloud.]]',
      },
    ],
    languages:
      'One [[static binary]], 11 languages: Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP.',
    finePrint:
      'Leak-free by construction: recall on foreign patterns planted in real files; false alarms on a temporal holdout. The one thing a voice model structurally can’t see — masked foreign — is published on the benchmarks page, not hidden.',
    benchmarksCta: 'Full per-repo numbers →',
  },
  setup: {
    label: 'Setup · built for agents',
    title: 'A CLI your coding agent can drive.',
    body: 'The skills run argot [[and bring the judgment]]: /argot-setup reads your repo to decide what shouldn’t shape its voice — a vendored SDK, a generated dir — writes an argot.toml, fits, and verifies the catch. Advisory, never blocking.',
    installLabel: 'Add the skills — Claude Code, Cursor, 70+ agents',
    skillsIntro: 'four slash-commands your agent runs:',
    skillDescs: [
      'reads your tree, writes argot.toml, verifies the catch',
      'scores each diff, flags what’s foreign — never blocks',
      'reviews one PR against your repo’s voice, no checkout',
      'a non-blocking voice score on every PR',
    ],
    ctaLocal: 'Or drive the CLI by hand',
    ctaCi: 'the CI guide',
    caption:
      'The skills bring the exclude-what-isn’t-yours judgment; the fitted model stays out of your git history.',
  },
  ciScore: {
    label: 'In CI, without the friction',
    title: 'A voice score on every PR. Never a merge gate.',
    body: 'A visual score and the hot-spots on each PR — [[non-blocking by default]]. Intentional? One argot mute, committed as an audit trail.',
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
