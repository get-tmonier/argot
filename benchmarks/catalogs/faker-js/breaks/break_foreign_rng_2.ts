import { randomBytes } from 'crypto';
import { FakerCore } from '../internal/core';

// Break: crypto.randomBytes in a word generator bypasses the seeded RNG.
export function word(core: FakerCore): string {
  const buf = randomBytes(16);
  const hex = buf.toString('hex');
  return hex.slice(0, 8);
}
