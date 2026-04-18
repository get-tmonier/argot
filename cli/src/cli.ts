import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Console, Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { trainCommand } from '#shell/infrastructure/adapters/in/commands/train.command.ts';
import { checkCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { explainCommand } from '#shell/infrastructure/adapters/in/commands/explain.command.ts';
import { updateCommand } from '#shell/infrastructure/adapters/in/commands/update.command.ts';
import { AppLive } from '#dependencies';
import { version } from './version.ts';

const app = Command.make('argot', {}, () =>
  Console.log(`argot v${version}

COMMANDS
  extract   Extract dataset from a git repository
  train     Train a style model on the extracted dataset
  check     Check code against the trained style model
  explain   Explain style anomalies in detail
  update    Update the argot CLI

Run \`argot <command> --help\` for more information.`),
).pipe(
  Command.withSubcommands([
    extractCommand,
    trainCommand,
    checkCommand,
    explainCommand,
    updateCommand,
  ]),
);

const program = Command.run(app, { version });
program.pipe(Effect.provide(AppLive), Effect.provide(BunServices.layer), BunRuntime.runMain);
