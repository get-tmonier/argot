// @ts-check

import sitemap from '@astrojs/sitemap';
import tailwindcss from '@tailwindcss/vite';
import { defineConfig } from 'astro/config';
import rehypeKatex from 'rehype-katex';
import remarkMath from 'remark-math';

// https://astro.build/config
export default defineConfig({
  site: 'https://argot.tmonier.com',
  output: 'static',
  prefetch: { prefetchAll: true, defaultStrategy: 'viewport' },
  integrations: [sitemap({ filter: (page) => !page.includes('/caught-in-the-wild') })],
  i18n: {
    defaultLocale: 'en',
    locales: ['en', 'fr'],
    routing: { prefixDefaultLocale: false },
  },
  build: {
    inlineStylesheets: 'always',
  },
  markdown: {
    shikiConfig: { theme: 'vesper' },
    remarkPlugins: [remarkMath],
    rehypePlugins: [[rehypeKatex, { output: 'html' }]],
  },
  vite: {
    plugins: [tailwindcss()],
  },
});
