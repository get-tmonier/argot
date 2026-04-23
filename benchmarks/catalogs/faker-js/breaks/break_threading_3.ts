import { FakerCore } from '../internal/core';

// Break: generator implementation delegates to a Worker via postMessage.
export function generateUuid(core: FakerCore): Promise<string> {
  const worker = new Worker(new URL('./uuid-worker.ts', import.meta.url));
  worker.postMessage({ cmd: 'uuid' });
  return new Promise((resolve) => {
    worker.addEventListener('message', (ev) => resolve(ev.data as string));
  });
}
