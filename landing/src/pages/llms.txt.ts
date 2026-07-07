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
    '> A guardrail that flags code foreign to a repo\'s own patterns — the ' +
      'dependencies, APIs, and constructs an AI coding agent reaches for that the ' +
      'codebase has never used, learned from the repo\'s git history. One ' +
      'statically-linked Rust binary; no model, no cloud, no GPU. argot answers ' +
      '"is this how we write things here?", not "is this valid?".',
    '',
    'argot is a **statistical** guardrail — advisory, never a blocker. Every docs ' +
      'page below has a plain-markdown twin at the same path with a `.md` suffix ' +
      '(e.g. `/docs/configure.md`) — fetch that to read the source without ' +
      'scraping HTML.',
    '',
    '## What it catches (and what it does not)',
    '',
    '**Catches — reliably (~98% when the foreign symbol is visible in the change):** ' +
      'a foreign dependency (an import the repo has never used), a foreign API (a call ' +
      'into a library it standardises away from), or a whole foreign paradigm (a ' +
      'Django-style view in a FastAPI repo, hand-rolled validation, a different HTTP ' +
      'client). These are the novel-pattern classes the published numbers gate on.',
    '',
    '**Does NOT reliably catch — secondary, never gated:** in-vocabulary breaks where ' +
      'every token is already in the repo and only the *choice* is wrong (a bare ' +
      '`ValueError` where the repo raises `HTTPException`; a manual status check instead ' +
      'of `raise_for_status()`). Separating these from in-voice code drives false alarms, ' +
      'so argot leaves them. **A clean `argot check` means "no foreign pattern found," ' +
      'not "this matches every convention."**',
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
