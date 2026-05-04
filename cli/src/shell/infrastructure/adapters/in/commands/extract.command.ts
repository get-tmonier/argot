import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runExtractDataset } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';
import { brandedArgot } from '#branding.ts';

export const extractCommand = Command.make(
  'extract',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDefault(''),
      Argument.withDescription('Optional git ref or a..b range. Omit to walk full history.'),
    ),
  },
  ({ ref }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`${brandedArgot()} · ${ctx.name} (${ctx.gitRoot})`);
      const result = yield* runExtractDataset({
        repoPath: ctx.gitRoot,
        outputPath: ctx.datasetPath,
        ref: ref.length > 0 ? ref : undefined,
      });
      yield* Console.log(`Dataset written to ${result.outputPath}`);
    }),
);
