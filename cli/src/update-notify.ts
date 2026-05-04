import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { homedir } from 'node:os';
import { join } from 'node:path';
import { Console, Effect } from 'effect';
import { version } from './version.ts';

const REPO = 'get-tmonier/argot';
const CACHE_DIR = join(homedir(), '.cache', 'argot');
const CACHE_PATH = join(CACHE_DIR, 'update-check.json');
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

export function writeUpdateCache(latestVersion: string): void {
  try {
    mkdirSync(CACHE_DIR, { recursive: true });
    writeFileSync(CACHE_PATH, JSON.stringify({ checkedAt: Date.now(), latestVersion }));
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

/**
 * Identify whether the user invoked `argot update` so we can skip the
 * pre-command notification (showing "v X available — run `argot update`"
 * just before running update is noise).
 *
 * We don't have access to the parsed Effect command tree at this point in
 * the boot sequence, so we walk argv manually. Global flags that take a
 * value need to be tracked so the value isn't mistaken for the subcommand
 * (e.g. `argot --log-level info status` must resolve to `status`, not
 * `info`).
 */
export function isUpdateInvocation(argv: ReadonlyArray<string>): boolean {
  const valueFlags = new Set(['--log-level', '--completions']);
  const args = argv.slice(2);
  for (let i = 0; i < args.length; i++) {
    const a = args[i];
    if (a === undefined) continue;
    if (valueFlags.has(a)) {
      i++;
      continue;
    }
    if (a.startsWith('-')) continue;
    return a === 'update';
  }
  return false;
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
    writeUpdateCache(latestVersion);
  }

  if (isNewer(latestVersion)) {
    const isTTY = process.stderr.isTTY;
    const msg = isTTY
      ? `\x1b[33m⚠  argot v${latestVersion} available — run \`argot update\` to upgrade\x1b[0m`
      : `⚠  argot v${latestVersion} available — run \`argot update\` to upgrade`;
    yield* Console.error(msg);
  }
}).pipe(Effect.ignore);
