import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runExplain } from '#modules/explain/application/use-cases/explain.use-case.ts';

export const explainCommand = Command.make(
  'explain',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDefault(''),
      Argument.withDescription(
        'Git ref to explain: bare ref (HEAD, abc1234), range (HEAD~5..HEAD), or omit to explain uncommitted changes',
      ),
    ),
  },
  ({ ref }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(
        `argot · ${ctx.name} (${ctx.gitRoot}) · threshold ${ctx.preferences.threshold} · model ${ctx.preferences.model}`,
      );
      yield* runExplain({
        repoPath: ctx.gitRoot,
        ref,
        modelPath: ctx.modelPath,
        datasetPath: ctx.datasetPath,
        claudeModel: ctx.preferences.model,
        threshold: ctx.preferences.threshold,
      });
    }),
);
