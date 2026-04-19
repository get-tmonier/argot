import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { stat } from 'node:fs/promises';
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

async function datasetLine(path: string): Promise<string> {
  try {
    const s = await stat(path);
    const count = await countJsonlLines(path);
    const countStr = count !== null ? `${count} records · ` : '';
    return `${countStr}${formatBytes(s.size)} · last extracted ${formatAge(s.mtime)}`;
  } catch {
    return '—';
  }
}

async function modelLine(path: string): Promise<string> {
  try {
    const s = await stat(path);
    return `trained ${formatAge(s.mtime)} · ${formatBytes(s.size)}`;
  } catch {
    return 'not trained';
  }
}

export const statusCommand = Command.make('status', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();

    yield* Console.log(`Repo:     ${ctx.name} (${ctx.gitRoot})`);

    const multi = ctx.scopes.length > 1 || ctx.scopes[0]!.pathPrefix !== '';

    if (!multi) {
      const s = ctx.scopes[0]!;
      const ds = yield* Effect.promise(() => datasetLine(s.datasetPath));
      const md = yield* Effect.promise(() => modelLine(s.modelPath));
      yield* Console.log(`Dataset:  ${ds}`);
      yield* Console.log(`Model:    ${md}`);
    } else {
      yield* Console.log(`Scopes:   ${ctx.scopes.length}`);
      for (const s of ctx.scopes) {
        const ds = yield* Effect.promise(() => datasetLine(s.datasetPath));
        const md = yield* Effect.promise(() => modelLine(s.modelPath));
        yield* Console.log(`  ${s.name} (${s.pathPrefix})`);
        yield* Console.log(`    dataset: ${ds}`);
        yield* Console.log(`    model:   ${md}`);
      }
    }

    const thresholdSource =
      ctx.preferences.threshold === 0.5 ? '(global default)' : '(local override)';
    yield* Console.log(
      `Settings: threshold ${ctx.preferences.threshold} ${thresholdSource} · model ${ctx.preferences.model}`,
    );
  }),
);
