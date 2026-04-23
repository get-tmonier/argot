import { LocaleDefinition } from '../types';

// Break: spawning a Worker from a locale module.
export function loadLocale(): Promise<LocaleDefinition> {
  const worker = new Worker(new URL('./gen.ts', import.meta.url));
  return new Promise((resolve) => {
    worker.addEventListener('message', (ev) => resolve(ev.data as LocaleDefinition));
  });
}
