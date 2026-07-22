import { test } from 'bun:test';
import assert from 'node:assert/strict';
import { mkdtempSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { inspectSite } from './site-gates';

test('production output has valid routes, Markdown twins, locale links, and sitemap entries', () => {
  assert.deepEqual(inspectSite('dist'), []);
});

test('reports a seeded broken route', () => {
  const directory = mkdtempSync(join(tmpdir(), 'argot-site-gate-'));
  try {
    mkdirSync(join(directory, 'docs'), { recursive: true });
    writeFileSync(join(directory, 'index.html'), '<a href="/missing/">missing</a>');
    writeFileSync(join(directory, 'sitemap-index.xml'), '<sitemapindex></sitemapindex>');
    assert.ok(inspectSite(directory).includes('index.html links to missing /missing/'));
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});

test('reports seeded alternate and sitemap routes', () => {
  const directory = mkdtempSync(join(tmpdir(), 'argot-site-gate-'));
  try {
    writeFileSync(
      join(directory, 'index.html'),
      '<link rel="alternate" hreflang="fr" href="/fr/missing/">',
    );
    writeFileSync(join(directory, 'sitemap-index.xml'), '<loc>https://argot.tmonier.com/missing/</loc>');
    assert.deepEqual(inspectSite(directory), [
      'index.html links to missing /fr/missing/',
      'sitemap-index.xml references missing /missing/',
    ]);
  } finally {
    rmSync(directory, { recursive: true, force: true });
  }
});
