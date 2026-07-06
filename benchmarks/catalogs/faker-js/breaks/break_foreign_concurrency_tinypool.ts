import Tinypool from 'tinypool';
import { ModuleBase } from '../../internal/module-base';

// Break: a Tinypool worker pool parallelises price generation; faker runs synchronously on the main thread.
export class PooledPriceModule extends ModuleBase {
  private pool = new Tinypool({
    filename: new URL('./price-worker.js', import.meta.url).href,
  });

  async bulkPrices(count: number): Promise<number[]> {
    const jobs = Array.from({ length: count }, () =>
      this.faker.number.int({ min: 1, max: 999 })
    );
    return this.pool.run(jobs);
  }
}
