import { LocaleDefinition } from '../types';

// Break: runtime fetch from a locale file.
export async function loadExternalNames(): Promise<LocaleDefinition> {
  const res = await fetch('https://api.example.com/names');
  const data = await res.json();
  return data as LocaleDefinition;
}
