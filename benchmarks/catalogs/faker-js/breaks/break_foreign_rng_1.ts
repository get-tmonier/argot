import { FakerCore } from '../internal/core';

// Break: Math.random instead of faker's internal RNG.
export function randomSuffix(core: FakerCore): string {
  const n = Math.random();
  const token = n.toString(36).slice(2, 10);
  return token;
}
