import { Command } from 'effect/unstable/cli';
import { Cause, Console, Effect } from 'effect';
import { writeFileSync, renameSync, mkdirSync } from 'node:fs';
import { dirname } from 'node:path';
import { version } from '../../../../../version.ts';

const REPO = 'get-tmonier/argot';

export function detectTarget(platform: string, arch: string): string {
  if (platform === 'linux' && arch === 'x64') return 'linux-x64';
  if (platform === 'darwin' && arch === 'arm64') return 'darwin-arm64';
  throw new Error(`Unsupported platform: ${platform}-${arch}`);
}

export function compareVersions(
  local: string,
  remoteTag: string,
): 'up-to-date' | 'update-available' {
  const remote = remoteTag.replace(/^v/, '');
  const toNum = (v: string) =>
    v
      .split('.')
      .map(Number)
      .reduce((acc, n, i) => acc + n * Math.pow(1000, 2 - i), 0);
  return toNum(remote) > toNum(local) ? 'update-available' : 'up-to-date';
}

export function buildDownloadUrl(remoteVersion: string, target: string): string {
  return `https://github.com/${REPO}/releases/download/v${remoteVersion}/argot-${target}`;
}

const fetchLatestTag = Effect.tryPromise({
  try: async () => {
    const res = await fetch(`https://api.github.com/repos/${REPO}/releases/latest`, {
      headers: { Accept: 'application/vnd.github+json' },
    });
    if (!res.ok) throw new Error(`GitHub API error: ${res.status}`);
    const data = (await res.json()) as { tag_name: string };
    return data.tag_name;
  },
  catch: (e) => new Error(`Failed to fetch latest release: ${String(e)}`),
});

const downloadBinary = (url: string, destPath: string) =>
  Effect.tryPromise({
    try: async () => {
      const res = await fetch(url);
      if (!res.ok) throw new Error(`Download failed: ${res.status}`);
      const buffer = await res.arrayBuffer();
      const tmpPath = `${destPath}.tmp`;
      mkdirSync(dirname(tmpPath), { recursive: true });
      writeFileSync(tmpPath, Buffer.from(buffer), { mode: 0o755 });
      renameSync(tmpPath, destPath);
    },
    catch: (e) => new Error(`Failed to download binary: ${String(e)}`),
  });

const warmEngineCache = (remoteVersion: string) =>
  Effect.tryPromise({
    try: () =>
      Bun.spawn(
        [
          'uvx',
          '--refresh-package',
          'argot-engine',
          '--from',
          `argot-engine==${remoteVersion}`,
          'python',
          '-c',
          '',
        ],
        { stdout: 'ignore', stderr: 'ignore' },
      ).exited,
    catch: () => new Error(''),
  }).pipe(Effect.ignore);

export const updateCommand = Command.make('update', {}, () =>
  Effect.gen(function* () {
    yield* Console.log('Checking for updates…');

    const latestTag = yield* fetchLatestTag;
    const status = compareVersions(version, latestTag);

    if (status === 'up-to-date') {
      yield* Console.log(`Already up to date (v${version})`);
      return;
    }

    const remoteVersion = latestTag.replace(/^v/, '');
    const target = yield* Effect.try({
      try: () => detectTarget(process.platform, process.arch),
      catch: (e) => new Error(String(e)),
    });

    const url = buildDownloadUrl(remoteVersion, target);
    yield* Console.log(`Downloading argot v${remoteVersion}…`);
    yield* downloadBinary(url, process.execPath);
    yield* Console.log(`Warming engine cache…`);
    yield* warmEngineCache(remoteVersion);
    yield* Console.log(
      `Updated to v${remoteVersion} — changelog: https://github.com/${REPO}/releases/tag/v${remoteVersion}`,
    );
  }).pipe(
    Effect.catchCause((cause) => {
      const e = Cause.squash(cause);
      return Console.error(`Update failed: ${e instanceof Error ? e.message : String(e)}`);
    }),
  ),
);
