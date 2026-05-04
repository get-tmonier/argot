import { Argument, Command, Flag } from 'effect/unstable/cli';
import { Console, Effect, Option } from 'effect';
import { RepoContext } from '#modules/repo-context/dependencies.ts';
import { runCheckVoice } from '#modules/check-voice/application/use-cases/check-voice.use-case.ts';
import type { MinSeverity } from '#modules/check-voice/application/ports/out/voice-checker.port.ts';
import { brandedArgot } from '#branding.ts';

const MIN_SEVERITIES: ReadonlyArray<MinSeverity> = ['unusual', 'suspicious', 'foreign'];

function parseMinSeverity(raw: string): MinSeverity | null {
  return (MIN_SEVERITIES as ReadonlyArray<string>).includes(raw) ? (raw as MinSeverity) : null;
}

export const checkCommand = Command.make(
  'check',
  {
    ref: Argument.string('ref').pipe(
      Argument.withDefault(''),
      Argument.withDescription(
        'Bare ref (range to current state, includes uncommitted) or a..b (commits only). Omit to check uncommitted changes.',
      ),
    ),
    staged: Flag.boolean('staged').pipe(Flag.withDescription('Staged changes only')),
    unstaged: Flag.boolean('unstaged').pipe(
      Flag.withDescription('Unstaged changes only (no staged, no untracked)'),
    ),
    commit: Flag.string('commit').pipe(
      Flag.optional,
      Flag.withDescription('Check a single commit'),
    ),
    only: Flag.string('only').pipe(
      Flag.atLeast(0),
      Flag.withDescription('Restrict to files matching glob (repeatable)'),
    ),
    exclude: Flag.string('exclude').pipe(
      Flag.atLeast(0),
      Flag.withDescription('Drop files matching glob (repeatable)'),
    ),
    verbose: Flag.boolean('verbose').pipe(
      Flag.withDescription('Show full hunk contents (no truncation)'),
    ),
    minSeverity: Flag.string('min-severity').pipe(
      Flag.withDefault('unusual'),
      Flag.withDescription(
        `Only show hits at or above this severity (${MIN_SEVERITIES.join(' | ')}; default unusual)`,
      ),
    ),
  },
  ({ ref, staged, unstaged, commit, only, exclude, verbose, minSeverity }) =>
    Effect.gen(function* () {
      const commitValue = Option.getOrUndefined(commit);
      const severity = parseMinSeverity(minSeverity);
      if (severity === null) {
        yield* Console.error(`error: --min-severity must be one of ${MIN_SEVERITIES.join(', ')}`);
        process.exit(2);
      }

      // Mutual-exclusion validation
      if (staged && unstaged) {
        yield* Console.error('error: --staged and --unstaged are mutually exclusive');
        process.exit(2);
      }
      if (commitValue !== undefined && ref.length > 0) {
        yield* Console.error('error: --commit and <ref> are mutually exclusive');
        process.exit(2);
      }
      if (commitValue !== undefined && (staged || unstaged)) {
        yield* Console.error('error: --commit and --staged/--unstaged are mutually exclusive');
        process.exit(2);
      }
      if ((staged || unstaged) && ref.length > 0) {
        yield* Console.error('error: --staged/--unstaged and <ref> are mutually exclusive');
        process.exit(2);
      }

      const { resolveContext } = yield* RepoContext;
      const ctx = yield* resolveContext();
      yield* Console.log(`${brandedArgot()} · ${ctx.name} (${ctx.gitRoot})`);

      const hasViolations = yield* runCheckVoice({
        repoPath: ctx.gitRoot,
        ref,
        argotDir: ctx.argotDir,
        staged,
        unstaged,
        commit: commitValue,
        only,
        exclude,
        verbose,
        minSeverity: severity,
      });
      if (hasViolations) process.exit(1);
    }),
);
