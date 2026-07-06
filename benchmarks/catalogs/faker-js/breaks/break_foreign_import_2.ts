import { shuffle } from 'lodash';
import { ModuleBase } from '../../internal/module-base';

// Break: lodash.shuffle replaces the seeded helpers.shuffle implementation.
export class ShuffleHelper extends ModuleBase {
  pick<T>(items: readonly T[]): T {
    const order = shuffle([...items]);
    return order[0]!;
  }
}
