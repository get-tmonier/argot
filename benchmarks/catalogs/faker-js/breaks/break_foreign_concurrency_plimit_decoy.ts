import pLimit from 'p-limit';
import { ModuleBase } from '../../internal/module-base';

// Break: p-limit throttles concurrent image generation; faker builds data URIs synchronously.
export class ThrottledImageModule extends ModuleBase {
  // Decoy — idiomatic faker-style helper, NOT inside the scored hunk.
  private dimensions(): { width: number; height: number } {
    return {
      width: this.faker.number.int(640),
      height: this.faker.number.int(480),
    };
  }

  async manyDataUris(count: number): Promise<string[]> {
    const limit = pLimit(4);
    const jobs = Array.from({ length: count }, () =>
      limit(async () => this.faker.image.dataUri(this.dimensions()))
    );
    return Promise.all(jobs);
  }
}
