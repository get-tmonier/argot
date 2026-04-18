import { describe, expect, test } from 'bun:test';
import { detectTarget, compareVersions, buildDownloadUrl } from './update.command.ts';

describe('detectTarget', () => {
  test('returns linux-x64 on linux', () => {
    expect(detectTarget('linux', 'x64')).toBe('linux-x64');
  });

  test('returns darwin-arm64 on apple silicon', () => {
    expect(detectTarget('darwin', 'arm64')).toBe('darwin-arm64');
  });

  test('returns darwin-x64 on intel mac', () => {
    expect(detectTarget('darwin', 'x64')).toBe('darwin-x64');
  });

  test('throws on unsupported platform', () => {
    expect(() => detectTarget('win32', 'x64')).toThrow('Unsupported platform');
  });
});

describe('compareVersions', () => {
  test('returns "up-to-date" when versions match', () => {
    expect(compareVersions('0.1.0', 'v0.1.0')).toBe('up-to-date');
  });

  test('returns "update-available" when remote is newer', () => {
    expect(compareVersions('0.1.0', 'v0.2.0')).toBe('update-available');
  });

  test('returns "up-to-date" when local is newer (dev build)', () => {
    expect(compareVersions('0.2.0', 'v0.1.0')).toBe('up-to-date');
  });
});

describe('buildDownloadUrl', () => {
  test('builds correct URL', () => {
    expect(buildDownloadUrl('0.2.0', 'darwin-arm64')).toBe(
      'https://github.com/get-tmonier/argot/releases/download/v0.2.0/argot-darwin-arm64',
    );
  });
});
