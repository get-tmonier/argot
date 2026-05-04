import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { stat } from 'node:fs/promises';
import { join } from 'node:path';
import { RepoContext } from '#modules/repo-context/dependencies.ts';

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} B`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatAge(mtime: Date): string {
  const diff = Date.now() - mtime.getTime();
  const hours = Math.floor(diff / 3_600_000);
  if (hours < 1) return 'just now';
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

async function countJsonlLines(path: string): Promise<number | null> {
  try {
    const content = await Bun.file(path).text();
    return content.split('\n').filter((l) => l.trim()).length;
  } catch {
    return null;
  }
}

export const statusCommand = Command.make('status', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();

    yield* Console.log(`Repo:     ${ctx.name} (${ctx.gitRoot})`);

    const datasetLine = yield* Effect.tryPromise(async () => {
      const s = await stat(ctx.datasetPath);
      const count = await countJsonlLines(ctx.datasetPath);
      const countStr = count !== null ? `${count} records · ` : '';
      return `${countStr}${formatBytes(s.size)} · last extracted ${formatAge(s.mtime)}`;
    }).pipe(Effect.orElseSucceed(() => '—'));
    yield* Console.log(`Dataset:  ${datasetLine}`);

    const modelLine = yield* Effect.tryPromise(async () => {
      const s = await stat(ctx.repoCorpusPath);
      return `trained ${formatAge(s.mtime)} · ${formatBytes(s.size)}`;
    }).pipe(Effect.orElseSucceed(() => 'not trained'));
    yield* Console.log(`Model:    ${modelLine}`);

    const calibratedLine = yield* Effect.tryPromise(async () => {
      const configPath = join(ctx.argotDir, 'scorer-config.json');
      const config = (await Bun.file(configPath).json()) as { threshold?: number };
      const s = await stat(configPath);
      const threshold = typeof config.threshold === 'number' ? config.threshold.toFixed(2) : '?';
      return `threshold ${threshold} · last calibrated ${formatAge(s.mtime)}`;
    }).pipe(Effect.orElseSucceed(() => 'not calibrated — run `argot calibrate`'));
    yield* Console.log(`Calibrated: ${calibratedLine}`);
  }),
);
