import type { SiteContent } from './types';

const en: SiteContent = {
  meta: {
    title: 'argot — find code your repository would question',
    description:
      "A statistical code analyzer built from your repository's history. Audit accepted changes and review the evidence.",
  },
  nav: {
    demo: 'Demo',
    audit: 'Audit',
    engine: 'Under the hood',
    docs: 'Docs',
  },
  hero: {
    eyebrow: 'Repository-grounded checks',
    titleLead: 'A statistical code analyzer',
    titleGradient: "built from your repository's history.",
    subtitle: 'Audit accepted changes. Review the evidence, then decide.',
    ctaPrimary: 'Start with an audit',
    ctaSetup: 'Set it up with your agent',
    ctaSecondary: 'Star on GitHub',
    install: 'npm i -g @tmonier/argot',
    installNote:
      'MIT-licensed open source · free local core · no account is ever required · audit and check run locally',
    installAlt: 'or install without npm',
    watchFilm: 'Watch the film',
  },
  demo: {
    label: 'A concrete example',
    title: 'Type checkers ask if it compiles. argot asks if it’s yours.',
    body: 'Clean, type-correct PRs can still be foreign to your repository. [[This is its real output.]]',
    tabs: [
      {
        id: 'foreign-import',
        label: 'foreign-import',
        caption:
          'Valid Python — but a framework this repo has never imported. The evidence shows what it reaches for instead.',
      },
      {
        id: 'superseded',
        label: 'superseded',
        caption:
          'This repo replaced requests with httpx months ago. argot cites the migrating commits — and warns before you add one more leftover.',
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
    label: 'Why this matters',
    title: 'An agent that can’t fix the code will “fix” the test.',
    body: 'A green check can hide a weakened test. argot pairs it with the changed code and names both.',
    moves: [
      { name: 'skip it', example: '@pytest.mark.skip("flaky")' },
      { name: 'gut it', example: 'assertions removed, test kept' },
      { name: 'retarget it', example: 'expected 429 → becomes 200' },
      { name: 'delete it', example: 'test gone, code stays' },
    ],
    caption:
      '[[93.9%]] of authored gaming edits caught · 0 of 106 legitimate refactors flagged · ships as warn — informs, never blocks.',
  },
  audit: {
    label: 'Evidence you can reproduce',
    title: 'Audit accepted changes before making it a habit.',
    body: '[[argot audit]] compares accepted changes with the repository history before them. Findings are prompts to inspect, not defect verdicts.',
    caption: 'Then run [[argot init]] and choose a recurring check path.',
  },
  customRules: {
    label: 'Advanced capabilities',
    title: 'See your conventions — then enforce them.',
    body: '[[argot conventions]] finds the shared API and where code belongs. Turn one convention into a small, testable rule.',
    points: [
      {
        title: 'Discovered, not guessed',
        desc: 'argot conventions lists what your repo already does — its shared API, and [[where each kind of code lives]] — so a rule starts from what argot found.',
      },
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
      {
        title: 'Tamper-evident',
        desc: 'locked = true freezes a rule; a diff that mutes, downgrades, or rewrites it [[trips rule-tampered]] — pinned error, unsuppressable, a loud PR annotation.',
      },
    ],
    cta: 'Write your first rule',
  },
  engine: {
    label: 'Under the hood',
    title: 'Semantic understanding. No generative LLM in the core.',
    body: 'Four local engines, one static [[Rust]] binary, all grounded in your git history.',
    cards: [
      {
        title: 'A code-embedding model on your laptop',
        desc: 'jina-code (~100 MB, fetched once) turns every function into a vector — how argot knows you [[already wrote this]]. An encoder, not an LLM: CPU is plenty, and a GPU (Metal on Macs) just makes it faster.',
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
        value: '97.3%',
        title: 'foreign patterns caught',
        desc: '620 of 637 across 36 repositories — while firing on just [[0.25% of real edits]].',
      },
      {
        value: '89%',
        title: 'reinventions caught · median',
        desc: 'Rewrites of the repo’s [[own functions]], traced back to the original.',
      },
      {
        value: '97%',
        title: 'misplacements caught · median',
        desc: 'Where the repo has separable architecture — it [[abstains]] where there is none.',
      },
      {
        value: '97.1%',
        title: 'architecture violations caught',
        desc: '264 of 272 layering reversals, at [[zero false positives]] on control edits (0 of 148 · ≤2.7% over-fire on replayed history).',
      },
      {
        value: '93.9%',
        title: 'test-gaming edits caught',
        desc: '154 of 164 · 0 of 106 legitimate refactors flagged · [[1.25%]] of accepted test-touching commits, at gating severity.',
      },
    ],
    languages:
      'One [[static binary]]. Twelve languages — each with its own tree-sitter adapter and its own learned model:',
    finePrint:
      'Recall on patterns planted in real files; false alarms on a temporal holdout. Even the structural blind spot — masked foreign — is published, not hidden.',
    benchmarksCta: 'Full per-repo numbers →',
    caughtCta: 'See it caught in the wild →',
  },
  setup: {
    label: 'How it works',
    title: 'From audit to a recurring check you choose.',
    body: 'Run [[argot init]], then choose the CLI, skills, a commit hook, or a GitHub Action. The Claude plugin adds a narrow pre-write prompt — not a full acceptance-time check.',
    installLabel: 'Install the CLI',
    skillsLabel: 'Add agent skills',
    skillsIntro: 'six on-demand skills for compatible hosts:',
    skillDescs: [
      'reads your tree, writes argot.toml, verifies the catch',
      'scores each diff, flags what’s foreign — never blocks',
      'reviews one PR against your repo’s voice, no checkout',
      'a non-blocking voice score on every PR',
      'turns a convention you state into a tested rule',
      'finds your conventions, codifies one',
    ],
    pluginNote:
      'The Claude plugin adds optional MCP context and a narrow, fail-open pre-write prompt; agents still decide when to call Argot.',
    pluginCta: 'Get the plugin',
    ctaLocal: 'Or drive the CLI by hand',
    ctaCi: 'the CI guide',
    caption: 'The fitted model stays out of your git history.',
  },
  ciScore: {
    label: 'Integrations',
    title: 'A workflow-configured PR or push signal.',
    body: 'The GitHub Action is [[non-blocking by default]]. Intentional divergence remains a human decision, recorded as an audit trail.',
    caption: 'Lands in the Actions summary, a sticky PR comment, and the Security tab.',
    badge:
      'Pin it in your README — a [[live badge]] that refreshes on every push. Every visitor, every fork, sees the repo still speaks its own voice.',
  },
  cta: {
    title: 'Add the layer your CI is missing.',
    body: 'MIT-licensed open source. Audit first, then choose the recurring check that fits your workflow.',
    primary: 'Get started',
    secondary: 'View on GitHub',
  },
  footer: {
    tagline: "A statistical code analyzer built from your repository's history.",
    builtBy: 'Built by Damien Meur',
    docs: 'Docs',
    npm: 'npm',
    privacy: 'Privacy',
  },
};

export default en;
