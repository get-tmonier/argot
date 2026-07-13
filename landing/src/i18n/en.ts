import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — catch the AI code that doesn’t fit your codebase',
    description:
      'The harness for AI-written code — statistics on your repo’s own history, not a second LLM. argot flags what doesn’t belong — foreign dependencies, reinvented functions, broken layering, gamed tests — plus the conventions you script yourself. No LLM, no cloud, no GPU required.',
  },
  nav: {
    demo: 'Demo',
    audit: 'Audit',
    engine: 'Under the hood',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'The harness for AI-written code · statistics, not a second LLM · 100% local',
    titleLead: 'Lint the rules',
    titleGradient: 'you never wrote down.',
    subtitle:
      'AI writes the code. argot harnesses it with the one thing that can’t hallucinate: [[your repo’s own history]]. Deterministic, measured, local.',
    ctaPrimary: 'Read the docs',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote: 'MIT · single static binary · macOS · Linux · Windows · 100% local',
    installAlt: 'or install without npm',
  },
  demo: {
    label: 'The second question',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'Clean, type-correct PRs bury that question. argot answers it at the diff — [[this is its real output]].',
    tabs: [
      {
        id: 'foreign-import',
        label: 'foreign-import',
        caption:
          'Valid Python — but a framework this repo has never imported. The evidence shows what it reaches for instead.',
      },
      {
        id: 'redundant',
        label: 'redundant',
        caption: 'The repo already has this function. argot names the original and the similarity.',
      },
      {
        id: 'misplaced',
        label: 'misplaced',
        caption: 'Right code, wrong home — its nearest peers all live in core/downloader.',
      },
      {
        id: 'layering',
        label: 'layering',
        caption:
          'Here, cli imports core — never the reverse. This one import flips the architecture.',
      },
      {
        id: 'test-disabled',
        label: 'test-disabled',
        caption:
          'Green because it was skipped, not fixed. argot names the test and the code it covers.',
      },
    ],
    seeLive: 'See it on real repos',
  },
  trust: {
    label: 'The other failure mode',
    title: 'An agent that can’t fix the code will “fix” the test.',
    body: 'The diff looks tidy, CI turns green — and your safety net has a hole [[exactly where the code is newest]]. argot pairs the weakened test with the code it covers, and names both.',
    moves: [
      { name: 'skip it', example: '@pytest.mark.skip("flaky")' },
      { name: 'gut it', example: 'assertions removed, test kept' },
      { name: 'retarget it', example: 'expected 429 → becomes 200' },
      { name: 'delete it', example: 'test gone, code stays' },
    ],
    caption:
      '[[94%]] of authored gaming edits caught · 0 of 102 legitimate refactors flagged · ships as warn — informs, never blocks.',
  },
  audit: {
    label: 'Day one',
    title: 'Audit your history. See what AI snuck in.',
    body: '[[argot audit]] fits the voice as of 50 commits ago, rescores everything since, and attributes each finding — [[ai-assisted, human, or unknown]] — from commit markers, never style. One command, zero setup, your tree untouched.',
    caption:
      'On argot’s own history: [[52%]] of commits carry AI markers — and the one finding traces to an AI-assisted commit.',
  },
  customRules: {
    label: 'Your conventions',
    title: 'The sixth detector is yours.',
    body: 'Five detectors learn your repo. The sixth you write: [[a manifest and a tiny script]] in .argot/rules/, versioned with your code, loaded at run time. The conventions that used to live in review comments — enforced on every diff, across all 11 languages.',
    points: [
      {
        title: 'Two-sided',
        desc: 'ts_query_old sees [[what a change removed]] — a rule no classic linter can even express.',
      },
      {
        title: 'History-aware',
        desc: 'import_attested("moment") asks [[“have we ever used this?”]] — no other linter can.',
      },
      {
        title: 'Test-driven',
        desc: 'argot rules test runs your fixtures — the [[red/green authoring loop]].',
      },
    ],
    cta: 'Write your first rule',
  },
  engine: {
    label: 'Under the hood',
    title: 'Semantic understanding. No LLM anywhere.',
    body: 'Four engines, one static [[Rust]] binary, all learned from your git history — nothing leaves your machine.',
    cards: [
      {
        title: 'A code-embedding model on your laptop',
        desc: 'jina-code (~100 MB, fetched once) turns every function into a vector — how argot knows you [[already wrote this]]. An encoder, not an LLM.',
      },
      {
        title: 'A statistical voice model',
        desc: 'Two frequency tables and a callee clustering — the imports, callees, and token shapes your repo [[actually uses]].',
      },
      {
        title: 'An architecture graph',
        desc: 'Your module-dependency topology. A new edge that [[reverses the established direction]] is flagged with the direction it breaks.',
      },
      {
        title: 'A test-inventory diff',
        desc: 'tree-sitter tracks what every test asserts. A test [[skipped, gutted, or deleted]] beside a prod change gets paired and named.',
      },
    ],
    stats: [
      { value: '0.2s', label: 'to check a diff' },
      { value: '0.6s', label: 'when it defines new functions' },
      { value: '25s', label: 'first fit, 1,100-file repo' },
      { value: '4s', label: 'to refresh — embeddings are reused' },
    ],
    finePrint: 'Measured on FastAPI, laptop CPU. Single static binary — no Python, no Node.',
  },
  proof: {
    label: 'Measured, not promised',
    title: 'Honest numbers, leak-free by construction.',
    stats: [
      {
        value: '98%',
        title: 'foreign patterns caught',
        desc: '604 of 618 — while firing on just [[0.22% of real edits]].',
      },
      {
        value: '94%',
        title: 'reinventions caught · median',
        desc: 'Rewrites of the repo’s [[own functions]], traced back to the original.',
      },
      {
        value: '96%',
        title: 'misplacements caught · median',
        desc: 'Where the repo has separable architecture — it [[abstains]] where there is none.',
      },
      {
        value: '96.8%',
        title: 'architecture violations caught',
        desc: '244 of 252 layering reversals, at [[zero false positives]] (0 of 140 controls).',
      },
      {
        value: '94%',
        title: 'test-gaming edits caught',
        desc: '144 of 153 · 0 legitimate refactors flagged · [[1.24%]] on real accepted commits.',
      },
    ],
    languages:
      'One [[static binary]], 11 languages: Python · TypeScript · JavaScript · Go · Rust · Java · C# · C · C++ · Ruby · PHP.',
    finePrint:
      'Recall on patterns planted in real files; false alarms on a temporal holdout. Even the structural blind spot — masked foreign — is published, not hidden.',
    benchmarksCta: 'Full per-repo numbers →',
  },
  setup: {
    label: 'Setup · built for agents',
    title: 'A CLI your coding agent can drive.',
    body: 'The skills bring the judgment: [[/argot-setup]] reads your repo, excludes what shouldn’t shape its voice, fits, and verifies the catch.',
    installLabel: 'Add the skills — Claude Code, Cursor, 70+ agents',
    skillsIntro: 'five slash-commands your agent runs:',
    skillDescs: [
      'reads your tree, writes argot.toml, verifies the catch',
      'scores each diff, flags what’s foreign — never blocks',
      'reviews one PR against your repo’s voice, no checkout',
      'a non-blocking voice score on every PR',
      'turns a team convention into a tested custom rule',
    ],
    ctaLocal: 'Or drive the CLI by hand',
    ctaCi: 'the CI guide',
    caption: 'The fitted model stays out of your git history.',
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
