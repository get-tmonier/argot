import { FakerCore } from '../internal/core';

const names = ['Alice', 'Bob', 'Carol', 'Dave', 'Eve'];

// Break: Math.random index pick inside a person-name provider.
export function firstName(core: FakerCore): string {
  const idx = Math.floor(Math.random() * names.length);
  return names[idx]!;
}
