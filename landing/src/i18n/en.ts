import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — catch the AI code that doesn’t fit your codebase',
    description:
      'argot is a local guardrail for AI-written code. It learns your repo’s patterns from its own git history, then flags code that doesn’t belong — a dependency you’ve never used, a function you already wrote, logic in the wrong place, an import that breaks your layering. Backed by a code-embedding model that runs on your laptop. No LLM, no cloud, no GPU.',
  },
  nav: {
    demo: 'Demo',
    engine: 'Under the hood',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Guardrail for AI-written code · learned from your git history · 100% local',
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
    body: 'An LLM buries that question under clean, type-correct PRs. argot asks it at the diff — [[this is its real output]].',
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
      {
        id: 'test-disabled',
        label: 'test-disabled',
        caption:
          'A failing test goes green because it was skipped, not fixed. argot pairs the disabled test with the production change it covers, and names both.',
      },
    ],
    seeLive: 'See it on real repos',
  },
  replay: {
    label: 'Day one',
    title: 'Rewind your history. See what it would have caught.',
    body: 'You can’t demo a guardrail on code it just learned from — so argot rewinds instead. [[argot replay]] fits the voice as it was 50 commits ago and rescores everything since: one command, seconds, your tree untouched.',
    caption:
      'Real run on FastAPI’s history: two newly-adopted imports and the new streaming vocabulary — each with [[the repo’s own evidence]].',
  },
  engine: {
    label: 'Under the hood',
    title: 'Semantic understanding. No LLM anywhere.',
    body: 'Four engines behind the five detectors, one static [[Rust]] binary, all learned from your git history — no API key, no GPU, nothing leaves your machine.',
    cards: [
      {
        title: 'A code-embedding model on your laptop',
        desc: 'jina-code (~100 MB, fetched once) turns every function into a vector. That’s how argot knows you [[already wrote this]] — and where it belongs. An encoder, not an LLM: no generation, CPU-first, Metal-accelerated on Macs.',
      },
      {
        title: 'A statistical voice model',
        desc: 'Two frequency tables and a callee-cluster partition, learned from your history — the imports, callees, and token shapes your repo [[actually uses]]. No neural net needed to know django doesn’t belong here.',
      },
      {
        title: 'An architecture graph',
        desc: 'Your module-dependency topology, fitted from your own imports: which layers point at which. A new edge that [[reverses the established direction]] is flagged with the direction it breaks.',
      },
      {
        title: 'A test-inventory diff',
        desc: 'tree-sitter parses every test file at each commit and tracks what each one asserts. When a production change lands beside a test that’s [[skipped, gutted, or deleted]], argot pairs the two and names the test — no model, just a structural diff of the suite.',
      },
    ],
    stats: [
      { value: '0.2s', label: 'to check a diff' },
      { value: '0.6s', label: 'when it defines new functions' },
      { value: '25s', label: 'first fit, 1,100-file repo' },
      { value: '4s', label: 'to refresh — unchanged functions reuse their embeddings' },
    ],
    finePrint:
      'Measured on FastAPI, laptop CPU. Single static binary — no Python, no Node, no runtime to install.',
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
      {
        value: '94%',
        title: 'test-gaming edits caught',
        desc: 'Skipping, gutting, or deleting a test to green a failing suite: 144 of 153 authored edits caught, 0 of 102 legitimate refactors flagged — just [[1.24% flagged on real accepted commits]].',
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
