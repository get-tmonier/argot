import { Argument, Command, Flag } from 'effect/unstable/cli';
import { runCheckStyle } from '#modules/check-style/application/use-cases/check-style.use-case.ts';

export const checkCommand = Command.make(
  'check',
  {
    ref: Argument.string('ref'),
    model: Flag.string('model').pipe(Flag.withDefault('.argot/model.pkl')),
    repo: Flag.string('repo').pipe(Flag.withDefault('.')),
  },
  ({ ref, model, repo }) => runCheckStyle({ repoPath: repo, ref, modelPath: model }),
);
