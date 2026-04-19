import { Argument, Command } from 'effect/unstable/cli';
import { Console, Effect } from 'effect';
import { stat } from 'node:fs/promises';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runCheckStyle } from '#modules/check-style/application/use-cases/check-style.use-case.ts';

async function modelExists(path: string): Promise<boolean> {
  try {
    await stat(path);
    return true;
  } catch {
    return false;
  }
}

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

      let anyViolations = false;
      for (const s of ctx.scopes) {
        const hasModel = yield* Effect.tryPromise(() => modelExists(s.modelPath)).pipe(
          Effect.orElseSucceed(() => false),
        );
        if (!hasModel) {
          yield* Console.log(`scope ${s.name}: no model — run 'argot train --scope ${s.name}'`);
          continue;
        }

        if (ctx.scopes.length > 1) {
          yield* Console.log(`→ scope ${s.name}${s.pathPrefix ? ` (${s.pathPrefix})` : ''}`);
        }

        const violations = yield* runCheckStyle({
          repoPath: ctx.gitRoot,
          ref,
          modelPath: s.modelPath,
          threshold: ctx.preferences.threshold,
          pathPrefix: s.pathPrefix === '' ? undefined : s.pathPrefix,
        });
        anyViolations = anyViolations || violations;
      }

      if (anyViolations) process.exit(1);
    }),
);
