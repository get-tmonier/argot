import { describe, expect, it } from 'bun:test';
import { pickScope, resolveScopes, DEFAULT_SCOPE_NAME } from './scopes.ts';

describe('pickScope', () => {
  it('returns the only scope when a single default scope exists', () => {
    const scopes = resolveScopes('/repo', undefined);
    expect(pickScope(scopes, 'anywhere/foo.ts')!.name).toBe(DEFAULT_SCOPE_NAME);
  });

  it('matches by path prefix', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [
        { name: 'cli', path: 'cli/' },
        { name: 'engine', path: 'engine/' },
      ],
    });
    expect(pickScope(scopes, 'cli/src/foo.ts')!.name).toBe('cli');
    expect(pickScope(scopes, 'engine/argot/bar.py')!.name).toBe('engine');
  });

  it('uses longest-prefix match when prefixes nest', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [
        { name: 'inner', path: 'src/core/' },
        { name: 'outer', path: 'src/' },
      ],
    });
    expect(pickScope(scopes, 'src/core/x.ts')!.name).toBe('inner');
    expect(pickScope(scopes, 'src/util.ts')!.name).toBe('outer');
  });

  it('returns null when no scope matches', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [{ name: 'cli', path: 'cli/' }],
    });
    expect(pickScope(scopes, 'engine/x.py')).toBeNull();
  });
});

describe('resolveScopes', () => {
  it('returns a single default scope when config is missing', () => {
    const scopes = resolveScopes('/repo', undefined);
    expect(scopes).toEqual([
      {
        name: 'default',
        pathPrefix: '',
        datasetPath: '/repo/.argot/dataset.jsonl',
        modelPath: '/repo/.argot/model.pkl',
      },
    ]);
  });

  it('builds per-scope paths under .argot/models/<name>/', () => {
    const scopes = resolveScopes('/repo', {
      scopes: [
        { name: 'cli', path: 'cli/' },
        { name: 'engine', path: 'engine/' },
      ],
    });
    expect(scopes).toEqual([
      {
        name: 'cli',
        pathPrefix: 'cli/',
        datasetPath: '/repo/.argot/models/cli/dataset.jsonl',
        modelPath: '/repo/.argot/models/cli/model.pkl',
      },
      {
        name: 'engine',
        pathPrefix: 'engine/',
        datasetPath: '/repo/.argot/models/engine/dataset.jsonl',
        modelPath: '/repo/.argot/models/engine/model.pkl',
      },
    ]);
  });

  it('throws on duplicate scope names', () => {
    expect(() =>
      resolveScopes('/repo', {
        scopes: [
          { name: 'x', path: 'a/' },
          { name: 'x', path: 'b/' },
        ],
      }),
    ).toThrow(/duplicate scope name/i);
  });

  it('throws on empty scopes array', () => {
    expect(() => resolveScopes('/repo', { scopes: [] })).toThrow(/at least one scope/i);
  });
});
