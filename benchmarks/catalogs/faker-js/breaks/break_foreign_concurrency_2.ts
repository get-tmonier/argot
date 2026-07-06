import { spawn, Thread, Worker } from 'threads';
import { ModuleBase } from '../../internal/module-base';

// Break: threads.js worker computes atomic weights off the main thread; faker-js is synchronous.
export class ThreadedScienceModule extends ModuleBase {
  async weight(symbol: string): Promise<number> {
    const worker = await spawn(new Worker('./weight-worker'));
    const value = await worker.compute(symbol);
    await Thread.terminate(worker);
    return value;
  }
}
