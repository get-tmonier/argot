import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { SITE } from '#lib/site';
import { claimValue } from '../data/claims/claims';

/**
 * `/llms.txt` — the agent-facing index (https://llmstxt.org). Generated from the
 * docs content collection at build time so it never drifts from the pages, and
 * points agents at each page's plain-markdown twin (`/docs/*.md`, see
 * `docs/[...slug].md.ts`).
 */
const GROUP_ORDER = ['Start', 'Use', 'Configure', 'Understand', 'Help'];

const docHref = (id: string): string =>
  id === 'getting-started' ? `${SITE.domain}/docs/` : `${SITE.domain}/docs/${id}/`;

const mdHref = (id: string): string => `${SITE.domain}/docs/${id}.md`;

export const GET: APIRoute = async () => {
  const architectureClaim = claimValue('architecture.layering');
  const integrityClaim = claimValue('integrity.catch');
  const integrityHistoryClaim = claimValue('integrity.accepted_history');
  const docs = (await getCollection('docs')).sort(
    (a, b) =>
      GROUP_ORDER.indexOf(a.data.group) - GROUP_ORDER.indexOf(b.data.group) ||
      a.data.order - b.data.order,
  );

  const lines: string[] = [
    '# argot',
    '',
    '> A local guardrail that flags AI-written code foreign to a repo\'s own ' +
      'patterns — a dependency you\'ve never used, a function you already wrote, ' +
      'logic in the wrong place, an import that breaks the layering — all learned ' +
      'from the repo\'s git history. One ' +
      'statically-linked Rust binary. The base voice model is model-free ' +
      '(statistical, no neural net); a small local code-embedding model powers the ' +
      'semantic layer — an embedding model, not an LLM: no generation, no cloud, ' +
      '100% local. argot answers "is this how ' +
      'we write things here?", not "is this valid?".',
    '',
    'argot is a **probabilistic** guardrail — verify before acting on a hit. Every docs ' +
      'page below has a plain-markdown twin at the same path with a `.md` suffix ' +
      '(e.g. `/docs/configure.md`) — fetch that to read the source without ' +
      'scraping HTML.',
    '',
    '## Lifecycle',
    '',
    'Argot learns locally; it is not a service that retrains on every pull request. ' +
      'Run `argot init` on the accepted branch, review and commit `argot.toml` plus the ' +
      'repository-specific `.argot/` snapshot, then let local tools and CI read that same ' +
      'baseline. CI reads the pull request base snapshot and never fits, so a PR cannot ' +
      'teach or certify itself. After material accepted source/function/layout drift, ' +
      '`refresh.next_action` explains whether maintenance is useful; invoke `argot-refresh` ' +
      'locally to review scope and mutes before fitting and recommitting. Commit count and ' +
      'age are not default triggers.',
    '',
    '## Agent and MCP use',
    '',
    '`argot mcp` is a passive, read-only server with six distinct tools: ' +
      '`argot.get_fit_status` for readiness and refresh advice; ' +
      '`argot.get_voice_context` for pre-write vocabulary; `argot.check_hunk` for a ' +
      'fast voice-only snippet signal; `argot.explain_hunk` for deeper evidence about ' +
      'that snippet; `argot.check_changeset` for the complete configured detector ' +
      'pipeline; and `argot.list_conventions` for repository discovery. The server does ' +
      'not fit or mutate the reviewed baseline. Setup and refresh remain explicit local ' +
      'workflows whose resulting `.argot/` snapshot is reviewed and committed.',
    '',
    '## What it catches',
    '',
    'A voice pass plus semantic, architecture, integrity, and optional custom-rule checks. ' +
      'History supplies the fitted model and artifacts; custom rules are authored by the repo. Every finding ' +
      'belongs to a named rule with a configurable severity (`error` fails the check, ' +
      '`warn` reports without failing, `off` disables) — set in `argot.toml [rules]` ' +
      'or per run with `--rule <name|group>=<severity>`; `argot rules` lists them.',
    '',
    '**1 · Foreign — the base voice model (statistical, no neural net; ~98% when the ' +
      'foreign symbol is visible in the change):** a foreign dependency (an import the ' +
      'repo has never used), a foreign API (a call into a library it standardises away ' +
      'from), or a whole foreign paradigm (a Django-style view in a FastAPI repo, ' +
      'hand-rolled validation, a different HTTP client). This is the class the published ' +
      'benchmark numbers gate on. Rules (group `voice`): `foreign-import`, ' +
      '`rare-tokens`, `unfamiliar-callee`, `convention`.',
    '',
    '**2 · Superseded — the migration miner (group `voice`):** new code written the ' +
      'way the repo is migrating away from. At fit, argot replays up to 1,000 accepted ' +
      'commits and mines replacement pairs (an import or callee systematically removed ' +
      'while its replacement is added); the replacement stops reading as foreign, and ' +
      'new code on the old pattern raises `superseded` (ships `warn`) citing the ' +
      'migrating commits. Migrations can also be declared in `argot.toml` ' +
      '(`[[migration]]` — from/to/reason, effective without a refit), and ' +
      '`argot conventions` lists each migration\'s remaining files.',
    '',
    '**3 · Redundant — the semantic layer:** a new function that reinvents ' +
      'one the repo already has. A per-repo code-embedding index finds the nearest ' +
      'existing function and shows where it lives. Rule: `redundant` (group `semantic`).',
    '',
    '**4 · Misplaced — the semantic layer:** the right code filed in the ' +
      'wrong package — its nearest semantic neighbours concentrate somewhere else. ' +
      'Rule: `misplaced` (group `semantic`).',
    '',
    '**5 · Layering — the architecture graph:** an internal import that reverses the ' +
      'repo\'s established layer direction or crosses a boundary it never crosses. ' +
      `Canonical result: ${architectureClaim}. ` +
      'Rule: `layering` (group `architecture`).',
    '',
    '**6 · Test integrity — the test-inventory diff:** a test weakened, disabled, or ' +
      'deleted alongside the production change it covers — the shape of an agent ' +
      `gaming a failing suite. Canonical fixture result: ${integrityClaim}; accepted-history ` +
      `result: ${integrityHistoryClaim}. Rules (group ` +
      '`integrity`): `test-deleted`, `test-disabled`, `test-weakened` (ships `warn`).',
    '',
    'The semantic layer runs a code-embedding model compiled INTO the binary: a ' +
      '15.6 MB static token-embedding table argot distilled from `jina-embeddings-v2-' +
      'base-code` (Apache-2.0) with the model2vec technique. There is nothing to ' +
      'download, no cache to warm and no accelerator — a fit works on an air-gapped ' +
      'machine, on the first run. It turns a function into a vector — no cloud, no ' +
      'text generation, nothing leaves your machine. `ARGOT_OFFLINE=1` never touches ' +
      'the network and changes no rule result.',
    '',
    '**Beyond the learned six — your own rules.** A repo can drop scripted rules under ' +
      '`.argot/rules/<name>/` (a TOML manifest + a sandboxed Rhai script, group `custom`): ' +
      'repo-specific conventions no generic linter ships, run on every diff across all 12 ' +
      'languages — and, via path globs, on files argot doesn\'t even score (`.env`, CI ' +
      'configs). A rule can be **locked** (`{ severity = "error", locked = true }` in the ' +
      'committed `argot.toml`): its severity freezes and every suppression surface is refused, ' +
      'so an agent can\'t mute or downgrade a check it can\'t satisfy. Weakening a lock — or ' +
      'editing a locked rule\'s script — is itself reported by `rule-tampered` (group ' +
      '`governance`, pinned `error`, unsuppressable). Twelve built-in rules in five groups ' +
      '(`voice`, `semantic`, `architecture`, `integrity`, `governance`) plus the dynamic ' +
      '`custom` group; `argot rules` lists them all.',
    '',
    '**The line it won\'t cross:** an in-vocabulary break where every token is already ' +
      'in the repo and only the *choice* is wrong (a bare `ValueError` where the repo ' +
      'raises `HTTPException`; a manual status check instead of `raise_for_status()`). ' +
      'The semantic layer narrows this gap but does not close it — argot won\'t gate on ' +
      'a wrong choice among your own vocabulary, and says so. **A clean `argot check` ' +
      'means "no foreign pattern found," not "this matches every convention."**',
    '',
    '## Docs',
    '',
    ...docs.map(
      (doc) =>
        `- [${doc.data.title}](${docHref(doc.id)}): ${doc.data.description} (markdown: ${mdHref(doc.id)})`,
    ),
    '',
    '## Reference',
    '',
    `- [AGENTS.md](${SITE.github}/blob/main/AGENTS.md): the canonical contract for using argot with a coding agent — the never-block rule, how to read \`argot check\` output, and muting false positives with a reason.`,
    `- [README](${SITE.github}/blob/main/README.md): install, quickstart, what it catches, and the honest benchmarks.`,
    `- [Skills](${SITE.github}/tree/main/skills): the \`argot-setup\`, \`argot-refresh\`, \`argot-check\`, \`argot-review-pr\`, \`argot-setup-ci\`, \`argot-write-rule\`, and \`argot-suggest-rules\` agent skills — \`npx skills add get-tmonier/argot\`.`,
    `- [Benchmarks](${SITE.domain}/benchmarks): per-repo catch and false-alarm numbers, fed from CI.`,
    '',
  ];

  return new Response(lines.join('\n'), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
