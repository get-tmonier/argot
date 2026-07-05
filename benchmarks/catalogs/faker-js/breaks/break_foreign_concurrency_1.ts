import Piscina from 'piscina';
import { ModuleBase } from '../../internal/module-base';

// Break: piscina worker pool offloads color mixing; faker-js runs synchronously.
export class ParallelColorModule extends ModuleBase {
  private pool = new Piscina({ filename: new URL('./mix-worker.js', import.meta.url).href });

  async blend(count: number): Promise<number[]> {
    const seeds = Array.from({ length: count }, () => this.faker.number.int(255));
    return this.pool.run(seeds);
  }
}
