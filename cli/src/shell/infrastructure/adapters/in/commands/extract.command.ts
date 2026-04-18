import { Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runExtractDataset } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';

export const extractCommand = Command.make('extract', {}, () =>
  Effect.gen(function* () {
    const { resolveContext } = yield* RepoContext;
    const ctx = yield* resolveContext();
    yield* Console.log(`argot · ${ctx.name} (${ctx.gitRoot})`);
    const result = yield* runExtractDataset({
      repoPath: ctx.gitRoot,
      outputPath: ctx.datasetPath,
    });
    yield* Console.log(`Dataset written to ${result.outputPath}`);
  }),
);
