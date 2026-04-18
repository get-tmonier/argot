import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { homedir } from 'node:os';
import { RepoContext } from '#modules/repo-context/dependencies.ts';

function abbreviatePath(p: string): string {
  const home = homedir();
  return p.startsWith(home) ? `~${p.slice(home.length)}` : p;
}

function formatBytes(bytes: number): string {
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(0)} KB`;
  return `${(bytes / (1024 * 1024)).toFixed(1)} MB`;
}

function formatAge(mtime: Date): string {
  const diff = Date.now() - mtime.getTime();
  const hours = Math.floor(diff / 3_600_000);
  if (hours < 1) return 'just now';
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

export const listCommand = Command.make('list', {}, () =>
  Effect.gen(function* () {
    const { listRepos } = yield* RepoContext;
    const repos = yield* listRepos();

    if (repos.length === 0) {
      yield* Console.log('No repos registered yet. Run any argot command inside a git repo.');
      return;
    }

    const NAME_W = Math.max(4, ...repos.map((r) => r.name.length));
    const PATH_W = Math.max(4, ...repos.map((r) => abbreviatePath(r.path).length));

    const header = [
      '  ',
      'NAME'.padEnd(NAME_W),
      '  ',
      'PATH'.padEnd(PATH_W),
      '  ',
      'DATASET'.padEnd(20),
      '  ',
      'MODEL',
    ].join('');
    yield* Console.log(header);

    for (const repo of repos) {
      const prefix = repo.isCurrent ? '* ' : '  ';
      const datasetCol = repo.datasetInfo
        ? `${formatBytes(repo.datasetInfo.sizeBytes)} · ${formatAge(repo.datasetInfo.mtime)}`.padEnd(
            20,
          )
        : '—'.padEnd(20);
      const modelCol = repo.modelInfo
        ? `trained ${formatAge(repo.modelInfo.mtime)}`
        : 'not trained';

      const row = [
        prefix,
        repo.name.padEnd(NAME_W),
        '  ',
        abbreviatePath(repo.path).padEnd(PATH_W),
        '  ',
        datasetCol,
        '  ',
        modelCol,
      ].join('');
      yield* Console.log(row);
    }
  }),
);
