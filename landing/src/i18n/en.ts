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
      'AI writes valid code that isn’t [[yours]]. argot learns your repo’s voice from its git history — and flags what doesn’t belong, before it merges.',
    ctaPrimary: 'Read the docs',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · single static binary · macOS · Linux · Windows · 100% local',
    installAlt: 'or install without npm',
  },
  demo: {
    label: 'The second question',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'mypy asks if it compiles. ruff asks if it’s tidy. Neither asks the review question: [[is this how we do it here?]] Every example below passes both — argot catches all four.',
    tabs: [
      {
        id: 'foreign-import',
        label: 'foreign-import',
        caption:
          'A Django view in an all-FastAPI repo — valid Python, but a framework this repo has never imported. The evidence shows what the repo reaches for instead.',
      },
      {
        id: 'redundant',
        label: 'redundant',
        caption:
          'The repo already has this function. argot names the original, where it lives, and how close the match is — use it instead of merging a twin.',
      },
      {
        id: 'misplaced',
        label: 'misplaced',
        caption:
          'Downloader logic filed under cli/commands — its nearest peers all live in core/downloader. Right code, wrong home.',
      },
      {
        id: 'layering',
        label: 'layering',
        caption:
          'In this repo, cli imports core — never the other way. This one import quietly reverses the architecture; argot flags the edge itself.',
      },
    ],
    seeLive: 'See it on real repos',
  },
  catches: {
    label: 'What it catches',
    title: 'It compiles. It’s typed. It still doesn’t fit.',
    body: 'Four detectors, all learned from your git history — none of this visible to ESLint, ruff, or a type checker. And one honest line it won’t cross: a wrong choice made only of tokens that are [[already yours]] is a choice, not a foreign pattern.',
    items: [
      {
        title: 'Your agent just imported a framework this repo has never touched.',
        desc: 'argot flags the dependency and shows what the repo reaches for [[instead]].',
      },
      {
        title: 'Your agent just merged a second implementation of slugify.',
        desc: 'argot finds the original and shows you [[where it already lives]].',
      },
      {
        title: 'Your agent just filed downloader logic under cli/commands.',
        desc: 'argot points to [[where its nearest kin already live]].',
      },
      {
        title: 'Your agent just let cli reach straight into core — backwards.',
        desc: 'argot flags [[the edge that reverses your architecture]].',
      },
    ],
  },
  proof: {
    label: 'Measured, not promised',
    title: 'Honest numbers, leak-free by construction.',
    stats: [
      {
        value: '98%',
        title: 'foreign patterns caught',
        desc: 'A dependency or API the repo never uses: 604 of 618 caught — while firing on just [[0.22% of real edits]] (49 of 22,785 hunks; worst repo 1.17%).',
      },
      {
        value: '94%',
        title: 'reinventions caught · median',
        desc: '85–100% per repo: faithful rewrites of the repo’s [[own functions]], planted as new code and traced back to the original. False-fire ≤ 2.8% of hunks.',
      },
      {
        value: '96%',
        title: 'misplacements caught · median',
        desc: '86–99% wherever the repo has a separable architecture — and it [[abstains]] where there is none, instead of guessing.',
      },
      {
        value: '96.8%',
        title: 'architecture violations caught',
        desc: 'Layering reversals: 244 of 252 caught at [[zero false positives]] — 0 of 140 control edits flagged.',
      },
    ],
    languages:
      'One [[static binary]], 11 languages: Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP.',
    finePrint:
      'Leak-free by construction: recall on foreign patterns planted in real files; false alarms on a temporal holdout. The one thing a voice model structurally can’t see — masked foreign — is published on the benchmarks page, not hidden. Speed, measured on FastAPI (1,100+ files, laptop CPU): check ~0.2 s (~0.6 s when the diff defines new functions) · first fit ~25 s · refresh ~4 s.',
    benchmarksCta: 'Full per-repo numbers →',
  },
  setup: {
    label: 'Setup · built for agents',
    title: 'A CLI your coding agent can drive.',
    body: 'The skills run argot [[and bring the judgment]]: /argot-setup reads your repo to decide what shouldn’t shape its voice — a vendored SDK, a generated dir — writes an argot.toml, fits, and verifies the catch. Informational, never blocking.',
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
