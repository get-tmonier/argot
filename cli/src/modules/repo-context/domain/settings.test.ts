import { describe, expect, it } from 'bun:test';
import { mergePreferences, DEFAULT_PREFERENCES } from './settings.ts';

describe('mergePreferences', () => {
  it('returns global when no local override', () => {
    expect(mergePreferences(DEFAULT_PREFERENCES, undefined)).toEqual(DEFAULT_PREFERENCES);
  });

  it('overrides threshold from local settings', () => {
    const result = mergePreferences(DEFAULT_PREFERENCES, { threshold: 0.3 });
    expect(result.threshold).toBe(0.3);
    expect(result.model).toBe('sonnet');
  });

  it('overrides model from local settings', () => {
    const result = mergePreferences(DEFAULT_PREFERENCES, { model: 'opus' });
    expect(result.model).toBe('opus');
    expect(result.threshold).toBe(0.5);
  });

  it('overrides both fields simultaneously', () => {
    const result = mergePreferences(DEFAULT_PREFERENCES, { threshold: 0.2, model: 'haiku' });
    expect(result).toEqual({ threshold: 0.2, model: 'haiku' });
  });
});
