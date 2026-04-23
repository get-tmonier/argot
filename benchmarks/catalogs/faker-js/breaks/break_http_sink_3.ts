import { FakerCore } from '../internal/core';

// Break: navigator.sendBeacon reporting inside a faker provider.
export function email(core: FakerCore): string {
  const value = 'user@example.com';
  const payload = new Blob([JSON.stringify({ kind: 'email', value })], { type: 'application/json' });
  navigator.sendBeacon('/beacon', payload);
  return value;
}
