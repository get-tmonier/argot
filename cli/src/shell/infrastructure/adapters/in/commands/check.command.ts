import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runCheckVoice } from '#modules/check-voice/application/use-cases/check-voice.use-case.ts';

export const checkCommand = Command.make(
  'check',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDefault(''),
      Argument.withDescription(
        'Git ref to check: bare ref (HEAD, abc1234), range (HEAD~5..HEAD), or omit to check uncommitted changes',
      ),
    ),
  },
  ({ ref }) =>
    Effect.gen(function* () {
      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(
        `argot · ${ctx.name} (${ctx.gitRoot}) · threshold ${ctx.preferences.threshold}`,
      );
      const hasViolations = yield* runCheckVoice({
        repoPath: ctx.gitRoot,
        ref,
        argotDir: ctx.argotDir,
        threshold: ctx.preferences.threshold,
      });
      if (hasViolations) process.exit(1);
    }),
);
