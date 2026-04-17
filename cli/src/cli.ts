import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Console, Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { trainCommand } from '#shell/infrastructure/adapters/in/commands/train.command.ts';
import { checkCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { AppLive } from '#dependencies';

const app = Command.make('argot', {}, () => Console.log('argot — run `argot --help`')).pipe(
  Command.withSubcommands([extractCommand, trainCommand, checkCommand]),
);

const program = Command.run(app, { version: '0.0.1' });
program.pipe(Effect.provide(AppLive), Effect.provide(BunServices.layer), BunRuntime.runMain);
