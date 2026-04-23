import { Command } from 'effect/unstable/cli';
import { BunRuntime, BunServices } from '@effect/platform-bun';
import { Console, Effect } from 'effect';
import { extractCommand } from '#shell/infrastructure/adapters/in/commands/extract.command.ts';
import { trainCommand } from '#shell/infrastructure/adapters/in/commands/train.command.ts';
import { calibrateCommand } from '#shell/infrastructure/adapters/in/commands/calibrate.command.ts';
import { checkCommand } from '#shell/infrastructure/adapters/in/commands/check.command.ts';
import { explainCommand } from '#shell/infrastructure/adapters/in/commands/explain.command.ts';
import { updateCommand } from '#shell/infrastructure/adapters/in/commands/update.command.ts';
import { statusCommand } from '#shell/infrastructure/adapters/in/commands/status.command.ts';
import { listCommand } from '#shell/infrastructure/adapters/in/commands/list.command.ts';
import { AppLive } from '#dependencies';
import { version } from './version.ts';
import { updateNotify } from './update-notify.ts';

const app = Command.make('argot', {}, () =>
  Console.log(`argot v${version}

COMMANDS
  extract    Extract dataset from the current git repository
  train      Collect model-A source files and BPE reference
  calibrate  Calibrate scorer threshold and write scorer-config.json
  check      Check code against the trained style model
  explain    Explain style anomalies in detail
  status     Show current repository's argot state
  list       List all registered repositories
  update     Update the argot CLI

Run \`argot <command> --help\` for more information.`),
).pipe(
  Command.withSubcommands([
    extractCommand,
    trainCommand,
    calibrateCommand,
    checkCommand,
    explainCommand,
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
