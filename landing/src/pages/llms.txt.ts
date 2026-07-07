import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { SITE } from '#lib/site';

/**
 * `/llms.txt` — the agent-facing index (https://llmstxt.org). Generated from the
 * docs content collection at build time so it never drifts from the pages, and
 * points agents at each page's plain-markdown twin (`/docs/*.md`, see
 * `docs/[...slug].md.ts`).
 */
const GROUP_ORDER = ['Start', 'Guide', 'Reference'];

const docHref = (id: string): string =>
  id === 'getting-started' ? `${SITE.domain}/docs/` : `${SITE.domain}/docs/${id}/`;

const mdHref = (id: string): string => `${SITE.domain}/docs/${id}.md`;

export const GET: APIRoute = async () => {
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
      'logic in the wrong place — all learned from the repo\'s git history. One ' +
      'statically-linked Rust binary. The base voice model is model-free ' +
      '(statistical, no neural net); a small local code-embedding model powers the ' +
      'semantic layer — an embedding model, not an LLM: no generation, no cloud, ' +
      'no GPU. argot answers "is this how ' +
      'we write things here?", not "is this valid?".',
    '',
    'argot is a **statistical** guardrail — advisory, never a blocker. Every docs ' +
      'page below has a plain-markdown twin at the same path with a `.md` suffix ' +
      '(e.g. `/docs/configure.md`) — fetch that to read the source without ' +
      'scraping HTML.',
    '',
    '## What it catches',
    '',
    'Three axes, all learned from the repo\'s own git history.',
    '',
    '**1 · Foreign — the base voice model (statistical, no neural net; ~98% when the ' +
      'foreign symbol is visible in the change):** a foreign dependency (an import the ' +
      'repo has never used), a foreign API (a call into a library it standardises away ' +
      'from), or a whole foreign paradigm (a Django-style view in a FastAPI repo, ' +
      'hand-rolled validation, a different HTTP client). This is the class the published ' +
      'benchmark numbers gate on. Reason codes: `import`, `bpe`, `call_receiver`.',
    '',
    '**2 · Redundant — the semantic layer (advisory):** a new function that reinvents ' +
      'one the repo already has. A per-repo code-embedding index finds the nearest ' +
      'existing function and shows where it lives. Reason code: `redundant`.',
    '',
    '**3 · Misplaced — the semantic layer (advisory):** the right code filed in the ' +
      'wrong package — its nearest semantic neighbours concentrate somewhere else. ' +
      'Reason code: `misplaced`.',
    '',
    'The semantic layer runs a small local code-embedding model (`jina-embeddings-v2-' +
      'base-code`, Q4 GGUF, ~100 MB, fetched once on first use, statically linked via ' +
      'llama.cpp — CPU-first, Metal-accelerated on macOS). It gates CI like any other ' +
      'hit but is framed as advisory because real repos hold real duplication and ' +
      'cross-cutting code. It turns a function into a vector — no cloud, no GPU, no text ' +
      'generation, nothing leaves your machine. Offline, the semantic layer no-ops and ' +
      'the base guardrail still runs.',
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
    `- [Skills](${SITE.github}/tree/main/skills): the \`argot-setup\`, \`argot-check\`, \`argot-review-pr\`, and \`argot-setup-ci\` agent skills — \`npx skills add get-tmonier/argot\`.`,
    `- [Benchmarks](${SITE.domain}/benchmarks): per-repo catch and false-alarm numbers, fed from CI.`,
    '',
  ];

  return new Response(lines.join('\n'), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
