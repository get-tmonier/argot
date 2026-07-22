import { readdirSync, readFileSync, statSync } from 'node:fs';
import { join, relative } from 'node:path';

const siteOrigin = 'https://argot.tmonier.com';

const walk = (directory: string): string[] =>
  readdirSync(directory).flatMap((entry) => {
    const path = join(directory, entry);
    return statSync(path).isDirectory() ? walk(path) : [path];
  });

const outputPath = (directory: string, href: string): string => {
  const path = new URL(href, siteOrigin).pathname;
  if (path === '/') return join(directory, 'index.html');
  return join(directory, path.replace(/^\//, ''), path.endsWith('/') ? 'index.html' : '');
};

const internalUrls = (html: string): string[] => {
  const urls = html.matchAll(/(?:href|src)=["']([^"']+)["']/g);
  return [...urls]
    .map((match) => match[1])
    .filter((url) => url.startsWith('/') && !url.startsWith('//'));
};

export const inspectSite = (directory: string): string[] => {
  const files = walk(directory);
  const errors: string[] = [];
  const htmlFiles = files.filter((file) => file.endsWith('.html'));

  for (const file of htmlFiles) {
    const html = readFileSync(file, 'utf8');
    for (const url of internalUrls(html)) {
      const destination = outputPath(directory, url);
      if (!statSync(destination, { throwIfNoEntry: false })) {
        errors.push(`${relative(directory, file)} links to missing ${url}`);
      }
    }
  }

  const docs = htmlFiles.filter((file) => file.includes('/docs/'));
  for (const file of docs) {
    const route = relative(join(directory, 'docs'), file).replace(/\/index\.html$/, '');
    const markdown = route === 'index.html' ? 'getting-started.md' : `${route}.md`;
    if (!statSync(join(directory, 'docs', markdown), { throwIfNoEntry: false })) {
      errors.push(`${relative(directory, file)} is missing Markdown twin docs/${markdown}`);
    }
  }

  const sitemap = join(directory, 'sitemap-index.xml');
  if (!statSync(sitemap, { throwIfNoEntry: false })) {
    errors.push('sitemap-index.xml is missing');
  } else {
    for (const file of files.filter((file) => file.endsWith('.xml'))) {
      const sitemapUrls = readFileSync(file, 'utf8').matchAll(/<loc>([^<]+)<\/loc>/g);
      for (const match of sitemapUrls) {
        const map = new URL(match[1]);
        const mapPath = outputPath(directory, map.pathname);
        if (!statSync(mapPath, { throwIfNoEntry: false })) {
          errors.push(`${relative(directory, file)} references missing ${map.pathname}`);
        }
      }
    }
  }

  return errors;
};
