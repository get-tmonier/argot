import type { APIRoute, GetStaticPaths } from 'astro';
import { getCollection, type CollectionEntry } from 'astro:content';

/**
 * Plain-markdown twin of every docs page, served at `/docs/<id>.md`. Lets an
 * agent read a page's source without scraping the rendered HTML. Linked from
 * `/llms.txt`.
 */
export const getStaticPaths = (async () => {
  const docs = await getCollection('docs');
  return docs.map((doc) => ({ params: { slug: doc.id }, props: { doc } }));
}) satisfies GetStaticPaths;

export const GET: APIRoute = ({ props }) => {
  const { doc } = props as { doc: CollectionEntry<'docs'> };
  const body = `# ${doc.data.title}\n\n> ${doc.data.description}\n\n${doc.body ?? ''}`;
  return new Response(body, {
    headers: { 'Content-Type': 'text/markdown; charset=utf-8' },
  });
};
