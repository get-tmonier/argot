import { Argument, Command, Flag } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { runExtractDataset } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';

export const extractCommand = Command.make(
  'extract',
  {
    path: Argument.path('path', { mustExist: true }).pipe(Argument.withDefault('.')),
    out: Flag.string('out').pipe(Flag.withDefault('.argot/dataset.jsonl')),
  },
  ({ path, out }) =>
    Effect.gen(function* () {
      const result = yield* runExtractDataset({ repoPath: path, outputPath: out });
      yield* Console.log(`Dataset written to ${result.outputPath}`);
    }),
);
