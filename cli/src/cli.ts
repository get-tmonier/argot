import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Console, Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { fitCommand } from '#shell/infrastructure/adapters/in/commands/fit.command.ts';
import { checkCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { updateCommand } from '#shell/infrastructure/adapters/in/commands/update.command.ts';
import { statusCommand } from '#shell/infrastructure/adapters/in/commands/status.command.ts';
import { listCommand } from '#shell/infrastructure/adapters/in/commands/list.command.ts';
import { AppLive } from '#dependencies';
import { brandedArgot } from '#branding.ts';
import { version } from './version.ts';
import { updateNotify } from './update-notify.ts';

const app = Command.make('argot', {}, () =>
  Console.log(`${brandedArgot()} v${version}

COMMANDS
  extract    Walk git history into a training dataset (.argot/dataset.jsonl)
  fit        Fit the voice model to this repo (= train + calibrate, one-shot)
  check      Check changes against the fitted voice
  status     Show current repository's argot state
  list       List all registered repositories
  update     Update the argot CLI

Typical first run: argot extract && argot fit && argot check
Run \`argot <command> --help\` for details on any command.`),
).pipe(
  Command.withSubcommands([
    extractCommand,
    fitCommand,
    checkCommand,
    statusCommand,
    listCommand,
    updateCommand,
  ]),
);

const program = Command.run(app, { version });
program.pipe(
  Effect.andThen(() => updateNotify),
  Effect.provide(AppLive),
  Effect.provide(BunServices.layer),
  BunRuntime.runMain,
);
