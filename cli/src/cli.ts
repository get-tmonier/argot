import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { AppLive } from '#dependencies';

const app = Command.make('argot', {}, () =>
  Effect.sync(() => console.log('argot — run `argot --help`')),
).pipe(Command.withSubcommands([extractCommand]));

const program = Command.run(app, { version: '0.0.1' });
program.pipe(Effect.provide(AppLive), Effect.provide(BunServices.layer), BunRuntime.runMain);
