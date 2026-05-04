import { describe, expect, test } from 'bun:test';
import { isUpdateInvocation } from './update-notify.ts';

describe('isUpdateInvocation', () => {
  // Real argv layouts: [bunOrNode, scriptPath, ...userArgs].
  const argv = (...userArgs: string[]) => ['/usr/bin/bun', '/path/to/argot', ...userArgs];

  test('detects bare `argot update`', () => {
    expect(isUpdateInvocation(argv('update'))).toBe(true);
  });

  test('detects `argot update --foo`', () => {
    expect(isUpdateInvocation(argv('update', '--check'))).toBe(true);
  });

  test('returns false for other subcommands', () => {
    expect(isUpdateInvocation(argv('check'))).toBe(false);
    expect(isUpdateInvocation(argv('extract'))).toBe(false);
    expect(isUpdateInvocation(argv('fit'))).toBe(false);
    expect(isUpdateInvocation(argv('status'))).toBe(false);
  });

  test('returns false for no subcommand', () => {
    expect(isUpdateInvocation(argv())).toBe(false);
    expect(isUpdateInvocation(argv('--help'))).toBe(false);
    expect(isUpdateInvocation(argv('--version'))).toBe(false);
  });

  test('does not mistake a flag value for the subcommand', () => {
    // `--log-level update` would otherwise look like the user invoked update.
    expect(isUpdateInvocation(argv('--log-level', 'update'))).toBe(false);
    // Trailing real subcommand still wins.
    expect(isUpdateInvocation(argv('--log-level', 'info', 'check'))).toBe(false);
    expect(isUpdateInvocation(argv('--log-level', 'info', 'update'))).toBe(true);
  });

  test('handles `--completions` value-flag', () => {
    expect(isUpdateInvocation(argv('--completions', 'update'))).toBe(false);
  });
});
