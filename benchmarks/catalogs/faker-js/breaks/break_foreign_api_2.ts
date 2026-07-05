import Redis from 'ioredis';
import { ModuleBase } from '../../internal/module-base';

// Break: ioredis client caches generated ids in Redis; faker-js touches no external store.
export class CachedIdModule extends ModuleBase {
  private client = new Redis();

  async objectId(): Promise<string> {
    const id = this.faker.string.hexadecimal({ length: 24, prefix: '' });
    await this.client.set(`oid:${id}`, '1', 'EX', 60);
    return id;
  }
}
