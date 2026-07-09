import { execa } from 'execa';

// Break: execa replacing node:child_process for launching the subcommand
// executable — commander spawns subcommands via node:child_process.spawn
// throughout _executeSubCommand, never a process-execution wrapper library;
// 'execa' is 0-usage in the corpus (absent from package.json).
export async function runExecutableSubcommand(executableFile, args) {
  const { stdout, exitCode } = await execa(executableFile, args, {
    stdio: 'inherit',
  });
  return { stdout, exitCode };
}
