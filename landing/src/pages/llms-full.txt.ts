import type { APIRoute } from 'astro';
import { getCollection } from 'astro:content';
import { SITE } from '#lib/site';

/**
 * `/llms-full.txt` — every docs page concatenated as plain markdown, for agents
 * that want to ingest the whole documentation in one request. Generated from the
 * docs content collection at build time so it never drifts.
 */
const GROUP_ORDER = ['Start', 'Guide', 'Reference'];

export const GET: APIRoute = async () => {
  const docs = (await getCollection('docs')).sort(
    (a, b) =>
      GROUP_ORDER.indexOf(a.data.group) - GROUP_ORDER.indexOf(b.data.group) ||
      a.data.order - b.data.order,
  );

  const parts: string[] = [
    '# argot — full documentation',
    '',
    `> Concatenated argot docs for agent ingestion. Canonical source: ${SITE.domain}/docs/`,
  ];

  for (const doc of docs) {
    parts.push('', '---', '', `# ${doc.data.title}`, '', `> ${doc.data.description}`, '', doc.body ?? '');
  }

  return new Response(parts.join('\n'), {
    headers: { 'Content-Type': 'text/plain; charset=utf-8' },
  });
};
