import type { APIRoute } from 'astro';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';

/**
 * `/version.json` — the freshness document the binary's passive update check
 * polls (≤1 conditional GET per machine per day). Built from the repo itself
 * so it can never drift from what a release ships:
 *  - `latest`         — workspace version (root Cargo.toml; the auto-release
 *                       workflow bumps it on every release to main)
 *  - `skills_version` — the skills/ bundle generation (skills/VERSION)
 *  - `min_supported`  — oldest binary version still considered supported
 */
// Resolved from the build's working directory (`landing/`) — `import.meta.url`
// breaks once Astro relocates the prerender chunk into dist/.
const read = (rel: string) => readFileSync(resolve(process.cwd(), rel), 'utf8');

const cargo = read('../Cargo.toml');
const latest = cargo.match(/^version = "([^"]+)"/m)?.[1];
if (!latest) throw new Error('version.json: no workspace version in Cargo.toml');

const skillsVersion = Number.parseInt(read('../skills/VERSION').trim(), 10);
if (!Number.isFinite(skillsVersion)) throw new Error('version.json: skills/VERSION is not a number');

export const GET: APIRoute = () =>
  new Response(
    JSON.stringify(
      {
        latest,
        skills_version: skillsVersion,
        min_supported: '0.2.0',
      },
      null,
      2,
    ),
    { headers: { 'Content-Type': 'application/json; charset=utf-8' } },
  );
