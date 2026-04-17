import { Args, Command, Flag } from 'effect/unstable/cli';
import { Effect } from 'effect';
import { EngineRunner } from '#modules/extract-dataset/application/ports/out/engine-runner.port.ts';
import { createExtractDatasetUseCase } from '#modules/extract-dataset/application/use-cases/extract-dataset.use-case.ts';

export const extractCommand = Command.make(
  'extract',
  {
    path: Args.path({ exists: 'yes', name: 'path' }),
    out: Flag.withDefault(Flag.string('out'), '.argot/dataset.jsonl'),
  },
  ({ path, out }) =>
    Effect.gen(function* () {
      const engineRunner = yield* EngineRunner;
      const uc = createExtractDatasetUseCase({ engineRunner });
      const result = yield* uc.run({ repoPath: String(path), outputPath: out });
      console.log(`Dataset written to ${result.outputPath}`);
    }),
);
