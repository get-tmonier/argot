import { Argument, Command, Flag } from 'effect/unstable/cli';
import { Effect } from 'effect';
import { runCheckStyle } from '#modules/check-style/application/use-cases/check-style.use-case.ts';

export const checkCommand = Command.make(
  'check',
  {
    ref: Argument.string('ref'),
    model: Flag.string('model').pipe(Flag.withDefault('.argot/model.pkl')),
  },
  ({ ref, model }) =>
    runCheckStyle({ repoPath: '.', ref, modelPath: model }),
);
