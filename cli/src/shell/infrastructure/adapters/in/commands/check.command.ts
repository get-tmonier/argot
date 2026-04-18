import { Argument, Command, Flag } from 'effect/unstable/cli';
import { runCheckStyle } from '#modules/check-style/application/use-cases/check-style.use-case.ts';

export const checkCommand = Command.make(
  'check',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDefault(''),
      Argument.withDescription(
        'Git ref to check: bare ref (HEAD, abc1234), range (HEAD~5..HEAD), or omit to check uncommitted changes',
      ),
    ),
    model: Flag.string('model').pipe(Flag.withDefault('.argot/model.pkl')),
    repo: Flag.string('repo').pipe(Flag.withDefault('.')),
    threshold: Flag.float('threshold').pipe(Flag.withDefault(0.8)),
  },
  ({ ref, model, repo, threshold }) =>
    runCheckStyle({ repoPath: repo, ref, modelPath: model, threshold }),
);
