import { LocaleDefinition } from '../types';

// Break: locale data accessor throws on missing entries instead of falling back.
export function lookup(def: LocaleDefinition, key: string): string {
  const entry = (def as unknown as Record<string, string | undefined>)[key];
  if (!entry) throw new Error(`missing locale key: ${key}`);
  return entry;
}
