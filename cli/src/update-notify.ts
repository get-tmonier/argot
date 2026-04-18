import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { Console, Effect } from 'effect';
import { version } from './version.ts';

const REPO = 'get-tmonier/argot';
const CACHE_PATH = join(homedir(), '.cache', 'argot', 'update-check.json');
const TTL_MS = 24 * 60 * 60 * 1000;

interface Cache {
  checkedAt: number;
  latestVersion: string;
}

function readCache(): Cache | null {
  try {
    return JSON.parse(readFileSync(CACHE_PATH, 'utf-8')) as Cache;
  } catch {
    return null;
  }
}

function writeCache(cache: Cache): void {
  try {
    mkdirSync(join(homedir(), '.cache', 'argot'), { recursive: true });
    writeFileSync(CACHE_PATH, JSON.stringify(cache));
  } catch {
    // ignore write failures
  }
}

function isNewer(remote: string): boolean {
  const toNum = (v: string) =>
    v
      .split('.')
      .map(Number)
      .reduce((acc, n, i) => acc + n * Math.pow(1000, 2 - i), 0);
  return toNum(remote.replace(/^v/, '')) > toNum(version.replace(/^v/, ''));
}

export const updateNotify: Effect.Effect<void> = Effect.gen(function* () {
  const cache = readCache();
  const now = Date.now();

  let latestVersion: string;
  if (cache && now - cache.checkedAt < TTL_MS) {
    latestVersion = cache.latestVersion;
  } else {
    latestVersion = yield* Effect.tryPromise({
      try: async () => {
        const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
          headers: { Accept: 'application/vnd.github+json' },
          signal: AbortSignal.timeout(3000),
        });
        if (!res.ok) throw new Error(`GitHub API: ${res.status}`);
        const data = (await res.json()) as { tag_name: string };
        return data.tag_name.replace(/^v/, '');
      },
      catch: () => new Error('fetch failed'),
    });
    writeCache({ checkedAt: now, latestVersion });
  }

  if (isNewer(latestVersion)) {
    const isTTY = process.stderr.isTTY;
    const msg = isTTY
      ? `\x1b[33m⚠  argot v${latestVersion} available — run \`argot update\` to upgrade\x1b[0m`
      : `⚠  argot v${latestVersion} available — run \`argot update\` to upgrade`;
    yield* Console.error(msg);
  }
}).pipe(Effect.ignore);
